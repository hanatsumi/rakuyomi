use std::path::Path;

use anyhow::Result;
use futures::{stream, StreamExt};

use crate::{
    database::Database,
    model::{Chapter, ChapterInformation, ChapterState, MangaId},
    source::Source,
};

pub async fn get_manga_chapters(
    db: &Database,
    source: &Source,
    downloads_folder_path: &Path,
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

            // TODO unify logic with `fetch_manga_chapter`
            let output_filename = format!(
                "{}-{}.cbz",
                &information.id.manga_id.source_id.0, &information.id.chapter_id
            );
            let output_path = downloads_folder_path.join(output_filename);
            let downloaded = output_path.exists();

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
