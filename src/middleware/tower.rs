use std::task::{Context, Poll};

use http::Request;
use tower::Service;
use tower_layer::Layer;
use tracing::instrument::Instrumented;

use crate::http_injector;

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

impl<S> Layer<S> for TraceLayer {
    type Service = TraceService<S>;

    fn layer(&self, service: S) -> Self::Service {
        TraceService { service }
    }
}

/// This service implements the Trace behavior
pub struct TraceService<S> {
    service: S,
}

impl<S, Body> Service<Request<Body>> for TraceService<S>
where
    S: Service<Request<Body>>,
{
    type Error = S::Error;
    type Future = Instrumented<S::Future>;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let span = http_injector::extract_opentelemetry_context_from_request(&request);

        tracing::Instrument::instrument(self.service.call(request), span)
    }
}
