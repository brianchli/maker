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

use crate::service::specification::prompt::{Filetype, ResolvedPrompt, TomlSpec};

type Req<B> = hyper::Request<B>;
type Response = hyper::Response<BoxBody<Bytes, Infallible>>;

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

    specifications.push(match file_t {
        Filetype::Make { .. } => "make.toml",
        Filetype::Cmake { .. } => "cmake.toml",
        Filetype::Readme { .. } => "readme.toml",
        Filetype::Docker { .. } => "docker.toml",
    });

    let spec: TomlSpec = bad_request!(toml::from_slice(
        server_err!(tokio::fs::read(&specifications).await).as_slice()
    ));

    let mut prompt: ResolvedPrompt = (spec, file_t).try_into()?;
    prompt.model.get_or_insert("deepseek-v3.2:cloud".into());

    let path = ollama_uri.path();
    let req = bad_request!(
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
                bad_request!(serde_json::to_string(&prompt)).into()
            ))
    );

    let res = server_err!(http.send_request(req).await);
    let (parts, body) = res.into_parts();
    let bytes = server_err!(body.collect().await).to_bytes();
    Ok(hyper::Response::from_parts(
        parts,
        BoxBody::new(Full::from(bytes)),
    ))
}
