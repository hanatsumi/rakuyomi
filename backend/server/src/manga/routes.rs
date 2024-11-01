use std::time::Duration;

use axum::extract::{Path, Query, State as StateExtractor};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use cli::model::{ChapterId, MangaId};
use cli::usecases;
use cli::usecases::fetch_all_manga_chapters::ProgressReport;
use futures::{pin_mut, Future, StreamExt};
use log::warn;
use serde::Deserialize;
use std::mem;
use tokio_util::sync::CancellationToken;

use crate::model::{Chapter, DownloadAllChaptersProgress, Manga};
use crate::source_extractor::SourceExtractor;
use crate::state::{DownloadAllChaptersState, State};
use crate::AppError;

pub fn routes() -> Router<State> {
    Router::new()
        .route("/library", get(get_manga_library))
        .route("/mangas", get(get_mangas))
        .route(
            "/mangas/:source_id/:manga_id/add-to-library",
            post(add_manga_to_library),
        )
        .route(
            "/mangas/:source_id/:manga_id/remove-from-library",
            post(remove_manga_from_library),
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
) -> Result<Json<Vec<Manga>>, AppError> {
    let source_manager = &*source_manager.lock().await;
    let results = cancel_after(Duration::from_secs(15), |token| {
        usecases::search_mangas(source_manager, &database, token, q)
    })
    .await
    .map_err(AppError::from_search_mangas_error)?
    .into_iter()
    .map(Manga::from)
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

    usecases::add_manga_to_library(&database, manga_id).await?;

    Ok(Json(()))
}

async fn remove_manga_from_library(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    SourceExtractor(_source): SourceExtractor,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);

    usecases::remove_manga_from_library(&database, manga_id).await?;

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
    let chapters =
        usecases::get_cached_manga_chapters(&database, &chapter_storage, manga_id).await?;

    let chapters = chapters.into_iter().map(Chapter::from).collect();

    Ok(Json(chapters))
}

async fn refresh_manga_chapters(
    StateExtractor(State { database, .. }): StateExtractor<State>,
    SourceExtractor(source): SourceExtractor,
    Path(params): Path<MangaChaptersPathParams>,
) -> Result<Json<()>, AppError> {
    let manga_id = MangaId::from(params);
    usecases::refresh_manga_chapters(&database, &source, manga_id).await?;

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
            usecases::fetch_all_manga_chapters(source, &database, &chapter_storage, manga_id);

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
    let output_path = usecases::fetch_manga_chapter(&source, &chapter_storage, &chapter_id)
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

    usecases::mark_chapter_as_read(&database, chapter_id).await;

    Json(())
}

async fn cancel_after<F, Fut>(duration: Duration, f: F) -> Fut::Output
where
    Fut: Future,
    F: FnOnce(CancellationToken) -> Fut,
{
    let token = CancellationToken::new();
    let future = f(token.clone());

    let request_cancellation_handle = tokio::spawn(async move {
        tokio::time::sleep(duration).await;

        warn!("cancellation requested!");
        token.cancel();
    });

    let result = future.await;

    request_cancellation_handle.abort();

    result
}
