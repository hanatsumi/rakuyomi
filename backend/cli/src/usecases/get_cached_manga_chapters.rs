use anyhow::Result;
use futures::{stream, StreamExt};

use crate::{
    chapter_storage::ChapterStorage,
    database::Database,
    model::{Chapter, MangaId},
};

pub async fn get_cached_manga_chapters(
    db: &Database,
    chapter_storage: &ChapterStorage,
    id: MangaId,
) -> Result<Vec<Chapter>> {
    let cached_chapter_informations = db.find_cached_chapter_informations(&id).await;

    let cached_chapters = stream::iter(cached_chapter_informations)
        .then(|information| async move {
            let state = db
                .find_chapter_state(&information.id)
                .await
                .unwrap_or_default();
            let downloaded = chapter_storage
                .get_stored_chapter(&information.id)
                .is_some();

            Chapter {
                information,
                state,
                downloaded,
            }
        })
        .collect::<Vec<_>>()
        .await;

    Ok(cached_chapters)
}
