use crate::{source::wasm_store::Html, util::has_internet_connection};
use anyhow::{Context, Result};
use futures::executor;
use log::warn;
use num_enum::FromPrimitive;
use reqwest::{Method, Request};
use scraper::Html as ScraperHtml;

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

pub const DEFAULT_USER_AGENT: &str =
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
fn init(mut caller: Caller<'_, WasmStore>, method: i32) -> Result<i32> {
    let method = method
        .try_into()
        .map(AidokuHttpMethod::from_primitive)
        .unwrap_or_default();
    let wasm_store = caller.data_mut();

    // TODO maybe also return a mut reference in create_request to building state?
    // should help with type safety down below. or maybe not idk ig its fine
    let request_descriptor = wasm_store.create_request();
    let request = match wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?
    {
        RequestState::Building(building_state) => building_state,
        _ => anyhow::bail!("unexpected request state"),
    };

    request.method = Some(method.into());
    request
        .headers
        .insert("User-Agent".into(), DEFAULT_USER_AGENT.into());

    Ok(request_descriptor as i32)
}

#[aidoku_wasm_function]
fn close(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> Result<()> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    *request = RequestState::Closed;

    Ok(())
}

#[aidoku_wasm_function]
fn set_url(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    url: Option<String>,
) -> Result<()> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request_builder = match wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?
    {
        RequestState::Building(builder) => Some(builder),
        _ => None,
    }
    .context("request is not in building state")?;

    request_builder.url =
        Some(Url::parse(&url.context("url is required")?).context("invalid url")?);

    Ok(())
}

#[aidoku_wasm_function]
fn set_header(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    name: Option<String>,
    value: Option<String>,
) -> Result<()> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request_builder = match wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?
    {
        RequestState::Building(builder) => Some(builder),
        _ => None,
    }
    .context("request is not in building state")?;

    request_builder.headers.insert(
        name.context("header name is required")?,
        value.context("header value is required")?,
    );

    Ok(())
}

#[aidoku_wasm_function]
fn set_body(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    bytes: Option<Vec<u8>>,
) -> Result<()> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request_builder = match wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?
    {
        RequestState::Building(builder) => Some(builder),
        _ => None,
    }
    .context("request is not in building state")?;

    request_builder.body = Some(bytes.context("body bytes are required")?);

    Ok(())
}

#[aidoku_wasm_function]
fn set_rate_limit(_caller: Caller<'_, WasmStore>, _rate_limit: i32) -> Result<()> {
    todo!("rate-limit functions are not supported at the moment")
}

#[aidoku_wasm_function]
fn set_rate_limit_period(_caller: Caller<'_, WasmStore>, _rate_limit_period: i32) -> Result<()> {
    todo!("rate-limit functions are not supported at the moment")
}

#[aidoku_wasm_function]
fn send(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> Result<()> {
    let wasm_store = caller.data_mut();
    let cancellation_token = wasm_store.context.cancellation_token.clone();

    // HACK Before everything, we want to fail fast if no internet connection is available.
    // In theory, it would be easier to just let things fail naturally and move on
    // with our lives; but DNS resolution takes forever (~5s or so) when we have no connection
    // available - due to musl's `getaddrinfo()` call not realizing we have no connection and
    // timing out (EAI_AGAIN). The overhead of checking for a connection here seems worth it.
    let has_internet_connection =
        executor::block_on(cancellation_token.run_until_cancelled(has_internet_connection()))
            .context("failed to check internet connection")?;
    if !has_internet_connection {
        anyhow::bail!("no internet connection available");
    }

    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;

    let request_state = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    let request_builder = match request_state {
        RequestState::Building(ref builder) => Some(builder),
        _ => None,
    }
    .context("request is not in building state")?;

    let client = reqwest::Client::new();
    let request = Request::try_from(request_builder).context("failed to build request")?;

    let warn_cancellation = || {
        warn!(
            "request to {:?} was cancelled mid-flight!",
            &request_builder.url
        );
    };

    let response =
        match executor::block_on(cancellation_token.run_until_cancelled(client.execute(request))) {
            Some(response) => response.context("failed to execute request")?,
            _ => {
                warn_cancellation();
                anyhow::bail!("request was cancelled mid-flight");
            }
        };

    let response_data = ResponseData {
        url: response.url().clone(),
        headers: response.headers().clone(),
        status_code: response.status(),
        body: match executor::block_on(cancellation_token.run_until_cancelled(response.bytes())) {
            Some(bytes) => bytes
                .context("failed to read response bytes")
                .map(|bytes| bytes.to_vec())
                .ok(),
            _ => {
                warn_cancellation();
                anyhow::bail!("request was cancelled mid-flight while reading body");
            }
        },
        bytes_read: 0,
    };

    *request_state = RequestState::Sent(response_data);

    Ok(())
}

#[aidoku_wasm_function]
fn get_url(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> Result<i32> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    // FIXME allow getting URLs from sent requests
    let request_builder = match request {
        RequestState::Building(ref builder) => Some(builder),
        _ => None,
    }
    .context("request is not in building state")?;

    let url: String = request_builder.url.clone().context("url not set")?.into();

    Ok(wasm_store.store_std_value(Value::from(url).into(), None) as i32)
}

#[aidoku_wasm_function]
fn get_data_size(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> Result<i32> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    let response = match request {
        RequestState::Sent(response) => Some(response),
        _ => None,
    }
    .context("request is not in sent state")?;

    let bytes_left = response
        .body
        .as_ref()
        .context("response body not found")?
        .len()
        - response.bytes_read;

    Ok(bytes_left as i32)
}

#[aidoku_wasm_function]
fn get_data(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    buffer: i32,
    size: i32,
) -> Result<()> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let buffer: usize = buffer.try_into().context("invalid buffer")?;
    let size: usize = size.try_into().context("invalid size")?;

    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    let response = match request {
        RequestState::Sent(response) => Some(response),
        _ => None,
    }
    .context("request is not in sent state")?;

    let bytes = response.body.as_ref().context("response body not found")?;
    if response.bytes_read + size >= bytes.len() {
        let slice = bytes[response.bytes_read..response.bytes_read + size].to_owned();

        response.bytes_read += size;

        // FIXME technically we should do this before updating the size, but the
        // borrow checker gets angy >:(
        let memory = get_memory(&mut caller).context("failed to get wasm memory")?;
        write_bytes(&memory, &mut caller, &slice, buffer)
            .context("failed to write bytes to wasm memory")?;
    }

    Ok(())
}

#[aidoku_wasm_function]
fn get_header(
    mut caller: Caller<'_, WasmStore>,
    request_descriptor_i32: i32,
    name: Option<String>,
) -> Result<i32> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    let response = match request {
        RequestState::Sent(response) => Some(response),
        _ => None,
    }
    .context("request is not in sent state")?;

    let value: String = response
        .headers
        .get(name.context("header name is required")?)
        .context("header not found")?
        .to_str()
        .context("header value is not valid utf-8")?
        .into();

    Ok(wasm_store.store_std_value(Value::from(value).into(), None) as i32)
}

#[aidoku_wasm_function]
fn get_status_code(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> Result<i32> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    let response = match request {
        RequestState::Sent(response) => Some(response),
        _ => None,
    }
    .context("request is not in sent state")?;

    let status_code = response.status_code.as_u16() as i64;

    Ok(wasm_store.store_std_value(Value::from(status_code).into(), None) as i32)
}

#[aidoku_wasm_function]
fn json(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> Result<i32> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    let response = match request {
        RequestState::Sent(response) => Some(response),
        _ => None,
    }
    .context("request is not in sent state")?;

    // PERF If we remove the response from the state, we can parse this with ownership of the body,
    // which might enable some optimizations to be done by serde.
    // Check if Aidoku's source allows us to read from the response _after_ we have read it.
    let value: Value = serde_json::from_slice(
        response
            .body
            .as_ref()
            .context("response body not found")?
            .as_slice(),
    )
    .context("failed to parse json")?;

    Ok(wasm_store.store_std_value(value.into(), None) as i32)
}

#[aidoku_wasm_function]
fn html(mut caller: Caller<'_, WasmStore>, request_descriptor_i32: i32) -> Result<i32> {
    let request_descriptor: usize = request_descriptor_i32
        .try_into()
        .context("invalid request descriptor")?;
    let wasm_store = caller.data_mut();

    let request = wasm_store
        .get_mut_request(request_descriptor)
        .context("failed to get request state")?;
    let response = match request {
        RequestState::Sent(response) => Some(response),
        _ => None,
    }
    .context("request is not in sent state")?;

    // FIXME we should consider the encoding that came on the request
    let html_string: String =
        String::from_utf8(response.body.clone().context("response body not found")?)
            .context("response body is not valid utf-8")?;

    // FIXME this is duplicated from the html module. not sure it's really worth refactoring
    // but here's a note
    let document = ScraperHtml::parse_document(&html_string);
    let node_id = document.root_element().id();
    let html_element = HTMLElement {
        document: Html::from(document).into(),
        node_id,
        base_uri: response.url.clone().into(),
    };

    Ok(wasm_store.store_std_value(Value::from(vec![html_element]).into(), None) as i32)
}
