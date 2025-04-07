use crate::{
    database::Database,
    model::{Manga, MangaInformation, MangaState, SourceInformation},
    source_collection::SourceCollection,
};
use futures::{stream, StreamExt};
use log::warn;
use tokio_util::sync::CancellationToken;

pub async fn search_mangas(
    source_collection: &impl SourceCollection,
    db: &Database,
    cancellation_token: CancellationToken,
    query: String,
) -> Result<Vec<Manga>, Error> {
    // FIXME this looks awful
    let query = &query;
    let cancellation_token = &cancellation_token;

    let source_results: Vec<SourceMangaSearchResults> = stream::iter(source_collection.sources())
        .then(|source| async move {
            // FIXME the conversion between `SourceManga` and `MangaInformation` probably should
            // be inside the source itself
            let search_result = source
                .search_mangas(cancellation_token.clone(), query.clone())
                .await;

            let manga_informations = match search_result {
                Ok(source_mangas) => source_mangas
                    .into_iter()
                    .map(MangaInformation::from)
                    .collect(),
                Err(e) => {
                    warn!(
                        "failed to search mangas from source {}: {}",
                        source.manifest().info.id,
                        e
                    );

                    vec![]
                }
            };

            // Write through to the database
            stream::iter(&manga_informations)
                .for_each(|information| db.upsert_cached_manga_information(information.clone()))
                .await;

            SourceMangaSearchResults {
                source_information: source.manifest().into(),
                mangas: manga_informations,
            }
        })
        .collect()
        .await;

    let mut mangas: Vec<_> = source_results
        .into_iter()
        .flat_map(|results| {
            let SourceMangaSearchResults {
                mangas,
                source_information,
            } = results;

            mangas.into_iter().map(move |manga| Manga {
                source_information: source_information.clone(),
                information: manga,
                state: MangaState {},
            })
        })
        .collect();

    mangas.sort_by_cached_key(|manga| manga.information.title.clone());

    Ok(mangas)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while fetching search results from the source")]
    SourceError(#[source] anyhow::Error),
}

struct SourceMangaSearchResults {
    source_information: SourceInformation,
    mangas: Vec<MangaInformation>,
}
