use std::{collections::HashMap, sync::Arc};

use futures::lock::Mutex;
use uuid::Uuid;

use crate::AppError;

use super::download_chapter::DownloadChapterJob;

pub enum JobState<Progress, Output, Error> {
    InProgress(Progress),
    Completed(Output),
    Errored(Error),
}

pub trait Job {
    type Progress;
    type Output;
    type Error;

    fn is_cancellable(&self) -> bool;
    async fn cancel(&self) -> Result<(), AppError>;
    async fn poll(&self) -> JobState<Self::Progress, Self::Output, Self::Error>;
}

pub enum RunningJob {
    DownloadChapter(DownloadChapterJob),
}

#[derive(Default, Clone)]
pub struct State {
    pub job_registry: Arc<Mutex<HashMap<Uuid, RunningJob>>>,
}
