use std::{net::SocketAddr, path::PathBuf};

use hyper::{Request, body::Incoming};
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

pub(crate) fn is_private_request(req: Req<Incoming>) -> (RequestOrigin, Req<Incoming>) {
    let (parts, body) = req.into_parts();
    let origin = classify_origin(
        parts.uri.host(),
        parts.headers.get("cf-connecting-ip").is_some(),
    );
    (origin, Request::from_parts(parts, body))
}

fn classify_origin(host: Option<&str>, is_cloudflare: bool) -> RequestOrigin {
    match (host.is_some_and(|h| h == "maker.bidn.dev"), is_cloudflare) {
        (false, false) => RequestOrigin::Internal,
        (public, cloudflare) => {
            tracing::warn!(
                host = host.unwrap_or("null"),
                public = public,
                cloudflare = cloudflare,
                "unexpected request src combination"
            );
            RequestOrigin::External
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_request_no_public_host_no_cloudflare() {
        let origin = classify_origin(Some("maker.app.io"), false);
        assert!(matches!(origin, RequestOrigin::Internal));
    }

    #[test]
    fn external_request_public_host_no_cloudflare() {
        let origin = classify_origin(Some("maker.bidn.dev"), false);
        assert!(matches!(origin, RequestOrigin::External));
    }

    #[test]
    fn external_request_internal_host_with_cloudflare() {
        let origin = classify_origin(Some("maker.app.io"), true);
        assert!(matches!(origin, RequestOrigin::External));
    }

    #[test]
    fn external_request_public_host_with_cloudflare() {
        let origin = classify_origin(Some("maker.bidn.dev"), true);
        assert!(matches!(origin, RequestOrigin::External));
    }

    #[test]
    fn internal_request_no_host_no_cloudflare() {
        let origin = classify_origin(None, false);
        assert!(matches!(origin, RequestOrigin::Internal));
    }
}
