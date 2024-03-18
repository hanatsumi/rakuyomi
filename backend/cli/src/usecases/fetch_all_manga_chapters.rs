use futures::{stream, StreamExt, TryStreamExt};

use crate::{
    chapter_downloader::ensure_chapter_is_in_storage,
    chapter_downloader::Error as ChapterDownloaderError,
    chapter_storage::ChapterStorage,
    database::Database,
    model::{ChapterInformation, MangaId},
    source::Source,
};

pub async fn fetch_all_manga_chapters(
    source: &Source,
    db: &Database,
    chapter_storage: &ChapterStorage,
    id: MangaId,
) -> Result<(), Error> {
    let chapter_informations: Vec<ChapterInformation> = source
        .get_chapter_list(id.manga_id.clone())
        .await
        .map_err(Error::DownloadError)?
        .into_iter()
        .map(|c| c.into())
        .collect();

    // FIXME introduce some kind of function for reading from the source and writing to the DB?
    // it would be cool if all reads from the source automatically updated the database
    db.upsert_cached_chapter_informations(chapter_informations.clone())
        .await;

    stream::iter(chapter_informations)
        .then(|information| async move {
            ensure_chapter_is_in_storage(chapter_storage, source, &information.id).await?;

            Ok(())
        })
        .try_collect()
        .await
        .map_err(|e| match e {
            ChapterDownloaderError::DownloadError(e) => Error::DownloadError(e),
            ChapterDownloaderError::Other(e) => Error::Other(e),
        })?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while downloading all chapters")]
    DownloadError(#[source] anyhow::Error),
    #[error("unknown error")]
    Other(#[from] anyhow::Error),
}
