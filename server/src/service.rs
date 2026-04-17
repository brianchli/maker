pub mod middlewares;
use std::convert::Infallible;

use http_body_util::{Full, combinators::BoxBody};
use hyper::body::Bytes;

pub async fn maker_run(
    _req: hyper::Request<hyper::body::Incoming>,
) -> Result<hyper::Response<BoxBody<Bytes, Infallible>>, Infallible> {
    Ok(hyper::Response::new(BoxBody::new(Full::new(Bytes::from(
        "Hello",
    )))))
}

pub async fn shutdown() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
