use std::collections::HashMap;

use anyhow::{Result, anyhow};
use serde_json::Value as JSONValue;
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasmi::{Caller, Linker};

use crate::source::wasm_store::{WasmStore, Value, ObjectValue};

pub fn register_json_imports(linker: &mut Linker<WasmStore>) -> Result<()> {
    register_wasm_function!(linker, "json", "parse", parse)?;

    Ok(())
}

#[aidoku_wasm_function]
fn parse(
    mut caller: Caller<'_, WasmStore>,
    json: Option<String>
) -> i32 {
    || -> Option<i32> {
        let json_value: JSONValue = serde_json::from_str(&json?).ok()?;
        let value: Value = json_value.try_into().ok()?;

        let wasm_store = caller.data_mut();

        Some(wasm_store.store_std_value(value, None) as i32)
    }().unwrap_or(-1)
}

impl TryFrom<JSONValue> for Value {
    type Error = anyhow::Error;

    // not my proudest code
    fn try_from(json_value: JSONValue) -> Result<Self> {
        Ok(match json_value {
            JSONValue::Array(arr) => {
                let converted_array: Vec<Value> = arr.iter()
                    .map(|v| v.clone().try_into().ok())
                    .collect::<Option<_>>()
                    .ok_or(anyhow!("failed to convert array"))?;

                Value::Array(converted_array)
            },
            JSONValue::Bool(b) => Value::Bool(b),
            JSONValue::Null => Value::Null,
            JSONValue::Number(n) => n.as_f64().map(|float| Value::Float(float))
                .or_else(|| n.as_i64().map(|int| Value::Int(int)))
                .or_else(|| n.as_u64().and_then(|int| int.try_into().ok()).map(|int| Value::Int(int)))
                .ok_or(anyhow!("could not convert {n} to a valid number"))?,
            JSONValue::Object(object) => {
                let converted_object: HashMap<String, Value> = object.iter()
                    .map(|(k, v)| v.clone().try_into().ok().and_then(|v| Some((k.clone(), v))))
                    .collect::<Option<_>>()
                    .ok_or(anyhow!("could not convert object to our values"))?;

                Value::Object(ObjectValue::HashMap(converted_object))
            }
            JSONValue::String(s) => Value::String(s)
        })
    }
}