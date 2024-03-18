mod model;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{extract::State as StateExtractor, Json, Router};
use clap::Parser;
use cli::chapter_downloader::download_chapter_pages_as_cbz;
use cli::source::Source;
use serde::Deserialize;
use tokio::task::spawn_blocking;

use model::{Chapter, Manga};

#[derive(Parser, Debug)]
struct Args {
    sources_path: PathBuf,
}

#[derive(Clone)]
struct State {
    source: Arc<Mutex<Source>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let source_path = args.sources_path.join("source.aix");
    let source = Source::from_aix_file(&source_path)?;
    let state = State {
        source: Arc::new(Mutex::new(source)),
    };

    let app = Router::new()
        .route("/mangas", get(get_mangas))
        .route(
            "/mangas/:source_id/:manga_id/chapters",
            get(get_manga_chapters),
        )
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

#[derive(Deserialize)]
struct GetMangasQuery {
    q: String,
}

async fn get_mangas(
    StateExtractor(State { source }): StateExtractor<State>,
    Query(GetMangasQuery { q }): Query<GetMangasQuery>,
) -> Result<Json<Vec<Manga>>, AppError> {
    // HACK we need to call these inside `spawn_blocking` because otherwise reqwest::blocking (used inside
    // the `net` import in Aidoku) panics...
    // i dont think we'll ever be able to make the WASM imports async, so maybe there's no way around it,
    // or maybe we could move this call inside the `Source` itself and make it asynchronous
    let mangas = spawn_blocking(move || -> Result<Vec<Manga>, AppError> {
        Ok(source
            .lock()
            .unwrap()
            .search_mangas(q)?
            .into_iter()
            .map(|source_manga| Manga::from(source_manga))
            .collect())
    })
    .await??;

    Ok(Json(mangas))
}

#[derive(Deserialize)]
struct GetMangaChaptersParams {
    source_id: String,
    manga_id: String,
}

async fn get_manga_chapters(
    StateExtractor(State { source }): StateExtractor<State>,
    Path(GetMangaChaptersParams { manga_id, .. }): Path<GetMangaChaptersParams>,
) -> Result<Json<Vec<Chapter>>, AppError> {
    let chapters = spawn_blocking(move || -> Result<Vec<Chapter>, AppError> {
        Ok(source
            .lock()
            .unwrap()
            .get_chapter_list(manga_id)?
            .into_iter()
            .map(|source_chapter| Chapter::from(source_chapter))
            .collect())
    })
    .await??;

    Ok(Json(chapters))
}

#[derive(Deserialize)]
struct DownloadMangaChapterParams {
    source_id: String,
    manga_id: String,
    chapter_id: String,
}

#[derive(Deserialize)]
struct DownloadMangaChapterBody {
    output_path: PathBuf,
}

async fn download_manga_chapter(
    StateExtractor(State { source }): StateExtractor<State>,
    Path(DownloadMangaChapterParams {
        manga_id,
        chapter_id,
        ..
    }): Path<DownloadMangaChapterParams>,
    Json(DownloadMangaChapterBody { output_path }): Json<DownloadMangaChapterBody>,
) -> Result<Json<()>, AppError> {
    let pages = spawn_blocking(move || source.lock().unwrap().get_page_list(manga_id, chapter_id))
        .await??;

    spawn_blocking(move || -> Result<(), anyhow::Error> {
        let output_file = fs::File::create(output_path)?;
        // FIXME we could make this async
        download_chapter_pages_as_cbz(output_file, pages)?;

        Ok(())
    })
    .await??;

    Ok(Json(()))
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
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
        Self(err.into())
    }
}