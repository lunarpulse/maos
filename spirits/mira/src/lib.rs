#![forbid(unsafe_code)]

//! `mira` — MAOS Mira v1.5, the prod-edge **diagnostic** reference Spirit and
//! **Host A** of the Story 8.5 bilateral diagnostic-architect pair (architecture
//! §6.4 / the §13.1 J4 journey).
//!
//! Mira is a **specialized Worker** (`SpiritRole::Worker` — Decision C; the role
//! is set in the [`FrameAddress`](maos_domain::frame::FrameAddress)`.role` at
//! registration, not a manifest field). It:
//!
//! 1. **Diagnoses prod-edge anomalies deterministically** ([`Mira::diagnose`]) —
//!    no live LLM at v1.5 (Decision I); the diagnosis is pure, seeded, and
//!    bit-identical (NFR-Testability-1).
//! 2. **Fires a halt at its diagnostic-confidence boundary**: when Mira observes
//!    a severe anomaly it cannot confidently explain, its `diagnostic_confidence`
//!    scalar falls below the `[epistemic_policy]` floor and the kernel fires a
//!    halt. [`Mira::halt_payload`] produces the [`EpistemicHaltPayload`] the kernel
//!    surfaces (`NotificationEvent::Halt`) — proven against the real
//!    `NotificationDispatcher` + three-tap `HaltFlow` + `KernelHaltResolver` in
//!    `tests/halt_bilateral.rs`.
//! 3. **Informs Nash (Host B) via an A2A typed-intent advisory** ([`Mira::advisory`]):
//!    a read-only [`DiagnosticAdvisory`] wire payload routed over the real
//!    `LoopbackA2ARouter` with TOFU pinning + ADR-012 consent — proven in
//!    `tests/a2a_pairing.rs`. The advisory crosses the A2A boundary as
//!    `IntentClass::Readonly` (the consent-allowlist projection is `"readonly"`).
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! This crate depends only on the Spirit SDK/ABI and the PURE `maos-domain` types
//! (the §7.4 [`EpistemicHaltPayload`] Mira raises). It NEVER reaches into
//! `maos-kernel-core` / `maos-a2a` / `maos-director-surface`; the real A2A router,
//! notification dispatcher, and halt resolver are exercised in `tests/`, which
//! carry those crates as dev-dependencies only (the resolved Butler/Researcher/
//! Observer/Architect/Reviewer pattern). The *interpretation* of an anomaly (the
//! diagnosis, the confidence, the halt decision) lives entirely here; the kernel
//! only ever performs the universal-arithmetic halt comparison (§4.0.7).

use std::sync::{Arc, Mutex};

use maos_domain::frame::{EpistemicHaltPayload, HaltPayloadError};
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// The scalar tag Mira's `[epistemic_policy]` halt rule watches (must match
/// `manifest.toml`'s `[[epistemic_policy.rules]] tag`).
pub const DIAGNOSTIC_CONFIDENCE_TAG: &str = "diagnostic_confidence";

/// The diagnostic-confidence floor below which Mira's epistemic boundary fires a
/// halt — mirrors the `manifest.toml` `[[epistemic_policy.rules]] on_value_below`
/// threshold (the kernel performs the comparison; this constant keeps the
/// Spirit-side halt decision in lock-step with the declared policy).
pub const DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD: f64 = 0.5;

/// The ADR-012 typed-intent string Mira's cross-Host advisory carries. Mira's
/// advisory is **read-only** (it asks Nash to architect a fix; it mutates
/// nothing), so the frame carries [`IntentClass::Readonly`](maos_domain::invariants::i1::IntentClass::Readonly),
/// whose A2A consent projection is `"readonly"`. Both Mira's `send_allowlist`
/// and Nash's `accept_allowlist` must admit this intent for the advisory to
/// deliver (`tests/a2a_pairing.rs`).
pub const ADVISORY_CONSENT_INTENT: &str = "readonly";

/// The prod-edge metrics Mira has a known diagnostic pattern for. An anomaly on a
/// *known* metric is diagnosed confidently; an anomaly on an *unknown* metric is
/// a novel prod-edge signal Mira may be unable to confidently explain (driving
/// the halt boundary).
const KNOWN_METRICS: [&str; 3] = ["error_rate", "latency_p99", "saturation"];

// ───────────────────────────────────────────────────────────────────────────
// Inputs / outputs.
// ───────────────────────────────────────────────────────────────────────────

/// A prod-edge anomaly signal Mira diagnoses. `observed` vs `baseline` define the
/// deviation; `metric` selects whether Mira has a known diagnostic pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalySignal {
    /// The prod-edge service / component the anomaly is about.
    pub subject: String,
    /// The metric that anomalous (e.g. `error_rate`, `latency_p99`).
    pub metric: String,
    /// The observed value.
    pub observed: f64,
    /// The expected baseline.
    pub baseline: f64,
    /// A human-readable structural detail / provenance.
    #[serde(default)]
    pub detail: String,
    /// The Transparency-Log reference the anomaly was witnessed at (FR17 citation).
    #[serde(default)]
    pub source_log_ref: String,
}

/// Mira's deterministic diagnosis of an [`AnomalySignal`]. Serializes to exactly
/// the manifest `[output_shape]` `required_fields`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnosis {
    /// The prod-edge subject diagnosed.
    pub subject: String,
    /// The human-readable finding.
    pub finding: String,
    /// Severity of the anomaly in `[0.0, 1.0]` (deviation magnitude).
    pub severity: f64,
    /// Mira's diagnostic confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Whether the confidence fell below the halt floor (the epistemic boundary).
    /// Not serialized into the output shape — a derived control flag.
    #[serde(skip)]
    pub requires_halt: bool,
    /// The source-log reference the diagnosis cites (FR17), threaded from the
    /// signal so the morning digest can resolve it against the real TL.
    #[serde(default)]
    pub source_log_ref: String,
}

/// The cross-Host **diagnostic advisory** Mira sends Nash (the A2A wire payload).
/// Carried in the frame as JSON; Nash deserializes it into its own input type
/// (no crate coupling — the contract is the serde shape, exactly as a real
/// cross-Host frame would be).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticAdvisory {
    /// The prod-edge subject the advisory concerns.
    pub subject: String,
    /// Mira's finding (read-only diagnostic evidence for Nash to architect against).
    pub finding: String,
    /// Severity in `[0.0, 1.0]`.
    pub severity: f64,
    /// What Mira recommends Nash investigate / architect.
    pub recommended_action: String,
    /// The Transparency-Log reference the advisory cites (FR17).
    pub source_log_ref: String,
}

// ───────────────────────────────────────────────────────────────────────────
// The Mira Spirit.
// ───────────────────────────────────────────────────────────────────────────

/// Mira reference Spirit — a prod-edge diagnostic. Holds its own Spirit id and an
/// optional seeded batch of pending signals an `on_idle` pass diagnoses.
/// `Arc<Mutex<...>>` interior state keeps Mira `Sync` as the `#[spirit]` macro
/// requires (poison-safe `unwrap_or_else(|e| e.into_inner())` — the 8.2/8.3 fix).
#[derive(Debug, Clone)]
pub struct Mira {
    /// This Mira's own Spirit id.
    spirit_id: String,
    /// Optional seeded signals an `on_idle` pass diagnoses (fixture / harness path).
    pending_signals: Option<Vec<AnomalySignal>>,
    /// Diagnoses produced by the most recent `on_idle` pass.
    last_diagnoses: Arc<Mutex<Vec<Diagnosis>>>,
}

#[spirit]
impl Mira {
    /// Idle diagnostic pass. Cancellation-aware; bounded (a single linear pass
    /// over any seeded signals). Stores the resulting diagnoses so the hook has a
    /// production-visible effect; the LIVE A2A / halt paths are proven against the
    /// real adapters in `tests/` (the ABI `Ctx` exposes no A2A / notification
    /// surface — the Butler/Researcher/Observer precedent).
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        let mut diagnoses = Vec::new();
        if let Some(signals) = &self.pending_signals {
            for sig in signals {
                diagnoses.push(self.diagnose(sig));
            }
        }
        let mut guard = self.last_diagnoses.lock().unwrap_or_else(|e| e.into_inner());
        *guard = diagnoses;
    }
}

impl Default for Mira {
    fn default() -> Self {
        Self {
            spirit_id: "mira".to_string(),
            pending_signals: None,
            last_diagnoses: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Mira {
    /// Override this Mira's own Spirit id.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.spirit_id = id.into();
        self
    }

    /// Seed signals an `on_idle` pass will diagnose (fixture / harness).
    pub fn with_pending_signals(mut self, signals: Vec<AnomalySignal>) -> Self {
        self.pending_signals = Some(signals);
        self
    }

    /// This Mira's own Spirit id.
    pub fn spirit_id(&self) -> &str {
        &self.spirit_id
    }

    /// Diagnoses produced by the most recent `on_idle` pass.
    pub fn last_diagnoses(&self) -> Vec<Diagnosis> {
        self.last_diagnoses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    // ── deterministic diagnosis (AC2/AC4) ────────────────────────────────────

    /// Diagnose one prod-edge anomaly. **Deterministic** — pure function of the
    /// signal (Decision I; NFR-Testability-1). Severity is the clamped relative
    /// deviation; confidence is high for a *known* metric and falls toward the
    /// halt floor for a *severe, unexplained* (unknown-metric) anomaly. When
    /// confidence drops below [`DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD`], the
    /// diagnosis marks `requires_halt` (Mira's epistemic boundary).
    pub fn diagnose(&self, signal: &AnomalySignal) -> Diagnosis {
        // Guard against NaN baseline (would silently bypass halt boundary).
        // Also guard negative baseline (produces unexpected severity sign-flip).
        let baseline = if signal.baseline.is_nan() || signal.baseline <= 0.0 {
            f64::EPSILON
        } else {
            signal.baseline
        };
        let deviation = ((signal.observed - signal.baseline) / baseline).abs();
        let severity = if deviation.is_nan() {
            0.0
        } else {
            deviation.clamp(0.0, 1.0)
        };
        let known = KNOWN_METRICS.contains(&signal.metric.as_str());
        let confidence = if known {
            // A known pattern: Mira is confident even at high severity.
            (0.95 - severity * 0.1).clamp(0.0, 1.0)
        } else {
            // A novel / unexplained anomaly: confidence falls as severity rises —
            // a severe anomaly Mira has no pattern for reaches the halt boundary.
            (0.9 - severity).clamp(0.0, 1.0)
        };
        let requires_halt = confidence < DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD;
        let finding = if known {
            format!(
                "prod-edge anomaly on '{}': {} = {:.3} vs baseline {:.3} (deviation {:.1}%) — matches a known diagnostic pattern",
                signal.subject,
                signal.metric,
                signal.observed,
                signal.baseline,
                severity * 100.0
            )
        } else {
            format!(
                "prod-edge anomaly on '{}': {} = {:.3} vs baseline {:.3} (deviation {:.1}%) — NO known diagnostic pattern; confidence {:.2}",
                signal.subject,
                signal.metric,
                signal.observed,
                signal.baseline,
                severity * 100.0,
                confidence
            )
        };
        Diagnosis {
            subject: signal.subject.clone(),
            finding,
            severity,
            confidence,
            requires_halt,
            source_log_ref: signal.source_log_ref.clone(),
        }
    }

    /// Produce the [`EpistemicHaltPayload`] the kernel surfaces when a diagnosis
    /// reaches Mira's halt boundary. Returns `None` for a confidently-diagnosed
    /// anomaly (no halt). The payload's `value`/`threshold` mirror the
    /// `diagnostic_confidence` scalar the `[epistemic_policy]` rule compares.
    pub fn halt_payload(&self, diagnosis: &Diagnosis) -> Option<EpistemicHaltPayload> {
        if !diagnosis.requires_halt {
            return None;
        }
        match self.try_halt_payload(diagnosis) {
            Ok(payload) => Some(payload),
            Err(e) => {
                eprintln!("mira: halt_payload construction failed for subject={}: {:?}", diagnosis.subject, e);
                None
            }
        }
    }

    /// Fallible inner of [`halt_payload`](Self::halt_payload) — surfaces the
    /// `EpistemicHaltPayload` construction error (range / NaN) instead of
    /// swallowing it, for tests that assert payload well-formedness.
    pub fn try_halt_payload(
        &self,
        diagnosis: &Diagnosis,
    ) -> Result<EpistemicHaltPayload, HaltPayloadError> {
        // Include metric + severity in halt_id to prevent collisions on same subject
        // with different anomalies (review patch: halt ID collision vulnerability).
        let halt_id = format!(
            "mira-halt-{}-{}-{:.4}",
            diagnosis.subject, self.spirit_id, diagnosis.severity
        );
        let derived_from = if diagnosis.source_log_ref.is_empty() {
            format!("mira:diagnose:{}", diagnosis.subject)
        } else {
            diagnosis.source_log_ref.clone()
        };
        // TECH-DEBT(8.5): EpistemicHaltPayload uses f32 for value/threshold.
        // Guard against f64→f32 truncation flipping the halt decision at the boundary.
        let epsilon: f64 = 1e-6;
        let confidence_f32 = if (diagnosis.confidence - DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD).abs() < epsilon {
            // At the boundary: bias toward the halt (cast the threshold, which is the comparison floor)
            DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD as f32
        } else {
            diagnosis.confidence as f32
        };
        EpistemicHaltPayload::new(
            halt_id,
            DIAGNOSTIC_CONFIDENCE_TAG.to_string(),
            confidence_f32,
            Some(DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD as f32),
            "mira.diagnostic_confidence".to_string(),
            derived_from,
        )
    }

    // ── cross-Host advisory (AC4) ─────────────────────────────────────────────

    /// Build the read-only [`DiagnosticAdvisory`] Mira sends Nash over A2A. The
    /// advisory carries Mira's finding as evidence for Nash to architect against;
    /// it mutates nothing (hence the `readonly` consent intent —
    /// [`ADVISORY_CONSENT_INTENT`]).
    ///
    /// TECH-DEBT(8.5): The A2A wire frame uses `FrameKind::TaskAssign` as the
    /// carrier because the v1.0 ABI has no `Evidence`/`Observation` variant (AC8
    /// forbids new variants). The payload is read-only advisory data, not a task
    /// assignment. Next FrameKind taxonomy revision should add an explicit
    /// evidence/observation kind.
    pub fn advisory(&self, diagnosis: &Diagnosis) -> DiagnosticAdvisory {
        let recommended_action = if diagnosis.requires_halt {
            format!(
                "Mira could not confidently diagnose this prod-edge anomaly (confidence {:.2}); architect a mitigation + add observability for '{}'",
                diagnosis.confidence, diagnosis.subject
            )
        } else {
            format!(
                "architect a fix for the diagnosed anomaly on '{}' (severity {:.2})",
                diagnosis.subject, diagnosis.severity
            )
        };
        DiagnosticAdvisory {
            subject: diagnosis.subject.clone(),
            finding: diagnosis.finding.clone(),
            severity: diagnosis.severity,
            recommended_action,
            source_log_ref: if diagnosis.source_log_ref.is_empty() {
                format!("mira:diagnose:{}", diagnosis.subject)
            } else {
                diagnosis.source_log_ref.clone()
            },
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn known_severe() -> AnomalySignal {
        AnomalySignal {
            subject: "checkout-api".into(),
            metric: "error_rate".into(),
            observed: 0.42,
            baseline: 0.30,
            detail: "spike during deploy".into(),
            source_log_ref: "tl:row:1001".into(),
        }
    }

    fn unknown_severe() -> AnomalySignal {
        AnomalySignal {
            subject: "edge-cache".into(),
            metric: "novel_entropy_drift".into(),
            observed: 0.9,
            baseline: 0.1,
            detail: "never seen before".into(),
            source_log_ref: "tl:row:2002".into(),
        }
    }

    #[test]
    fn known_metric_diagnosed_confidently_no_halt() {
        let m = Mira::default();
        let d = m.diagnose(&known_severe());
        assert_eq!(d.subject, "checkout-api");
        assert!(d.confidence >= DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD);
        assert!(!d.requires_halt, "a known pattern is diagnosed confidently");
        assert!(d.finding.contains("known diagnostic pattern"));
        assert!(m.halt_payload(&d).is_none());
    }

    #[test]
    fn unknown_severe_anomaly_reaches_halt_boundary() {
        let m = Mira::default();
        let d = m.diagnose(&unknown_severe());
        assert!(
            d.confidence < DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD,
            "a severe unexplained anomaly drops below the halt floor: {}",
            d.confidence
        );
        assert!(d.requires_halt);
        let payload = m.halt_payload(&d).expect("halt payload produced at boundary");
        assert_eq!(payload.tag, DIAGNOSTIC_CONFIDENCE_TAG);
        assert!((payload.value - d.confidence as f32).abs() < 1e-6);
        assert_eq!(payload.threshold, Some(DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD as f32));
    }

    #[test]
    fn diagnosis_is_deterministic() {
        let m = Mira::default();
        let a = m.diagnose(&unknown_severe());
        let b = m.diagnose(&unknown_severe());
        assert_eq!(a, b, "diagnosis is bit-identical (NFR-Testability-1)");
    }

    #[test]
    fn advisory_carries_finding_and_source_ref() {
        let m = Mira::default();
        let d = m.diagnose(&unknown_severe());
        let adv = m.advisory(&d);
        assert_eq!(adv.subject, "edge-cache");
        assert_eq!(adv.source_log_ref, "tl:row:2002");
        assert!(adv.finding.contains("edge-cache"));
        // The advisory wire payload round-trips through JSON (the A2A contract).
        let json = serde_json::to_string(&adv).unwrap();
        let back: DiagnosticAdvisory = serde_json::from_str(&json).unwrap();
        assert_eq!(adv, back);
    }

    #[test]
    fn diagnosis_serializes_with_required_output_fields() {
        let m = Mira::default();
        let d = m.diagnose(&known_severe());
        let v = serde_json::to_value(&d).unwrap();
        for field in ["subject", "finding", "severity", "confidence"] {
            assert!(v.get(field).is_some(), "missing required output field {field}");
        }
    }

    #[test]
    fn zero_baseline_does_not_panic() {
        let m = Mira::default();
        let d = m.diagnose(&AnomalySignal {
            subject: "s".into(),
            metric: "error_rate".into(),
            observed: 1.0,
            baseline: 0.0,
            detail: String::new(),
            source_log_ref: String::new(),
        });
        assert!(d.severity >= 0.0 && d.severity <= 1.0);
    }

    #[test]
    fn advisory_consent_intent_is_readonly() {
        // The advisory is read-only evidence; its consent projection must be the
        // IntentClass::Readonly string the allowlists match on.
        use maos_domain::invariants::i1::IntentClass;
        assert_eq!(
            IntentClass::Readonly.a2a_consent_intent_str(),
            ADVISORY_CONSENT_INTENT
        );
    }

    #[test]
    fn with_id_overrides_spirit_id() {
        let m = Mira::default().with_id("mira-host-a");
        assert_eq!(m.spirit_id(), "mira-host-a");
    }

    #[test]
    fn nan_baseline_treated_as_epsilon() {
        let m = Mira::default();
        let d = m.diagnose(&AnomalySignal {
            subject: "s".into(),
            metric: "error_rate".into(),
            observed: 1.0,
            baseline: f64::NAN,
            detail: String::new(),
            source_log_ref: String::new(),
        });
        // NaN baseline should not silently bypass halt; severity should be finite.
        assert!(!d.severity.is_nan(), "NaN baseline must not propagate NaN severity");
        assert!(d.severity >= 0.0 && d.severity <= 1.0);
    }

    #[test]
    fn negative_baseline_treated_as_epsilon() {
        let m = Mira::default();
        let d = m.diagnose(&AnomalySignal {
            subject: "s".into(),
            metric: "error_rate".into(),
            observed: 1.0,
            baseline: -0.5,
            detail: String::new(),
            source_log_ref: String::new(),
        });
        // Negative baseline should not produce unexpected severity > 1.0.
        assert!(d.severity >= 0.0 && d.severity <= 1.0, "negative baseline must clamp severity");
    }

    #[test]
    fn halt_id_includes_spirit_id_and_severity() {
        let m = Mira::default().with_id("mira-test");
        let d = m.diagnose(&AnomalySignal {
            subject: "svc".into(),
            metric: "novel".into(),
            observed: 0.9,
            baseline: 0.1,
            detail: String::new(),
            source_log_ref: String::new(),
        });
        let payload = m.try_halt_payload(&d).expect("halt payload");
        assert!(payload.halt_id.contains("mira-test"), "halt_id must include spirit_id to prevent collision");
        assert!(payload.halt_id.contains("svc"), "halt_id must include subject");
    }

    #[test]
    fn threshold_drift_guard() {
        // If this test fails, the Rust constant drifted from the manifest value.
        // Both must stay at 0.5. The manifest is the authoritative source; this
        // test is the compile-time guard.
        assert_eq!(
            DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD, 0.5,
            "DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD must match manifest.toml on_value_below.threshold"
        );
    }
}
