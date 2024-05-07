use crate::source::{model::SettingDefinition, Source};

pub fn get_source_setting_definitions(source: &Source) -> Vec<SettingDefinition> {
    source.setting_definitions()
}
