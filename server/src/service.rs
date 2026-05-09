#[macro_use]
mod macros;
mod http;
pub(crate) mod middlewares;
mod specification;

pub(crate) use http::error_response;

use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use std::{convert::Infallible, path::PathBuf};

pub(crate) use crate::service::specification::prompt::{Filetype, ResolvedPrompt, TomlSpec};
use serde::Deserialize;

pub(crate) type Req<B> = hyper::Request<B>;
pub(crate) type Response = hyper::Response<BoxBody<Bytes, Infallible>>;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(crate) struct OllamaResponse {
    pub(crate) response: String,
    pub(crate) done: bool,
    pub(crate) model: String,
    pub(crate) created_at: String,
    pub(crate) done_reason: String,
    pub(crate) thinking: Option<String>,
    pub(crate) total_duration: u64,
    pub(crate) prompt_eval_count: u64,
    pub(crate) eval_count: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    pub(crate) ollama_uri: hyper::Uri,
    pub(crate) specifications: PathBuf,
    pub(crate) default_model: String,
}

impl AppState {
    pub(crate) fn new(
        ollama_uri: hyper::Uri,
        specifications: PathBuf,
        default_model: String,
    ) -> Self {
        Self {
            ollama_uri,
            specifications,
            default_model,
        }
    }
}
