mod error;
mod server;
mod service;

use hyper::server::conn::http1;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use service::maker_run;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::info;

use crate::{
    server::{server_init, server_shutdown},
    service::middlewares,
};

#[tokio::main]
async fn main() -> Result<(), error::ServerError> {
    let addr = server_init()?;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let svc = TowerToHyperService::new(
        ServiceBuilder::new()
            .concurrency_limit(1)
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .layer(middlewares::HttpResponseLayer::new())
            .layer(middlewares::TimeoutLayer::new(180))
            .service(tower::service_fn(maker_run)),
    );

    let listener = TcpListener::bind(addr).await?;
    let http = http1::Builder::new();
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    let mut signal = std::pin::pin!(server_shutdown());

    info!("listening on {:?}", &addr);
    let (tx, _rx) = tokio::sync::broadcast::channel::<u8>(100);
    loop {
        tokio::select! {
            Ok((tcp, _)) = listener.accept() => {
                let fut = graceful.watch(http.serve_connection(TokioIo::new(tcp), svc.clone()));
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
