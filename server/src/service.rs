#[macro_use]
mod macros;
mod http;
pub mod middlewares;
mod specification;

use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    Method, Request, StatusCode,
    body::Bytes,
    client::conn::http1,
    header::{CONTENT_TYPE, HOST},
};
use hyper_util::rt::TokioIo;
use std::{convert::Infallible, fmt::Display, path::PathBuf};
use tokio::net::TcpStream;
use tower::BoxError;
use tracing::info;

use crate::service::specification::prompt::{Filetype, ResolvedPrompt, TomlSpec};
use serde::Deserialize;

type Req<B> = hyper::Request<B>;
type Response = hyper::Response<BoxBody<Bytes, Infallible>>;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(crate) struct OllamaResponse {
    pub(crate) response: String,
    pub(crate) done: bool,
    pub(crate) model: String,
    pub(crate) created_at: String,
    pub(crate) done_reason: String,
    pub(crate) thinking: Option<String>,
    pub(crate) total_duration: u64,
    pub(crate) prompt_eval_count: u64,
    pub(crate) eval_count: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    ollama_uri: hyper::Uri,
    specifications: PathBuf,
}

impl AppState {
    pub(crate) fn new(ollama_uri: hyper::Uri, specifications: PathBuf) -> Self {
        Self {
            ollama_uri,
            specifications,
        }
    }
}

pub async fn maker_run<B>(
    AppState {
        ollama_uri,
        mut specifications,
    }: AppState,
    req: Req<B>,
) -> Result<Response, BoxError>
where
    B: hyper::body::Body,
    B::Error: Display,
{
    let stream = server_err!(
        TcpStream::connect(format!(
            "{}:{}",
            some_or_err!(ollama_uri.host(), "missing uri host"),
            some_or_err!(ollama_uri.port(), "missing uri port"),
        ))
        .await
    );

    let io = TokioIo::new(stream);
    let (mut http, conn) = server_err!(http1::handshake(io).await);

    tokio::task::spawn(async move {
        Ok::<_, BoxError>(server_err!(
            conn.await
                .map(|_| http::error_response(StatusCode::INTERNAL_SERVER_ERROR))
        ))
    });

    let (_parts, body) = req.into_parts();
    let file_t: Filetype = bad_request!(serde_json::from_slice(
        &server_err!(body.collect().await).to_bytes()
    ));

    specifications.push(match &file_t {
        Filetype::Make { .. } => "make.toml",
        Filetype::Cmake { .. } => "cmake.toml",
        Filetype::Readme { .. } => "readme.toml",
        Filetype::Docker { .. } => "docker.toml",
        Filetype::Spec { .. } => "spec.toml",
    });

    let spec: TomlSpec = server_err!(toml::from_slice(
        server_err!(tokio::fs::read(&specifications).await).as_slice()
    ));

    info!("ollama request for {}", &file_t);
    let mut prompt = server_err!(ResolvedPrompt::try_from((spec, file_t)));

    prompt.model.get_or_insert("qwen3.5:cloud".into());
    let path = ollama_uri.path();
    let req = server_err!(
        Request::builder()
            .method(Method::POST)
            .uri(path)
            .header(
                HOST,
                some_or_err!(
                    ollama_uri.authority(),
                    "malformed authority for ollama path"
                )
                .as_str()
            )
            .header(CONTENT_TYPE, r#"application/json"#)
            .body(Full::<Bytes>::new(
                server_err!(serde_json::to_string(&prompt)).into()
            ))
    );

    let res = server_err!(http.send_request(req).await);
    let (parts, body) = res.into_parts();
    let bytes = server_err!(body.collect().await).to_bytes();
    let OllamaResponse {
        response,
        model,
        created_at,
        total_duration,
        prompt_eval_count,
        eval_count,
        ..
    }: OllamaResponse = server_err!(serde_json::from_slice(&bytes));

    info!(
    created_at = %created_at,
    model = %model,
    prompt_size = %prompt_eval_count,
    eval_count = %eval_count,
    sec_elapsed= %total_duration/ 1_000_000_000,
    ms_elapsed= %(total_duration % 1_000_000_000) / 1_000_000,
    "ollama response received"
    );

    let body: Full<Bytes> = Full::from(response.into_bytes());
    Ok(hyper::Response::from_parts(parts, BoxBody::new(body)))
}
