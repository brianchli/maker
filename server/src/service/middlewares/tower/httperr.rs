use std::{
    convert::Infallible,
    fmt::{Debug, Display},
    pin::Pin,
    task::{Poll::Ready, ready},
};

use futures_util::StreamExt;
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hyper::{body::Bytes, header::HOST};
use pin_project_lite::pin_project;
use tracing::info;

pub struct HttpErrResponseLayer {}
#[derive(Clone)]
pub struct HttpErrService<S> {
    inner: S,
}

impl HttpErrResponseLayer {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S> tower::Layer<S> for HttpErrResponseLayer {
    type Service = HttpErrService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service { inner }
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct HttpErrFuture<F> {
        #[pin]
        response: F,
    }
}

impl<F> HttpErrFuture<F> {
    fn new(response: F) -> Self {
        Self { response }
    }
}

impl<F, B, E> Future for HttpErrFuture<F>
where
    F: Future<Output = Result<hyper::Response<B>, E>>,
    B: hyper::body::Body<Data = Bytes, Error = Infallible> + Send + Sync + 'static,
    E: Display,
{
    type Output = Result<hyper::Response<BoxBody<Bytes, Infallible>>, E>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let result = ready!(this.response.poll(cx));
        match result {
            Ok(resp) => {
                let (parts, body) = resp.into_parts();
                // this handles network back pressure for us.
                let stream = body
                    .into_data_stream()
                    .map(|res| res.map(hyper::body::Frame::data));
                let body = StreamBody::new(stream);
                let body = BoxBody::new(body);
                Ready(Ok(hyper::Response::from_parts(parts, body)))
            }
            Err(e) => {
                let body = BoxBody::new(Full::new(Bytes::from(e.to_string())));
                Ready(Ok(hyper::Response::new(body)))
            }
        }
    }
}

type Req<B> = hyper::Request<B>;
impl<S, B1> tower::Service<Req<B1>> for HttpErrService<S>
where
    S: tower::Service<Req<B1>, Response = hyper::Response<BoxBody<Bytes, Infallible>>>,
    S::Error: Display,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = HttpErrFuture<S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map(Into::into)
    }

    fn call(&mut self, req: Req<B1>) -> Self::Future {
        let (parts, body) = req.into_parts();

        if let Some(host) = parts.headers.get(HOST) {
            if let Ok(host) = host.to_str() {
                info!("Incoming request from [{}]", host);
            };
        };

        HttpErrFuture::new(self.inner.call(hyper::Request::from_parts(parts, body)))
    }
}
