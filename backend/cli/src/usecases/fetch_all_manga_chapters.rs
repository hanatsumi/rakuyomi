use async_stream::stream;
use futures::Stream;
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

pub fn fetch_all_manga_chapters<'a>(
    source: &'a Source,
    db: &'a Database,
    chapter_storage: &'a ChapterStorage,
    id: MangaId,
) -> (CancellationToken, impl Stream<Item = ProgressReport> + 'a) {
    let cancellation_token = CancellationToken::new();
    let cloned_cancellation_token = cancellation_token.clone();
    let stream = stream! {
        let chapter_informations: Vec<ChapterInformation> = match source
            .get_chapter_list(id.value().clone())
            .await {
            Ok(chapters) => chapters.into_iter().map(|c| c.into()).collect(),
            Err(e) => {
                yield ProgressReport::Errored(Error::DownloadError(e));

                return;
            },
        };

        // FIXME introduce some kind of function for reading from the source and writing to the DB?
        // it would be cool if all reads from the source automatically updated the database
        db.upsert_cached_chapter_informations(&id, chapter_informations.clone())
            .await;

        let total = chapter_informations.len();
        yield ProgressReport::Progressing { downloaded: 0, total };

        // We download the chapters in reverse order, in order to prioritize earlier chapters.
        // Normal source order has recent chapters first.
        for (index, information) in chapter_informations.into_iter().rev().enumerate() {
            let ensure_in_storage_result = select! {
                _ = cloned_cancellation_token.cancelled() => {
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
    };

    (cancellation_token, stream)
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
