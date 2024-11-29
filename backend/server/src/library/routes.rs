use axum::extract::{Path, State as StateExtractor};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use cli::model::MangaId;
use cli::usecases;
use serde::Deserialize;

use crate::model::Manga;
use crate::source_extractor::SourceExtractor;
use crate::state::State;
use crate::AppError;

pub fn routes() -> Router<State> {
    Router::new()
        .route("/library", get(get_manga_library))
        .route("/library/:source_id/:manga_id", post(add_manga_to_library))
        .route(
            "/library/:source_id/:manga_id",
            delete(remove_manga_from_library),
        )
}

#[derive(Deserialize)]
struct LibraryEntryPathParams {
    source_id: String,
    manga_id: String,
}

async fn get_manga_library(
    StateExtractor(State {
        database,
        source_manager,
        ..
    }): StateExtractor<State>,
) -> Result<Json<Vec<Manga>>, AppError> {
    let mangas = usecases::get_manga_library(&database, &*source_manager.lock().await)
        .await?
        .into_iter()
        .map(Manga::from)
        .collect();

    Ok(Json(mangas))
}

async fn add_manga_to_library(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    SourceExtractor(_source): SourceExtractor,
    Path(params): Path<LibraryEntryPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);

    usecases::add_manga_to_library(&database, manga_id).await?;

    Ok(Json(()))
}

async fn remove_manga_from_library(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    SourceExtractor(_source): SourceExtractor,
    Path(params): Path<LibraryEntryPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);

    usecases::remove_manga_from_library(&database, manga_id).await?;

    Ok(Json(()))
}

impl From<LibraryEntryPathParams> for MangaId {
    fn from(value: LibraryEntryPathParams) -> Self {
        MangaId::from_strings(value.source_id, value.manga_id)
    }
}
