use async_stream::stream;
use futures::{Stream, StreamExt, TryStreamExt};

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
) -> impl Stream<Item = ProgressReport> + 'a {
    stream! {
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
        db.upsert_cached_chapter_informations(chapter_informations.clone())
            .await;

        let total = chapter_informations.len();

        // We download the chapters in reverse order, in order to prioritize earlier chapters.
        // Normal source order has recent chapters first.
        for (index, information) in chapter_informations.into_iter().rev().enumerate() {
            match ensure_chapter_is_in_storage(chapter_storage, source, &information.id).await {
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

pub enum ProgressReport {
    // FIXME this is only used on the server's main.rs; we probably should indicate this state in some other way
    Initializing,
    Progressing { downloaded: usize, total: usize },
    Finished,
    Errored(Error),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while downloading all chapters")]
    DownloadError(#[source] anyhow::Error),
    #[error("unknown error")]
    Other(#[from] anyhow::Error),
}
