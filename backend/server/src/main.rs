mod job;
mod manga;
mod model;
mod settings;
mod source;
mod source_extractor;
mod state;

use anyhow::Context;
use log::{error, info, warn};
use state::State;
use std::env::current_exe;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use cli::chapter_storage::ChapterStorage;
use cli::database::Database;
use cli::settings::Settings;
use cli::source_manager::SourceManager;
use cli::usecases::{
    fetch_manga_chapter::Error as FetchMangaChaptersError,
    search_mangas::Error as SearchMangasError,
};
use serde::Serialize;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
struct Args {
    home_path: PathBuf,
}

const SOCKET_PATH: &str = "/tmp/rakuyomi.sock";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        "starting rakuyomi, version: {}",
        get_version().unwrap_or_else(|| "unknown".into())
    );

    let args = Args::parse();
    fs::create_dir_all(&args.home_path)
        .with_context(|| "while trying to ensure rakuyomi's home folder exists")?;

    let sources_path = args.home_path.join("sources");
    let database_path = args.home_path.join("database.db");
    let downloads_folder_path = args.home_path.join("downloads");
    let settings_path = args.home_path.join("settings.json");

    let settings = Settings::from_file_or_default(&settings_path)?;
    let source_manager = SourceManager::from_folder(sources_path, settings.clone())?;
    let database = Database::new(&database_path).await?;
    let chapter_storage =
        ChapterStorage::new(downloads_folder_path, settings.storage_size_limit.0)?;

    let state = State {
        source_manager: Arc::new(Mutex::new(source_manager)),
        database: Arc::new(database),
        chapter_storage,
        settings: Arc::new(Mutex::new(settings)),
        settings_path,
        job_state: Default::default(),
    };

    let app = Router::new()
        .route("/health-check", get(health_check))
        .merge(manga::routes())
        .merge(job::routes())
        .merge(settings::routes())
        .merge(source::routes())
        .with_state(state);

    // run our app with hyper, listening on the domain socket
    let _ = std::fs::remove_file(SOCKET_PATH)
        .inspect_err(|e| warn!("could not remove existing socket path: {}", e));
    let listener = tokio::net::UnixListener::bind(SOCKET_PATH).unwrap();

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<()> {
    Json(())
}

fn get_version() -> Option<String> {
    let version_file_path = current_exe().ok()?.with_file_name("VERSION");

    fs::read_to_string(version_file_path).ok()
}

// Make our own error that wraps `anyhow::Error`.
pub enum AppError {
    SourceNotFound,
    DownloadAllChaptersProgressNotFound,
    NetworkFailure(anyhow::Error),
    Other(anyhow::Error),
}

#[derive(Serialize, Clone)]
pub struct ErrorResponse {
    message: String,
}

impl AppError {
    fn from_search_mangas_error(value: SearchMangasError) -> Self {
        match value {
            SearchMangasError::SourceError(e) => Self::NetworkFailure(e),
        }
    }

    fn from_fetch_manga_chapters_error(value: FetchMangaChaptersError) -> Self {
        match value {
            FetchMangaChaptersError::DownloadError(e) => Self::NetworkFailure(e),
            FetchMangaChaptersError::Other(e) => Self::Other(e),
        }
    }
}

impl From<&AppError> for StatusCode {
    fn from(value: &AppError) -> Self {
        match &value {
            AppError::SourceNotFound | AppError::DownloadAllChaptersProgressNotFound => {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<&AppError> for ErrorResponse {
    fn from(value: &AppError) -> Self {
        let message = match value {
            AppError::SourceNotFound => "Source was not found".to_string(),
            AppError::DownloadAllChaptersProgressNotFound => {
                "No download is in progress.".to_string()
            }
            AppError::NetworkFailure(_) => {
                "There was a network error. Check your connection and try again.".to_string()
            }
            AppError::Other(ref e) => format!("Something went wrong: {}", e),
        };

        Self { message }
    }
}

impl From<AppError> for ErrorResponse {
    fn from(value: AppError) -> Self {
        Self::from(&value)
    }
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = StatusCode::from(&self);
        let error_response = ErrorResponse::from(&self);

        let inner_exception = match self {
            Self::NetworkFailure(ref e) => Some(e),
            Self::Other(ref e) => Some(e),
            _ => None,
        };

        if let Some(e) = inner_exception {
            error!("Error caused by: {:?}", e);
        }

        (status_code, Json(error_response)).into_response()
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
