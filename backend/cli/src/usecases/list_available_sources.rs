use anyhow::{Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use url::Url;

use crate::model::SourceInformation;

pub async fn list_available_sources(source_lists: Vec<Url>) -> Result<Vec<SourceInformation>> {
    let mut source_informations: Vec<SourceInformation> = stream::iter(source_lists)
        .then(|source_list| async move {
            let response = reqwest::get(source_list.clone())
                .await
                .with_context(|| format!("failed to fetch source list at {}", &source_list))?;

            let source_informations = response
                .json::<Vec<SourceInformation>>()
                .await
                .with_context(|| format!("failed to parse source list at {}", &source_list))?;

            anyhow::Ok(source_informations)
        })
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .collect();

    source_informations.sort_by_key(|source| source.name.clone());

    Ok(source_informations)
}
