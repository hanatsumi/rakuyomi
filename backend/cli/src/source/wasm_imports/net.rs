use std::sync::Arc;

use crate::util::has_internet_connection;
use anyhow::Result;
use futures::executor;
use num_enum::FromPrimitive;
use reqwest::Method;
use scraper::Html;
use serde_json::Value as JSONValue;
use url::Url;
use wasm_macros::{aidoku_wasm_function, register_wasm_function};
use wasm_shared::{get_memory, memory_reader::write_bytes};
use wasmi::{Caller, Linker};

use crate::source::wasm_store::{HTMLElement, RequestState, ResponseData, Value, WasmStore};

pub fn register_net_imports(linker: &mut Linker<WasmStore>) -> Result<()> {
    register_wasm_function!(linker, "net", "init", init)?;
    register_wasm_function!(linker, "net", "close", close)?;
    register_wasm_function!(linker, "net", "set_url", set_url)?;
    register_wasm_function!(linker, "net", "set_header", set_header)?;
    register_wasm_function!(linker, "net", "set_body", set_body)?;
    register_wasm_function!(linker, "net", "set_rate_limit", set_rate_limit)?;
    register_wasm_function!(
        linker,
        "net",
        "set_rate_limit_period",
        set_rate_limit_period
    )?;
    register_wasm_function!(linker, "net", "send", send)?;
    register_wasm_function!(linker, "net", "get_url", get_url)?;
    register_wasm_function!(linker, "net", "get_data_size", get_data_size)?;
    register_wasm_function!(linker, "net", "get_data", get_data)?;
    register_wasm_function!(linker, "net", "get_header", get_header)?;
    register_wasm_function!(linker, "net", "get_status_code", get_status_code)?;
    register_wasm_function!(linker, "net", "json", json)?;
    register_wasm_function!(linker, "net", "html", html)?;

    Ok(())
}

const DEFAULT_USER_AGENT: &'static str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:107.0) Gecko/20100101 Firefox/107.0";

#[derive(Debug, Default, FromPrimitive)]
#[repr(u8)]
enum AidokuHttpMethod {
    #[default]
    Get = 0,
    Post = 1,
    Head = 2,
    Put = 3,
    Delete = 4,
}

impl From<AidokuHttpMethod> for Method {
    fn from(value: AidokuHttpMethod) -> Self {
        match value {
            AidokuHttpMethod::Get => Method::GET,
            AidokuHttpMethod::Post => Method::POST,
            AidokuHttpMethod::Head => Method::HEAD,
            AidokuHttpMethod::Put => Method::PUT,
            AidokuHttpMethod::Delete => Method::DELETE,
        }
    }
}

#[aidoku_wasm_function]
fn init(mut caller: Caller<'_, WasmStore>, method: i32) -> i32 {
    let method = method
        .try_into()
        .map(|method| AidokuHttpMethod::from_primitive(method))
        .unwrap_or_default();
    let wasm_store = caller.data_mut();

    // TODO maybe also return a mut reference in create_request to building state?
    // should help with type safety down below. or maybe not idk ig its fine
    let request_descriptor = wasm_store.create_request();
    let request = match wasm_store.get_mut_request(request_descriptor).unwrap() {
        RequestState::Building(building_state) => building_state,
        _ => panic!("what the fuck"),
    };

    request.method = Some(method.into());
    request
        .headers
        .insert("User-Agent".into(), DEFAULT_USER_AGENT.into());

    request_descriptor as i32
}

#[aidoku_wasm_function]
fn close(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) {
    || -> Option<()> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        *request = RequestState::Closed;

        Some(())
    }();
}

#[aidoku_wasm_function]
fn set_url(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32, url: Option<String>) {
    || -> Option<()> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request_builder = match wasm_store.get_mut_request(request_descriptor)? {
            RequestState::Building(builder) => Some(builder),
            _ => None,
        }?;

        request_builder.url = Some(Url::parse(&url?).ok()?);

        Some(())
    }();
}

#[aidoku_wasm_function]
fn set_header(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    name: Option<String>,
    value: Option<String>,
) {
    || -> Option<()> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request_builder = match wasm_store.get_mut_request(request_descriptor)? {
            RequestState::Building(builder) => Some(builder),
            _ => None,
        }?;

        request_builder.headers.insert(name?, value?);

        Some(())
    }();
}

#[aidoku_wasm_function]
fn set_body(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    bytes: Option<Vec<u8>>,
) {
    || -> Option<()> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request_builder = match wasm_store.get_mut_request(request_descriptor)? {
            RequestState::Building(builder) => Some(builder),
            _ => None,
        }?;

        request_builder.body = Some(bytes?);

        Some(())
    }();
}

#[aidoku_wasm_function]
fn set_rate_limit(_caller: Caller<'_, WasmStore>, _rate_limit: i32) {
    todo!("rate-limit functions are not supported at the moment")
}

#[aidoku_wasm_function]
fn set_rate_limit_period(_caller: Caller<'_, WasmStore>, _rate_limit_period: i32) {
    todo!("rate-limit functions are not supported at the moment")
}

#[aidoku_wasm_function]
fn send(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) {
    || -> Option<()> {
        // HACK Before everything, we want to fail fast if no internet connection is available.
        // In theory, it would be easier to just let things fail naturally and move on
        // with our lives; but DNS resolution takes forever (~5s or so) when we have no connection
        // available - due to musl's `getaddrinfo()` call not realizing we have no connection and
        // timing out (EAI_AGAIN). The overhead of checking for a connection here seems worth it.
        let has_internet_connection = executor::block_on(has_internet_connection());
        if !has_internet_connection {
            return None;
        }

        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        let request_builder = match request {
            RequestState::Building(ref builder) => Some(builder),
            _ => None,
        }?;

        let client = reqwest::blocking::Client::new();
        let mut builder = client.request(
            request_builder.method.clone()?,
            request_builder.url.clone()?,
        );

        for (k, v) in request_builder.headers.iter() {
            builder = builder.header(k, v);
        }

        if let Some(body) = &request_builder.body {
            builder = builder.body(body.clone());
        }

        let response = builder.send().ok()?;
        let response_data = ResponseData {
            headers: response.headers().clone(),
            status_code: response.status(),
            body: response.bytes().ok().map(|bytes| bytes.to_vec()),
            bytes_read: 0,
        };

        *request = RequestState::Sent(response_data);

        Some(())
    }();
}

#[aidoku_wasm_function]
fn get_url(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        // FIXME allow getting URLs from sent requests
        let request_builder = match request {
            RequestState::Building(ref builder) => Some(builder),
            _ => None,
        }?;

        let url: String = request_builder.url.clone()?.into();

        Some(wasm_store.store_std_value(Value::String(url), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn get_data_size(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        let response = match request {
            RequestState::Sent(response) => Some(response),
            _ => None,
        }?;

        let bytes_left = response.body.clone()?.len() - response.bytes_read;

        Some(bytes_left as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn get_data(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    buffer: i32,
    size: i32,
) {
    || -> Option<()> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let buffer: usize = buffer.try_into().ok()?;
        let size: usize = size.try_into().ok()?;

        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        let response = match request {
            RequestState::Sent(response) => Some(response),
            _ => None,
        }?;

        let bytes = response.body.as_ref()?;
        if response.bytes_read + size >= bytes.len() {
            let slice = bytes[response.bytes_read..response.bytes_read + size].to_owned();

            response.bytes_read += size;

            // FIXME technically we should do this before updating the size, but the
            // borrow checker gets angy >:(
            let memory = get_memory(&mut caller)?;
            write_bytes(&memory, &mut caller, &slice, buffer)?;
        }

        Some(())
    }();
}

#[aidoku_wasm_function]
fn get_header(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    name: Option<String>,
) -> i32 {
    || -> Option<i32> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        let response = match request {
            RequestState::Sent(response) => Some(response),
            _ => None,
        }?;

        let value: String = response.headers.get(name?)?.to_str().ok()?.into();

        Some(wasm_store.store_std_value(Value::String(value), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn get_status_code(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        let response = match request {
            RequestState::Sent(response) => Some(response),
            _ => None,
        }?;

        let status_code = response.status_code.as_u16() as i64;

        Some(wasm_store.store_std_value(Value::Int(status_code), None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn json(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        let response = match request {
            RequestState::Sent(response) => Some(response),
            _ => None,
        }?;

        let json: JSONValue = serde_json::from_slice(response.body.clone()?.as_slice()).ok()?;
        let value: Value = json.try_into().ok()?;

        Some(wasm_store.store_std_value(value, None) as i32)
    }()
    .unwrap_or(-1)
}

#[aidoku_wasm_function]
fn html(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> i32 {
    || -> Option<i32> {
        let request_descriptor: usize = request_descriptor_i32.try_into().ok()?;
        let wasm_store = caller.data_mut();

        let request = wasm_store.get_mut_request(request_descriptor)?;
        let response = match request {
            RequestState::Sent(response) => Some(response),
            _ => None,
        }?;

        // FIXME we should consider the encoding that came on the request
        let html_string: String = String::from_utf8(response.body.clone()?).ok()?;

        // FIXME this is duplicated from the html module. not sure it's really worth refactoring
        // but here's a note
        let fragment = Arc::new(Html::parse_fragment(&html_string));
        let node_id = fragment.root_element().id();
        let html_element = HTMLElement {
            document: fragment,
            node_id,
        };

        Some(wasm_store.store_std_value(Value::HTMLElements(vec![html_element]), None) as i32)
    }()
    .unwrap_or(-1)
}
