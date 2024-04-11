use anyhow::Result;
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasmi::{Caller, Linker};

use crate::source::{
    model::SettingDefinition,
    wasm_store::{Value, WasmStore},
};

pub fn register_defaults_imports(linker: &mut Linker<WasmStore>) -> Result<()> {
    register_wasm_function!(linker, "defaults", "get", get)?;
    register_wasm_function!(linker, "defaults", "set", set)?;

    Ok(())
}

#[aidoku_wasm_function]
fn get(mut caller: Caller<'_, WasmStore>, key: Option<String>) -> i32 {
    || -> Option<i32> {
        let key = key?;
        let wasm_store = caller.data_mut();
        let setting_definitions = &wasm_store.setting_definitions;

        // FIXME actually implement a defaults system
        if key == "languages" {
            return Some(
                wasm_store.store_std_value(Value::Array(vec![Value::String("en".into())]), None)
                    as i32,
            );
        }

        let setting_definition = find_setting_definition_by_key(setting_definitions, &key)?;
        let default_value = match setting_definition {
            SettingDefinition::Select { default, .. } => Value::String(default.clone()),
            SettingDefinition::Switch { default, .. } => Value::Bool(*default),
            SettingDefinition::Text { default, .. } => Value::String(default.clone()?),
            _ => return None,
        };

        // FIXME actually implement a defaults system
        Some(wasm_store.store_std_value(default_value, None) as i32)
    }()
    .unwrap_or(-1)
}

fn find_setting_definition_by_key<'a>(
    setting_definitions: &'a [SettingDefinition],
    needle_key: &str,
) -> Option<&'a SettingDefinition> {
    setting_definitions
        .iter()
        .find_map(|setting_definition| match setting_definition {
            SettingDefinition::Group { items, .. } => {
                find_setting_definition_by_key(items, needle_key)
            }
            SettingDefinition::Select { key, .. } if key == needle_key => Some(setting_definition),
            SettingDefinition::Switch { key, .. } if key == needle_key => Some(setting_definition),
            SettingDefinition::Text { key, .. } if key == needle_key => Some(setting_definition),
            _ => None,
        })
}

#[aidoku_wasm_function]
fn set(_caller: Caller<'_, WasmStore>, key: Option<String>, value: i32) {
    println!("defaults.set: {:?} -> {value}", key)
}
