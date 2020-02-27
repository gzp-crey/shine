use super::{SignedCookie, SignedCookieOptions};
use actix_service::Service;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    HttpMessage,
};
use futures::future::{FutureExt, LocalBoxFuture};
use std::task::{Context, Poll};

/// Signed cookie session middleware
pub struct SignedCookieMiddleware<O, C, S>
where
    O: SignedCookieOptions,
    C: 'static,
{
    pub(crate) service: S,
    pub(crate) inner: SignedCookie<O, C>,
}

impl<O, C, S, B: 'static> Service for SignedCookieMiddleware<O, C, S>
where
    O: SignedCookieOptions,
    C: 'static,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>>,
    S::Future: 'static,
    S::Error: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let inner = self.inner.clone();
        let data = inner.load(&mut req);
        req.extensions_mut().insert(data.clone());

        let fut = self.service.call(req);

        async move {
            fut.await.map(|mut res| {
                inner.store(data, &mut res);
                res
            })
        }
        .boxed_local()
    }
}
