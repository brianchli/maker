use std::net::SocketAddr;

use tokio::signal::unix::SignalKind;

use crate::error;

pub fn server_init() -> Result<SocketAddr, error::ServerError> {
    let port = std::env::var("BACKEND_PORT")
        .map_err(|_| error::ServerError::new("Missing backend port".into()))?;

    Ok(SocketAddr::from((
        [0, 0, 0, 0],
        port.parse()
            .map_err(|_| error::ServerError::new("Unable to parse port number".into()))?,
    )))
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
