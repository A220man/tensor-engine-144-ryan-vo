# Product development plan

Each round completes one feature or repair. Future items are planned, not implemented claims.

## Current round

otel-trace-extraction

## Roadmap

### ocsf-trace-correlation — OCSF Trace Correlation (openshell-ocsf crate)

Status: done

Acceptance:
- Trace correlation type (TraceContext) with trace.uid field
- Builder setter .trace_context() to add trace object and profile
- trace serialization in OCSF events
- metadata.profiles includes 'trace' when trace context present
- Vendored Trace profile schema for OCSF 1.8
- Emission hook for automatic trace context injection
- Downgrade stripping for OCSF 1.3/1.1

Implementation: `crates/openshell-ocsf/src/trace.rs`, `crates/openshell-ocsf/src/builders/mod.rs`, `crates/openshell-ocsf/src/emit.rs`, `crates/openshell-ocsf/src/schema/trace_profile.json`

Tests: `crates/openshell-ocsf/tests/trace_correlation.rs`

### otel-trace-extraction — OTel Sampled Trace Context Extraction (openshell-otel crate)

Status: in_progress

Acceptance:
- Function to extract active OTel context
- Returns trace context only when span is valid and sampled
- No OTel dependency in openshell-ocsf

Implementation: `crates/openshell-otel/src/trace.rs`

Tests: `crates/openshell-otel/tests/trace_extraction.rs`

### deny-spans — INFO-level Deny Spans (openshell-supervisor-network)

Status: planned

Acceptance:
- INFO-level span for deny decisions
- Span entered around OCSF emission at deny sites
- Carries decision attributes (policy, reason, destination)

Implementation: `crates/openshell-supervisor-network/src/deny.rs`

Tests: `crates/openshell-supervisor-network/tests/deny_spans.rs`

### binary-integration — Supervisor/Gateway Binary Integration

Status: planned

Acceptance:
- Binaries register OTel extractor at startup
- OCSF emission hook fills trace context automatically

Implementation: `crates/openshell-supervisor/src/main.rs`, `crates/openshell-gateway/src/main.rs`

