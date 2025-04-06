use std::{fs::File, path::Path};

use anyhow::Result;

use super::schema::Settings;

impl Settings {
    pub fn from_file_or_default(path: &Path) -> Result<Self> {
        if let Ok(file) = File::open(path) {
            Ok(serde_json_lenient::from_reader(file)?)
        } else {
            Ok(Default::default())
        }
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;

        Ok(serde_json_lenient::to_writer_pretty(file, self)?)
    }
}
