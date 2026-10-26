//! OCSF event emission with automatic trace context injection.

use crate::trace::OcsfVersion;
use parking_lot::RwLock;
use serde_json::Value;
use tracing::info;

/// Global OCSF version for emission.
static OCSF_VERSION: RwLock<OcsfVersion> = RwLock::new(OcsfVersion::V1_8);

/// Sets the global OCSF version for emission.
///
/// Should be called at application startup. Defaults to OCSF 1.8 if not set.
pub fn set_ocsf_version(version: OcsfVersion) {
    *OCSF_VERSION.write() = version;
}

/// Gets the current OCSF version.
pub fn ocsf_version() -> OcsfVersion {
    *OCSF_VERSION.read()
}

/// Emits an OCSF event with automatic trace context injection.
///
/// If the builder has no trace context set, this macro attempts to fill it
/// from the global trace extractor (registered via `set_global_trace_extractor`).
/// An explicit `.trace_context()` call on the builder takes precedence.
///
/// # Example
/// ```ignore
/// ocsf_emit!(BaseEventBuilder::new(4001, 1).metadata(&["network_proxy"]));
/// ```
#[macro_export]
macro_rules! ocsf_emit {
    ($builder:expr) => {{
        let mut builder = $builder;

        // If no trace context explicitly set, try to get from global extractor
        if builder.trace.is_none() {
            if let Some(trace_ctx) = $crate::trace::get_current_trace_context() {
                builder = builder.trace_context(trace_ctx);
            }
        }

        let event = builder.build_for_version($crate::emit::ocsf_version());

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
fn emit_event(event: Value) {
    // In production, this emits via tracing::info! with a structured field
    // For testing, we store in a thread-local
    info!(target: "ocsf", event = %event, "OCSF event");
}

/// Thread-local storage for the last emitted event (for testing).
thread_local! {
    static LAST_EVENT: std::cell::RefCell<Option<Value>> = std::cell::RefCell::new(None);
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
    use crate::trace::TraceContext;
    use crate::builders::BaseEventBuilder;

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
    fn test_ocsf_emit_downgrade_v1_3() {
        set_ocsf_version(OcsfVersion::V1_3);
        let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
        ocsf_emit!(BaseEventBuilder::new(4001, 1)
            .metadata(&["network_proxy", "trace"])
            .trace_context(trace));
        let event = take_last_event().expect("event should be emitted");
        assert!(event.get("trace").is_none(), "trace should be stripped for v1.3");
        assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
        set_ocsf_version(OcsfVersion::V1_8); // reset
    }

    #[test]
    fn test_ocsf_emit_downgrade_v1_1() {
        set_ocsf_version(OcsfVersion::V1_1);
        let trace = TraceContext::new("0af7651916cd43dd8448eb211c80319c");
        ocsf_emit!(BaseEventBuilder::new(4001, 1)
            .metadata(&["network_proxy", "trace"])
            .trace_context(trace));
        let event = take_last_event().expect("event should be emitted");
        assert!(event.get("trace").is_none(), "trace should be stripped for v1.1");
        assert!(!event["metadata"]["profiles"].as_array().unwrap().iter().any(|v| v == "trace"));
        set_ocsf_version(OcsfVersion::V1_8); // reset
    }
}
