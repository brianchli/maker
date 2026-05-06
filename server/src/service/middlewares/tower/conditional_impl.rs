use std::{
    fmt::Debug,
    future::Future,
    task::{Context, Poll, Poll::Ready, ready},
};

use futures_util::future::Either;
use hyper::{
    Response,
    body::{Body, Incoming},
};
use tower::{BoxError, Service};
use tracing::{Instrument, Span, instrument::Instrumented};

use crate::service::{
    Req,
    middlewares::{
        GuardDecision::{Bypass, Continue},
        PredicateFn,
    },
};

#[derive(Clone)]
pub struct ConditionalService<S1, S2> {
    service_1: S1,
    service_2: S2,
    predicate: PredicateFn<Incoming>,
    span_generator: fn(&Req<Incoming>) -> Span,
}

impl<S1, S2> ConditionalService<S1, S2> {
    pub fn new(
        service_1: S1,
        service_2: S2,
        predicate: PredicateFn<Incoming>,
        span_generator: fn(&Req<Incoming>) -> Span,
    ) -> Self {
        Self {
            service_1,
            service_2,
            predicate,
            span_generator,
        }
    }
}

type TracedFuture<T> = Instrumented<T>;
impl<S1, S2, B1> Service<Req<Incoming>> for ConditionalService<S1, S2>
where
    B1: Body,
    S1: Service<Req<Incoming>, Response = Response<B1>>,
    S2: Service<Req<Incoming>, Response = Response<B1>>,
    S1::Error: Into<BoxError> + Debug,
    S2::Error: Into<BoxError> + Debug,
    S1::Future: Future<Output = Result<Response<B1>, BoxError>>,
    S2::Future: Future<Output = Result<Response<B1>, BoxError>>,
{
    type Response = Response<B1>;
    type Error = BoxError;
    type Future = Either<TracedFuture<S1::Future>, TracedFuture<S2::Future>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match ready!(self.service_1.poll_ready(cx)) {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!("error: {:?}", e);
                return Ready(Err(e.into()));
            }
        }
        match ready!(self.service_2.poll_ready(cx)) {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!("error: {:?}", e);
                return Ready(Err(e.into()));
            }
        }
        tracing::debug!("poll ready");
        Ready(Ok(()))
    }

    fn call(&mut self, req: Req<Incoming>) -> Self::Future {
        let span = (self.span_generator)(&req);
        tracing::debug!(parent: &span, "call");
        match (self.predicate)(req) {
            Bypass(req) => Either::Left(self.service_1.call(req).instrument(span)),
            Continue(req) => Either::Right(self.service_2.call(req).instrument(span)),
        }
    }
}
