#![allow(clippy::too_many_arguments)]

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone};
use wasm_shared::{
    get_memory,
    memory_reader::{read_string as read_memory_string, write_bytes},
};
use wasmi::{core::F64, Caller, Linker};

use crate::source::{
    model::{Filter, FilterType, Manga, MangaPageResult},
    wasm_store::{ObjectValue, Value, ValueMap, ValueRef, WasmStore},
};

use super::util::timestamp_f64;

enum ObjectType {
    Null = 0,
    Int = 1,
    Float = 2,
    String = 3,
    Bool = 4,
    Array = 5,
    Object = 6,
    Date = 7,
    Node = 8,
    #[allow(dead_code)]
    Unknown = 9,
}

trait FieldAsValue {
    fn field_as_value(&self, field: &str) -> Option<Value>;
}

pub fn register_std_imports(linker: &mut Linker<WasmStore>) -> Result<()> {
    linker.func_wrap("std", "copy", copy)?;
    linker.func_wrap("std", "destroy", destroy)?;

    linker.func_wrap("std", "create_null", create_null)?;
    linker.func_wrap("std", "create_int", create_int)?;
    linker.func_wrap("std", "create_float", create_float)?;
    linker.func_wrap("std", "create_string", create_string)?;
    linker.func_wrap("std", "create_bool", create_bool)?;
    linker.func_wrap("std", "create_array", create_array)?;
    linker.func_wrap("std", "create_object", create_object)?;
    linker.func_wrap("std", "create_date", create_date)?;

    linker.func_wrap("std", "typeof", type_of)?;

    linker.func_wrap("std", "string_len", string_len)?;
    linker.func_wrap("std", "read_string", read_string)?;
    linker.func_wrap("std", "read_int", read_int)?;
    linker.func_wrap("std", "read_float", read_float)?;
    linker.func_wrap("std", "read_bool", read_bool)?;
    linker.func_wrap("std", "read_date", read_date)?;
    linker.func_wrap("std", "read_date_string", read_date_string)?;

    linker.func_wrap("std", "object_len", object_len)?;
    linker.func_wrap("std", "object_get", object_get)?;
    linker.func_wrap("std", "object_set", object_set)?;
    linker.func_wrap("std", "object_remove", object_remove)?;
    linker.func_wrap("std", "object_keys", object_keys)?;
    linker.func_wrap("std", "object_values", object_values)?;

    linker.func_wrap("std", "array_len", array_len)?;
    linker.func_wrap("std", "array_get", array_get)?;
    linker.func_wrap("std", "array_set", array_set)?;
    linker.func_wrap("std", "array_append", array_append)?;
    linker.func_wrap("std", "array_remove", array_remove)?;

    Ok(())
}

fn copy(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    descriptor_i32
        .try_into()
        .ok()
        .and_then(|descriptor| {
            let wasm_store = caller.data_mut();

            wasm_store
                .get_std_value(descriptor)
                .map(|value| wasm_store.store_std_value(value, None) as i32)
        })
        .unwrap_or(-1)
}

fn destroy(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) {
    if let Ok(descriptor) = descriptor_i32.try_into() {
        caller.data_mut().remove_std_value(descriptor);
    }
}

fn create_null(caller: Caller<'_, WasmStore>) -> i32 {
    create_value(caller, Value::Null)
}

fn create_int(caller: Caller<'_, WasmStore>, value: i64) -> i32 {
    create_value(caller, value.into())
}

fn create_float(caller: Caller<'_, WasmStore>, value: F64) -> i32 {
    create_value(caller, value.to_float().into())
}

fn create_string(mut caller: Caller<'_, WasmStore>, offset: i32, length: i32) -> i32 {
    if let Some(memory) = get_memory(&mut caller) {
        read_memory_string(&memory, &caller, offset as usize, length as usize)
            .map(|string| create_value(caller, string.into()))
            .unwrap_or(-1)
    } else {
        -1
    }
}

fn create_bool(caller: Caller<'_, WasmStore>, value_i32: i32) -> i32 {
    create_value(caller, Value::Bool(value_i32 != 0))
}

fn create_array(caller: Caller<'_, WasmStore>) -> i32 {
    create_value(caller, Value::Array(Vec::default()))
}

fn create_date(caller: Caller<'_, WasmStore>, seconds_since_1970: F64) -> i32 {
    let seconds_since_1970 = seconds_since_1970.to_float();
    let full_seconds = seconds_since_1970.floor() as i64;
    let nanos_remainder = ((seconds_since_1970 - full_seconds as f64) * (10f64.powi(9))) as u32;
    let naive_date_time = NaiveDateTime::from_timestamp_opt(full_seconds, nanos_remainder).unwrap();
    let date_time: DateTime<chrono_tz::Tz> = chrono_tz::UTC
        .from_local_datetime(&naive_date_time)
        .unwrap();

    create_value(caller, date_time.into())
}

fn create_value(mut caller: Caller<'_, WasmStore>, value: Value) -> i32 {
    let wasm_store = caller.data_mut();

    wasm_store.store_std_value(value.into(), None) as i32
}

fn create_object(caller: Caller<'_, WasmStore>) -> i32 {
    create_value(caller, ValueMap::default().into())
}

fn type_of(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<ObjectType> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();
        let value = wasm_store.get_std_value(descriptor)?;

        Some(match *value {
            Value::Null => ObjectType::Null,
            Value::Int(_) => ObjectType::Int,
            Value::Float(_) => ObjectType::Float,
            Value::String(_) => ObjectType::String,
            Value::Bool(_) => ObjectType::Bool,
            Value::Array(_) => ObjectType::Array,
            Value::Object(_) => ObjectType::Object,
            Value::Date(_) => ObjectType::Date,
            Value::HTMLElements(_) => ObjectType::Node,
        })
    }()
    .unwrap_or(ObjectType::Null) as i32
}

fn string_len(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();
        let value = wasm_store.get_std_value(descriptor)?;

        match value.as_ref() {
            Value::String(s) => Some(s.len() as i32),
            _ => None,
        }
    }()
    .unwrap_or(-1)
}

fn read_string(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    buffer_i32: i32,
    size_i32: i32,
) {
    || -> Option<()> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let buffer: usize = buffer_i32.try_into().ok()?;
        let size: usize = size_i32.try_into().ok()?;

        let wasm_store = caller.data();
        let value_ref = wasm_store.get_std_value(descriptor)?;
        let string = value_ref.try_unwrap_string_ref().ok()?;

        let memory = get_memory(&mut caller)?;
        if size <= string.len() {
            let string_slice = &string.as_str()[..size];
            write_bytes(&memory, &mut caller, string_slice.as_bytes(), buffer)?;
        };

        Some(())
    }();
}

fn read_int(caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i64 {
    || -> Option<i64> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data();
        let value: ValueRef = wasm_store.get_std_value(descriptor)?;

        match value.as_ref() {
            Value::Bool(b) => Some(if *b { 1i64 } else { 0i64 }),
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(f.trunc() as i64),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }()
    .unwrap_or(-1)
}

fn read_float(caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> F64 {
    || -> Option<f64> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data();
        let value = wasm_store.get_std_value(descriptor)?;

        match value.as_ref() {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }()
    .unwrap_or(-1f64)
    .into()
}

fn read_bool(caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data();
        let value = wasm_store.get_std_value(descriptor)?;

        match value.as_ref() {
            Value::Bool(b) => Some(if *b { 1i32 } else { 0i32 }),
            Value::Int(i) => Some(if *i != 0 { 1i32 } else { 0i32 }),
            _ => None,
        }
    }()
    .unwrap_or(0)
}

fn read_date(caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> F64 {
    || -> Option<f64> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data();
        let value = wasm_store.get_std_value(descriptor)?;

        match value.as_ref() {
            Value::Date(date) => Some(
                date.timestamp() as f64 + (date.timestamp_subsec_nanos() as f64) / (10f64.powi(9)),
            ),
            _ => None,
        }
    }()
    .unwrap_or(0f64)
    .into()
}

fn read_date_string(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    format_i32: i32,
    format_len_i32: i32,
    locale_i32: i32,
    locale_len_i32: i32,
    timezone_i32: i32,
    timezone_len_i32: i32,
) -> F64 {
    || -> Option<f64> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let format: usize = format_i32.try_into().ok()?;
        let format_len: usize =
            format_len_i32
                .try_into()
                .ok()
                .and_then(|len| if len > 0 { Some(len) } else { None })?;

        let locale: Option<usize> = locale_i32.try_into().ok();
        let locale_len: Option<usize> =
            locale_len_i32
                .try_into()
                .ok()
                .and_then(|len| if len > 0 { Some(len) } else { None });

        let timezone: Option<usize> = timezone_i32.try_into().ok();
        let timezone_len: Option<usize> =
            timezone_len_i32
                .try_into()
                .ok()
                .and_then(|len| if len > 0 { Some(len) } else { None });

        let wasm_store = caller.data();
        let value_ref = wasm_store.get_std_value(descriptor)?;
        let string = match value_ref.as_ref() {
            Value::String(s) => Some(s),
            _ => None,
        }?;

        let memory = get_memory(&mut caller)?;
        let format_string = read_memory_string(&memory, &caller, format, format_len)?;
        let _locale_string = match (locale, locale_len) {
            (Some(locale), Some(locale_len)) => {
                Some(read_memory_string(&memory, &caller, locale, locale_len)?)
            }
            _ => None,
        };
        let timezone_string = match (timezone, timezone_len) {
            (Some(timezone), Some(timezone_len)) => Some(read_memory_string(
                &memory,
                &caller,
                timezone,
                timezone_len,
            )?),
            _ => None,
        };

        let timezone: chrono_tz::Tz = timezone_string
            .and_then(|tz_str| tz_str.parse().ok())
            .unwrap_or(chrono_tz::UTC);
        let date_time = NaiveDateTime::parse_from_str(&string, &format_string)
            .ok()?
            .and_local_timezone(timezone)
            .single()?;

        Some(timestamp_f64(date_time.naive_local()))
    }()
    .unwrap_or(-1f64)
    .into()
}

// FIXME this entire object part stinks, and is probably going to be buggy as hell because we copy stuff around
// probably not stop being dumb!!!!!!!!!!!!
// probably yeah because swift store classes by reference and we store them by value
fn object_len(caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data();

        if let Value::Object(ObjectValue::ValueMap(hm)) =
            wasm_store.get_std_value(descriptor)?.as_ref()
        {
            Some(hm.len() as i32)
        } else {
            None
        }
    }()
    .unwrap_or(0)
}

fn object_get(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    key_i32: i32,
    key_len_i32: i32,
) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let key_offset: usize = key_i32.try_into().ok()?;
        let key_len: usize =
            key_len_i32
                .try_into()
                .ok()
                .and_then(|key_len| if key_len > 0 { Some(key_len) } else { None })?;
        let key = {
            let memory = get_memory(&mut caller)?;
            read_memory_string(&memory, &caller, key_offset, key_len)?
        };

        let wasm_store = caller.data_mut();
        let object_ref = wasm_store
            .get_std_value(descriptor)?
            .try_project(|value| match value {
                Value::Object(obj) => Ok(obj),
                _ => Err(()),
            })
            .ok()?;

        // FIXME see above comment
        let value = match object_ref.as_ref() {
            // FIXME This is kinda ugly, but since we know it's a `ValueMap` we force a projection here.
            ObjectValue::ValueMap(_) => object_ref
                .try_project(|object| {
                    if let ObjectValue::ValueMap(map) = object {
                        map.get(&key).ok_or(())
                    } else {
                        panic!("expected object to be a ValueMap")
                    }
                })
                .ok()?,
            // PERF We can't really use Parc projections here, as we wrap things in another enum..?
            // Hopefully this has little impact.
            ObjectValue::Manga(m) => m.field_as_value(&key)?.into(),
            ObjectValue::MangaPageResult(mpr) => mpr.field_as_value(&key)?.into(),
            ObjectValue::Filter(f) => f.field_as_value(&key)?.into(),
            _ => todo!("missing implementation"),
        };

        Some(wasm_store.store_std_value(value, Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

fn object_set(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    key_i32: i32,
    key_len_i32: i32,
    value_i32: i32,
) {
    || -> Option<()> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let value_descriptor: usize = value_i32.try_into().ok()?;

        let key_offset: usize = key_i32.try_into().ok()?;
        let key_len: usize =
            key_len_i32
                .try_into()
                .ok()
                .and_then(|key_len| if key_len > 0 { Some(key_len) } else { None })?;
        let key = {
            let memory = get_memory(&mut caller)?;
            read_memory_string(&memory, &caller, key_offset, key_len)?
        };

        let wasm_store = caller.data_mut();
        let value = wasm_store.get_std_value(value_descriptor)?.as_ref().clone();
        let mut hashmap_object = if let Value::Object(ObjectValue::ValueMap(hm)) =
            wasm_store.get_std_value(descriptor)?.as_ref()
        {
            Some(hm.clone())
        } else {
            None
        }?;

        hashmap_object.insert(key, value);

        wasm_store.set_std_value(
            descriptor,
            Value::Object(ObjectValue::ValueMap(hashmap_object)).into(),
        );

        Some(())
    }();
}

fn object_remove(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    key_i32: i32,
    key_len_i32: i32,
) {
    || -> Option<()> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let key_offset: usize = key_i32.try_into().ok()?;
        let key_len: usize =
            key_len_i32
                .try_into()
                .ok()
                .and_then(|key_len| if key_len > 0 { Some(key_len) } else { None })?;
        let key = {
            let memory = get_memory(&mut caller)?;
            read_memory_string(&memory, &caller, key_offset, key_len)?
        };

        let wasm_store = caller.data_mut();
        let mut hashmap_object = if let Value::Object(ObjectValue::ValueMap(hm)) =
            wasm_store.get_std_value(descriptor)?.as_ref()
        {
            Some(hm.clone())
        } else {
            None
        }?;

        hashmap_object.remove(&key);

        wasm_store.set_std_value(
            descriptor,
            Value::Object(ObjectValue::ValueMap(hashmap_object)).into(),
        );

        Some(())
    }();
}

fn object_keys(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let hashmap_object = std_value
            .try_unwrap_object_ref()
            .ok()?
            .try_unwrap_value_map_ref()
            .ok()?;

        let keys: Vec<Value> = hashmap_object.keys().cloned().map(Value::String).collect();
        Some(wasm_store.store_std_value(Value::Array(keys).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

fn object_values(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let hashmap_object = std_value
            .try_unwrap_object_ref()
            .ok()?
            .try_unwrap_value_map_ref()
            .ok()?;

        let keys: Vec<Value> = hashmap_object.values().cloned().collect();
        Some(wasm_store.store_std_value(Value::Array(keys).into(), Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

fn array_len(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let std_value = wasm_store.get_std_value(descriptor)?;
        let array = std_value.try_unwrap_array_ref().ok()?;

        Some(array.len() as i32)
    }()
    .unwrap_or(-1)
}

fn array_get(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32, index_i32: i32) -> i32 {
    || -> Option<i32> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let index: usize = index_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();

        let value_ref = wasm_store
            .get_std_value(descriptor)?
            .try_project(|maybe_array| match maybe_array {
                Value::Array(arr) => arr.get(index).ok_or(()),
                _ => Err(()),
            })
            .ok()?;

        Some(wasm_store.store_std_value(value_ref, Some(descriptor)) as i32)
    }()
    .unwrap_or(-1)
}

fn array_set(
    mut caller: Caller<'_, WasmStore>,
    descriptor_i32: i32,
    index_i32: i32,
    value_i32: i32,
) {
    || -> Option<()> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let value_descriptor: usize = value_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let value_ref = wasm_store.get_std_value(value_descriptor)?;
        let mut array = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::Array(arr) => Some(arr.clone()),
            _ => None,
        }?;

        let index: usize = index_i32.try_into().ok().and_then(|index| {
            if index < array.len() {
                Some(index)
            } else {
                None
            }
        })?;

        array[index] = value_ref.as_ref().clone();

        wasm_store.set_std_value(descriptor, Value::Array(array).into());

        Some(())
    }();
}

fn array_append(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32, value_i32: i32) {
    || -> Option<()> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;
        let value_descriptor: usize = value_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let value_ref = wasm_store.get_std_value(value_descriptor)?;
        let mut array = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::Array(arr) => Some(arr.clone()),
            _ => None,
        }?;

        // PERF arrays could store ValueRefs too.
        array.push(value_ref.as_ref().clone());

        wasm_store.set_std_value(descriptor, Value::Array(array).into());

        Some(())
    }();
}

fn array_remove(mut caller: Caller<'_, WasmStore>, descriptor_i32: i32, index_i32: i32) {
    || -> Option<()> {
        let descriptor: usize = descriptor_i32.try_into().ok()?;

        let wasm_store = caller.data_mut();
        let mut array = match wasm_store.get_std_value(descriptor)?.as_ref() {
            Value::Array(arr) => Some(arr.clone()),
            _ => None,
        }?;

        let index: usize = index_i32.try_into().ok().and_then(|index| {
            if index < array.len() {
                Some(index)
            } else {
                None
            }
        })?;

        array.remove(index);

        wasm_store.set_std_value(descriptor, Value::Array(array).into());

        Some(())
    }();
}

// TODO maybe write a macro for this
impl FieldAsValue for Manga {
    fn field_as_value(&self, field: &str) -> Option<Value> {
        match field {
            "source_id" => Some(Value::String(self.source_id.clone())),
            "id" => Some(Value::String(self.id.clone())),
            "title" => self.title.clone().map(Value::String).or(Some(Value::Null)),
            "author" => self.author.clone().map(Value::String).or(Some(Value::Null)),
            "artist" => self.artist.clone().map(Value::String).or(Some(Value::Null)),
            "description" => self
                .description
                .clone()
                .map(Value::String)
                .or(Some(Value::Null)),
            "tags" => self
                .tags
                .clone()
                .map(|tags| {
                    Value::Array(tags.iter().map(|tag| Value::String(tag.clone())).collect())
                })
                .or(Some(Value::Null)),
            "cover_url" => self
                .cover_url
                .clone()
                .map(|url| Value::String(url.to_string()))
                .or(Some(Value::Null)),
            "url" => self
                .url
                .clone()
                .map(|url| Value::String(url.to_string()))
                .or(Some(Value::Null)),
            "status" => Some(Value::Int(self.status.clone() as i64)),
            "nsfw" => Some(Value::Int(self.nsfw.clone() as i64)),
            "viewer" => Some(Value::Int(self.viewer.clone() as i64)),
            "last_updated" => self.last_updated.map(Value::Date).or(Some(Value::Null)),
            "last_opened" => self.last_opened.map(Value::Date).or(Some(Value::Null)),
            "last_read" => self.last_read.map(Value::Date).or(Some(Value::Null)),
            "date_added" => self.date_added.map(Value::Date).or(Some(Value::Null)),
            _ => None,
        }
    }
}

impl FieldAsValue for MangaPageResult {
    fn field_as_value(&self, field: &str) -> Option<Value> {
        match field {
            "manga" => {
                let value_array = self
                    .manga
                    .iter()
                    .map(|m| Value::Object(ObjectValue::Manga(m.clone())))
                    .collect();

                Some(Value::Array(value_array))
            }
            "has_next_page" => Some(Value::Bool(self.has_next_page)),
            _ => None,
        }
    }
}

impl FieldAsValue for Filter {
    fn field_as_value(&self, field: &str) -> Option<Value> {
        match field {
            "type" => Some(Value::Int(FilterType::from(self) as i64)),
            "name" => Some(Value::String(self.name())),
            // FIXME i dont think this should be here but
            "value" => match &self {
                Filter::Title(title) => Some(Value::String(title.clone())),
            },
            _ => None,
        }
    }
}
