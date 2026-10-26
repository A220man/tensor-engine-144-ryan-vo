//! OpenShell OpenTelemetry trace context extraction.
//!
//! This crate provides the [`OtelTraceExtractor`] which implements the
//! [`TraceExtractor`] trait from `openshell-ocsf` by extracting the active
//! OpenTelemetry span context and returning it only when the span is valid
//! and sampled.

pub mod trace;

pub use trace::{OtelTraceExtractor, TraceExtractionError, register_global_trace_extractor};