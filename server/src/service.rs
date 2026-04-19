pub mod middlewares;
mod specification;

use std::convert::Infallible;

use http_body_util::{Full, combinators::BoxBody};
use hyper::body::Bytes;
use tokio::signal::unix::SignalKind;

type Req<B> = hyper::Request<B>;
type Response = Result<hyper::Response<BoxBody<Bytes, Infallible>>, Infallible>;

pub async fn maker_run(req: Req<impl hyper::body::Body>) -> Response {
    Ok(hyper::Response::new(BoxBody::new(Full::new(Bytes::from(
        "Hello",
    )))))
}
