//! OpenShell gateway binary.
//!
//! Registers the OpenTelemetry trace extractor for automatic OCSF trace correlation.

use openshell_ocsf::emit::{set_ocsf_version, OcsfVersion};
use openshell_otel::register_global_trace_extractor;
use opentelemetry::trace::TracerProvider as _;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize OpenTelemetry SDK
    let tracer = opentelemetry_sdk::trace::TracerProvider::builder()
        .build()
        .tracer("openshell-gateway");

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().with_test_writer();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(telemetry_layer)
        .try_init()?;

    // Register the global trace extractor for OCSF events
    register_global_trace_extractor()?;

    // Set OCSF version (defaults to 1.8)
    set_ocsf_version(OcsfVersion::V1_8);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_tracing()?;

    tracing::info!("OpenShell gateway started");

    // Simulate gateway work - in real implementation this would run the gateway
    // The key point is that the trace extractor is registered and OCSF events
    // emitted within traced spans will automatically include trace context.

    // Example: emit an OCSF event with automatic trace context
    let _ = openshell_ocsf::ocsf_emit!(
        openshell_ocsf::builders::BaseEventBuilder::new(4001, 1)
            .metadata(&["network_proxy", "host"])
    );

    tracing::info!("OpenShell gateway shutting down");
    Ok(())
}
