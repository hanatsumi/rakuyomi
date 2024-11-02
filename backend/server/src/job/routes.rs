use std::{path::PathBuf, sync::Arc};

use crate::{job::State, AppError};
use anyhow::anyhow;
use axum::{
    extract::{Path, State as StateExtractor},
    routing::{get, post},
    Json, Router,
};
use cli::{
    chapter_storage::ChapterStorage, model::ChapterId, source_collection::SourceCollection,
    source_manager::SourceManager, usecases,
};
use serde::Deserialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::job::dto::JobDetail;
use crate::job::state::Job;
use crate::state::State as AppState;

pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/jobs/download-chapter", post(create_download_chapter_job))
        .route("/jobs/download-chapter/:id", get(get_download_chapter_job))
}

#[derive(Deserialize)]
struct CreateDownloadChapterJobBody {
    source_id: String,
    manga_id: String,
    chapter_id: String,
}

impl From<CreateDownloadChapterJobBody> for ChapterId {
    fn from(value: CreateDownloadChapterJobBody) -> Self {
        ChapterId::from_strings(value.source_id, value.manga_id, value.chapter_id)
    }
}

async fn create_download_chapter_job(
    StateExtractor(AppState {
        source_manager,
        chapter_storage,
        ..
    }): StateExtractor<AppState>,
    StateExtractor(State { job_registry }): StateExtractor<State>,
    Json(body): Json<CreateDownloadChapterJobBody>,
) -> Result<Json<Uuid>, AppError> {
    let id = Uuid::new_v4();
    let job = tokio::spawn(download_chapter_job(
        source_manager,
        chapter_storage,
        body.into(),
    ));

    job_registry.lock().await.insert(id, Job::FetchChapter(job));

    Ok(Json(id))
}

async fn download_chapter_job(
    source_manager: Arc<Mutex<SourceManager>>,
    chapter_storage: ChapterStorage,
    chapter_id: ChapterId,
) -> Result<PathBuf, AppError> {
    let source_manager = source_manager.lock().await;
    let source = source_manager
        .get_by_id(chapter_id.source_id())
        .ok_or(AppError::SourceNotFound)?;

    Ok(usecases::fetch_manga_chapter(source, &chapter_storage, &chapter_id).await?)
}

#[derive(Deserialize)]
struct GetDownloadChapterJobParams {
    id: Uuid,
}

async fn get_download_chapter_job(
    StateExtractor(State { job_registry }): StateExtractor<State>,
    Path(GetDownloadChapterJobParams { id }): Path<GetDownloadChapterJobParams>,
) -> Result<Json<JobDetail>, AppError> {
    let job = job_registry
        .lock()
        .await
        .remove(&id)
        .ok_or_else(|| anyhow!("couldn't find job"))?;

    let (detail, incomplete_job) = JobDetail::from_job(job).await;

    if let Some(incomplete_job) = incomplete_job {
        job_registry.lock().await.insert(id, incomplete_job);
    }

    Ok(Json(detail))
}
