#![forbid(unsafe_code)]

//! Self-telemetry domain types (Story 4.3 — FR56).
//!
//! Spirits read their own performance telemetry scoped to their principal
//! namespace per FR31, without requiring per-read operator admission.
//! These types are pure domain — no I/O, no metrics backend.

use serde::{Deserialize, Serialize};

/// Return shape for `telemetry.self()` — per-Spirit performance data.
/// `PartialEq` only — contains `f32` via `HaltTelemetryEntry`.
#[doc = "Construct via [`SelfTelemetryReport::new`] to enforce validation; struct literals bypass non-empty / window checks."]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfTelemetryReport {
    pub spirit_pid: u32,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub latency_p50_us: u64,
    pub latency_p95_us: u64,
    pub latency_p99_us: u64,
    pub halt_events: Vec<HaltTelemetryEntry>,
    pub distillation_outcomes: Vec<DistillationOutcomeEntry>,
    pub generated_ns: u64,
}

impl SelfTelemetryReport {
    pub fn new(
        spirit_pid: u32,
        window_start_ns: u64,
        window_end_ns: u64,
        success_count: u64,
        failure_count: u64,
        latency_p50_us: u64,
        latency_p95_us: u64,
        latency_p99_us: u64,
        halt_events: Vec<HaltTelemetryEntry>,
        distillation_outcomes: Vec<DistillationOutcomeEntry>,
        generated_ns: u64,
    ) -> Result<Self, &'static str> {
        if window_start_ns > window_end_ns {
            return Err("window_start_ns must not exceed window_end_ns");
        }
        Ok(Self {
            spirit_pid,
            window_start_ns,
            window_end_ns,
            success_count,
            failure_count,
            latency_p50_us,
            latency_p95_us,
            latency_p99_us,
            halt_events,
            distillation_outcomes,
            generated_ns,
        })
    }
}

/// One halt event that fired within the telemetry window.
/// `PartialEq` only — `f32` is not `Eq`.
#[doc = "Construct via [`HaltTelemetryEntry::new`] to enforce validation; struct literals bypass non-empty checks."]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HaltTelemetryEntry {
    pub halt_id: String,
    pub tag: String,
    pub predicate_kind: String,
    pub value: f32,
    pub threshold: Option<f32>,
    pub fired_ns: u64,
    pub resolution: Option<ResolutionKindLabel>,
}

impl HaltTelemetryEntry {
    pub fn new(
        halt_id: impl Into<String>,
        tag: impl Into<String>,
        predicate_kind: impl Into<String>,
        value: f32,
        threshold: Option<f32>,
        fired_ns: u64,
        resolution: Option<ResolutionKindLabel>,
    ) -> Result<Self, &'static str> {
        let halt_id: String = halt_id.into();
        if halt_id.is_empty() {
            return Err("halt_id must be non-empty");
        }
        Ok(Self {
            halt_id,
            tag: tag.into(),
            predicate_kind: predicate_kind.into(),
            value,
            threshold,
            fired_ns,
            resolution,
        })
    }
}

/// One distillation-outcome entry (v0.3-β proxy from Decision frames).
/// Story 4.4 lands the explicit `FrameKind::Distillate` variant.
#[doc = "Construct via [`DistillationOutcomeEntry::new`] to enforce validation; struct literals bypass non-empty checks."]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationOutcomeEntry {
    pub digest_frame_id: [u8; 16],
    pub source_log_ref_count: u32,
    pub distillation_depth: u8,
    pub written_ns: u64,
}

impl DistillationOutcomeEntry {
    pub fn new(
        digest_frame_id: [u8; 16],
        source_log_ref_count: u32,
        distillation_depth: u8,
        written_ns: u64,
    ) -> Self {
        Self {
            digest_frame_id,
            source_log_ref_count,
            distillation_depth,
            written_ns,
        }
    }
}

/// Enum copying the `Resolution::kind_label()` strings as variants so
/// the wire shape is forward-compatible with future resolution kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionKindLabel {
    #[serde(rename = "provided_context")]
    ProvidedContext,
    #[serde(rename = "accepted_halt")]
    AcceptedHalt,
    #[serde(rename = "authorized_override")]
    AuthorizedOverride,
}

/// Self-telemetry error taxonomy.
#[derive(Debug, thiserror::Error)]
pub enum SelfTelemetryError {
    #[error("unknown spirit_pid: {spirit_pid}")]
    Unknown { spirit_pid: u32 },
    #[error("invalid window: since_ns {since_ns} is not before now_ns {now_ns}")]
    WindowInvalid { since_ns: u64, now_ns: u64 },
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_telemetry_report_serde_round_trip() {
        let report = SelfTelemetryReport {
            spirit_pid: 1,
            window_start_ns: 100,
            window_end_ns: 200,
            success_count: 10,
            failure_count: 2,
            latency_p50_us: 500,
            latency_p95_us: 900,
            latency_p99_us: 1500,
            halt_events: vec![HaltTelemetryEntry {
                halt_id: "halt-1".into(),
                tag: "test_tag".into(),
                predicate_kind: "on_value_above".into(),
                value: 0.8,
                threshold: Some(0.7),
                fired_ns: 150,
                resolution: Some(ResolutionKindLabel::ProvidedContext),
            }],
            distillation_outcomes: vec![DistillationOutcomeEntry {
                digest_frame_id: [0xAA; 16],
                source_log_ref_count: 3,
                distillation_depth: 2,
                written_ns: 180,
            }],
            generated_ns: 250,
        };

        let json = serde_json::to_string(&report).unwrap();
        let back: SelfTelemetryReport = serde_json::from_str(&json).unwrap();

        assert_eq!(back.spirit_pid, 1);
        assert_eq!(back.window_start_ns, 100);
        assert_eq!(back.success_count, 10);
        assert_eq!(back.halt_events.len(), 1);
        assert_eq!(back.halt_events[0].halt_id, "halt-1");
        assert_eq!(
            back.halt_events[0].resolution,
            Some(ResolutionKindLabel::ProvidedContext)
        );
        assert_eq!(back.distillation_outcomes.len(), 1);
        assert_eq!(back.distillation_outcomes[0].digest_frame_id, [0xAA; 16]);
        assert_eq!(back.generated_ns, 250);
    }

    #[test]
    fn resolution_kind_label_serde_round_trip() {
        for label in [
            ResolutionKindLabel::ProvidedContext,
            ResolutionKindLabel::AcceptedHalt,
            ResolutionKindLabel::AuthorizedOverride,
        ] {
            let json = serde_json::to_string(&label).unwrap();
            let back: ResolutionKindLabel = serde_json::from_str(&json).unwrap();
            assert_eq!(label, back);
        }
    }

    #[test]
    fn self_telemetry_error_display_unknown() {
        let e = SelfTelemetryError::Unknown { spirit_pid: 42 };
        assert!(e.to_string().contains("42"));
    }

    #[test]
    fn self_telemetry_error_display_window_invalid() {
        let e = SelfTelemetryError::WindowInvalid {
            since_ns: 200,
            now_ns: 100,
        };
        assert!(e.to_string().contains("200"));
        assert!(e.to_string().contains("100"));
    }

    #[test]
    fn self_telemetry_error_display_backend_unavailable() {
        let e = SelfTelemetryError::BackendUnavailable("disk full".into());
        assert!(e.to_string().contains("disk full"));
    }

    #[test]
    fn empty_report_json_snapshot() {
        let report = SelfTelemetryReport {
            spirit_pid: 0,
            window_start_ns: 0,
            window_end_ns: 0,
            success_count: 0,
            failure_count: 0,
            latency_p50_us: 0,
            latency_p95_us: 0,
            latency_p99_us: 0,
            halt_events: vec![],
            distillation_outcomes: vec![],
            generated_ns: 0,
        };
        let json = serde_json::to_string(&report).unwrap();
        // Round-trip
        let _back: SelfTelemetryReport = serde_json::from_str(&json).unwrap();
    }
}
