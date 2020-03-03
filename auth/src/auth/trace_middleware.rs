use super::State;
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    web, HttpMessage,
};
use futures::future::{ok, FutureExt, LocalBoxFuture, Ready};
use std::task::{Context, Poll};

pub struct Trace;

impl<S, B: 'static> Transform<S> for Trace
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>>,
    S::Future: 'static,
    S::Error: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type InitError = ();
    type Transform = TraceMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(TraceMiddleware { service })
    }
}

/// Signed cookie session middleware
pub struct TraceMiddleware<S> {
    service: S,
}

impl<S, B: 'static> Service for TraceMiddleware<S>
where
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
        if let Some(state) = req.extensions().get::<web::Data<State>>() {
            if let Err(_) = state.try_reload_tera() {
                log::info!("Failed to refresh tera");
            } else {
                log::info!("Tera refreshed");
            }
        }

        let fut = self.service.call(req);

        async move { fut.await }.boxed_local()
    }
}
