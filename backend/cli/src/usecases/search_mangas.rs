use crate::{
    database::Database,
    model::{MangaInformation, SourceInformation},
    source_collection::SourceCollection,
};
use futures::{stream, StreamExt, TryStreamExt};

pub async fn search_mangas(
    source_collection: &SourceCollection,
    db: &Database,
    query: String,
) -> Result<Vec<SourceMangaSearchResults>, Error> {
    // FIXME this looks awful
    let query = &query;

    // FIXME the conversion between `SourceManga` and `MangaInformation` probably should
    // be inside the source itself
    let source_results: Vec<SourceMangaSearchResults> = stream::iter(source_collection.sources())
        .then(|source| async move {
            let manga_informations: Vec<_> = source
                .search_mangas(query.clone())
                .await
                .map_err(Error::SourceError)?
                .into_iter()
                .map(MangaInformation::from)
                .collect();

            // Write through to the database
            stream::iter(&manga_informations)
                .for_each(|information| db.upsert_cached_manga_information(information.clone()))
                .await;

            Ok(SourceMangaSearchResults {
                source_information: source.manifest().into(),
                mangas: manga_informations,
            })
        })
        .try_collect()
        .await?;

    Ok(source_results)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while fetching search results from the source")]
    SourceError(#[source] anyhow::Error),
}

pub struct SourceMangaSearchResults {
    pub source_information: SourceInformation,
    pub mangas: Vec<MangaInformation>,
}
