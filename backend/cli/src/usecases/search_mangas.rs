use crate::{database::Database, model::MangaInformation, source::Source};
use anyhow::Result;
use futures::{stream, StreamExt};

pub async fn search_mangas(
    source: &Source,
    db: &Database,
    query: String,
) -> Result<Vec<MangaInformation>> {
    // FIXME the conversion between `SourceManga` and `MangaInformation` probably should
    // be inside the source itself
    let manga_informations: Vec<_> = source
        .search_mangas(query)
        .await?
        .into_iter()
        .map(|source_manga| MangaInformation::from(source_manga))
        .collect();

    // Write through to the database
    stream::iter(&manga_informations)
        .for_each(|information| db.upsert_cached_manga_information(information.clone()))
        .await;

    Ok(manga_informations)
}
