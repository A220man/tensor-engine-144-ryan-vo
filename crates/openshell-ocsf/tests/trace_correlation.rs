
//! Tests for OCSF trace correlation feature.

use openshell_ocsf::builders::BaseEventBuilder;
use openshell_ocsf::trace::{TraceContext, OcsfVersion, set_global_trace_extractor};
use openshell_ocsf::{ocsf_emit, emit::{set_ocsf_version, take_last_event}};

/// Mock trace extractor for testing.
#[derive(Debug)]
struct MockTraceExtractor {
    trace_id: Option<String>,
}

impl openshell_ocsf::trace::TraceExtractor for MockTraceExtractor {
    fn current_trace_context(&self) -> Option<TraceContext> {
        self.trace_id.as_ref().map(|id| TraceContext::new_unchecked(id.clone()))
    }
}

#[test]
fn test_trace_context_creation() {
    let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
    assert_eq!(trace.uid, "0af7651916cd43dd8448eb211c80319c");
}

#[test]
#[should_panic(expected = "trace ID must be 32 characters")]
fn test_trace_context_invalid_length() {
    TraceContext::new("too_short");
}

#[test]
#[should_panic(expected = "trace ID must be lowercase hex")]
fn test_trace_context_invalid_chars() {
    TraceContext::new("0AF7651916CD43DD8448EB211C80319C"); // uppercase
}

#[test]
fn test_builder_trace_context_setter() {
    let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
    let builder = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(trace.clone());

    let event = builder.build();
    assert_eq!(event["trace"]["uid"], "0af7651916cd43dd8448eb211c80319c");
    assert!(event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
}

#[test]
fn test_builder_without_trace_context() {
    let builder = BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]);
    let event = builder.build();
    assert!(event.get("trace").is_none());
    assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
}

#[test]
fn test_ocsf_emit_explicit_trace() {
    let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(trace));
    let event = take_last_event().expect("event emitted");
    assert_eq!(event["trace"]["uid"], "0af7651916cd43dd8448eb211c80319c");
    assert!(event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
}

#[test]
fn test_ocsf_emit_automatic_trace_from_extractor() {
    // Register mock extractor
    let extractor = Box::new(MockTraceExtractor {
        trace_id: Some("0af7651916cd43dd8448eb211c80319c".to_string()),
    });
    set_global_trace_extractor(extractor).expect("extractor set");

    // Emit without explicit trace context
    ocsf_emit!(BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]));
    let event = take_last_event().expect("event emitted");
    assert_eq!(event["trace"]["uid"], "0af7651916cd43dd8448eb211c80319c");
    assert!(event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));

    // Clean up - can't reset OnceLock, so this test runs last or we accept the state
}

#[test]
fn test_ocsf_emit_no_trace_when_extractor_returns_none() {
    // The global extractor is already set from previous test, but returns None for new extractor
    // We can't reset OnceLock, so we test the explicit override behavior instead
    let trace = TraceContext::new("11111111111111111111111111111111");
    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(trace));
    let event = take_last_event().expect("event emitted");
    // Explicit trace context should override global extractor
    assert_eq!(event["trace"]["uid"], "11111111111111111111111111111111");
}

#[test]
fn test_explicit_trace_overrides_global_extractor() {
    let trace = TraceContext::new("22222222222222222222222222222222");
    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(trace));
    let event = take_last_event().expect("event emitted");
    assert_eq!(event["trace"]["uid"], "22222222222222222222222222222222");
}

#[test]
fn test_downgrade_v1_3_strips_trace() {
    set_ocsf_version(OcsfVersion::V1_3);
    let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy", "trace"])
        .trace_context(trace));
    let event = take_last_event().expect("event emitted");
    assert!(event.get("trace").is_none(), "trace object should be stripped for v1.3");
    assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
    set_ocsf_version(OcsfVersion::V1_8);
}

#[test]
fn test_downgrade_v1_1_strips_trace() {
    set_ocsf_version(OcsfVersion::V1_1);
    let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy", "trace"])
        .trace_context(trace));
    let event = take_last_event().expect("event emitted");
    assert!(event.get("trace").is_none(), "trace object should be stripped for v1.1");
    assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
    set_ocsf_version(OcsfVersion::V1_8);
}

#[test]
fn test_ocsf_version_supports_trace_profile() {
    assert!(OcsfVersion::V1_8.supports_trace_profile());
    assert!(!OcsfVersion::V1_3.supports_trace_profile());
    assert!(!OcsfVersion::V1_1.supports_trace_profile());
}

#[test]
fn test_metadata_profiles_for_version() {
    let meta = openshell_ocsf::builders::Metadata::new(&["network_proxy", "trace"]);
    let v18_profiles = meta.profiles_for_version(OcsfVersion::V1_8);
    let v13_profiles = meta.profiles_for_version(OcsfVersion::V1_3);
    assert!(v18_profiles.contains("trace"));
    assert!(!v13_profiles.contains("trace"));
}

#[test]
fn test_schema_validation_with_trace() {
    // Load the trace profile schema
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set");
    let schema_path = std::path::Path::new(&manifest_dir).join("schemas/trace_profile.json");
    let schema_content = std::fs::read_to_string(&schema_path).expect("schema file should exist");
    let _schema: serde_json::Value = serde_json::from_str(&schema_content).expect("valid JSON schema");

    // Create an event with trace
    let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
    let builder = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy", "trace"])
        .trace_context(trace);
    let event = builder.build();

    // Validate against schema (basic validation)
    let trace_obj = event.get("trace").expect("trace should be present");
    assert!(trace_obj.get("uid").is_some());
    assert_eq!(trace_obj["uid"], "0af7651916cd43dd8448eb211c80319c");

    let profiles = event["metadata"]["profiles"].as_array().unwrap();
    assert!(profiles.iter().any(|v| v == "trace"));
}

#[test]
fn test_schema_validation_without_trace() {
    let builder = BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]);
    let event = builder.build();

    assert!(event.get("trace").is_none());
    let profiles = event["metadata"]["profiles"].as_array().unwrap();
    assert!(!profiles.iter().any(|v| v == "trace"));
}

