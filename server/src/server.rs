use std::{net::SocketAddr, path::PathBuf};

use tokio::signal::unix::SignalKind;

use crate::{error, service::AppState};

pub fn server_init() -> Result<(AppState, SocketAddr), error::ServerError> {
    let port = std::env::var("BACKEND_PORT")
        .map_err(|_| error::ServerError::new("Missing backend port".into()))?;
    let ollama_port = std::env::var("OLLAMA_PORT")
        .map_err(|_| error::ServerError::new("Missing ollama backend port".into()))?;
    let specifications = PathBuf::from("/app/specifications");
    if !specifications
        .try_exists()
        .map_err(|_| error::ServerError::new("Unable find specifications directory".into()))?
    {
        return Err(error::ServerError::new(
            "Unable find specifications directory".into(),
        ));
    }

    let state = AppState::new(
        format!("http://ollama:{}/api/generate", ollama_port)
            .parse()
            .map_err(|_| error::ServerError::new("Unable to parse ollama uri".into()))?,
        specifications,
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

pub async fn server_shutdown() {
    let cntl_c = tokio::signal::ctrl_c();
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())
        .expect("unable to configure sigterm handler");
    tokio::select! {
        _ = cntl_c => {}
        _ = sigterm.recv() => {}
    }
}
