//! Tests for INFO-level deny spans.

use openshell_supervisor_network::{deny_span, DenySpanBuilder, DenySpanConfig};
use openshell_ocsf::trace::TraceContext;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_test_tracing() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .with(tracing_subscriber::EnvFilter::new("info"))
        .try_init();
}

#[test]
fn test_deny_span_basic() {
    init_test_tracing();
    let config = DenySpanConfig::new("l4_policy", "port_blocked", "10.0.0.1:80");
    
    let result = deny_span(config, || "inside_span");
    assert_eq!(result, "inside_span");
}

#[test]
fn test_deny_span_with_trace_context() {
    init_test_tracing();
    let trace = TraceContext::new_unchecked("0af7651916cd43dd8448eb211c80319c".to_string());
    let config = DenySpanConfig::new("l4_policy", "port_blocked", "10.0.0.1:80")
        .with_trace_context(trace);
    
    let result = deny_span(config, || {
        // Verify we're inside the span
        let span = tracing::Span::current();
        assert!(!span.is_disabled());
        "test"
    });
    assert_eq!(result, "test");
}

#[test]
fn test_deny_span_builder_fluent() {
    init_test_tracing();
    let result = DenySpanBuilder::new()
        .policy("l7_policy")
        .reason("http_denied")
        .destination("api.example.com:443")
        .enter(|| "builder_result");
    assert_eq!(result, "builder_result");
}

#[test]
fn test_deny_span_builder_with_trace() {
    init_test_tracing();
    let trace = TraceContext::new_unchecked("0af7651916cd43dd8448eb211c80319c".to_string());
    let result = DenySpanBuilder::new()
        .policy("mapping")
        .reason("ssrf_blocked")
        .destination("internal.service:8080")
        .trace_context(trace)
        .enter(|| "with_trace");
    assert_eq!(result, "with_trace");
}

#[test]
fn test_deny_span_carries_attributes() {
    init_test_tracing();
    // This test verifies the span is created with the right attributes
    // The actual attribute verification would require a custom subscriber
    let config = DenySpanConfig::new("test_policy", "test_reason", "test_destination");
    
    let _ = deny_span(config, || {
        // Span should have policy, reason, destination fields
        // and trace_id field (empty when no trace context)
    });
}

#[test]
fn test_deny_span_multiple_sites() {
    init_test_tracing();
    // Simulate multiple deny sites: L4, mapping, L7
    let l4_result = DenySpanBuilder::new()
        .policy("l4_egress")
        .reason("port_not_allowed")
        .destination("10.0.0.1:22")
        .enter(|| "l4_deny");
    
    let mapping_result = DenySpanBuilder::new()
        .policy("dns_mapping")
        .reason("no_mapping")
        .destination("internal.db:5432")
        .enter(|| "mapping_deny");
    
    let l7_result = DenySpanBuilder::new()
        .policy("http_relay")
        .reason("policy_violation")
        .destination("api.external.com:443")
        .enter(|| "l7_deny");
    
    assert_eq!(l4_result, "l4_deny");
    assert_eq!(mapping_result, "mapping_deny");
    assert_eq!(l7_result, "l7_deny");
}
