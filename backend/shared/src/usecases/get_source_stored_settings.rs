use std::collections::HashMap;

use crate::{
    model::SourceId,
    settings::{Settings, SourceSettingValue},
};

pub fn get_source_stored_settings(
    settings: &Settings,
    source_id: &SourceId,
) -> HashMap<String, SourceSettingValue> {
    settings
        .source_settings
        .get(source_id.value())
        .cloned()
        .unwrap_or_default()
}
