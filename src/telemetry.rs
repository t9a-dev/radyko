use opentelemetry::{
    KeyValue, global,
    trace::{Span, Tracer, TracerProvider},
};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt};

pub static RADYKO_TRACER: std::sync::OnceLock<opentelemetry::global::BoxedTracer> =
    std::sync::OnceLock::new();
pub static SERVICE_NAME: &str = "radyko";

pub fn get_radyko_tracer(service_name: &str) -> &'static opentelemetry::global::BoxedTracer {
    RADYKO_TRACER.get_or_init(|| opentelemetry::global::tracer(service_name.to_owned()))
}

pub fn init_telemetry(
    service_name: &str,
    level_arg: Option<&str>,
) -> opentelemetry_sdk::trace::SdkTracerProvider {
    let (tracer, provider) = init_tracer(service_name);
    let telemetry = OpenTelemetryLayer::new(tracer);
    let default_env_filter_directive = if cfg!(debug_assertions) {
        LevelFilter::INFO.into()
    } else {
        LevelFilter::ERROR.into()
    };
    let env_filter = EnvFilter::builder()
        .with_default_directive(default_env_filter_directive)
        .parse(level_arg.unwrap_or(""))
        .unwrap();
    let timer_format = tracing_subscriber::fmt::time::LocalTime::rfc_3339();
    let subscriber = Registry::default().with(env_filter).with(telemetry).with(
        fmt::Layer::default()
            .with_timer(timer_format.clone())
            .with_ansi(true)
            .with_test_writer(),
    );
    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to install `tracing` subscriber");

    provider
}

pub fn send_otel_connectivity_check() {
    let tracer = get_radyko_tracer(SERVICE_NAME);
    let mut span = tracer.start("connectivity-check-span");
    span.set_attribute(KeyValue::new("test", "true"));
    span.end();
}

fn init_tracer(service_name: &str) -> (opentelemetry_sdk::trace::Tracer, SdkTracerProvider) {
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("failed otel exporter build");

    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(service_name.to_string())
                .build(),
        )
        .with_batch_exporter(otlp_exporter)
        .build();
    global::set_tracer_provider(provider.clone());

    (provider.tracer(service_name.to_owned()), provider)
}
