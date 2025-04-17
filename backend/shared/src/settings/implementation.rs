use std::{fs::File, path::Path};

use anyhow::{Context, Result};

use super::schema::Settings;

impl Settings {
    pub fn from_file(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| "Couldn't open file")?;
        let settings = serde_json_lenient::from_reader(file)
            .with_context(|| "Couldn't parse file contents")?;

        Ok(settings)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;

        Ok(serde_json_lenient::to_writer_pretty(file, self)?)
    }
}
