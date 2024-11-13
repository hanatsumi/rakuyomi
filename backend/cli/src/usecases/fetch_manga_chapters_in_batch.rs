use async_stream::stream;
use futures::Stream;
use std::collections::HashSet;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{
    chapter_downloader::ensure_chapter_is_in_storage,
    chapter_downloader::Error as ChapterDownloaderError,
    chapter_storage::ChapterStorage,
    database::Database,
    model::{ChapterInformation, MangaId},
    source::Source,
};

pub fn fetch_manga_chapters_in_batch<'a>(
    cancellation_token: CancellationToken,
    source: &'a Source,
    db: &'a Database,
    chapter_storage: &'a ChapterStorage,
    id: MangaId,
    filter: Filter,
) -> impl Stream<Item = ProgressReport> + 'a {
    stream! {
        let all_chapters = db.find_cached_chapter_informations(&id).await;
        let chapters_to_download = apply_chapter_filter(db, all_chapters, filter).await;

        let total = chapters_to_download.len();
        yield ProgressReport::Progressing { downloaded: 0, total };

        for (index, information) in chapters_to_download.into_iter().enumerate() {
            let ensure_in_storage_result = select! {
                _ = cancellation_token.cancelled() => {
                    yield ProgressReport::Cancelled;

                    return;
                },
                result = ensure_chapter_is_in_storage(chapter_storage, source, &information.id) => result
            };

            match ensure_in_storage_result {
                Ok(_) => yield ProgressReport::Progressing { downloaded: index + 1, total },
                Err(e) => {
                    let error = match e {
                        ChapterDownloaderError::DownloadError(e) => Error::DownloadError(e),
                        ChapterDownloaderError::Other(e) => Error::Other(e),
                    };

                    yield ProgressReport::Errored(error);
                    return;
                },
            }
        };

        yield ProgressReport::Finished;
    }
}

async fn apply_chapter_filter(
    db: &Database,
    all_chapters: Vec<ChapterInformation>,
    filter: Filter,
) -> Vec<ChapterInformation> {
    let mut last_read_chapter = None;

    // Starting from the newest chapter (in source order), find out the first one marked as read.
    for chapter in all_chapters.iter() {
        let read = db
            .find_chapter_state(&chapter.id)
            .await
            .is_some_and(|state| state.read);

        if read {
            last_read_chapter = Some(chapter.clone());

            break;
        }
    }

    // In reverse source order (oldest-to-newest), find out which unread chapters to download.
    let unread_chapters = all_chapters.into_iter().rev().skip_while(|chapter| {
        last_read_chapter.as_ref().is_some_and(|last_read_chapter| {
            last_read_chapter.chapter_number.unwrap_or_default()
                >= chapter.chapter_number.unwrap_or_default()
        })
    });

    let filtered_chapters: Vec<_> = match filter {
        Filter::AllUnreadChapters => unread_chapters.collect(),
        Filter::NextUnreadChapters(amount) => {
            let mut seen_chapter_numbers = HashSet::new();

            unread_chapters
                .take_while(|chapter| {
                    seen_chapter_numbers.insert(chapter.chapter_number.unwrap_or_default());

                    seen_chapter_numbers.len() <= amount
                })
                .collect()
        }
    };

    filtered_chapters
}

pub enum Filter {
    NextUnreadChapters(usize),
    AllUnreadChapters,
}

pub enum ProgressReport {
    Progressing { downloaded: usize, total: usize },
    Finished,
    Cancelled,
    Errored(Error),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while downloading all chapters")]
    DownloadError(#[source] anyhow::Error),
    #[error("unknown error")]
    Other(#[from] anyhow::Error),
}
