use crate::{model::MangaInformation, source::Source};
use anyhow::Result;

pub async fn search_mangas(source: &Source, query: String) -> Result<Vec<MangaInformation>> {
    // FIXME the conversion between `SourceManga` and `MangaInformation` probably should
    // be inside the source itself
    let manga_informations = source
        .search_mangas(query)
        .await?
        .into_iter()
        .map(|source_manga| MangaInformation::from(source_manga))
        .collect();

    Ok(manga_informations)
}
