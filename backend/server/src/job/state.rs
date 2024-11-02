use std::{collections::HashMap, path::PathBuf, sync::Arc};

use futures::lock::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::AppError;

pub enum Job {
    FetchChapter(JoinHandle<Result<PathBuf, AppError>>),
}

#[derive(Default, Clone)]
pub struct State {
    pub job_registry: Arc<Mutex<HashMap<Uuid, Job>>>,
}
