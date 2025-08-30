use anyhow::Result;
use opentelemetry::{global, sdk::trace as sdktrace};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_telemetry() -> Result<()> {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "https://otel-collector.amazonaws.com".to_string())
        );

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            sdktrace::config()
                .with_resource(sdktrace::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", "todo-backend"),
                    opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
                .with_sampler(
                    match std::env::var("RUST_ENV").as_deref() {
                        Ok("production") => sdktrace::Sampler::TraceIdRatioBased(0.1),
                        _ => sdktrace::Sampler::AlwaysOn,
                    }
                ),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().json())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        );

    subscriber.init();

    Ok(())
}

pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}