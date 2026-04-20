use std::time::Duration;

use hyper::body::Incoming;
use tower::timeout::Timeout;
use tracing::Span;

use crate::service::{
    Req,
    middlewares::tower::conditional_impl::{ConditionalService, ConditionalServiceLayer},
};

pub struct TimeoutLayer<F> {
    duration: Duration,
    layer: ConditionalServiceLayer<F>,
}

impl<F> TimeoutLayer<F>
where
    F: Clone,
{
    pub fn from_secs(seconds: u64, f: F) -> Self {
        Self {
            duration: Duration::from_secs(seconds),
            layer: ConditionalServiceLayer::new(f),
        }
    }

    pub fn from_mins(minutes: u64, f: F) -> Self {
        Self {
            duration: Duration::from_mins(minutes),
            layer: ConditionalServiceLayer::new(f),
        }
    }
}

type TimeoutService<S, S1, F, F1> = ConditionalService<S, S1, F, F1>;
impl<S, F> tower::Layer<S> for TimeoutLayer<F>
where
    S: Clone,
    F: Clone,
{
    type Service = TimeoutService<S, tower::timeout::Timeout<S>, F, fn(&Req<Incoming>) -> Span>;

    fn layer(&self, inner: S) -> Self::Service {
        let other = inner.clone();

        fn timeout_span<B>(req: &Req<B>) -> Span {
            tracing::info_span!(
                "timeout",
                path = %req.uri().path()
            )
        }
        Self::Service::new(
            inner,
            Timeout::new(other, self.duration),
            self.layer.func().clone(),
            timeout_span,
        )
    }
}
