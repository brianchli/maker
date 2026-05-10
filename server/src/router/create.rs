use std::fmt::Display;

use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    Method, Request, StatusCode,
    body::{Body, Bytes},
    client::conn::http1,
    header::{CONTENT_TYPE, HOST},
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tower::BoxError;
use tracing::{event, info};

use crate::{
    bad_request, server_err,
    service::{
        AppState, Filetype, OllamaResponse, Req, ResolvedPrompt, Response, TomlSpec, error_response,
    },
    some_or_err,
};

use crate::router::OllamaEndpoints;

pub(crate) async fn create_route<B>(state: AppState, req: Req<B>) -> Result<Response, BoxError>
where
    B: Body,
    B::Error: Display,
{
    if req.method() != Method::POST {
        return Err("method not allowed".into());
    }
    if !req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("application/json"))
    {
        return Ok(error_response(StatusCode::UNSUPPORTED_MEDIA_TYPE));
    }
    maker_run(state, req).await
}

async fn maker_run<B>(
    AppState { ollama_uri, mut specifications, default_model }: AppState,
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
        if let Err(e) = conn.await {
            tracing::error!("connection error: {}", e);
        }
    });

    let (parts, body) = req.into_parts();
    let filetype: Filetype =
        bad_request!(serde_json::from_slice(&server_err!(body.collect().await).to_bytes()));

    specifications.push(match &filetype {
        Filetype::Make { .. } => "make.toml",
        Filetype::Cmake { .. } => "cmake.toml",
        Filetype::Readme { .. } => "readme.toml",
        Filetype::Docker { .. } => "docker.toml",
        Filetype::Spec { .. } => "spec.toml",
    });

    let spec: TomlSpec = server_err!(toml::from_slice(
        server_err!(tokio::fs::read(&specifications).await).as_slice()
    ));

    info!("ollama request for {}", &filetype);
    let mut prompt = server_err!(ResolvedPrompt::try_from((spec, filetype)));
    event!(target:module_path!(),tracing::Level::DEBUG,"{:?}", &prompt);
    if prompt.model.is_none() {
        prompt.model = Some(default_model);
    }
    let req = server_err!(
        Request::builder()
            .method(parts.method)
            .uri(OllamaEndpoints::Generate)
            .header(
                HOST,
                some_or_err!(ollama_uri.authority(), "malformed authority for ollama path")
                    .as_str()
            )
            .header(CONTENT_TYPE, r#"application/json"#)
            .body(Full::<Bytes>::new(server_err!(serde_json::to_string(&prompt)).into()))
    );

    let res = server_err!(http.send_request(req).await);
    let (_parts, body) = res.into_parts();
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
    let mut response = hyper::Response::new(BoxBody::new(body));
    response.headers_mut().insert(CONTENT_TYPE, "text/plain; charset=utf-8".parse().unwrap());
    Ok(response)
}
