use anyhow::Result;
use futures::{stream, StreamExt};

use crate::{database::Database, model::MangaInformation};

pub async fn get_manga_library(db: &Database) -> Result<Vec<MangaInformation>> {
    // FIXME its a bit weird that we're calling `get_manga_library` and then
    // getting the informations for each entry, maybe the method shouldnt be called
    // that?
    let manga_ids = db.get_manga_library().await;
    let manga_informations: Vec<_> = stream::iter(&manga_ids)
        .then(|id| db.find_cached_manga_information(id))
        .map(|opt| opt.unwrap())
        .collect()
        .await;

    Ok(manga_informations)
}
