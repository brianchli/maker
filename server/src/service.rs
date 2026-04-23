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
use std::convert::Infallible;
use tokio::net::TcpStream;
use tower::BoxError;

type Req<B> = hyper::Request<B>;
type Response = hyper::Response<BoxBody<Bytes, Infallible>>;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    ollama_uri: hyper::Uri,
}

impl AppState {
    pub(crate) fn new(ollama_uri: hyper::Uri) -> Self {
        Self { ollama_uri }
    }
}
pub async fn maker_run<B>(
    AppState { ollama_uri }: AppState,
    _req: Req<B>,
) -> Result<Response, BoxError> {
    let stream = server_err!(
        TcpStream::connect(format!(
            "{}:{}",
            some_or_err!(ollama_uri.host(), "missing uri host"),
            some_or_err!(ollama_uri.port(), "missing uri port"),
        ))
        .await
    );

    let io = TokioIo::new(stream);
    let (mut send, conn) = server_err!(http1::handshake(io).await);

    tokio::task::spawn(async move {
        Ok::<_, BoxError>(server_err!(
            conn.await
                .map(|_| http::error_response(StatusCode::INTERNAL_SERVER_ERROR))
        ))
    });

    // TOOO - replace this with serialisation of a specification
    let json = r#"
    {
        "model": "deepseek-v3.2:cloud",
        "prompt": "Why is the sky blue?",
        "stream": false
    }
    "#;

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
            .body(Full::<Bytes>::new(json.into()))
    );

    let res = server_err!(send.send_request(req).await);
    let (parts, body) = res.into_parts();
    let bytes = server_err!(body.collect().await).to_bytes();
    Ok(hyper::Response::from_parts(
        parts,
        BoxBody::new(Full::from(bytes)),
    ))
}
