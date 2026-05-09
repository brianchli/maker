use std::{net::SocketAddr, path::PathBuf};

use hyper::{HeaderMap, Request, body::Incoming};
use tokio::signal::unix::SignalKind;

use crate::error;
use crate::service::{AppState, Req};

pub(crate) fn server_init() -> Result<(AppState, SocketAddr), error::ServerError> {
    let port = std::env::var("BACKEND_PORT")
        .map_err(|_| error::ServerError::new("Missing server backend port".into()))?;

    let ollama_port = std::env::var("OLLAMA_PORT")
        .map_err(|_| error::ServerError::new("Missing ollama backend port".into()))?;

    let default_model = std::env::var("DEFAULT_OLLAMA_MODEL")
        .map_err(|_| error::ServerError::new("Missing default ollama model fallback".into()))?;

    let specifications = PathBuf::from("/app/specifications");

    if !specifications
        .try_exists()
        .map_err(|_| error::ServerError::new("Unable to find specifications directory".into()))?
    {
        return Err(error::ServerError::new(
            "Unable to find specifications directory".into(),
        ));
    }

    let state = AppState::new(
        format!("http://ollama:{}", ollama_port)
            .parse()
            .map_err(|_| error::ServerError::new("Unable to parse ollama uri".into()))?,
        specifications,
        default_model,
    );

    Ok((
        state,
        SocketAddr::from((
            [0, 0, 0, 0],
            port.parse()
                .map_err(|_| error::ServerError::new("Unable to parse port number".into()))?,
        )),
    ))
}

pub(crate) enum RequestOrigin {
    Internal,
    External,
}

fn is_public_host(uri: &hyper::http::Uri) -> bool {
    uri.host().is_some_and(|val| val == "maker.bidn.dev")
}

fn is_cloudflare_proxied(headers: &HeaderMap) -> bool {
    // server will only be accessible via cloudflare proxy
    headers.get("cf-connecting-ip").is_some()
}

pub(crate) fn is_private_request(req: Req<Incoming>) -> (RequestOrigin, Req<Incoming>) {
    let (parts, body) = req.into_parts();
    let is_public = is_public_host(&parts.uri);
    let is_cloudflare = is_cloudflare_proxied(&parts.headers);
    match (is_public, is_cloudflare) {
        (false, false) => (RequestOrigin::Internal, Request::from_parts(parts, body)),
        (false, _) | (_, false) => {
            tracing::warn!(
                host = parts.uri.host().unwrap_or("null"),
                ip = parts
                    .headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("null"),
                r#"unexpected request src combination: public={is_public}, cloudflare={is_cloudflare}"#
            );
            (RequestOrigin::External, Request::from_parts(parts, body))
        }
        _ => (RequestOrigin::External, Request::from_parts(parts, body)),
    }
}

pub(crate) async fn server_shutdown() {
    let cntl_c = tokio::signal::ctrl_c();
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())
        .expect("unable to configure sigterm handler");
    tokio::select! {
        _ = cntl_c => {}
        _ = sigterm.recv() => {}
    }
}
