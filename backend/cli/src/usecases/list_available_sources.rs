use anyhow::Result;
use futures::{stream, StreamExt, TryStreamExt};
use url::Url;

use crate::model::SourceInformation;

pub async fn list_available_sources(source_lists: Vec<Url>) -> Result<Vec<SourceInformation>> {
    let mut source_informations: Vec<SourceInformation> = stream::iter(source_lists)
        .then(|source_list| async move {
            anyhow::Ok(
                reqwest::get(source_list)
                    .await?
                    .json::<Vec<SourceInformation>>()
                    .await?,
            )
        })
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .collect();

    source_informations.sort_by_key(|source| source.name.clone());

    Ok(source_informations)
}
