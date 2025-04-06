use anyhow::Result;
use futures::{stream, StreamExt};

use crate::{
    database::Database,
    model::{MangaInformation, SourceInformation},
    source_collection::SourceCollection,
};

pub async fn get_manga_library(
    db: &Database,
    source_collection: &impl SourceCollection,
) -> Result<Vec<(SourceInformation, MangaInformation)>> {
    // FIXME its a bit weird that we're calling `get_manga_library` and then
    // getting the informations for each entry, maybe the method shouldnt be called
    // that?
    let manga_ids = db.get_manga_library().await;
    let manga_and_source_informations: Vec<_> = stream::iter(&manga_ids)
        .filter_map(|id| db.find_cached_manga_information(id))
        .filter_map(|manga| async move {
            Some((
                SourceInformation::from(
                    source_collection
                        .get_by_id(manga.id.source_id())?
                        .manifest(),
                ),
                manga,
            ))
        })
        .collect()
        .await;

    Ok(manga_and_source_informations)
}
