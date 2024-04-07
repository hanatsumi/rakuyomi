use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{Context, Result};

use crate::{model::SourceId, source::Source, source_collection::SourceCollection};

pub struct SourceManager {
    sources_folder: PathBuf,
    sources_by_id: HashMap<SourceId, Source>,
}

impl SourceManager {
    pub fn from_folder(path: PathBuf) -> Result<Self> {
        let files = fs::read_dir(&path).with_context(|| {
            format!(
                "while attempting to read source collection at {}",
                &path.display()
            )
        })?;

        let sources: Vec<Source> = files
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("aix"))
            })
            .map(|path| Source::from_aix_file(&path))
            .collect::<Result<_>>()?;

        let sources_by_id = sources
            .into_iter()
            .map(|source| (SourceId::new(source.manifest().info.id.clone()), source))
            .collect();

        Ok(Self {
            sources_folder: path,
            sources_by_id,
        })
    }

    pub fn install_source(&mut self, id: &SourceId, contents: impl AsRef<[u8]>) -> Result<()> {
        let target_path = self.sources_folder.join(format!("{}.aix", id.value()));
        fs::write(&target_path, contents)?;

        self.sources_by_id
            .insert(id.clone(), Source::from_aix_file(&target_path)?);

        Ok(())
    }
}

impl SourceCollection for SourceManager {
    fn get_by_id(&self, id: &SourceId) -> Option<&Source> {
        self.sources_by_id.get(id)
    }

    fn sources(&self) -> Vec<&Source> {
        self.sources_by_id.values().collect()
    }
}
