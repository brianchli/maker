use std::{
    fmt::{Debug, Display},
    sync::Arc,
    task::Poll::{Pending, Ready},
};

use futures_util::future::Either;
use hyper::body::Body;
use tower::{BoxError, Service};
use tracing::{Instrument, Span, event, info, instrument::Instrumented};

use crate::service::{
    Req,
    middlewares::{
        GuardDecision::{Bypass, Continue},
        PredicateFn,
    },
};

#[derive(Clone)]
pub struct ConditionalServiceLayer<F> {
    predicate: Arc<F>,
}

impl<F> ConditionalServiceLayer<F> {
    pub fn new(f: F) -> Self {
        Self {
            predicate: Arc::new(f),
        }
    }

    pub fn func(&self) -> Arc<F> {
        self.predicate.clone()
    }
}

#[derive(Clone)]
pub struct ConditionalService<S1, S2, F, F1> {
    name: &'static str,
    service_1: S1,
    service_2: S2,
    predicate: F,
    span_generator: F1,
}

impl<S1, S2, F, F1> ConditionalService<S1, S2, F, F1> {
    pub fn new(
        name: &'static str,
        service_1: S1,
        service_2: S2,
        predicate: F,
        span_generator: F1,
    ) -> Self {
        Self {
            name,
            service_1,
            service_2,
            predicate,
            span_generator,
        }
    }
}

impl<S1, S2, F, F1> Display for ConditionalService<S1, S2, F, F1> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

type TracedFuture<T> = Instrumented<T>;
impl<S1, S2, B, B1, F1> Service<Req<B>> for ConditionalService<S1, S2, PredicateFn<B>, F1>
where
    B: Body,
    B1: Body,
    S1: Service<Req<B>, Response = hyper::Response<B1>>,
    S2: Service<Req<B>, Response = hyper::Response<B1>>,
    S1::Error: Into<BoxError> + Debug,
    S2::Error: Into<BoxError> + Debug,
    S1::Future: Future<Output = Result<hyper::Response<B1>, BoxError>>,
    S2::Future: Future<Output = Result<hyper::Response<B1>, BoxError>>,
    F1: Fn(&Req<B>) -> Span,
{
    type Response = hyper::Response<B1>;
    type Error = BoxError;
    type Future = Either<TracedFuture<S1::Future>, TracedFuture<S2::Future>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let a = self.service_1.poll_ready(cx);
        if let Ready(Err(e)) = a {
            info!("{} error: {:?}", self, e);
            return Ready(Err(e.into()));
        };
        let b = self.service_2.poll_ready(cx);
        if let Ready(Err(e)) = b {
            info!("{} error: {:?}", self, e);
            return Ready(Err(e.into()));
        };
        event!(target:module_path!(),tracing::Level::DEBUG,"poll ready");
        match (a, b) {
            (Ready(_), Ready(_)) => Ready(Ok(())),
            _ => Pending,
        }
    }
    fn call(&mut self, req: Req<B>) -> Self::Future {
        let span = (self.span_generator)(&req);
        event!(target:module_path!(),tracing::Level::DEBUG, "call");
        match (self.predicate)(req) {
            Continue(request) => Either::Right(self.service_2.call(request).instrument(span)),
            Bypass(request) => Either::Left(self.service_1.call(request).instrument(span)),
        }
    }
}
