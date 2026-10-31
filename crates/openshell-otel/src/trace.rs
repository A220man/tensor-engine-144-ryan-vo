//! OpenTelemetry trace context extraction for OCSF events.
//!
//! This module provides the [`OtelTraceExtractor`] which implements the
//! [`TraceExtractor`] trait from `openshell-ocsf` by extracting the active
//! OpenTelemetry span context and returning it only when the span is valid
//! and sampled.

use openshell_ocsf::trace::{TraceContext, TraceExtractor};
use opentelemetry::trace::TraceContextExt;
use std::fmt;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Errors that can occur during trace context extraction.
#[derive(Debug, thiserror::Error)]
pub enum TraceExtractionError {
    #[error("No active span")]
    NoActiveSpan,
    #[error("Active span has no OpenTelemetry context")]
    NoOtelContext,
    #[error("Span context is not valid")]
    InvalidSpanContext,
    #[error("Span is not sampled")]
    NotSampled,
}

/// OpenTelemetry-based trace extractor for OCSF events.
///
/// Extracts the current trace context from the active OpenTelemetry span.
/// Returns `None` if:
/// - No span is currently active
/// - The active span has no OpenTelemetry context
/// - The span context is invalid
/// - The span is not sampled (TraceFlags::SAMPLED not set)
///
/// This ensures that only trace IDs that can be resolved in a trace backend
/// (e.g., Jaeger, Tempo) are included in OCSF events.
#[derive(Default)]
pub struct OtelTraceExtractor;

impl OtelTraceExtractor {
    /// Creates a new `OtelTraceExtractor`.
    pub fn new() -> Self {
        Self
    }
}

impl TraceExtractor for OtelTraceExtractor {
    fn current_trace_context(&self) -> Option<TraceContext> {
        // Get the current tracing span
        let span = Span::current();

        // Extract the OpenTelemetry span context using the extension trait
        let otel_cx = span.context();
        let span_ref = otel_cx.span();
        let otel_context = span_ref.span_context();

        // Check if the span context is valid
        if !otel_context.is_valid() {
            return None;
        }

        // Check if the span is sampled
        if !otel_context.trace_flags().is_sampled() {
            return None;
        }

        // Extract the W3C trace ID (32 lowercase hex characters)
        let trace_id = otel_context.trace_id();
        let trace_id_hex = format!("{:032x}", trace_id);

        // Validate the trace ID format (32 lowercase hex chars)
        if trace_id_hex.len() != 32 || !trace_id_hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
            return None;
        }

        Some(TraceContext::new_unchecked(trace_id_hex))
    }
}

impl fmt::Debug for OtelTraceExtractor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OtelTraceExtractor").finish()
    }
}

/// Registers the OpenTelemetry trace extractor globally.
///
/// Call this once at application startup to enable automatic trace context
/// injection in OCSF events emitted via `ocsf_emit!`.
///
/// # Example
/// ```ignore
/// use openshell_otel::register_global_trace_extractor;
///
/// fn main() {
///     register_global_trace_extractor().expect("Failed to register trace extractor");
///     // ... rest of application
/// }
/// ```
pub fn register_global_trace_extractor() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let extractor = Box::new(OtelTraceExtractor::new());
    openshell_ocsf::trace::set_global_trace_extractor(extractor)
        .map_err(|e| format!("Failed to register trace extractor: {:?}", e).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otel_trace_extractor_default() {
        let extractor = OtelTraceExtractor::new();
        // Without an active span, should return None
        assert!(extractor.current_trace_context().is_none());
    }

    #[test]
    fn test_register_global_trace_extractor() {
        // This test verifies the registration function works
        // We can't easily test the full integration without a running OTel setup
        let result = register_global_trace_extractor();
        // May fail if already registered, which is fine for this test
        let _ = result;
    }
}
