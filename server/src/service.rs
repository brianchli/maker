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
use tracing::error;

type Req<B> = hyper::Request<B>;
type Response = hyper::Response<BoxBody<Bytes, Infallible>>;

#[macro_use]
pub(crate) mod macros {

    #[macro_export]
    macro_rules! ok_or_http_response {
        ($expr:expr, $status:expr) => {
            match $expr {
                Ok(ok) => ok,
                Err(e) => {
                    error!("[{}] {}", $status, e);
                    return Ok($crate::service::http::error_response($status));
                }
            }
        };
    }

    #[macro_export]
    macro_rules! some_or_http_response {
        ($expr:expr, $reason:literal, $status:expr) => {
            match $expr {
                Some(ok) => ok,
                None => {
                    error!("[{}] {}", $status, $reason);
                    return Ok($crate::service::http::error_response($status));
                }
            }
        };
    }

    #[macro_export]
    macro_rules! some_or_err {
        ($expr:expr, $reason:literal) => {
            some_or_http_response!($expr, $reason, StatusCode::INTERNAL_SERVER_ERROR)
        };
    }

    #[macro_export]
    macro_rules! server_err {
        ($expr:expr) => {
            ok_or_http_response!($expr, StatusCode::INTERNAL_SERVER_ERROR)
        };
    }

    #[macro_export]
    macro_rules! bad_request {
        ($expr:expr) => {
            ok_or_http_response!($expr, StatusCode::BAD_REQUEST)
        };
    }
}

pub(crate) mod http {

    use super::Response as ServerResponse;
    use http_body_util::{BodyExt, Empty};
    use hyper::Response;
    use hyper::body::Bytes;
    use hyper::http::StatusCode;

    pub(crate) fn error_response(status_code: StatusCode) -> ServerResponse {
        let mut resp = Response::new(
            Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed(),
        );
        *resp.status_mut() = status_code;
        resp
    }
}

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
        let res = server_err!(
            conn.await
                .map(|_| http::error_response(StatusCode::INTERNAL_SERVER_ERROR))
        );
        Ok::<_, BoxError>(res)
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
