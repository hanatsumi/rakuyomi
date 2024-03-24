use anyhow::Result;

use crate::{database::Database, model::MangaId, source::Source};

pub async fn refresh_manga_chapters(db: &Database, source: &Source, id: MangaId) -> Result<()> {
    let fresh_chapter_informations = source
        .get_chapter_list(id.value().clone())
        .await?
        .into_iter()
        .map(From::from)
        .collect();

    db.upsert_cached_chapter_informations(fresh_chapter_informations)
        .await;

    Ok(())
}
