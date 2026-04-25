use std::time::Duration;

use hyper::body::Incoming;
use tower::limit::{RateLimit, rate::Rate};
use tracing::Span;

use crate::service::{Req, middlewares::PredicateFn};

use super::conditional_impl::ConditionalService;

#[derive(Clone)]
pub struct RateLimiter {
    requests: u64,
    duration: Duration,
    f: PredicateFn<Incoming>,
}

impl RateLimiter {
    pub fn new(requests: u64, seconds: u64, f: PredicateFn<Incoming>) -> Self {
        Self {
            requests,
            duration: Duration::from_secs(seconds),
            f,
        }
    }
}

type RateLimitService<S, S1, F, F1> = ConditionalService<S, S1, F, F1>;
impl<S> tower::Layer<S> for RateLimiter
where
    S: Clone,
{
    type Service =
        RateLimitService<S, RateLimit<S>, PredicateFn<Incoming>, fn(&Req<Incoming>) -> Span>;

    fn layer(&self, inner: S) -> Self::Service {
        let other = inner.clone();

        fn ratelimit_span<B>(_req: &Req<B>) -> Span {
            Span::current()
        }

        Self::Service::new(
            "ratelimit",
            inner,
            RateLimit::new(other, Rate::new(self.requests, self.duration)),
            self.f.clone(),
            ratelimit_span,
        )
    }
}
