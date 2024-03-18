mod model;

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
use cli::source::Source;
use cli::usecases::{
    add_manga_to_library::add_manga_to_library as add_manga_to_library_usecase,
    fetch_all_manga_chapters::fetch_all_manga_chapters,
    fetch_all_manga_chapters::Error as FetchAllMangaChaptersError,
    fetch_manga_chapter::fetch_manga_chapter,
    fetch_manga_chapter::Error as FetchMangaChaptersError,
    get_manga_chapters::get_manga_chapters as get_manga_chapters_usecase,
    get_manga_chapters::Response as GetMangaChaptersUsecaseResponse,
    get_manga_library::get_manga_library as get_manga_library_usecase,
    search_mangas::search_mangas, search_mangas::Error as SearchMangasError,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use model::{Chapter, Manga};

#[derive(Parser, Debug)]
struct Args {
    home_path: PathBuf,
}

#[derive(Clone)]
struct State {
    source: Arc<Mutex<Source>>,
    database: Arc<Database>,
    chapter_storage: ChapterStorage,
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
        // FIXME i dont think the route should be named download because it doesnt
        // always download the file...
        .route(
            "/mangas/:source_id/:manga_id/chapters/:chapter_id/download",
            post(download_manga_chapter),
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

async fn add_manga_to_library(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    Path(MangaChaptersPathParams {
        source_id,
        manga_id,
    }): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId {
        source_id: SourceId(source_id),
        manga_id,
    };

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
    Path(MangaChaptersPathParams {
        source_id,
        manga_id,
    }): Path<MangaChaptersPathParams>,
) -> Result<Json<Vec<Chapter>>, AppError> {
    let manga_id = MangaId {
        source_id: SourceId(source_id),
        manga_id,
    };
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
        ..
    }): StateExtractor<State>,
    Path(MangaChaptersPathParams {
        source_id,
        manga_id,
    }): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId {
        source_id: SourceId(source_id),
        manga_id,
    };

    fetch_all_manga_chapters(&*source.lock().await, &database, &chapter_storage, manga_id)
        .await
        .map_err(AppError::from_fetch_all_manga_chapters_error)?;

    Ok(Json(()))
}

#[derive(Deserialize)]
struct DownloadMangaChapterParams {
    source_id: String,
    manga_id: String,
    chapter_id: String,
}

async fn download_manga_chapter(
    StateExtractor(State {
        source,
        chapter_storage,
        ..
    }): StateExtractor<State>,
    Path(DownloadMangaChapterParams {
        source_id,
        manga_id,
        chapter_id,
    }): Path<DownloadMangaChapterParams>,
) -> Result<Json<String>, AppError> {
    let chapter_id = ChapterId {
        manga_id: MangaId {
            manga_id,
            source_id: SourceId(source_id),
        },
        chapter_id,
    };
    let output_path = fetch_manga_chapter(&*source.lock().await, &chapter_storage, &chapter_id)
        .await
        .map_err(AppError::from_fetch_manga_chapters_error)?;

    Ok(Json(output_path.to_string_lossy().into()))
}

// Make our own error that wraps `anyhow::Error`.
enum AppError {
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
        let message = match self {
            Self::NetworkFailure(_) => {
                "There was a network error. Check your connection and try again.".to_string()
            }
            Self::Other(e) => format!("Something went wrong: {}", e),
        };

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { message }),
        )
            .into_response()
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
