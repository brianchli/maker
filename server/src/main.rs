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
    service::middlewares::{self, policy},
};

#[tokio::main]
async fn main() -> Result<(), error::ServerError> {
    let (state, addr) = server_init()?;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .with_target(false)
        .init();

    let svc = TowerToHyperService::new(
        ServiceBuilder::new()
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .buffer(2000)
            .layer(middlewares::RateLimiter::new(10, 10, policy::ALWAYS))
            .concurrency_limit(2000)
            .layer(middlewares::HttpResponseLayer::new())
            .layer(middlewares::TimeoutLayer::from_mins(3, policy::ALWAYS))
            .service(tower::service_fn(move |req| {
                let appstate = state.clone();
                async move { maker_run(appstate, req).await }
            })),
    );

    let listener = TcpListener::bind(addr).await?;
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    let mut signal = std::pin::pin!(server_shutdown());

    info!("listening on {:?}", &addr);
    let (tx, _rx) = tokio::sync::broadcast::channel::<u8>(100);
    loop {
        tokio::select! {
            Ok((tcp, _)) = listener.accept() => {
                let fut = graceful.watch(http1::Builder::new().serve_connection(TokioIo::new(tcp), svc.clone()));
                let mut shutdown = tx.subscribe();
               tokio::spawn(async move {
                    // Solves the issue with Higher Order Trait Bounds:
                    // https://github.com/rust-lang/rust/issues/102211
                    let fut = always_send::AlwaysSend::new(fut);
                    tokio::select!{
                        _ = shutdown.recv() => { },
                        _ = fut => { }
                    };
                });
            },
            _ = &mut signal => { drop(listener);
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
