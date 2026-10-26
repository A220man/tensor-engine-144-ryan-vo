
//! OCSF event builders with trace correlation support.

use crate::trace::{TraceContext, OcsfVersion};
use serde::Serialize;
use std::collections::HashSet;

/// Metadata for an OCSF event, including profiles.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// OCSF profiles this event conforms to.
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    pub profiles: HashSet<String>,

    /// Additional metadata fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Metadata {
    /// Creates new metadata with the given profiles.
    pub fn new(profiles: &[&str]) -> Self {
        Self {
            profiles: profiles.iter().map(|s| s.to_string()).collect(),
            extra: Default::default(),
        }
    }

    /// Adds a profile to the metadata.
    pub fn add_profile(&mut self, profile: &str) {
        self.profiles.insert(profile.to_string());
    }

    /// Returns the profiles for a given OCSF version, stripping unsupported ones.
    pub fn profiles_for_version(&self, version: OcsfVersion) -> HashSet<String> {
        if version.supports_trace_profile() {
            self.profiles.clone()
        } else {
            self.profiles.iter()
                .filter(|p| *p != "trace")
                .cloned()
                .collect()
        }
    }
}

/// Base builder for OCSF events with trace correlation support.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseEventBuilder {
    /// Event class UID
    pub class_uid: u16,

    /// Event activity ID
    pub activity_id: u16,

    /// Event metadata including profiles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Trace context (OCSF 1.8 Trace profile)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<TraceContext>,

    /// Unmapped fields for schema validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmapped: Option<serde_json::Map<String, serde_json::Value>>,
}

impl BaseEventBuilder {
    /// Creates a new builder with the given class and activity IDs.
    pub fn new(class_uid: u16, activity_id: u16) -> Self {
        Self {
            class_uid,
            activity_id,
            metadata: None,
            trace: None,
            unmapped: None,
        }
    }

    /// Sets the metadata profiles for this event.
    pub fn metadata(mut self, profiles: &[&str]) -> Self {
        self.metadata = Some(Metadata::new(profiles));
        self
    }

    /// Sets the trace context for this event (OCSF 1.8 Trace profile).
    ///
    /// When set, adds "trace" to metadata.profiles and includes the trace object.
    /// An explicit call overrides any automatic trace context from the emission hook.
    pub fn trace_context(mut self, trace: TraceContext) -> Self {
        self.trace = Some(trace);
        if let Some(ref mut meta) = self.metadata {
            meta.add_profile("trace");
        } else {
            self.metadata = Some(Metadata::new(&["trace"]));
        }
        self
    }

    /// Adds an unmapped field.
    pub fn unmapped(mut self, key: &str, value: serde_json::Value) -> Self {
        self.unmapped
            .get_or_insert_with(Default::default)
            .insert(key.to_string(), value);
        self
    }

    /// Builds the event for a specific OCSF version, applying downgrades.
    pub fn build_for_version(self, version: OcsfVersion) -> serde_json::Value {
        let mut value = serde_json::to_value(self).expect("serialization should not fail");

        // Apply version-specific downgrades
        if !version.supports_trace_profile() {
            // Strip trace object
            if let Some(obj) = value.as_object_mut() {
                obj.remove("trace");
            }
            // Strip trace from profiles
            if let Some(obj) = value.as_object_mut() {
                if let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
                    if let Some(profiles) = meta.get_mut("profiles").and_then(|p| p.as_array_mut()) {
                        profiles.retain(|v| v != "trace");
                    }
                }
            }
        }

        value
    }

    /// Builds the event for OCSF 1.8 (full feature set).
    pub fn build(self) -> serde_json::Value {
        self.build_for_version(OcsfVersion::V1_8)
    }
}

/// Trait for builders that support trace context.
pub trait TraceContextBuilder {
    /// Sets the trace context.
    fn trace_context(self, trace: TraceContext) -> Self;
}

impl TraceContextBuilder for BaseEventBuilder {
    fn trace_context(self, trace: TraceContext) -> Self {
        self.trace_context(trace)
    }
}
