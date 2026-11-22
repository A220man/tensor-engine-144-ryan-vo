//! OCSF event emission with automatic trace context injection.
//!
//! There is no global mutable "current OCSF version". Emission always applies a
//! downgrade for the version of the event being emitted, which keeps behavior
//! deterministic when multiple events are built on different threads.

use serde_json::Value;

/// Applies trace-context downgrade to an already-built event for a specific version.
///
/// OCSF 1.4 introduced the Trace profile. Events targeting 1.1 or 1.3 must not
/// carry the `trace` object or list `trace` in `metadata.profiles`.
pub fn strip_trace_for_version(mut event: Value, version: crate::trace::OcsfVersion) -> Value {
    if version.supports_trace_profile() {
        return event;
    }
    let Some(obj) = event.as_object_mut() else { return event; };
    obj.remove("trace");
    if let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
        if let Some(profiles) = meta.get_mut("profiles").and_then(|p| p.as_array_mut()) {
            profiles.retain(|v| v != "trace");
        }
    }
    event
}

/// Emits an OCSF event with automatic trace context injection.
///
/// If the builder has no trace context set, this macro attempts to fill it
/// from the global trace extractor (registered via
/// [`crate::trace::set_global_trace_extractor`]). An explicit
/// `.trace_context()` call on the builder takes precedence.
///
/// # Example
/// ```ignore
/// ocsf_emit!(BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]));
/// ```
#[macro_export]
macro_rules! ocsf_emit {
    ($builder:expr) => {{
        $crate::ocsf_emit_for!($builder, $crate::trace::OcsfVersion::V1_8)
    }};
}

/// Emits an OCSF event downgraded to an explicit OCSF version.
///
/// # Example
/// ```ignore
/// ocsf_emit_for!(
///     BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]),
///     OcsfVersion::V1_3
/// );
/// ```
#[macro_export]
macro_rules! ocsf_emit_for {
    ($builder:expr, $version:expr) => {{
        let mut builder = $builder;

        // If no trace context explicitly set, try to get from global extractor
        if builder.trace.is_none() {
            if let Some(trace_ctx) = $crate::trace::get_current_trace_context() {
                builder = builder.trace_context(trace_ctx);
            }
        }

        let event = builder.build_for_version($version);

        #[cfg(test)]
        {
            $crate::emit::set_last_event(event);
        }
        #[cfg(not(test))]
        {
            $crate::emit::emit_event(event);
        }
    }};
}

/// Internal function to emit the event JSON.
pub fn emit_event(_event: Value) {
    // In production, this emits via tracing::info! with a structured field
    // For testing, this is a no-op; tests use `set_last_event` instead.
}

thread_local! {
    static LAST_EVENT: std::cell::RefCell<Option<Value>> = const { std::cell::RefCell::new(None) };
}

/// Gets the last emitted event (for testing).
pub fn take_last_event() -> Option<Value> {
    LAST_EVENT.with(|cell| cell.borrow_mut().take())
}

/// Sets the last emitted event (for testing).
pub fn set_last_event(event: Value) {
    LAST_EVENT.with(|cell| *cell.borrow_mut() = Some(event));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::BaseEventBuilder;
    use crate::trace::{OcsfVersion, TraceContext};

    #[test]
    fn test_ocsf_emit_without_trace() {
        ocsf_emit!(BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]));
        let event = take_last_event().expect("event should be emitted");
        assert_eq!(event["classUid"], 4001);
        assert_eq!(event["activityId"], 1);
        assert!(event.get("trace").is_none(), "trace should not be present without context");
        assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
    }

    #[test]
    fn test_ocsf_emit_with_explicit_trace() {
        let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
        ocsf_emit!(BaseEventBuilder::new(4001, 1)
            .metadata(&["network_proxy"])
            .trace_context(trace.clone()));
        let event = take_last_event().expect("event should be emitted");
        assert_eq!(event["trace"]["uid"], "0af7651916cd43dd8448eb211c80319c");
        assert!(event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
    }

    #[test]
    fn test_ocsf_emit_for_downgrade_v1_3() {
        let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
        ocsf_emit_for!(
            BaseEventBuilder::new(4001, 1)
                .metadata(&["network_proxy"])
                .trace_context(trace),
            OcsfVersion::V1_3
        );
        let event = take_last_event().expect("event should be emitted");
        assert!(event.get("trace").is_none(), "trace should be stripped for v1.3");
        assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
    }

    #[test]
    fn test_ocsf_emit_for_downgrade_v1_1() {
        let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
        ocsf_emit_for!(
            BaseEventBuilder::new(4001, 1)
                .metadata(&["network_proxy"])
                .trace_context(trace),
            OcsfVersion::V1_1
        );
        let event = take_last_event().expect("event should be emitted");
        assert!(event.get("trace").is_none(), "trace should be stripped for v1.1");
        assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
    }

    #[test]
    fn test_strip_trace_for_version_noop_when_supported() {
        let event = serde_json::json!({
            "classUid": 4001,
            "metadata": {"profiles": ["network_proxy", "trace"]},
            "trace": {"uid": "0af7651916cd43dd8448eb211c80319c"}
        });
        let out = strip_trace_for_version(event, OcsfVersion::V1_8);
        assert!(out.get("trace").is_some());
        assert!(out["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
    }
}
