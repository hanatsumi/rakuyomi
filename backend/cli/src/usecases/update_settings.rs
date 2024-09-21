use std::path::Path;

use anyhow::Result;

use crate::settings::{Settings, UpdateableSettings};

pub fn update_settings(
    settings: &mut Settings,
    settings_path: &Path,
    settings_to_update: UpdateableSettings,
) -> Result<()> {
    // Clone the settings and save the cloned one first, so that we only change the application settings
    // iff everything goes well
    let mut updated_settings = settings.clone();
    settings_to_update.apply_updates(&mut updated_settings);
    updated_settings.save_to_file(settings_path)?;

    *settings = updated_settings;

    Ok(())
}
