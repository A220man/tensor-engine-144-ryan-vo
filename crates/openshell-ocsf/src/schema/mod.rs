
//! Vendored OCSF schema files for validation.

use std::path::Path;

/// Returns the path to the vendored Trace profile schema.
pub fn trace_profile_path() -> &'static Path {
    Path::new("schemas/trace_profile.json")
}

/// Returns the path to the vendored AI Operation profile schema.
pub fn ai_operation_profile_path() -> &'static Path {
    Path::new("schemas/ai_operation_profile.json")
}
