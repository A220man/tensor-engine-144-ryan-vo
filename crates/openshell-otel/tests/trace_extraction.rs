//! Tests for OpenTelemetry trace context extraction.
//!
//! A real SDK tracer provider is installed so sampled and unsampled span
//! contexts are produced by OpenTelemetry itself, not mocked by hand.

use openshell_otel::{OtelTraceExtractor, register_global_trace_extractor};
use openshell_ocsf::trace::{TraceContext, TraceExtractor, reset_global_trace_extractor};
use opentelemetry::trace::{Span as _, SpanKind, TraceContextExt, Tracer, TracerProvider as _};
use opentelemetry_sdk::trace::{Config, Sampler, TracerProvider};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_opentelemetry::OpenTelemetrySpanExt;

fn provider(sampler: Sampler) -> TracerProvider {
    TracerProvider::builder()
        .with_config(Config::default().with_sampler(sampler))
        .build()
}

/// Runs `f` inside a tracing span produced by an OTel layer, returning its result.
fn within_span<R>(sampler: Sampler, f: impl FnOnce() -> R) -> R {
    let provider = provider(sampler);
    let tracer = provider.tracer("trace-extraction-test");
    let layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing::subscriber::with_default(
        tracing_subscriber::registry().with(layer),
        || tracing::info_span!(target: "trace-extraction-test", "span", kind = ?SpanKind::Internal).in_scope(f),
    )
}

#[test]
fn test_sampled_span_yields_trace_context() {
    let extractor = OtelTraceExtractor::new();
    let mut got: Option<TraceContext> = None;

    within_span(Sampler::AlwaysOn, || {
        got = extractor.current_trace_context();
    });

    let ctx = got.expect("sampled span should produce a trace context");
    assert_eq!(ctx.uid.len(), 32);
    assert!(ctx.uid.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

#[test]
fn test_unsampled_span_yields_no_trace_context() {
    let extractor = OtelTraceExtractor::new();
    let mut got: Option<TraceContext> = None;

    within_span(Sampler::AlwaysOff, || {
        got = extractor.current_trace_context();
    });

    assert!(got.is_none(), "unsampled span must not contribute a trace context");
}

#[test]
fn test_no_span_yields_no_trace_context() {
    let extractor = OtelTraceExtractor::new();
    // No OTel layer / span is active in this thread-local context.
    assert!(extractor.current_trace_context().is_none());
}

#[test]
fn test_extracted_id_matches_span_trace_id() {
    let extractor = OtelTraceExtractor::new();
    let mut expected = String::new();
    let mut got: Option<TraceContext> = None;

    within_span(Sampler::AlwaysOn, || {
        let cx = tracing::Span::current().context();
        expected = format!("{:032x}", cx.span().span_context().trace_id());
        got = extractor.current_trace_context();
    });

    assert_eq!(got.expect("trace context").uid, expected);
}

#[test]
fn test_register_global_trace_extractor_rejects_duplicate() {
    reset_global_trace_extractor();
    register_global_trace_extractor().expect("first registration should succeed");

    // A second registration must be rejected.
    let second = register_global_trace_extractor();
    assert!(second.is_err(), "duplicate registration must be rejected");
    reset_global_trace_extractor();
}

#[test]
fn test_debug_format_and_trait_object() {
    let extractor = OtelTraceExtractor::new();
    assert!(format!("{extractor:?}").contains("OtelTraceExtractor"));
    let _: &dyn TraceExtractor = &extractor;
}

#[test]
fn test_register_and_emit_inside_sampled_span_carries_trace() {
    // The extractor registry is process-global, so registering and emitting are
    // exercised in one test to keep the outcome independent of test ordering.
    reset_global_trace_extractor();
    register_global_trace_extractor().expect("registration");
    assert!(
        register_global_trace_extractor().is_err(),
        "duplicate registration must be rejected"
    );

    within_span(Sampler::AlwaysOn, || {
        openshell_ocsf::ocsf_emit!(
            openshell_ocsf::builders::BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"])
        );
    });

    let event = openshell_ocsf::emit::take_last_event().expect("event should be emitted");
    assert_eq!(event["trace"]["uid"].as_str().map(str::len), Some(32));
    assert!(event["metadata"]["profiles"]
        .as_array()
        .expect("profiles array")
        .iter()
        .any(|v| v == "trace"));
    reset_global_trace_extractor();
}

#[test]
fn test_sampled_span_for_tracer_is_observed_by_sampler_helper() {
    // Guards the test helper's sampling contract without a global subscriber.
    let provider = provider(Sampler::AlwaysOn);
    let tracer = provider.tracer("sampler-check");
    let span = tracer.start("s");
    assert!(span.span_context().trace_flags().is_sampled());
    assert!(span.span_context().is_valid());
}
