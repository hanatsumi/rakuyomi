use anyhow::Result;
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasmi::{Caller, Linker};

use crate::source::wasm_store::WasmStore;

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

        // FIXME actually implement a defaults system
        if key == "languages" {
            return Some(
                wasm_store.store_std_value(wasm_store.settings.languages.clone().into(), None)
                    as i32,
            );
        }

        let value = wasm_store.source_settings.get(&key)?.clone().into();

        Some(wasm_store.store_std_value(value, None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn set(_caller: Caller<'_, WasmStore>, key: Option<String>, value: i32) {
    println!("defaults.set: {:?} -> {value}", key)
}
