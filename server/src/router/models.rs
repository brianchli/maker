use std::fmt::Display;

use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{
    Method, Request, StatusCode,
    body::{Body, Bytes},
    client::conn::http1,
    header::{CONTENT_TYPE, HOST},
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tower::BoxError;

use crate::{
    router::OllamaEndpoints,
    server_err,
    service::{AppState, Req, Response, error_response},
    some_or_err,
};

pub async fn list_models<B>(
    AppState {
        ollama_uri,
        specifications: _,
    }: AppState,
    req: Req<B>,
) -> Result<Response, BoxError>
where
    B: Body,
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
                .map(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR))
        ))
    });

    let req = server_err!(
        Request::builder()
            .method(req.into_parts().0.method)
            .uri(OllamaEndpoints::Tags)
            .header(
                HOST,
                some_or_err!(
                    ollama_uri.authority(),
                    "malformed authority for ollama path"
                )
                .as_str()
            )
            .header(CONTENT_TYPE, r#"application/json"#)
            .body(Empty::<Bytes>::new())
    );

    let res = server_err!(http.send_request(req).await);
    let (parts, body) = res.into_parts();
    let bytes = server_err!(body.collect().await).to_bytes();
    let body: Full<Bytes> = Full::from(bytes);
    Ok(hyper::Response::from_parts(parts, BoxBody::new(body)))
}

pub async fn models_route<B>(state: AppState, req: Req<B>) -> Result<Response, BoxError>
where
    B: Body,
    B::Error: Display,
{
    match req.method() {
        &Method::GET => list_models(state, req).await,
        _ => Err("".into()),
    }
}
