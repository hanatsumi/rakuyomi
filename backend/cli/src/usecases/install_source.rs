use anyhow::{anyhow, Result};
use futures::{stream, StreamExt, TryStreamExt};
use serde::Deserialize;
use url::Url;

use crate::{model::SourceId, source_manager::SourceManager};

pub async fn install_source(
    source_manager: &mut SourceManager,
    source_lists: &Vec<Url>,
    source_id: SourceId,
) -> Result<()> {
    let (source_list, source_list_item) = stream::iter(source_lists)
        .then(|source_list| async move {
            let source_list_items = reqwest::get(source_list.clone())
                .await?
                .json::<Vec<SourceListItem>>()
                .await?;

            anyhow::Ok((source_list, source_list_items))
        })
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flat_map(|(source_list, items)| {
            items
                .into_iter()
                .map(|item| (source_list.clone(), item))
                .collect::<Vec<_>>()
        })
        .find(|(_, item)| item.id == source_id)
        .ok_or_else(|| anyhow!("couldn't find source with id '{:?}'", source_id))?;

    let aix_url = source_list
        .join(&format!("sources/{}", &source_list_item.file))
        .unwrap();

    let aix_content = reqwest::get(aix_url).await?.bytes().await?;

    source_manager.install_source(&source_id, aix_content)?;

    Ok(())
}

#[derive(Deserialize)]
struct SourceListItem {
    id: SourceId,
    file: String,
}
