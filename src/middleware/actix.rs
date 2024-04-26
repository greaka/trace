use std::{future, future::Ready};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpRequest,
};
use tracing::instrument::Instrumented;

use crate::{http_injector, http_injector::HttpHeaderProvider};

/// Injects tracing data to route handlers.
///
/// Generally, the middleware should be used on every http route, this usually
/// means that it can be registered globally and in the last position, to be the
/// first to run, even before general logging layers.
///
/// The `TraceLayer` will not log http requests. For that, another solution
/// needs to be added additionally.
///
/// ```ignore
/// let app = Router::new()
///     .route("/foo", get(|| async {}))
///     .route("/bar", get(|| async {}))
///     .layer(TraceLayer);
/// ```
pub struct TraceLayer;

impl<S, B> Transform<S, ServiceRequest> for TraceLayer
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Response = ServiceResponse<B>;
    type Transform = TraceService<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ready(Ok(TraceService { service }))
    }
}

/// This service implements the Trace behavior
pub struct TraceService<S> {
    service: S,
}

impl<S, Body> Service<ServiceRequest> for TraceService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<Body>, Error = Error>,
    S::Future: 'static,
    Body: 'static,
{
    type Error = Error;
    type Future = Instrumented<S::Future>;
    type Response = ServiceResponse<Body>;

    // This service is ready when its next service is ready
    forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let span = http_injector::extract_opentelemetry_context_from_request(request.request());

        tracing::Instrument::instrument(self.service.call(request), span)
    }
}

impl HttpHeaderProvider for HttpRequest {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers().get(key).and_then(|x| x.to_str().ok())
    }

    fn keys(&self) -> impl Iterator<Item = &str> {
        self.headers().keys().map(|x| x.as_str())
    }
}
