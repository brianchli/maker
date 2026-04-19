mod error;
mod service;

use std::{net::SocketAddr, time::Duration};

use hyper::server::conn::http1;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use service::maker_run;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::info;

use crate::service::{middlewares, shutdown};

#[tokio::main]
async fn main() -> Result<(), error::ServerError> {
    let port = std::env::var("BACKEND_PORT")
        .map_err(|_| error::ServerError::new("Missing backend port".into()))?;

    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        port.parse()
            .map_err(|_| error::ServerError::new("Unable to parse port number".into()))?,
    ));

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let svc = ServiceBuilder::new()
        .concurrency_limit(1)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(middlewares::HttpResponseLayer::new())
        .layer(middlewares::TimeoutLayer::new(Duration::from_secs(180)))
        .service(tower::service_fn(maker_run));

    let svc = TowerToHyperService::new(svc);

    let listener = TcpListener::bind(addr).await?;
    let http = http1::Builder::new();
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    let mut signal = std::pin::pin!(shutdown());
    info!("listening on {:?}", &addr);
    let (tx, _rx) = tokio::sync::broadcast::channel::<u8>(100);
    loop {
        tokio::select! {
            Ok((tcp, _)) = listener.accept() => {
                let io = TokioIo::new(tcp);
                let fut = graceful.watch(http.serve_connection(io, svc.clone()));
                let mut shutdown = tx.subscribe();
                tokio::spawn(async move {
                    tokio::select!{
                        _ = shutdown.recv() => {},
                        _ = fut => {}
                    };
                });
            },
            _ = &mut signal => {
                drop(listener);
                info!("exit signal received");
                // we currently only send an exit signal
                let _ = tx.send(0);
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
