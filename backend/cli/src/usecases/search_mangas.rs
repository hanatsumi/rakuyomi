use crate::{database::Database, model::MangaInformation, source::Source};
use futures::{stream, StreamExt};

pub async fn search_mangas(
    source: &Source,
    db: &Database,
    query: String,
) -> Result<Vec<MangaInformation>, Error> {
    // FIXME the conversion between `SourceManga` and `MangaInformation` probably should
    // be inside the source itself
    let manga_informations: Vec<_> = source
        .search_mangas(query)
        .await
        .map_err(Error::SourceError)?
        .into_iter()
        .map(|source_manga| MangaInformation::from(source_manga))
        .collect();

    // Write through to the database
    stream::iter(&manga_informations)
        .for_each(|information| db.upsert_cached_manga_information(information.clone()))
        .await;

    Ok(manga_informations)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while fetching search results from the source")]
    SourceError(#[source] anyhow::Error),
}
