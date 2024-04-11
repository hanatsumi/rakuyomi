mod model;
mod source_extractor;

use log::error;
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
use cli::model::{ChapterId, MangaId, SourceId};
use cli::settings::Settings;
use cli::source_manager::SourceManager;
use cli::usecases::fetch_all_manga_chapters::ProgressReport;
use cli::usecases::{
    add_manga_to_library::add_manga_to_library as add_manga_to_library_usecase,
    fetch_all_manga_chapters::fetch_all_manga_chapters,
    fetch_all_manga_chapters::Error as FetchAllMangaChaptersError,
    fetch_manga_chapter::fetch_manga_chapter,
    fetch_manga_chapter::Error as FetchMangaChaptersError,
    get_cached_manga_chapters::get_cached_manga_chapters as get_cached_manga_chapters_usecase,
    get_manga_library::get_manga_library as get_manga_library_usecase,
    install_source::install_source as install_source_usecase,
    list_available_sources::list_available_sources as list_available_sources_usecase,
    list_installed_sources::list_installed_sources as list_installed_sources_usecase,
    mark_chapter_as_read::mark_chapter_as_read as mark_chapter_as_read_usecase,
    refresh_manga_chapters::refresh_manga_chapters as refresh_manga_chapters_usecase,
    search_mangas::search_mangas, search_mangas::Error as SearchMangasError,
};
use futures::{pin_mut, StreamExt};
use serde::{Deserialize, Serialize};
use source_extractor::SourceExtractor;
use tokio::sync::Mutex;

use model::{
    Chapter, DownloadAllChaptersProgress, Manga, SourceInformation, SourceMangaSearchResults,
};
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
struct Args {
    home_path: PathBuf,
}

#[derive(Default)]
enum DownloadAllChaptersState {
    #[default]
    Idle,
    Initializing,
    Progressing {
        cancellation_token: CancellationToken,
        downloaded: usize,
        total: usize,
    },
    Finished,
    Cancelled,
    Errored(FetchAllMangaChaptersError),
}

#[derive(Clone)]
struct State {
    source_manager: Arc<Mutex<SourceManager>>,
    database: Arc<Database>,
    chapter_storage: ChapterStorage,
    download_all_chapters_state: Arc<Mutex<DownloadAllChaptersState>>,
    settings: Settings,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let sources_path = args.home_path.join("sources");
    let database_path = args.home_path.join("database.db");
    let downloads_folder_path = args.home_path.join("downloads");
    let settings_path = args.home_path.join("settings.json");

    let source_manager = SourceManager::from_folder(sources_path)?;
    let database = Database::new(&database_path).await?;
    let chapter_storage = ChapterStorage::new(downloads_folder_path);
    let settings = Settings::from_file_or_default(&settings_path)?;

    let state = State {
        source_manager: Arc::new(Mutex::new(source_manager)),
        database: Arc::new(database),
        chapter_storage,
        settings,
        download_all_chapters_state: Default::default(),
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
            get(get_cached_manga_chapters),
        )
        .route(
            "/mangas/:source_id/:manga_id/refresh-chapters",
            post(refresh_manga_chapters),
        )
        .route(
            "/mangas/:source_id/:manga_id/chapters/download-all",
            post(download_all_manga_chapters),
        )
        .route(
            "/mangas/:source_id/:manga_id/chapters/cancel-download-all",
            post(cancel_download_all_manga_chapters),
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
        .route("/available-sources", get(list_available_sources))
        .route(
            "/available-sources/:source_id/install",
            post(install_source),
        )
        .route("/installed-sources", get(list_installed_sources))
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
        .map(Manga::from)
        .collect();

    Ok(Json(mangas))
}

#[derive(Deserialize)]
struct GetMangasQuery {
    q: String,
}

async fn get_mangas(
    StateExtractor(State {
        database,
        source_manager,
        ..
    }): StateExtractor<State>,
    Query(GetMangasQuery { q }): Query<GetMangasQuery>,
) -> Result<Json<Vec<SourceMangaSearchResults>>, AppError> {
    let results = search_mangas(&*source_manager.lock().await, &database, q)
        .await
        .map_err(AppError::from_search_mangas_error)?
        .into_iter()
        .map(SourceMangaSearchResults::from)
        .collect();

    Ok(Json(results))
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
    SourceExtractor(_source): SourceExtractor,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);

    add_manga_to_library_usecase(&database, manga_id).await?;

    Ok(Json(()))
}

async fn get_cached_manga_chapters(
    StateExtractor(State {
        database,
        chapter_storage,
        ..
    }): StateExtractor<State>,
    SourceExtractor(_source): SourceExtractor,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<Vec<Chapter>>, AppError> {
    let manga_id = MangaId::from(params);
    let chapters = get_cached_manga_chapters_usecase(&database, &chapter_storage, manga_id).await?;

    let chapters = chapters.into_iter().map(Chapter::from).collect();

    Ok(Json(chapters))
}

async fn refresh_manga_chapters(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    SourceExtractor(source): SourceExtractor,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);
    refresh_manga_chapters_usecase(&database, &source, manga_id).await?;

    Ok(Json(()))
}

async fn download_all_manga_chapters(
    StateExtractor(State {
        database,
        chapter_storage,
        download_all_chapters_state,
        ..
    }): StateExtractor<State>,
    SourceExtractor(source): SourceExtractor,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);

    *download_all_chapters_state.lock().await = DownloadAllChaptersState::Initializing;

    tokio::spawn(async move {
        let source = &source;
        let (cancellation_token, progress_report_stream) =
            fetch_all_manga_chapters(source, &database, &chapter_storage, manga_id);

        *download_all_chapters_state.lock().await = DownloadAllChaptersState::Initializing;

        pin_mut!(progress_report_stream);

        let mut terminated = false;
        while !terminated {
            let progress_report = progress_report_stream.next().await.unwrap();
            terminated = matches!(
                &progress_report,
                ProgressReport::Finished | ProgressReport::Errored(_) | ProgressReport::Cancelled
            );

            *download_all_chapters_state.lock().await = match progress_report {
                ProgressReport::Progressing { downloaded, total } => {
                    DownloadAllChaptersState::Progressing {
                        cancellation_token: cancellation_token.clone(),
                        downloaded,
                        total,
                    }
                }
                ProgressReport::Finished => DownloadAllChaptersState::Finished,
                ProgressReport::Errored(e) => DownloadAllChaptersState::Errored(e),
                ProgressReport::Cancelled => DownloadAllChaptersState::Cancelled,
            };
        }
    });

    Ok(Json(()))
}

async fn cancel_download_all_manga_chapters(
    StateExtractor(State {
        download_all_chapters_state,
        ..
    }): StateExtractor<State>,
) -> Result<Json<()>, StatusCode> {
    match &*download_all_chapters_state.lock().await {
        DownloadAllChaptersState::Progressing {
            cancellation_token, ..
        } => {
            cancellation_token.cancel();

            Ok(Json(()))
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_download_all_manga_chapters_progress(
    StateExtractor(State {
        download_all_chapters_state,
        ..
    }): StateExtractor<State>,
) -> Result<Json<DownloadAllChaptersProgress>, AppError> {
    let mut state_lock = download_all_chapters_state.lock().await;
    let state = mem::take(&mut *state_lock);

    // iff we're not on a terminal state, place a copy of the state into the app state
    match &state {
        DownloadAllChaptersState::Idle => {
            *state_lock = DownloadAllChaptersState::Idle;
        }
        DownloadAllChaptersState::Initializing => {
            *state_lock = DownloadAllChaptersState::Initializing;
        }
        DownloadAllChaptersState::Progressing {
            cancellation_token,
            downloaded,
            total,
        } => {
            *state_lock = DownloadAllChaptersState::Progressing {
                cancellation_token: cancellation_token.clone(),
                downloaded: downloaded.to_owned(),
                total: total.to_owned(),
            };
        }
        _ => {}
    };

    let download_progress = match state {
        DownloadAllChaptersState::Idle => {
            return Err(AppError::DownloadAllChaptersProgressNotFound)
        }
        DownloadAllChaptersState::Initializing => DownloadAllChaptersProgress::Initializing,
        DownloadAllChaptersState::Progressing {
            downloaded, total, ..
        } => DownloadAllChaptersProgress::Progressing { downloaded, total },
        DownloadAllChaptersState::Finished => DownloadAllChaptersProgress::Finished,
        DownloadAllChaptersState::Errored(e) => {
            return Err(AppError::from_fetch_all_manga_chapters_error(e))
        }
        DownloadAllChaptersState::Cancelled => DownloadAllChaptersProgress::Cancelled,
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
        chapter_storage, ..
    }): StateExtractor<State>,
    SourceExtractor(source): SourceExtractor,
    Path(params): Path<DownloadMangaChapterParams>,
) -> Result<Json<String>, AppError> {
    let chapter_id = ChapterId::from(params);
    let output_path = fetch_manga_chapter(&source, &chapter_storage, &chapter_id)
        .await
        .map_err(AppError::from_fetch_manga_chapters_error)?;

    Ok(Json(output_path.to_string_lossy().into()))
}

async fn mark_chapter_as_read(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    SourceExtractor(_source): SourceExtractor,
    Path(params): Path<DownloadMangaChapterParams>,
) -> Json<()> {
    let chapter_id = ChapterId::from(params);

    mark_chapter_as_read_usecase(&database, chapter_id).await;

    Json(())
}

async fn list_available_sources(
    StateExtractor(State { settings, .. }): StateExtractor<State>,
) -> Result<Json<Vec<SourceInformation>>, AppError> {
    let available_sources = list_available_sources_usecase(settings.source_lists)
        .await?
        .into_iter()
        .map(SourceInformation::from)
        .collect();

    Ok(Json(available_sources))
}

#[derive(Deserialize)]
struct InstallSourceParams {
    source_id: String,
}

async fn install_source(
    StateExtractor(State {
        source_manager,
        settings,
        ..
    }): StateExtractor<State>,
    Path(InstallSourceParams { source_id }): Path<InstallSourceParams>,
) -> Result<Json<()>, AppError> {
    install_source_usecase(
        &mut *source_manager.lock().await,
        &settings.source_lists,
        SourceId::new(source_id),
    )
    .await?;

    Ok(Json(()))
}

async fn list_installed_sources(
    StateExtractor(State { source_manager, .. }): StateExtractor<State>,
) -> Json<Vec<SourceInformation>> {
    let installed_sources = list_installed_sources_usecase(&*source_manager.lock().await)
        .into_iter()
        .map(SourceInformation::from)
        .collect();

    Json(installed_sources)
}

// Make our own error that wraps `anyhow::Error`.
enum AppError {
    SourceNotFound,
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
            Self::SourceNotFound | Self::DownloadAllChaptersProgressNotFound => {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = match self {
            Self::SourceNotFound => "Source was not found".to_string(),
            Self::DownloadAllChaptersProgressNotFound => "No download is in progress.".to_string(),
            Self::NetworkFailure(_) => {
                "There was a network error. Check your connection and try again.".to_string()
            }
            Self::Other(ref e) => format!("Something went wrong: {}", e),
        };

        let inner_exception = match self {
            Self::NetworkFailure(ref e) => Some(e),
            Self::Other(ref e) => Some(e),
            _ => None,
        };

        if let Some(e) = inner_exception {
            error!("Error caused by: {:?}", e);
        }

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
