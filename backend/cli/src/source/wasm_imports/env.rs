use anyhow::Result;
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasmi::{Caller, Linker};

use crate::source::wasm_store::WasmStore;

pub fn register_env_imports(linker: &mut Linker<WasmStore>) -> Result<()> {
    register_wasm_function!(linker, "env", "print", print)?;
    register_wasm_function!(linker, "env", "abort", abort)?;

    Ok(())
}

#[aidoku_wasm_function]
fn print(_caller: Caller<'_, WasmStore>, string: Option<String>) {
    let string = string.unwrap_or_default();

    println!("env.print: {string}");
}

#[aidoku_wasm_function]
fn abort(
    _caller: Caller<'_, WasmStore>,
    msg_offset: i32,
    file_name_offset: i32,
    line: i32,
    column: i32,
) {
    println!("env.abort called with {msg_offset} {file_name_offset} {line} {column}");
}
