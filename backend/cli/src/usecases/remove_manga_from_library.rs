use anyhow::Result;

use crate::{database::Database, model::MangaId};

pub async fn remove_manga_from_library(db: &Database, id: MangaId) -> Result<()> {
    db.add_manga_to_library(id).await;

    Ok(())
}
