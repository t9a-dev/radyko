use opentelemetry::{
    KeyValue, global,
    trace::{Span, Tracer, TracerProvider},
};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, Registry, filter::Directive, fmt, layer::SubscriberExt};

pub static RADYKO_TRACER: std::sync::OnceLock<opentelemetry::global::BoxedTracer> =
    std::sync::OnceLock::new();
pub static SERVICE_NAME: &str = "radyko";

pub fn get_radyko_tracer(service_name: &str) -> &'static opentelemetry::global::BoxedTracer {
    RADYKO_TRACER.get_or_init(|| opentelemetry::global::tracer(service_name.to_owned()))
}

pub fn init_telemetry(
    service_name: &str,
    level_arg: Option<&str>,
) -> Option<opentelemetry_sdk::trace::SdkTracerProvider> {
    let default_env_filter_directive: Directive = if cfg!(debug_assertions) {
        LevelFilter::INFO.into()
    } else {
        LevelFilter::ERROR.into()
    };
    let Ok(env_filter) = EnvFilter::builder()
        .with_default_directive(default_env_filter_directive.clone())
        .parse(level_arg.unwrap_or(""))
    else {
        error!(
            "failed env_filter build default_env_filter_directive: {default_env_filter_directive:#?} level_arg: {level_arg:#?}"
        );
        return None;
    };
    let timer_format = tracing_subscriber::fmt::time::LocalTime::rfc_3339();
    let fmt_layer = fmt::Layer::default()
        .with_timer(timer_format)
        .with_ansi(true)
        .with_test_writer();

    match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(endpoint) => info!("OTEL_EXPORTER_OTLP_ENDPOINT={}", endpoint),
        Err(_) => {
            info!("OTEL_EXPORTER_OTLP_ENDPOINT is empty. otel sdk is disabled");
            let subscriber = Registry::default().with(env_filter).with(fmt_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("failed to install `tracing` subscriber");
            return None;
        }
    };

    match init_tracer(service_name) {
        Ok((tracer, provider)) => {
            let telemetry = OpenTelemetryLayer::new(tracer);
            let subscriber = Registry::default().with(env_filter).with(telemetry);
            tracing::subscriber::set_global_default(subscriber)
                .expect("failed to install `tracing` subscriber");

            Some(provider)
        }
        Err(err) => {
            eprintln!("OpenTelemetry disabled: {err}");

            let subscriber = Registry::default().with(env_filter).with(fmt_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("failed to install `tracing` subscriber");
            None
        }
    }
}

pub fn send_otel_connectivity_check() {
    let tracer = get_radyko_tracer(SERVICE_NAME);
    let mut span = tracer.start("connectivity-check-span");
    span.set_attribute(KeyValue::new("test", "true"));
    span.end();
}

fn init_tracer(
    service_name: &str,
) -> anyhow::Result<(opentelemetry_sdk::trace::Tracer, SdkTracerProvider)> {
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(service_name.to_string())
                .build(),
        )
        .with_batch_exporter(otlp_exporter)
        .build();
    global::set_tracer_provider(provider.clone());

    Ok((provider.tracer(service_name.to_owned()), provider))
}
