mod create;
mod models;

use std::fmt::Display;
use std::str::FromStr;

use hyper::Uri;
use hyper::body::Body;
use tower::BoxError;

use crate::router::create::create_route;
use crate::router::models::models_route;

use crate::service::Response;
use crate::{
    bad_request,
    service::{AppState, Req},
};

#[derive(Debug)]
pub(crate) enum OllamaEndpoints {
    Generate,
    Tags,
}

impl Display for OllamaEndpoints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            OllamaEndpoints::Generate => write!(f, "/api/generate"),
            OllamaEndpoints::Tags => todo!(),
        }
    }
}

impl From<OllamaEndpoints> for Uri {
    fn from(value: OllamaEndpoints) -> Self {
        match value {
            OllamaEndpoints::Generate => Uri::from_str("/api/generate")
                .expect("conversion of valid static string for OllamaEndpoints::Generate"),
            OllamaEndpoints::Tags => Uri::from_str("/api/tags")
                .expect("conversion of valid static string for OllamaEndpoints::Tags"),
        }
    }
}

pub(crate) async fn router<B>(state: AppState, req: Req<B>) -> Result<Response, BoxError>
where
    B: Body,
    B::Error: Display,
{
    Ok(match req.uri().path() {
        "/create" => bad_request!(create_route(state, req).await),
        "/models" => bad_request!(models_route(state, req).await),
        _ => bad_request!(Err("")),
    })
}
