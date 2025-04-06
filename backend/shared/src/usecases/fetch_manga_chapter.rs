use std::path::PathBuf;

use crate::{
    chapter_downloader::ensure_chapter_is_in_storage,
    chapter_downloader::Error as ChapterDownloaderError, chapter_storage::ChapterStorage,
    model::ChapterId, source::Source,
};

pub async fn fetch_manga_chapter(
    source: &Source,
    chapter_storage: &ChapterStorage,
    chapter_id: &ChapterId,
) -> Result<PathBuf, Error> {
    ensure_chapter_is_in_storage(chapter_storage, source, chapter_id)
        .await
        .map_err(|e| match e {
            ChapterDownloaderError::DownloadError(e) => Error::DownloadError(e),
            ChapterDownloaderError::Other(e) => Error::Other(e),
        })
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while downloading the chapter pages")]
    DownloadError(#[source] anyhow::Error),
    #[error("unknown error")]
    Other(#[from] anyhow::Error),
}
