use crate::{model::SourceInformation, source_collection::SourceCollection};

pub fn list_installed_sources(source_collection: &impl SourceCollection) -> Vec<SourceInformation> {
    let mut source_informations: Vec<SourceInformation> = source_collection
        .sources()
        .into_iter()
        .map(|source| source.manifest().into())
        .collect();

    source_informations.sort_by_key(|source| source.name.clone());

    source_informations
}
