use anyhow::{anyhow, Result};
use chrono::DateTime;
use memory_reader::{read_bytes, read_string};
use wasmi::{core::ValType, Caller, Extern, Memory, Val};

pub mod memory_reader;

pub fn get_memory<T>(caller: &mut Caller<'_, T>) -> Option<Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(memory)) => Some(memory),
        _ => None,
    }
}

pub trait FromWasmValues<T> {
    const WASM_VALUE_COUNT: usize;

    fn get_wasm_value_types() -> &'static [ValType];
    fn from_wasm_values(caller: &mut Caller<'_, T>, values: &[Val]) -> Self;
}

pub trait TryFromWasmValues<T>
where
    Self: Sized,
{
    const WASM_VALUE_COUNT: usize;

    fn get_wasm_value_types() -> &'static [ValType];
    fn try_from_wasm_values(caller: &mut Caller<'_, T>, values: &[Val]) -> Result<Self>;
}

impl<T> TryFromWasmValues<T> for String {
    const WASM_VALUE_COUNT: usize = 2;

    fn get_wasm_value_types() -> &'static [ValType] {
        &[ValType::I32, ValType::I32]
    }

    fn try_from_wasm_values(caller: &mut Caller<'_, T>, values: &[Val]) -> Result<Self> {
        let offset: usize = values[0]
            .i32()
            .ok_or_else(|| anyhow!("expected to receive a i32 as the offset argument"))?
            .try_into()?;
        let length: usize = values[1]
            .i32()
            .ok_or_else(|| anyhow!("expected to receive a i32 as the length argument"))?
            .try_into()
            .ok()
            .and_then(|length: usize| if length > 0 { Some(length) } else { None })
            .ok_or_else(|| anyhow!("expected the length argument to be strictly positive"))?;

        let memory = get_memory(caller).ok_or_else(|| anyhow!("could not get WASM memory"))?;
        read_string(&memory, caller, offset, length)
            .ok_or_else(|| anyhow!("could not read string from WASM memory"))
    }
}

impl<T> TryFromWasmValues<T> for DateTime<chrono_tz::Tz> {
    const WASM_VALUE_COUNT: usize = 1;

    fn get_wasm_value_types() -> &'static [ValType] {
        &[ValType::F64]
    }

    fn try_from_wasm_values(_caller: &mut Caller<'_, T>, values: &[Val]) -> Result<Self> {
        use chrono::TimeZone;
        let seconds_since_1970 = values[0]
            .f64()
            .ok_or_else(|| anyhow!("expected to receive a f64"))?
            .to_float();
        let full_seconds = seconds_since_1970.floor() as i64;
        let nanos_remainder = ((seconds_since_1970 - full_seconds as f64) * (10f64.powi(9))) as u32;
        let date_time: DateTime<chrono_tz::Tz> = chrono_tz::UTC
            .timestamp_opt(full_seconds, nanos_remainder)
            .single()
            .ok_or_else(|| {
                anyhow!("could not convert naive date time into date time with timestamp")
            })?;

        Ok(date_time)
    }
}

impl<T> TryFromWasmValues<T> for Vec<u8> {
    const WASM_VALUE_COUNT: usize = 2;

    fn get_wasm_value_types() -> &'static [ValType] {
        &[ValType::I32, ValType::I32]
    }

    fn try_from_wasm_values(caller: &mut Caller<'_, T>, values: &[Val]) -> Result<Self> {
        let offset: usize = values[0]
            .i32()
            .ok_or_else(|| anyhow!("expected to receive a i32 as the offset argument"))?
            .try_into()?;
        let length: usize = values[1]
            .i32()
            .ok_or_else(|| anyhow!("expected to receive a i32 as the length argument"))?
            .try_into()
            .ok()
            .and_then(|length: usize| if length > 0 { Some(length) } else { None })
            .ok_or_else(|| anyhow!("expected the length argument to be strictly positive"))?;

        let memory = get_memory(caller).ok_or_else(|| anyhow!("could not get WASM memory"))?;
        read_bytes(&memory, caller, offset, length)
            .ok_or_else(|| anyhow!("could not read bytes from WASM memory"))
    }
}

impl<T, U> FromWasmValues<T> for Option<U>
where
    U: TryFromWasmValues<T>,
{
    const WASM_VALUE_COUNT: usize = U::WASM_VALUE_COUNT;

    fn get_wasm_value_types() -> &'static [ValType] {
        U::get_wasm_value_types()
    }

    fn from_wasm_values(caller: &mut Caller<'_, T>, values: &[Val]) -> Self {
        U::try_from_wasm_values(caller, values).ok()
    }
}

// Some native WASM types implementations
impl<T> FromWasmValues<T> for i32 {
    const WASM_VALUE_COUNT: usize = 1;

    fn get_wasm_value_types() -> &'static [ValType] {
        &[ValType::I32]
    }

    fn from_wasm_values(_caller: &mut Caller<'_, T>, values: &[Val]) -> Self {
        values[0].i32().unwrap()
    }
}

impl<T> FromWasmValues<T> for i64 {
    const WASM_VALUE_COUNT: usize = 1;

    fn get_wasm_value_types() -> &'static [ValType] {
        &[ValType::I64]
    }

    fn from_wasm_values(_caller: &mut Caller<'_, T>, values: &[Val]) -> Self {
        values[0].i64().unwrap()
    }
}

impl<T> FromWasmValues<T> for f32 {
    const WASM_VALUE_COUNT: usize = 1;

    fn get_wasm_value_types() -> &'static [ValType] {
        &[ValType::F32]
    }

    fn from_wasm_values(_caller: &mut Caller<'_, T>, values: &[Val]) -> Self {
        values[0].f32().unwrap().to_float()
    }
}

impl<T> FromWasmValues<T> for f64 {
    const WASM_VALUE_COUNT: usize = 1;

    fn get_wasm_value_types() -> &'static [ValType] {
        &[ValType::F64]
    }

    fn from_wasm_values(_caller: &mut Caller<'_, T>, values: &[Val]) -> Self {
        values[0].f64().unwrap().to_float()
    }
}

pub trait ToWasmValue {
    const WASM_VALUE_TYPE: ValType;

    fn to_wasm_value(&self) -> Val;
}

impl ToWasmValue for i32 {
    const WASM_VALUE_TYPE: ValType = ValType::I32;

    fn to_wasm_value(&self) -> Val {
        Val::I32(*self)
    }
}
