use std::io;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::client::legacy::{Client, Error as HyperError};
use hyperlocal::{UnixClientExt, Uri};
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Deserialize)]
struct Request {
    socket_path: PathBuf,
    path: String,
    method: String,
    headers: HashMap<String, String>,
    body: String,
    timeout_seconds: f64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum RequestResult {
    #[serde(rename = "ERROR")]
    Error { message: String },
    #[serde(rename = "RESPONSE")]
    Response { status: u16, body: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(io::stderr)
                .with_target(true),
        )
        .init();

    let mut request_json = String::new();
    std::io::stdin().read_line(&mut request_json)?;

    let request: Request = serde_json::from_str(request_json.as_str())?;

    let client = Client::unix();

    let timeout_duration = Duration::from_secs_f64(request.timeout_seconds);
    let response_future = client.request(request.into());
    let request_result: RequestResult = match timeout(timeout_duration, response_future).await {
        Ok(request_result) => RequestResult::from_hyper(request_result).await,
        Err(e) => RequestResult::Error {
            message: e.to_string(),
        },
    };

    println!("{}", serde_json::to_string(&request_result).unwrap());

    Ok(())
}

impl From<Request> for HyperRequest<Full<Bytes>> {
    fn from(value: Request) -> Self {
        let uri = Uri::new(value.socket_path, value.path.as_str());
        let mut request_builder = HyperRequest::builder()
            .uri(uri)
            .method(value.method.as_str());

        for (k, v) in value.headers {
            request_builder = request_builder.header(k, v);
        }

        request_builder.body(Full::from(value.body)).unwrap()
    }
}

type HyperRequestResult = Result<HyperResponse<hyper::body::Incoming>, HyperError>;

impl RequestResult {
    async fn from_hyper(value: HyperRequestResult) -> RequestResult {
        match value {
            Err(e) => Self::Error {
                message: e.to_string(),
            },
            Ok(response) => {
                let status = response.status().as_u16();
                let body = match response.collect().await {
                    Ok(body) => String::from_utf8(body.to_bytes().to_vec()).unwrap(),
                    Err(e) => {
                        return Self::Error {
                            message: e.to_string(),
                        }
                    }
                };

                Self::Response { status, body }
            }
        }
    }
}
