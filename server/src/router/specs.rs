use std::{fmt::Display, os::unix::ffi::OsStrExt};

use http_body_util::{Full, combinators::BoxBody};
use hyper::{Method, StatusCode, body::Body};
use tower::BoxError;

use crate::service::{AppState, Req, Response, error_response};

pub async fn specs_route<B>(state: AppState, req: Req<B>) -> Result<Response, BoxError>
where
    B: Body,
    B::Error: Display,
{
    if req.method() != Method::GET {
        return Ok(error_response(StatusCode::METHOD_NOT_ALLOWED));
    }
    let mut specs = tokio::fs::read_dir(state.specifications).await?;
    let mut specifications = vec![];
    while let Some(file) = specs.next_entry().await? {
        if file.path().extension().is_some_and(|s| s == "toml") {
            specifications.extend_from_slice(file.file_name().as_bytes());
            specifications.push(b'\n');
        }
    }
    let body = Full::from(specifications);
    Ok(hyper::Response::new(BoxBody::new(body)))
}
