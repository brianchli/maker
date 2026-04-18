pub mod middlewares;

use std::{convert::Infallible, time::Duration};

use http_body_util::{Full, combinators::BoxBody};
use hyper::body::Bytes;

type Request = hyper::Request<hyper::body::Incoming>;
type Response = Result<hyper::Response<BoxBody<Bytes, Infallible>>, Infallible>;

pub async fn maker_run(_req: Request) -> Response {
    tokio::time::sleep(Duration::from_secs(5)).await;
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
