use std::fmt::Display;

use http_body_util::{Full, combinators::BoxBody};
use hyper::{Method, StatusCode, body::Body};
use tower::BoxError;

use crate::{
    service::{AppState, Req, Response, error_response},
    some_or_err,
};

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
            specifications.push(
                some_or_err!(
                    file.file_name().to_string_lossy().strip_suffix(".toml"),
                    "unable to parse toml specification due to extension"
                )
                .to_owned(),
            )
        }
    }
    let body = Full::from(serde_json::to_vec(&specifications)?);
    Ok(hyper::Response::new(BoxBody::new(body)))
}
