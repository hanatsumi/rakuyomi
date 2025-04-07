use anyhow::Result;
use futures::{stream, StreamExt};

use crate::{
    database::Database,
    model::{Manga, MangaState, SourceInformation},
    source_collection::SourceCollection,
};

pub async fn get_manga_library(
    db: &Database,
    source_collection: &impl SourceCollection,
) -> Result<Vec<Manga>> {
    // FIXME its a bit weird that we're calling `get_manga_library` and then
    // getting the informations for each entry, maybe the method shouldnt be called
    // that?
    let manga_ids = db.get_manga_library().await;
    let mangas: Vec<_> = stream::iter(&manga_ids)
        .filter_map(|id| db.find_cached_manga_information(id))
        .filter_map(|manga| async move {
            Some(Manga {
                source_information: SourceInformation::from(
                    source_collection
                        .get_by_id(manga.id.source_id())?
                        .manifest(),
                ),
                information: manga,
                state: MangaState {},
            })
        })
        .collect()
        .await;

    Ok(mangas)
}
