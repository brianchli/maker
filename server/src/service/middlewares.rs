#![allow(dead_code)]
mod tower;
pub use tower::HttpResponseLayer;
pub use tower::RateLimiter;
pub use tower::TimeoutLayer;

use crate::service::AppState;
use crate::service::Req;

type PredicateFn<B> = fn(Req<B>) -> (bool, Req<B>);
type PredicateFnWithState<B> = fn(AppState, Req<B>) -> (bool, Req<B>);

pub(crate) enum Predicate<B> {
    Stateless(PredicateFn<B>),
    Stateful(PredicateFnWithState<B>),
}

pub(crate) enum GuardDecision<B> {
    Continue(Req<B>),
    Bypass(Req<B>),
}

pub(crate) mod policy {

    use hyper::body::{Body, Incoming};
    use tracing::event;

    pub const BYPASS: fn(hyper::Request<Incoming>) -> GuardDecision<Incoming> = never;
    pub const ALWAYS: fn(hyper::Request<Incoming>) -> GuardDecision<Incoming> = always;

    use crate::service::{
        AppState, Req,
        middlewares::{GuardDecision, Predicate, PredicateFn, PredicateFnWithState},
    };

    impl<B> From<(Option<AppState>, Predicate<B>)> for MiddlewareGuardBuilder<B>
    where
        B: Body,
    {
        fn from(value: (Option<AppState>, Predicate<B>)) -> Self {
            Self::new(value.0, value.1)
        }
    }

    impl<B> From<MiddlewareGuardBuilder<B>> for (Option<AppState>, Predicate<B>)
    where
        B: Body,
    {
        fn from(value: MiddlewareGuardBuilder<B>) -> Self {
            (value.state, value.pred)
        }
    }

    pub struct MiddlewareGuardBuilder<B> {
        state: Option<AppState>,
        pred: Predicate<B>,
    }

    impl<B> MiddlewareGuardBuilder<B>
    where
        B: Body,
    {
        pub(crate) fn new(state: Option<AppState>, pred: Predicate<B>) -> Self {
            Self { state, pred }
        }

        pub(crate) fn state(self, state: AppState) -> Self {
            Self {
                state: Some(state),
                ..self
            }
        }

        pub(crate) fn predicate(self, pred: PredicateFn<B>) -> Self {
            Self {
                pred: Predicate::Stateless(pred),
                ..self
            }
        }

        pub(crate) fn generate(self) -> Result<impl FnMut(Req<B>) -> GuardDecision<B>, String> {
            let (mut state, pred) = self.into();
            if let Predicate::Stateful(_) = pred
                && state.is_none()
            {
                return Err("Stateful predicate requires AppState".into());
            };

            Ok(move |req| match pred {
                Predicate::Stateless(f) => (custom_fn(f))(req),
                Predicate::Stateful(f) => (custom_fn_with_state(
                    state
                        .take()
                        .expect("Stateful predicate without an AppState"),
                    f,
                ))(req),
            })
        }
    }

    pub(crate) fn custom_fn<B>(pred: PredicateFn<B>) -> impl Fn(Req<B>) -> GuardDecision<B>
    where
        B: hyper::body::Body,
    {
        move |req| -> GuardDecision<B> {
            match pred(req) {
                (true, request) => GuardDecision::Continue(request),
                (_, request) => GuardDecision::Bypass(request),
            }
        }
    }

    pub(crate) fn custom_fn_with_state<B>(
        state: AppState,
        pred: PredicateFnWithState<B>,
    ) -> impl Fn(Req<B>) -> GuardDecision<B>
    where
        B: hyper::body::Body,
    {
        move |req| -> GuardDecision<B> {
            match pred(state.clone(), req) {
                (true, request) => GuardDecision::Continue(request),
                (_, request) => GuardDecision::Bypass(request),
            }
        }
    }

    fn never<B>(req: hyper::Request<B>) -> GuardDecision<B>
    where
        B: hyper::body::Body,
    {
        let (parts, body) = req.into_parts();
        event!(target:module_path!(),tracing::Level::INFO,"middleware bypassed");
        GuardDecision::Bypass(hyper::Request::from_parts(parts, body))
    }

    fn always<B>(req: hyper::Request<B>) -> GuardDecision<B>
    where
        B: hyper::body::Body,
    {
        let (parts, body) = req.into_parts();
        event!(target:module_path!(),tracing::Level::INFO,"middleware executed");
        GuardDecision::Continue(hyper::Request::from_parts(parts, body))
    }
}
