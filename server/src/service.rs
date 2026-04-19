pub mod middlewares;

use std::{convert::Infallible, time::Duration};

use http_body_util::{Full, combinators::BoxBody};
use hyper::body::Bytes;
use tokio::signal::unix::SignalKind;

type Request = hyper::Request<hyper::body::Incoming>;
type Response = Result<hyper::Response<BoxBody<Bytes, Infallible>>, Infallible>;

pub async fn maker_run(req: Request) -> Response {
    Ok(hyper::Response::new(BoxBody::new(Full::new(Bytes::from(
        "Hello",
    )))))
}

pub async fn shutdown() {
    // Wait for the CTRL+C signal
    let cntl_c = tokio::signal::ctrl_c();
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate()).expect("unable to configure sigterm handler");
    tokio::select! {
        _ = cntl_c => {}
        _ = sigterm.recv() => {}
    }
}
