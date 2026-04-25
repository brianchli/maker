use std::time::Duration;

use hyper::body::Incoming;
use tower::limit::{RateLimit, rate::Rate};
use tracing::Span;

use crate::service::Req;

use super::conditional_impl::{ConditionalService, ConditionalServiceLayer};

#[derive(Clone)]
pub struct RateLimiter<F> {
    requests: u64,
    duration: Duration,
    layer: ConditionalServiceLayer<F>,
}

impl<F: Clone> RateLimiter<F>
where
    F: Clone,
{
    pub fn new(requests: u64, seconds: u64, f: F) -> Self {
        Self {
            requests,
            duration: Duration::from_secs(seconds),
            layer: ConditionalServiceLayer::new(f),
        }
    }
}

type RateLimitService<S, S1, F, F1> = ConditionalService<S, S1, F, F1>;
impl<S, F> tower::Layer<S> for RateLimiter<F>
where
    S: Clone,
    F: Clone,
{
    type Service = RateLimitService<S, RateLimit<S>, F, fn(&Req<Incoming>) -> Span>;

    fn layer(&self, inner: S) -> Self::Service {
        let other = inner.clone();

        fn ratelimit_span<B>(_req: &Req<B>) -> Span {
            Span::current()
        }

        Self::Service::new(
            inner,
            RateLimit::new(other, Rate::new(self.requests, self.duration)),
            self.layer.func().clone(),
            ratelimit_span,
        )
    }
}
