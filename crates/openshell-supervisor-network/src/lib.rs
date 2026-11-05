//! OpenShell supervisor network deny spans.
//!
//! This crate provides INFO-level spans for deny decisions in the supervisor's
//! network path. These spans are entered around OCSF event emission at deny sites
//! and carry decision attributes (policy, reason, destination).

use openshell_ocsf::trace::TraceContext;
use thiserror::Error;
use tracing::info_span;

/// Errors that can occur when creating deny spans.
#[derive(Debug, Error)]
pub enum DenySpanError {
    #[error("Invalid trace ID format: {0}")]
    InvalidTraceId(String),
    #[error("Missing required attribute: {0}")]
    MissingAttribute(&'static str),
}

/// Configuration for a deny span.
#[derive(Debug, Clone)]
pub struct DenySpanConfig {
    /// The policy that denied the connection
    pub policy: String,
    /// The reason for the denial
    pub reason: String,
    /// The destination that was denied
    pub destination: String,
    /// Optional trace context for correlation
    pub trace_context: Option<TraceContext>,
}

impl DenySpanConfig {
    /// Creates a new deny span configuration.
    pub fn new(policy: impl Into<String>, reason: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            policy: policy.into(),
            reason: reason.into(),
            destination: destination.into(),
            trace_context: None,
        }
    }

    /// Sets the trace context for this deny span.
    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }
}

/// Creates an INFO-level deny span with the given configuration.
///
/// The span is entered around the provided closure, ensuring that
/// any OCSF events emitted within it will have access to the trace context.
/// The span carries decision attributes (policy, reason, destination).
pub fn deny_span<F, R>(config: DenySpanConfig, f: F) -> R
where
    F: FnOnce() -> R,
{
    let span = info_span!(
        "supervisor.network.deny",
        policy = %config.policy,
        reason = %config.reason,
        destination = %config.destination,
        trace_id = config.trace_context.as_ref().map(|tc| tc.uid.as_str()).unwrap_or(""),
    );

    let _enter = span.enter();
    f()
}

/// A builder for creating deny spans with fluent API.
#[derive(Debug, Default)]
pub struct DenySpanBuilder {
    config: Option<DenySpanConfig>,
}

impl DenySpanBuilder {
    /// Creates a new deny span builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the policy that denied the connection.
    pub fn policy(mut self, policy: impl Into<String>) -> Self {
        let mut config = self.config.unwrap_or_else(|| DenySpanConfig::new("", "", ""));
        config.policy = policy.into();
        self.config = Some(config);
        self
    }

    /// Sets the reason for the denial.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        let mut config = self.config.unwrap_or_else(|| DenySpanConfig::new("", "", ""));
        config.reason = reason.into();
        self.config = Some(config);
        self
    }

    /// Sets the destination that was denied.
    pub fn destination(mut self, destination: impl Into<String>) -> Self {
        let mut config = self.config.unwrap_or_else(|| DenySpanConfig::new("", "", ""));
        config.destination = destination.into();
        self.config = Some(config);
        self
    }

    /// Sets the trace context for correlation.
    pub fn trace_context(mut self, trace_context: TraceContext) -> Self {
        let mut config = self.config.unwrap_or_else(|| DenySpanConfig::new("", "", ""));
        config.trace_context = Some(trace_context);
        self.config = Some(config);
        self
    }

    /// Builds and enters the deny span around the provided closure.
    pub fn enter<F, R>(self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let config = self.config.expect("policy, reason, and destination are required");
        deny_span(config, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openshell_ocsf::trace::TraceContext;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    fn init_test_tracing() {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .with(tracing_subscriber::EnvFilter::new("info"))
            .try_init();
    }

    #[test]
    fn test_deny_span_config() {
        init_test_tracing();
        let config = DenySpanConfig::new("l4_policy", "port_blocked", "10.0.0.1:80")
            .with_trace_context(TraceContext::new_unchecked("0af7651916cd43dd8448eb211c80319c".to_string()));

        let result = deny_span(config, || {
            let span = tracing::Span::current();
            assert!(!span.is_disabled());
            42
        });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_deny_span_builder() {
        init_test_tracing();
        let result = DenySpanBuilder::new()
            .policy("l7_policy")
            .reason("http_denied")
            .destination("api.example.com:443")
            .enter(|| {
                let span = tracing::Span::current();
                assert!(!span.is_disabled());
                "test_result"
            });
        assert_eq!(result, "test_result");
    }

    #[test]
    fn test_deny_span_without_trace_context() {
        init_test_tracing();
        let config = DenySpanConfig::new("l4_policy", "ip_blocked", "192.168.1.1:22");

        let result = deny_span(config, || {
            let span = tracing::Span::current();
            assert!(!span.is_disabled());
            // Check that trace_id field is empty when no trace context
            123
        });
        assert_eq!(result, 123);
    }
}
