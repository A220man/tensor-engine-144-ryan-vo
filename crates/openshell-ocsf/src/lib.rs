
//! OpenShell OCSF event builders and emission.
//!
//! This crate provides OCSF-compliant event builders with support for the
//! OCSF 1.8 Trace profile for correlating security events with OpenTelemetry traces.

pub mod builders;
pub mod emit;
pub mod trace;
pub mod schema;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_compiles() {
        // Smoke test that the crate compiles and exports expected items
        let _ = trace::TraceContext::new("0af7651916cd43dd8448eb211c80319c");
        let _ = builders::BaseEventBuilder::new(4001, 1);
        let _ = emit::ocsf_version();
    }
}
