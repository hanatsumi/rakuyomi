use std::path::Path;

use anyhow::Result;
use futures::{stream, StreamExt};

use crate::{
    chapter_storage::ChapterStorage,
    database::Database,
    model::{Chapter, MangaId},
    source::Source,
};

pub async fn get_manga_chapters(
    db: &Database,
    source: &Source,
    chapter_storage: &ChapterStorage,
    id: MangaId,
) -> Result<Response> {
    let (cached, chapter_informations) = match source.get_chapter_list(id.manga_id.clone()).await {
        Ok(chapters) => (false, chapters.into_iter().map(|c| c.into()).collect()),
        Err(_) => (true, db.find_cached_chapter_informations(&id).await),
    };

    if !cached {
        db.upsert_cached_chapter_informations(chapter_informations.clone())
            .await;
    }

    let chapters = stream::iter(chapter_informations)
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

    let response_type = match cached {
        true => ResponseType::Cached,
        false => ResponseType::Fresh,
    };

    Ok(Response(response_type, chapters))
}

pub enum ResponseType {
    Cached,
    Fresh,
}

pub struct Response(pub ResponseType, pub Vec<Chapter>);
