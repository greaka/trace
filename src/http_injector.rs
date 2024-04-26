use http::{HeaderName, Request};
use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Injects the current [`opentelemetry::Context`] into a [`Request`]
/// headers to allow propagation downstream.
pub fn inject_opentelemetry_context_into_request<T>(request: &mut Request<T>) -> &mut Request<T> {
    let context = Span::current().context();

    global::get_text_map_propagator(|injector| {
        injector.inject_context(&context, &mut RequestInjector::new(request))
    });

    request
}

/// Constructs a [`opentelemetry::Context`] from [`Request`] headers
/// and assigns parent to the returned [`Span`].
#[track_caller]
pub fn extract_opentelemetry_context_from_request<T: HttpHeaderProvider>(request: &T) -> Span {
    let context = global::get_text_map_propagator(|extractor| {
        extractor.extract(&RequestExtractor::new(request))
    });

    let span = tracing::info_span!("request");
    span.set_parent(context);

    span
}

// "traceparent" => https://www.w3.org/TR/trace-context/#trace-context-http-headers-format

/// Injector used via opentelemetry propagator to tell the extractor how to
/// insert the "traceparent" header value. This will allow the propagator to
/// inject opentelemetry context into a standard data structure. Will basically
/// insert a "traceparent" string value
/// "{version}-{trace_id}-{span_id}-{trace_flags}" of the spans context into the
/// headers. Listeners can then re-hydrate the context to add additional spans
/// to the same trace.
struct RequestInjector<'a, T> {
    request: &'a mut Request<T>,
}

impl<'a, T> RequestInjector<'a, T> {
    pub fn new(request: &'a mut Request<T>) -> Self {
        RequestInjector { request }
    }
}

impl<'a, T> Injector for RequestInjector<'a, T> {
    fn set(&mut self, key: &str, value: String) {
        let Ok(key) = key.parse::<HeaderName>() else {
            tracing::debug!(%key, "failed to parse header name");
            return;
        };
        let Ok(value) = value.parse() else {
            tracing::debug!(%value, "failed to parse header value");
            return;
        };
        self.request.headers_mut().insert(key, value);
    }
}

struct RequestExtractor<'a, T: HttpHeaderProvider> {
    headers: &'a T,
}

impl<'a, T: HttpHeaderProvider> RequestExtractor<'a, T> {
    pub fn new(headers: &'a T) -> Self {
        RequestExtractor { headers }
    }
}

impl<'a, T: HttpHeaderProvider> Extractor for RequestExtractor<'a, T> {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key)
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.keys().collect()
    }
}

pub trait HttpHeaderProvider {
    fn get(&self, key: &str) -> Option<&str>;

    fn keys(&self) -> impl Iterator<Item = &str>;
}

impl<T> HttpHeaderProvider for Request<T> {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers().get(key).and_then(|x| x.to_str().ok())
    }

    fn keys(&self) -> impl Iterator<Item = &str> {
        self.headers().keys().map(|x| x.as_str())
    }
}
