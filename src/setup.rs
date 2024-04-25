use std::{env, error::Error};

use opentelemetry::{global, logs::LogError, metrics::MetricsError, trace::TraceError, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{logs::Config, runtime, Resource};
use tracing_core::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use crate::trace_id::OpenTelemetryTracingBridge;

/// Sets up tracing, metrics and logging via otlp exporter.
/// The service name can be configured using the env var `SERVICE_NAME`,
/// otherwise the cargo name will be used. By default, everything is exported to `http://localhost:4317`.
/// This can be changed via env var `OTEL_EXPORTER_OTLP_ENDPOINT`.
///
/// This should generally be the first statement of any server binary's main
/// function.
pub fn setup() -> Result<(), Box<dyn Error>> {
    let service = env::var("SERVICE_NAME").unwrap_or(env!("CARGO_PKG_NAME").to_string());
    let service: &'static str = service.leak();
    let endpoint =
        env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or("http://localhost:4317".to_string());
    let endpoint: &'static str = endpoint.leak();

    init_metrics(service, endpoint)?;
    // needs to run before init_tracer
    init_logs(service, endpoint)?;
    init_tracer(service, endpoint)?;

    tracing::info!("starting server");
    Ok(())
}

fn init_tracer(service: &'static str, endpoint: &'static str) -> Result<(), TraceError> {
    global::set_text_map_propagator(opentelemetry_jaeger_propagator::Propagator::new());
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                service,
            )])),
        )
        .install_batch(runtime::Tokio)?;

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let log_layer = OpenTelemetryTracingBridge::new(&global::logger_provider());
    Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
        }))
        .with(telemetry)
        .with(log_layer)
        .init();

    Ok(())
}

fn init_metrics(service: &'static str, endpoint: &'static str) -> Result<(), MetricsError> {
    let _meter = opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            service,
        )]))
        .build()?;

    Ok(())
}

fn init_logs(service: &'static str, endpoint: &'static str) -> Result<(), LogError> {
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_log_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                service,
            )])),
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .install_batch(runtime::Tokio)?;

    Ok(())
}

pub fn teardown() {
    global::shutdown_logger_provider();
    global::shutdown_tracer_provider();
}
