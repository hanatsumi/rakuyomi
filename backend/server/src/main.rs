mod model;

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{extract::State as StateExtractor, Json, Router};
use clap::Parser;
use cli::database::Database;
use cli::model::{ChapterId, MangaId, MangaInformation, SourceId};
use cli::source::Source;
use cli::usecases::fetch_manga_chapter::fetch_manga_chapter;
use cli::usecases::{
    get_manga_chapters::get_manga_chapters as get_manga_chapters_usecase,
    get_manga_chapters::Response as GetMangaChaptersUsecaseResponse, search_mangas::search_mangas,
};
use serde::Deserialize;
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
    downloads_folder_path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let source_path = args.home_path.join("sources/source.aix");
    let database_path = args.home_path.join("database.db");
    let downloads_folder_path = args.home_path.join("downloads");

    let source = Source::from_aix_file(&source_path)?;
    let database = Database::new(&database_path).await?;

    let state = State {
        source: Arc::new(Mutex::new(source)),
        database: Arc::new(database),
        downloads_folder_path,
    };

    let app = Router::new()
        .route("/mangas", get(get_mangas))
        .route(
            "/mangas/:source_id/:manga_id/chapters",
            get(get_manga_chapters),
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
        .await?
        .into_iter()
        .map(|source_manga| Manga::from(source_manga))
        .collect();

    Ok(Json(mangas))
}

#[derive(Deserialize)]
struct GetMangaChaptersParams {
    source_id: String,
    manga_id: String,
}

async fn get_manga_chapters(
    StateExtractor(State {
        source, database, ..
    }): StateExtractor<State>,
    Path(GetMangaChaptersParams {
        source_id,
        manga_id,
    }): Path<GetMangaChaptersParams>,
) -> Result<Json<Vec<Chapter>>, AppError> {
    let manga_id = MangaId {
        source_id: SourceId(source_id),
        manga_id,
    };
    let GetMangaChaptersUsecaseResponse(_, chapters) =
        get_manga_chapters_usecase(&database, &*source.lock().await, manga_id).await?;

    let chapters = chapters
        .into_iter()
        .map(|(source_chapter, _)| Chapter::from(source_chapter))
        .collect();

    Ok(Json(chapters))
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
        downloads_folder_path,
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
    let output_path =
        fetch_manga_chapter(&*source.lock().await, &downloads_folder_path, &chapter_id).await?;

    Ok(Json(output_path.to_string_lossy().into()))
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
