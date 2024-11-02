use std::{path::PathBuf, sync::Arc};

use axum_macros::FromRef;
use cli::usecases::fetch_all_manga_chapters::Error as FetchAllMangaChaptersError;
use cli::{
    chapter_storage::ChapterStorage, database::Database, settings::Settings,
    source_manager::SourceManager,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::job::State as JobState;

#[derive(Default)]
pub enum DownloadAllChaptersState {
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

#[derive(Clone, FromRef)]
pub struct State {
    pub source_manager: Arc<Mutex<SourceManager>>,
    pub database: Arc<Database>,
    pub chapter_storage: ChapterStorage,
    pub download_all_chapters_state: Arc<Mutex<DownloadAllChaptersState>>,
    pub settings: Arc<Mutex<Settings>>,
    pub settings_path: PathBuf,
    pub job_state: JobState,
}
