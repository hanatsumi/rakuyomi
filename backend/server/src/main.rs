mod model;

use std::mem;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{extract::State as StateExtractor, Json, Router};
use clap::Parser;
use cli::chapter_storage::ChapterStorage;
use cli::database::Database;
use cli::model::{ChapterId, MangaId};
use cli::source::Source;
use cli::usecases::fetch_all_manga_chapters::ProgressReport;
use cli::usecases::{
    add_manga_to_library::add_manga_to_library as add_manga_to_library_usecase,
    fetch_all_manga_chapters::fetch_all_manga_chapters,
    fetch_all_manga_chapters::Error as FetchAllMangaChaptersError,
    fetch_manga_chapter::fetch_manga_chapter,
    fetch_manga_chapter::Error as FetchMangaChaptersError,
    get_manga_chapters::get_manga_chapters as get_manga_chapters_usecase,
    get_manga_chapters::Response as GetMangaChaptersUsecaseResponse,
    get_manga_library::get_manga_library as get_manga_library_usecase,
    mark_chapter_as_read::mark_chapter_as_read as mark_chapter_as_read_usecase,
    search_mangas::search_mangas, search_mangas::Error as SearchMangasError,
};
use futures::{pin_mut, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use model::{Chapter, DownloadAllChaptersProgress, Manga};

#[derive(Parser, Debug)]
struct Args {
    home_path: PathBuf,
}

#[derive(Clone)]
struct State {
    source: Arc<Mutex<Source>>,
    database: Arc<Database>,
    chapter_storage: ChapterStorage,
    fetch_all_chapters_progress_report: Arc<Mutex<Option<ProgressReport>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let source_path = args.home_path.join("sources/source.aix");
    let database_path = args.home_path.join("database.db");
    let downloads_folder_path = args.home_path.join("downloads");

    let source = Source::from_aix_file(&source_path)?;
    let database = Database::new(&database_path).await?;
    let chapter_storage = ChapterStorage::new(downloads_folder_path);

    let state = State {
        source: Arc::new(Mutex::new(source)),
        database: Arc::new(database),
        chapter_storage,
        fetch_all_chapters_progress_report: Default::default(),
    };

    let app = Router::new()
        .route("/library", get(get_manga_library))
        .route("/mangas", get(get_mangas))
        .route(
            "/mangas/:source_id/:manga_id/add-to-library",
            post(add_manga_to_library),
        )
        .route(
            "/mangas/:source_id/:manga_id/chapters",
            get(get_manga_chapters),
        )
        .route(
            "/mangas/:source_id/:manga_id/chapters/download-all",
            post(download_all_manga_chapters),
        )
        .route(
            "/mangas/:source_id/:manga_id/chapters/download-all-progress",
            get(get_download_all_manga_chapters_progress),
        )
        // FIXME i dont think the route should be named download because it doesnt
        // always download the file...
        .route(
            "/mangas/:source_id/:manga_id/chapters/:chapter_id/download",
            post(download_manga_chapter),
        )
        .route(
            "/mangas/:source_id/:manga_id/chapters/:chapter_id/mark-as-read",
            post(mark_chapter_as_read),
        )
        .with_state(state);

    // run our app with hyper, listening globally on port 30727
    let listener = tokio::net::TcpListener::bind("0.0.0.0:30727")
        .await
        .unwrap();

    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_manga_library(
    StateExtractor(State { database, .. }): StateExtractor<State>,
) -> Result<Json<Vec<Manga>>, AppError> {
    let mangas = get_manga_library_usecase(&database)
        .await?
        .into_iter()
        .map(|source_manga| Manga::from(source_manga))
        .collect();

    Ok(Json(mangas))
}

#[derive(Deserialize)]
struct GetMangasQuery {
    q: String,
}

async fn get_mangas(
    StateExtractor(State {
        source, database, ..
    }): StateExtractor<State>,
    Query(GetMangasQuery { q }): Query<GetMangasQuery>,
) -> Result<Json<Vec<Manga>>, AppError> {
    let mangas = search_mangas(&*source.lock().await, &database, q)
        .await
        .map_err(AppError::from_search_mangas_error)?
        .into_iter()
        .map(|source_manga| Manga::from(source_manga))
        .collect();

    Ok(Json(mangas))
}

#[derive(Deserialize)]
struct MangaChaptersPathParams {
    source_id: String,
    manga_id: String,
}

impl From<MangaChaptersPathParams> for MangaId {
    fn from(value: MangaChaptersPathParams) -> Self {
        MangaId::from_strings(value.source_id, value.manga_id)
    }
}

async fn add_manga_to_library(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);

    add_manga_to_library_usecase(&database, manga_id).await?;

    Ok(Json(()))
}

async fn get_manga_chapters(
    StateExtractor(State {
        source,
        database,
        chapter_storage,
        ..
    }): StateExtractor<State>,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<Vec<Chapter>>, AppError> {
    let manga_id = MangaId::from(params);
    let GetMangaChaptersUsecaseResponse(_, chapters) =
        get_manga_chapters_usecase(&database, &*source.lock().await, &chapter_storage, manga_id)
            .await?;

    let chapters = chapters
        .into_iter()
        .map(|domain_chapter| Chapter::from(domain_chapter))
        .collect();

    Ok(Json(chapters))
}

async fn download_all_manga_chapters(
    StateExtractor(State {
        source,
        database,
        chapter_storage,
        fetch_all_chapters_progress_report,
        ..
    }): StateExtractor<State>,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);

    *fetch_all_chapters_progress_report.lock().await = Some(ProgressReport::Initializing);

    tokio::spawn(async move {
        let source = &*source.lock().await;
        let progress_report_stream =
            fetch_all_manga_chapters(&source, &database, &chapter_storage, manga_id);

        pin_mut!(progress_report_stream);

        let mut terminated = false;
        while !terminated {
            let progress_report = progress_report_stream.next().await.unwrap();
            terminated = match &progress_report {
                ProgressReport::Finished | ProgressReport::Errored(_) => true,
                _ => false,
            };

            *fetch_all_chapters_progress_report.lock().await = Some(progress_report);
        }
    });

    Ok(Json(()))
}

async fn get_download_all_manga_chapters_progress(
    StateExtractor(State {
        fetch_all_chapters_progress_report,
        ..
    }): StateExtractor<State>,
) -> Result<Json<DownloadAllChaptersProgress>, AppError> {
    let mut maybe_progress_report = fetch_all_chapters_progress_report.lock().await;
    let progress_report = mem::take(&mut *maybe_progress_report)
        .ok_or(AppError::DownloadAllChaptersProgressNotFound)?;

    // iff we're not on a terminal state, place a copy of the progress report back into the state
    match &progress_report {
        ProgressReport::Initializing => {
            *maybe_progress_report = Some(ProgressReport::Initializing);
        }
        ProgressReport::Progressing { downloaded, total } => {
            *maybe_progress_report = Some(ProgressReport::Progressing {
                downloaded: *downloaded,
                total: *total,
            })
        }
        _ => {}
    };

    let download_progress = match progress_report {
        ProgressReport::Initializing => DownloadAllChaptersProgress::Initializing,
        ProgressReport::Progressing { downloaded, total } => {
            DownloadAllChaptersProgress::Progressing {
                downloaded: downloaded.to_owned(),
                total: total.to_owned(),
            }
        }
        ProgressReport::Finished => DownloadAllChaptersProgress::Finished,
        ProgressReport::Errored(e) => return Err(AppError::from_fetch_all_manga_chapters_error(e)),
    };

    Ok(Json(download_progress))
}

#[derive(Deserialize)]
struct DownloadMangaChapterParams {
    source_id: String,
    manga_id: String,
    chapter_id: String,
}

impl From<DownloadMangaChapterParams> for ChapterId {
    fn from(value: DownloadMangaChapterParams) -> Self {
        ChapterId::from_strings(value.source_id, value.manga_id, value.chapter_id)
    }
}

async fn download_manga_chapter(
    StateExtractor(State {
        source,
        chapter_storage,
        ..
    }): StateExtractor<State>,
    Path(params): Path<DownloadMangaChapterParams>,
) -> Result<Json<String>, AppError> {
    let chapter_id = ChapterId::from(params);
    let output_path = fetch_manga_chapter(&*source.lock().await, &chapter_storage, &chapter_id)
        .await
        .map_err(AppError::from_fetch_manga_chapters_error)?;

    Ok(Json(output_path.to_string_lossy().into()))
}

async fn mark_chapter_as_read(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    Path(params): Path<DownloadMangaChapterParams>,
) -> Json<()> {
    let chapter_id = ChapterId::from(params);

    mark_chapter_as_read_usecase(&database, chapter_id).await;

    Json(())
}

// Make our own error that wraps `anyhow::Error`.
enum AppError {
    DownloadAllChaptersProgressNotFound,
    NetworkFailure(anyhow::Error),
    Other(anyhow::Error),
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

impl AppError {
    fn from_search_mangas_error(value: SearchMangasError) -> Self {
        match value {
            SearchMangasError::SourceError(e) => Self::NetworkFailure(e),
        }
    }

    fn from_fetch_all_manga_chapters_error(value: FetchAllMangaChaptersError) -> Self {
        match value {
            FetchAllMangaChaptersError::DownloadError(e) => Self::NetworkFailure(e),
            FetchAllMangaChaptersError::Other(e) => Self::Other(e),
        }
    }

    fn from_fetch_manga_chapters_error(value: FetchMangaChaptersError) -> Self {
        match value {
            FetchMangaChaptersError::DownloadError(e) => Self::NetworkFailure(e),
            FetchMangaChaptersError::Other(e) => Self::Other(e),
        }
    }
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = match &self {
            Self::DownloadAllChaptersProgressNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = match self {
            Self::DownloadAllChaptersProgressNotFound => "No download is in progress.".to_string(),
            Self::NetworkFailure(_) => {
                "There was a network error. Check your connection and try again.".to_string()
            }
            Self::Other(e) => format!("Something went wrong: {}", e),
        };

        (status_code, Json(ErrorResponse { message })).into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Other(err.into())
    }
}
