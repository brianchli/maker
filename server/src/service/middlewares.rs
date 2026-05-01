mod tower;
use std::sync::Arc;

pub use tower::HttpErrResolver;
pub use tower::RateLimiter;
pub use tower::TimeoutLayer;

use crate::service::Req;

pub(crate) type PredicateFn<B> = Arc<dyn Fn(Req<B>) -> GuardDecision<B> + Send + Sync + 'static>;

pub enum GuardDecision<B> {
    Continue(Req<B>),
    Bypass(Req<B>),
}

pub(crate) mod policy {

    use std::sync::Arc;

    use hyper::body::Incoming;

    use crate::service::middlewares::GuardDecision::{self, Continue, Bypass};

    #[allow(non_snake_case)]
    pub(crate) fn ALWAYS()
    -> Arc<dyn Fn(hyper::Request<Incoming>) -> GuardDecision<Incoming> + Send + Sync> {
        Arc::new(|req| Continue(req))
    }

    #[allow(non_snake_case)]
    pub(crate) fn BYPASS()
    -> Arc<dyn Fn(hyper::Request<Incoming>) -> GuardDecision<Incoming> + Send + Sync> {
        Arc::new(|req| Bypass(req))
    }
}
