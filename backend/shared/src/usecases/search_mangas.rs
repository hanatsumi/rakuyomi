use crate::{
    database::Database,
    model::{Manga, MangaInformation, MangaState, SourceInformation},
    source_collection::SourceCollection,
};
use futures::{stream, StreamExt};
use log::warn;
use tokio_util::sync::CancellationToken;

const CONCURRENT_SEARCH_REQUESTS: usize = 5;

pub async fn search_mangas(
    source_collection: &impl SourceCollection,
    db: &Database,
    cancellation_token: CancellationToken,
    query: String,
) -> Result<Vec<Manga>, Error> {
    // FIXME this looks awful
    let query = &query;
    let cancellation_token = &cancellation_token;

    // FIXME this kinda of works because cloning a source is cheap
    // (it has internal mutability yadda yadda).
    // we can't keep `source_collection` alive across async await points
    // because lifetimes fuckery
    let sources = source_collection
        .sources()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();

    let source_results: Vec<SourceMangaSearchResults> = stream::iter(sources)
        .map(async |source| {
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

            // Fetch unread chapters count for each manga
            let mangas = stream::iter(manga_informations)
                .then(|manga| async move {
                    let unread_count = db.count_unread_chapters(&manga.id).await;

                    (manga, unread_count)
                })
                .collect::<Vec<_>>()
                .await;

            SourceMangaSearchResults {
                source_information: source.manifest().into(),
                mangas,
            }
        })
        .buffered(CONCURRENT_SEARCH_REQUESTS)
        .collect::<Vec<_>>()
        .await;

    let mut mangas: Vec<_> = source_results
        .into_iter()
        .flat_map(|results| {
            let SourceMangaSearchResults {
                mangas,
                source_information,
            } = results;

            mangas.into_iter().map(move |(manga, unread_count)| Manga {
                source_information: source_information.clone(),
                information: manga,
                state: MangaState {},
                unread_chapters_count: unread_count,
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
    mangas: Vec<(MangaInformation, Option<usize>)>,
}
