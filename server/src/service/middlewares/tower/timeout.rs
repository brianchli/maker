use std::{
    fmt::Debug,
    task::Poll::{Pending, Ready},
    time::Duration,
};

use pin_project_lite::pin_project;
use tower::BoxError;

#[derive(Clone)]
pub struct TimeoutLayer {
    duration: u64,
}

#[derive(Clone)]
pub struct TimeoutService<S> {
    duration: u64,
    inner: S,
}

impl TimeoutLayer {
    pub fn new(duration: u64) -> Self {
        Self { duration }
    }
}

impl<S> tower::Layer<S> for TimeoutLayer {
    type Service = TimeoutService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            duration: self.duration,
            inner,
        }
    }
}

pin_project! {
#[derive(Debug)]
pub struct ConditionalTimeoutFuture<T> {
    skip: bool,
    #[pin]
    response: T,
    #[pin]
    sleep: tokio::time::Sleep
    }
}

impl<T> ConditionalTimeoutFuture<T> {
    fn new(response: T, duration: u64, skip: bool) -> Self {
        Self {
            skip,
            response,
            sleep: tokio::time::sleep(Duration::from_secs(duration)),
        }
    }
}

impl<F, T, E> Future for ConditionalTimeoutFuture<F>
where
    F: Future<Output = Result<T, E>>,
    E: Into<BoxError>,
{
    type Output = Result<T, BoxError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let poll = match this.response.poll(cx) {
            Ready(v) => return Ready(v.map_err(Into::into)),
            Pending => Pending,
        };

        if !*this.skip {
            match this.sleep.poll(cx) {
                Ready(_) => Ready(Err("Elapsed".into())), // TODO: custom error type
                Pending => Pending,
            }
        } else {
            poll
        }
    }
}

type Req<B> = hyper::Request<B>;
impl<S, B> tower::Service<Req<B>> for TimeoutService<S>
where
    S: tower::Service<Req<B>>,
    S::Error: Into<BoxError>,
{
    type Response = S::Response;
    type Error = BoxError;
    type Future = ConditionalTimeoutFuture<S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Req<B>) -> Self::Future {
        let (parts, body) = req.into_parts();
        ConditionalTimeoutFuture::new(
            self.inner.call(hyper::Request::from_parts(parts, body)),
            self.duration,
            false,
        )
    }
}
