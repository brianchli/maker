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
    service::{AppState, OllamaResponse, Req, ResolvedPrompt, Response, TomlSpec, error_response},
    some_or_err,
};
use crate::{router::OllamaEndpoints, service::File_t};

pub async fn create_route<B>(state: AppState, req: Req<B>) -> Result<Response, BoxError>
where
    B: Body,
    B::Error: Display,
{
    match req.method() {
        &Method::POST => Ok(maker_run(state, req).await?),
        _ => Err("".into()),
    }
}

async fn maker_run<B>(
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
    let stream = server_err!(TcpStream::connect(ollama_uri.to_string()).await);

    let io = TokioIo::new(stream);
    let (mut http, conn) = server_err!(http1::handshake(io).await);

    tokio::task::spawn(async move {
        Ok::<_, BoxError>(server_err!(
            conn.await
                .map(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR))
        ))
    });

    let (parts, body) = req.into_parts();
    let file_t: File_t = bad_request!(serde_json::from_slice(
        &server_err!(body.collect().await).to_bytes()
    ));

    specifications.push(match &file_t {
        File_t::Make { .. } => "make.toml",
        File_t::Cmake { .. } => "cmake.toml",
        File_t::Readme { .. } => "readme.toml",
        File_t::Docker { .. } => "docker.toml",
        File_t::Spec { .. } => "spec.toml",
    });

    let spec: TomlSpec = server_err!(toml::from_slice(
        server_err!(tokio::fs::read(&specifications).await).as_slice()
    ));

    info!("ollama request for {}", &file_t);
    let mut prompt = server_err!(ResolvedPrompt::try_from((spec, file_t)));
    event!(target:module_path!(),tracing::Level::DEBUG,"{:?}", &prompt);
    prompt.model.get_or_insert("qwen3.5:cloud".into());
    let req = server_err!(
        Request::builder()
            .method(parts.method)
            .uri(OllamaEndpoints::Generate)
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
