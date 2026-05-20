#![forbid(unsafe_code)]

//! Self-telemetry aggregator — composes IacRtMetrics + HaltRegistry +
//! TransparencyLogAdapter to produce per-Spirit performance reports
//! (Story 4.3 — FR56).
//!
//! Per FR56: "Spirit's own data; Spirit reads it" — the calling
//! `spirit_pid` is kernel-set, not Spirit-supplied.  Cross-Spirit reads
//! are not surfaced.  The call does NOT generate an approval prompt
//! (wired through `cap_policy`'s always-allow rule for
//! `SelfTelemetryRead`).

use std::sync::Arc;

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::SelfTelemetryPort;
use maos_domain::self_telemetry::{
    DistillationOutcomeEntry, HaltTelemetryEntry, ResolutionKindLabel, SelfTelemetryError,
    SelfTelemetryReport,
};

use crate::halt::HaltRegistry;
use maos_domain::halt::HaltState;
use crate::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use crate::telemetry::iac_rt::IacRtMetrics;

/// Self-telemetry aggregator — read-only composer over existing kernel
/// state.  Does NOT retain its own state across calls.
#[maos_attrs::i9_exempt(reason = "self-telemetry aggregator — read-only composer over existing kernel state (IacRtMetrics, HaltRegistry, TransparencyLogAdapter); does not retain its own state across calls; FR56 surface for Spirit-side calibration without per-read operator admission")]
pub struct SelfTelemetryAggregator {
    iac_rt_metrics: Arc<IacRtMetrics>,
    halt_registry: Arc<HaltRegistry>,
    transparency_log: Arc<TransparencyLogAdapter>,
}

impl SelfTelemetryAggregator {
    pub fn new(
        iac_rt_metrics: Arc<IacRtMetrics>,
        halt_registry: Arc<HaltRegistry>,
        transparency_log: Arc<TransparencyLogAdapter>,
    ) -> Self {
        Self {
            iac_rt_metrics,
            halt_registry,
            transparency_log,
        }
    }

    fn now_ns() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Translate a `Resolution::kind_label()` string into the enum variant.
    fn resolution_label(kind: &str) -> Option<ResolutionKindLabel> {
        match kind {
            "provided_context" => Some(ResolutionKindLabel::ProvidedContext),
            "accepted_halt" => Some(ResolutionKindLabel::AcceptedHalt),
            "authorized_override" => Some(ResolutionKindLabel::AuthorizedOverride),
            _ => None,
        }
    }

    /// v0.3-β latency quantile stub — returns (0, 0, 0) for P50/P95/P99.
    ///
    /// Precise per-Spirit latency labels land when Story 5.1 introduces
    /// per-Spirit Tokio task supervision and per-pid histogram labels on
    /// `IacRtMetrics`.  At v0.3-β the histograms are aggregated
    /// kernel-wide and not filterable by `spirit_pid`.
    fn latency_quantiles(&self, _spirit_pid: u32) -> (u64, u64, u64) {
        // v0.3-β: kernel-aggregate only — return zeros per story spec.
        // Story 5.1 adds per-Spirit histogram labels.
        (0, 0, 0)
    }

    /// Build a `HaltTelemetryEntry` from registry metadata + current state.
    fn build_halt_entry(
        &self,
        halt_id: &str,
        meta: &crate::halt::PendingHaltMetadata,
        state: Option<HaltState>,
    ) -> Result<HaltTelemetryEntry, SelfTelemetryError> {
        let resolution = state.and_then(|s| match s {
            HaltState::Resumed => Some(ResolutionKindLabel::ProvidedContext),
            HaltState::Terminated => Some(ResolutionKindLabel::AcceptedHalt),
            HaltState::Overridden => Some(ResolutionKindLabel::AuthorizedOverride),
            HaltState::PendingResolution => None,
        });
        HaltTelemetryEntry::new(
            halt_id,
            &meta.payload.tag,
            &meta.payload.policy_id,
            meta.payload.value,
            meta.payload.threshold,
            meta.fired_ns,
            resolution,
        )
        .map_err(|e| SelfTelemetryError::BackendUnavailable(format!("halt entry build: {e}")))
    }
}

impl SelfTelemetryPort for SelfTelemetryAggregator {
    fn self_telemetry(
        &self,
        spirit_pid: u32,
        since_ns: Option<u64>,
    ) -> Result<SelfTelemetryReport, SelfTelemetryError> {
        let window_start_ns = since_ns.unwrap_or(0);
        let window_end_ns = Self::now_ns();

        if window_start_ns > window_end_ns {
            return Err(SelfTelemetryError::WindowInvalid {
                since_ns: window_start_ns,
                now_ns: window_end_ns,
            });
        }

        // 1. Latency quantiles (v0.3-β stub).
        let (latency_p50_us, latency_p95_us, latency_p99_us) =
            self.latency_quantiles(spirit_pid);

        // 2. Halt events from registry — scoped to calling spirit_pid.
        let halt_events: Vec<HaltTelemetryEntry> = {
            let metas = self.halt_registry.halt_metadata_for_spirit(spirit_pid, window_start_ns);
            let mut entries = Vec::with_capacity(metas.len());
            for (hid, meta) in &metas {
                // Best-effort: look up current state in the pending map.
                // If the metadata was already cleaned up after resolution,
                // we skip it (resolution-side cleanup is intentional).
                let state = self.halt_registry.lookup_state(hid);
                entries.push(self.build_halt_entry(hid.as_str(), meta, state)?);
            }
            entries
        };

        // 3. Success/failure counts from Transparency Log.
        let (success_count, failure_count) = {
            let filter = FrameFilter {
                spirit_pid: Some(spirit_pid),
                since_ns: Some(window_start_ns),
                ..Default::default()
            };

            let frames = self
                .transparency_log
                .query_frames(filter)
                .map_err(|e| SelfTelemetryError::BackendUnavailable(e.to_string()))?;
            let sc = frames
                .iter()
                .filter(|f| f.kind == FrameKind::TaskComplete)
                .count() as u64;
            let fc = frames
                .iter()
                .filter(|f| f.kind == FrameKind::EpistemicHalt)
                .count() as u64;
            (sc, fc)
        };

        // 4. Distillation outcomes (v0.4 onwards: precise FrameKind::Distillate filter;
        // the v0.3-β Decision proxy is gone).
        let distillation_outcomes: Vec<DistillationOutcomeEntry> = {
            let filter = FrameFilter {
                spirit_pid: Some(spirit_pid),
                since_ns: Some(window_start_ns),
                kind: Some(FrameKind::Distillate),
                ..Default::default()
            };

            let frames = self
                .transparency_log
                .query_frames(filter)
                .map_err(|e| SelfTelemetryError::BackendUnavailable(e.to_string()))?;
            frames
                .into_iter()
                .map(|f| DistillationOutcomeEntry::new(f.frame_id, 1, 1, f.timestamp_ns))
                .collect()
        };

        let generated_ns = Self::now_ns();

        // 5. Write CapabilityInvocation audit row (FR4 mediation).
        let payload = format!(
            "self_telemetry: pid={} window=[{},{}]",
            spirit_pid, window_start_ns, window_end_ns
        );
        // I2 binding: insert_frame_event panics on SQLite failure, so
        // there is no silent-drop path.  Per D4 consensus, audit integrity
        // is non-negotiable.
        let _token = self.transparency_log.insert_frame_event(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            "telemetry.self",
            payload.as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        SelfTelemetryReport::new(
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
        )
        .map_err(|e| SelfTelemetryError::BackendUnavailable(format!("report build: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::SelfTelemetryPort;

    fn make_aggregator() -> SelfTelemetryAggregator {
        let metrics = Arc::new(IacRtMetrics::new());
        let registry = Arc::new(HaltRegistry::new());
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEEF));
        SelfTelemetryAggregator::new(metrics, registry, tl)
    }

    #[test]
    fn empty_window_returns_zeros() {
        let agg = make_aggregator();
        let report = agg.self_telemetry(1, None).unwrap();
        assert_eq!(report.spirit_pid, 1);
        assert_eq!(report.success_count, 0);
        assert_eq!(report.failure_count, 0);
        assert!(report.halt_events.is_empty());
        assert!(report.distillation_outcomes.is_empty());
        assert!(report.window_start_ns <= report.window_end_ns);
        assert!(report.generated_ns > 0);
    }

    #[test]
    fn invalid_window_returns_error() {
        let agg = make_aggregator();
        let err = agg
            .self_telemetry(1, Some(u64::MAX))
            .unwrap_err();
        assert!(matches!(err, SelfTelemetryError::WindowInvalid { .. }));
    }

    #[test]
    fn different_pids_produce_different_reports() {
        let agg = make_aggregator();
        let r1 = agg.self_telemetry(1, None).unwrap();
        let r2 = agg.self_telemetry(2, None).unwrap();
        assert_eq!(r1.spirit_pid, 1);
        assert_eq!(r2.spirit_pid, 2);
    }
}
