//! Tests for the OCSF trace correlation feature.
//!
//! No test relies on process-global mutable state: each test either builds an
//! event directly or passes an explicit OCSF version to `ocsf_emit_for!`.

use openshell_ocsf::builders::{BaseEventBuilder, TraceContextForVersion};
use openshell_ocsf::emit::{set_last_event, take_last_event};
use openshell_ocsf::trace::{set_global_trace_extractor, reset_global_trace_extractor, OcsfVersion, TraceContext, TraceExtractor};
use openshell_ocsf::{ocsf_emit, ocsf_emit_for};

const TRACE_ID: &str = "0af7651916cd43dd8448eb211c80319c";

/// Mock extractor returning a fixed trace ID, mirroring an OTel extractor.
#[derive(Debug)]
struct MockTraceExtractor {
    trace_id: Option<String>,
}

impl TraceExtractor for MockTraceExtractor {
    fn current_trace_context(&self) -> Option<TraceContext> {
        self.trace_id.as_ref().map(|id| TraceContext::new_unchecked(id.clone()))
    }
}

fn emitted() -> serde_json::Value {
    take_last_event().expect("event should have been emitted")
}

fn has_profile(event: &serde_json::Value, profile: &str) -> bool {
    event["metadata"]["profiles"]
        .as_array()
        .expect("metadata.profiles should be an array")
        .iter()
        .any(|v| v == profile)
}

#[test]
fn test_trace_context_creation() {
    let trace = TraceContext::new(TRACE_ID);
    assert_eq!(trace.uid, TRACE_ID);
}

#[test]
#[should_panic(expected = "trace ID must be 32 characters")]
fn test_trace_context_invalid_length() {
    TraceContext::new("too_short");
}

#[test]
#[should_panic(expected = "trace ID must be lowercase hex")]
fn test_trace_context_invalid_chars() {
    TraceContext::new("0AF7651916CD43DD8448EB211C80319C");
}

#[test]
fn test_builder_trace_context_setter() {
    let event = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(TraceContext::new(TRACE_ID))
        .build();

    assert_eq!(event["trace"]["uid"], TRACE_ID);
    assert!(has_profile(&event, "trace"));
}

#[test]
fn test_builder_without_trace_context() {
    let event = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .build();
    assert!(event.get("trace").is_none());
    assert!(!has_profile(&event, "trace"));
}

#[test]
fn test_ocsf_emit_explicit_trace() {
    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(TraceContext::new(TRACE_ID)));

    let event = emitted();
    assert_eq!(event["trace"]["uid"], TRACE_ID);
    assert!(has_profile(&event, "trace"));
}

#[test]
fn test_ocsf_emit_for_downgrade_v1_3_strips_profile_added_by_setter() {
    ocsf_emit_for!(
        BaseEventBuilder::new(4001, 1)
            .metadata(&["network_proxy"])
            .trace_context(TraceContext::new(TRACE_ID)),
        OcsfVersion::V1_3
    );

    let event = emitted();
    assert!(event.get("trace").is_none(), "trace object must be stripped for 1.3");
    assert!(!has_profile(&event, "trace"), "profile added by the setter must be stripped for 1.3");
    assert!(has_profile(&event, "network_proxy"), "unrelated profiles are preserved");
}

#[test]
fn test_ocsf_emit_for_downgrade_v1_1_strips_profile_added_by_setter() {
    ocsf_emit_for!(
        BaseEventBuilder::new(4001, 1)
            .metadata(&["network_proxy"])
            .trace_context(TraceContext::new(TRACE_ID)),
        OcsfVersion::V1_1
    );

    let event = emitted();
    assert!(event.get("trace").is_none(), "trace object must be stripped for 1.1");
    assert!(!has_profile(&event, "trace"), "profile added by the setter must be stripped for 1.1");
}

#[test]
fn test_ocsf_emit_for_downgrade_keeps_explicitly_requested_trace_profile_stripped() {
    // Even if a caller asks for the profile by name, downgrade wins.
    ocsf_emit_for!(
        BaseEventBuilder::new(4001, 1).metadata(&["network_proxy", "trace"]),
        OcsfVersion::V1_3
    );

    let event = emitted();
    assert!(event.get("trace").is_none());
    assert!(!has_profile(&event, "trace"));
    assert!(has_profile(&event, "network_proxy"));
}

#[test]
fn test_ocsf_emit_default_version_keeps_trace() {
    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(TraceContext::new(TRACE_ID)));

    let event = emitted();
    assert_eq!(event["trace"]["uid"], TRACE_ID);
    assert!(has_profile(&event, "trace"));
}

#[test]
fn test_ocsf_emit_no_trace_when_no_extractor() {
    reset_global_trace_extractor();

    ocsf_emit!(BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]));

    let event = emitted();
    assert!(event.get("trace").is_none(), "trace absent without extractor");
    assert!(!has_profile(&event, "trace"));
}

#[test]
fn test_ocsf_emit_automatic_trace_from_extractor() {
    reset_global_trace_extractor();
    set_global_trace_extractor(Box::new(MockTraceExtractor { trace_id: Some(TRACE_ID.to_string()) }))
        .expect("extractor should register");

    ocsf_emit!(BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]));

    let event = emitted();
    assert_eq!(event["trace"]["uid"], TRACE_ID);
    assert!(has_profile(&event, "trace"));

    reset_global_trace_extractor();
}

#[test]
fn test_ocsf_emit_unsampled_extractor_yields_no_trace() {
    reset_global_trace_extractor();
    set_global_trace_extractor(Box::new(MockTraceExtractor { trace_id: None }))
        .expect("extractor should register");

    ocsf_emit!(BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]));

    let event = emitted();
    assert!(event.get("trace").is_none(), "unsampled span must not contribute a trace");
    assert!(!has_profile(&event, "trace"));

    reset_global_trace_extractor();
}

#[test]
fn test_explicit_trace_overrides_global_extractor() {
    reset_global_trace_extractor();
    set_global_trace_extractor(Box::new(MockTraceExtractor { trace_id: Some("1111111111111111111111111111111a".to_string()) }))
        .expect("extractor should register");

    ocsf_emit!(BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context(TraceContext::new("22222222222222222222222222222222")));

    let event = emitted();
    assert_eq!(event["trace"]["uid"], "22222222222222222222222222222222");

    reset_global_trace_extractor();
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
    assert!(meta.profiles_for_version(OcsfVersion::V1_8).contains("trace"));
    assert!(!meta.profiles_for_version(OcsfVersion::V1_3).contains("trace"));
    assert!(meta.profiles_for_version(OcsfVersion::V1_3).contains("network_proxy"));
}

#[test]
fn test_trace_context_for_version_builds_downgraded_event() {
    let v13 = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context_for_version(TraceContext::new(TRACE_ID), OcsfVersion::V1_3);
    assert!(v13.get("trace").is_none());
    assert!(!has_profile(&v13, "trace"));

    let v18 = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .trace_context_for_version(TraceContext::new(TRACE_ID), OcsfVersion::V1_8);
    assert_eq!(v18["trace"]["uid"], TRACE_ID);
    assert!(has_profile(&v18, "trace"));
}

fn trace_profile_schema() -> serde_json::Value {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set");
    let path = std::path::Path::new(&manifest_dir).join("schemas/trace_profile.json");
    let content = std::fs::read_to_string(&path).expect("vendored trace profile schema should exist");
    serde_json::from_str(&content).expect("trace profile schema should be valid JSON")
}

#[test]
fn test_schema_validation_accepts_event_with_trace() {
    let event = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy", "trace"])
        .trace_context(TraceContext::new(TRACE_ID))
        .build();

    let compiled = jsonschema::JSONSchema::compile(&trace_profile_schema())
        .expect("trace profile schema should compile");
    let errors: Vec<String> = match compiled.validate(&event) {
        Ok(()) => Vec::new(),
        Err(iter) => iter.map(|e| e.to_string()).collect(),
    };
    assert!(errors.is_empty(), "event with trace should validate: {errors:?}");
}

#[test]
fn test_schema_validation_accepts_event_without_trace() {
    let event = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy"])
        .build();

    let compiled = jsonschema::JSONSchema::compile(&trace_profile_schema())
        .expect("trace profile schema should compile");
    let errors: Vec<String> = match compiled.validate(&event) {
        Ok(()) => Vec::new(),
        Err(iter) => iter.map(|e| e.to_string()).collect(),
    };
    assert!(errors.is_empty(), "event without trace should validate: {errors:?}");
}

#[test]
fn test_schema_validation_rejects_malformed_trace_uid() {
    // Directly pin the schema's contract, independent of TraceContext::new.
    let mut event = BaseEventBuilder::new(4001, 1)
        .metadata(&["network_proxy", "trace"])
        .trace_context(TraceContext::new(TRACE_ID))
        .build();
    event["trace"]["uid"] = serde_json::Value::String("NOT-A-TRACE-ID".to_string());

    let compiled = jsonschema::JSONSchema::compile(&trace_profile_schema())
        .expect("trace profile schema should compile");
    let result = compiled.validate(&event);
    assert!(result.is_err(), "malformed trace.uid must fail schema validation");
}

#[test]
fn test_set_last_event_roundtrip_is_thread_local() {
    // `emit::set_last_event` is only used by the test harness.
    set_last_event(serde_json::json!({"classUid": 4001}));
    assert_eq!(take_last_event().unwrap()["classUid"], 4001);
    assert!(take_last_event().is_none(), "take should consume the stored event");
}
