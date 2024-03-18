use anyhow::Result;
use futures::{stream, StreamExt};

use crate::{
    database::Database,
    model::{ChapterInformation, ChapterState, MangaId},
    source::Source,
};

pub async fn get_manga_chapters(
    db: &Database,
    source: &Source,
    id: MangaId,
) -> Result<Response> {
    let (cached, chapters) = match source.get_chapter_list(id.manga_id.clone()).await {
        Ok(chapters) => (false, chapters.into_iter().map(|c| c.into()).collect()),
        Err(_) => (true, db.find_cached_chapter_informations(&id).await),
    };

    if !cached {
        // TODO Update the information on the database
    }

    let chapters_with_state = stream::iter(chapters)
        .then(|chapter| async move {
            let chapter_state = db.find_chapter_state(&chapter.id).await.unwrap_or_default();

            (chapter, chapter_state)
        })
        .collect::<Vec<_>>()
        .await;

    let response_type = match cached {
        true => ResponseType::Cached,
        false => ResponseType::Fresh,
    };

    Ok(Response(response_type, chapters_with_state))
}

pub enum ResponseType {
    Cached,
    Fresh,
}

pub struct Response(
    pub ResponseType,
    pub Vec<(ChapterInformation, ChapterState)>,
);
