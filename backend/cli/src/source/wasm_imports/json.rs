use anyhow::Result;
use serde::de::{Deserialize, MapAccess, Visitor};
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasmi::{Caller, Linker};

use crate::source::wasm_store::{ObjectValue, Value, ValueMap, WasmStore};

pub fn register_json_imports(linker: &mut Linker<WasmStore>) -> Result<()> {
    register_wasm_function!(linker, "json", "parse", parse)?;

    Ok(())
}

#[aidoku_wasm_function]
fn parse(mut caller: Caller<'_, WasmStore>, json: Option<String>) -> i32 {
    || -> Option<i32> {
        let value: Value = serde_json::from_str(&json?).ok()?;

        let wasm_store = caller.data_mut();

        Some(wasm_store.store_std_value(value, None) as i32)
    }()
    .unwrap_or(-1)
}

impl<'de> Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value supported by rakuyomi")
            }

            fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_bool<E>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Int(v))
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // FIXME no
                Ok(Value::Int(v.try_into().map_err(|_| todo!())?))
            }

            fn visit_f64<E>(self, v: f64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Float(v))
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::String(v))
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut array: Vec<Value> = Vec::with_capacity(seq.size_hint().unwrap_or(0));

                while let Some(element) = seq.next_element()? {
                    array.push(element);
                }

                Ok(Value::Array(array))
            }

            fn visit_map<A>(self, mut access: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut map = ValueMap::new();

                while let Some((key, value)) = access.next_entry()? {
                    map.insert(key, value);
                }

                Ok(Value::Object(ObjectValue::ValueMap(map)))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}
