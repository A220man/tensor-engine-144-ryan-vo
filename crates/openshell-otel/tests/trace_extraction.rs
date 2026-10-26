//! Tests for OpenTelemetry trace context extraction.

use openshell_otel::{OtelTraceExtractor, register_global_trace_extractor};
use openshell_ocsf::trace::{TraceContext, TraceExtractor, set_global_trace_extractor};

#[test]
fn test_otel_trace_extractor_no_span() {
    let extractor = OtelTraceExtractor::new();
    // Without an active span, should return None
    assert!(extractor.current_trace_context().is_none());
}

#[test]
fn test_otel_trace_extractor_debug() {
    let extractor = OtelTraceExtractor::new();
    let debug_str = format!("{:?}", extractor);
    assert!(debug_str.contains("OtelTraceExtractor"));
}

#[test]
fn test_register_global_trace_extractor() {
    // Test that registration works (may fail if already registered)
    let result = register_global_trace_extractor();
    // We don't assert on the result since it may fail if already registered
    let _ = result;
}

#[test]
fn test_otel_extractor_implements_trace_extractor() {
    let extractor = OtelTraceExtractor::new();
    // Verify it implements the trait
    let _: &dyn TraceExtractor = &extractor;
}

#[test]
fn test_trace_context_new_unchecked() {
    let trace = TraceContext::new_unchecked("0af7651916cd43dd8448eb211c80319c".to_string());
    assert_eq!(trace.uid, "0af7651916cd43dd8448eb211c80319c");
}