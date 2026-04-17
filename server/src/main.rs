mod error;
mod service;

use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use service::maker_run;
use tokio::net::TcpListener;
use tracing::info;

use crate::service::{middlewares, shutdown};

#[tokio::main]
async fn main() -> Result<(), error::ServerError> {
    let port = std::env::var("BACKEND_PORT")
        .map_err(|_| error::ServerError::new("Missing backend port"))?;

    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        port.parse()
            .map_err(|_| error::ServerError::new("Unable to parse port number"))?,
    ));

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let svc = TowerToHyperService::new(
        tower::ServiceBuilder::new()
            .concurrency_limit(1)
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .layer(middlewares::Timeout::new(1))
            .service(tower::service_fn(maker_run)),
    );

    let listener = TcpListener::bind(addr).await?;
    let http = http1::Builder::new();
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    let mut signal = std::pin::pin!(shutdown());

    info!("listening on {:?}", &addr);
    loop {
        tokio::select! {
            Ok((tcp, _)) = listener.accept() => {
                let io = TokioIo::new(tcp);
                let fut = graceful.watch(http.serve_connection(io, svc.clone()));
                tokio::spawn(async move {
                    if let Err(e) = fut.await {
                       info!("error occurred for request: {:?}", e);
                    }
                });
            },
            _ = &mut signal => {
                drop(listener);
                info!("exit signal received");
                break;
            }
        }
    }

    tokio::select! {
        _ = graceful.shutdown() => {
            info!("all connections closed");
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            info!("failed to close all connections");
        }
    }
    Ok(())
}
