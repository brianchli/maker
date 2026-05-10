use super::Response as ServerResponse;
use http_body_util::{BodyExt, Empty};
use hyper::Response;
use hyper::body::Bytes;
use hyper::http::StatusCode;

pub(crate) fn error_response(status_code: StatusCode) -> ServerResponse {
    let mut resp = Response::new(Empty::<Bytes>::new().map_err(|never| match never {}).boxed());
    *resp.status_mut() = status_code;
    resp
}
