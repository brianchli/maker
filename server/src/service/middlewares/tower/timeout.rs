use std::time::Duration;

use hyper::{
    body::Incoming,
    header::{HOST, HeaderValue},
};
use tower::timeout::Timeout;
use tracing::Span;

use crate::service::{
    Req,
    middlewares::{PredicateFn, tower::conditional_impl::ConditionalService},
};

pub struct TimeoutLayer {
    duration: Duration,
    f: PredicateFn<Incoming>,
}

impl TimeoutLayer {
    pub fn from_secs(seconds: u64, f: PredicateFn<Incoming>) -> Self {
        Self {
            duration: Duration::from_secs(seconds),
            f,
        }
    }

    pub fn from_mins(minutes: u64, f: PredicateFn<Incoming>) -> Self {
        Self {
            duration: Duration::from_mins(minutes),
            f,
        }
    }
}

const EMPTY_HOST: &str = "";
type TimeoutService<S, S1, F, F1> = ConditionalService<S, S1, F, F1>;
impl<S> tower::Layer<S> for TimeoutLayer
where
    S: Clone,
{
    type Service = TimeoutService<
        S,
        tower::timeout::Timeout<S>,
        PredicateFn<Incoming>,
        fn(&Req<Incoming>) -> Span,
    >;

    fn layer(&self, inner: S) -> Self::Service {
        let other = inner.clone();

        fn timeout_span<B>(req: &Req<B>) -> Span {
            let empty_header = HeaderValue::from_static(EMPTY_HOST);
            let host = req.headers().get(HOST).unwrap_or(&empty_header);
            tracing::info_span!(
                "timeout",
                path = %req.uri().path(),
                host = %host.to_str().unwrap_or(EMPTY_HOST),
                method = %req.method().as_str(),
            )
        }
        Self::Service::new(
            "timeout",
            inner,
            Timeout::new(other, self.duration),
            self.f.clone(),
            timeout_span,
        )
    }
}
