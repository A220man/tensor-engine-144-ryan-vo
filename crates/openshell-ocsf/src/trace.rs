//! Trace correlation for OCSF events.
//!
//! This module provides the [`TraceContext`] type and the [`TraceExtractor`] trait
//! for integrating OpenTelemetry trace context into OCSF events without
//! depending on OpenTelemetry directly.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Trace context carried in OCSF events per the OCSF 1.8 Trace profile.
///
/// Contains the W3C trace ID (32 lowercase hex characters) in `uid`.
/// The optional `span` object is not populated in this version since
/// start_time and end_time are unknown while the span is still open at emission time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceContext {
    /// W3C trace ID: 32 lowercase hex characters (16 bytes)
    pub uid: String,
}

impl TraceContext {
    /// Creates a new `TraceContext` with the given trace ID.
    ///
    /// # Panics
    /// Panics if `uid` is not 32 lowercase hex characters.
    pub fn new(uid: impl Into<String>) -> Self {
        let uid = uid.into();
        assert_eq!(uid.len(), 32, "trace ID must be 32 characters");
        assert!(uid.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
            "trace ID must be lowercase hex");
        Self { uid }
    }

    /// Creates a `TraceContext` without validation (for testing).
    pub fn new_unchecked(uid: String) -> Self {
        Self { uid }
    }
}

/// Trait for extracting trace context from the current execution context.
///
/// Implementations are provided by crates that have an OpenTelemetry dependency
/// (e.g., `openshell-otel`). The `openshell-ocsf` crate remains independent
/// of OpenTelemetry.
pub trait TraceExtractor: Send + Sync + 'static {
    /// Returns the current trace context if a sampled span is active.
    ///
    /// Returns `None` if:
    /// - No span is currently active
    /// - The active span is not sampled
    /// - The span context is invalid
    fn current_trace_context(&self) -> Option<TraceContext>;
}

/// Combined trait for trace extractor with Debug for error handling.
pub trait DebugTraceExtractor: TraceExtractor + std::fmt::Debug {}

impl<T: TraceExtractor + std::fmt::Debug> DebugTraceExtractor for T {}

/// Global trace extractor registry.
///
/// Binaries that emit OCSF events should call [`set_global_trace_extractor`]
/// at startup to register their OTel-based extractor. The `ocsf_emit!` macro
/// uses this to automatically fill in trace context when the builder hasn't
/// set one explicitly.
static GLOBAL_TRACE_EXTRACTOR: OnceLock<Box<dyn DebugTraceExtractor>> = OnceLock::new();

/// Sets the global trace extractor.
///
/// Should be called once at application startup. Returns an error if
/// an extractor has already been registered.
pub fn set_global_trace_extractor(extractor: Box<dyn DebugTraceExtractor>) -> Result<(), Box<dyn DebugTraceExtractor>> {
    GLOBAL_TRACE_EXTRACTOR.set(extractor)
}

/// Gets the current trace context from the global extractor.
///
/// Returns `None` if no extractor is registered or the extractor returns `None`.
pub fn get_current_trace_context() -> Option<TraceContext> {
    GLOBAL_TRACE_EXTRACTOR.get()?.current_trace_context()
}

/// OCSF schema version for downgrade logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcsfVersion {
    /// OCSF 1.1.0
    V1_1,
    /// OCSF 1.3.0
    V1_3,
    /// OCSF 1.8.0
    V1_8,
}

impl OcsfVersion {
    /// Returns true if the Trace profile is supported (OCSF 1.4+).
    pub fn supports_trace_profile(self) -> bool {
        matches!(self, OcsfVersion::V1_8)
    }
}
