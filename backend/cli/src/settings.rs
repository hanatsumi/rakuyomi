use std::{fs::File, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_lists: Vec<Url>,
}

impl Settings {
    pub fn from_file_or_default(path: &Path) -> Result<Self> {
        if let Ok(file) = File::open(path) {
            Ok(serde_json::from_reader(file)?)
        } else {
            Ok(Default::default())
        }
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;

        Ok(serde_json::to_writer(file, self)?)
    }
}
