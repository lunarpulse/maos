//! NFR-Rel-7 / NFR-Scale-2 A2A churn-drill report — Story 11.3.
//!
//! Story 11.3 REPLACES the v0.5 `run_scaffold` canned-constant scaffold
//! (`detection=30`/`recovery=60`/`blast=min(adv,host)` literals — the exact
//! 10.2/J4 "green by construction" pattern) with a report type whose EVERY
//! numeric field is DERIVED from real events on a live N-host mTLS mesh
//! (`maos-a2a-tcp/tests/t_11_3_scale_churn.rs`). This module adds NO
//! detection logic — it observes and times the EXISTING detectors at both
//! the mTLS handshake layer (`maos-a2a-tcp/src/verifier.rs`) and the router
//! NACK layer (`maos-a2a-core::router::A2ARouterCore`) — F-new, two-surface.
//!
//! Binding vs reported (F3-ledger, ratified 2026-07-03): `max_blast_radius`
//! and `recovery_secs` are BINDING v2.0 gate floors (NFR-Rel-7-named);
//! `rto_secs` is DERIVED + REPORTED + advisory-if-breached (own falsifier),
//! NOT a v2.0 ship-block — a real >4h RTO breach is physically unobservable
//! on the co-located loopback mesh this drill runs on (L5). `rto_secs`
//! PROMOTES to a binding floor at v2.5 once real geo-distributed hosts make a
//! breach observable. `rto_secs` is `None` iff the harness cannot construct
//! BOTH independent falsifiers (isolation-blind / re-pin-blind) — cut by
//! construction rather than shipped un-falsifiable (D5).

use std::collections::BTreeSet;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

/// Adversarial attempt class (per architecture §7.2 threat row). WIRED by
/// Story 11.3 (previously orphaned — zero references workspace-wide): each
/// variant is planted as a real host into the live mesh and detected at the
/// surface named in its doc comment (F-new, two-surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdversarialAttempt {
    /// Unpinned/spoofed leaf fingerprint — rejected at the mTLS HANDSHAKE
    /// layer (`verifier.rs` TOFU pin check, run AFTER WebPKI succeeds) as a
    /// `TcpTransportError::TofuPinMismatch` → `A2AError::HandshakeFailed`.
    TofuPinSpoofing,
    /// A validly-pinned peer sends an intent outside its accept-allowlist —
    /// rejected at the ROUTER NACK layer (`CODE_INTENT_DENIED`,
    /// `router.rs::handle_intake`) as `A2AError::IntentDeniedAtPeer`.
    AdrLevel012ConsentBypass,
    /// Stale/expired leaf presented post-rotation-grace — rejected at the
    /// mTLS HANDSHAKE layer (`verifier.rs` WebPKI validity step, which runs
    /// BEFORE any pin check) as `TcpTransportError::CertExpired` →
    /// `A2AError::HandshakeFailed`.
    CertRotationRaceExploit,
}

impl AdversarialAttempt {
    /// Stable label for report/CLI rendering.
    pub fn label(self) -> &'static str {
        match self {
            AdversarialAttempt::TofuPinSpoofing => "tofu_pin_spoofing",
            AdversarialAttempt::AdrLevel012ConsentBypass => "adr_level_012_consent_bypass",
            AdversarialAttempt::CertRotationRaceExploit => "cert_rotation_race_exploit",
        }
    }

    /// Which surface this class is detected at (F-new, two-surface fix — the
    /// as-drafted "first router NACK" was a spec defect for the two
    /// handshake-layer classes).
    pub fn detection_surface(self) -> &'static str {
        match self {
            AdversarialAttempt::TofuPinSpoofing | AdversarialAttempt::CertRotationRaceExploit => {
                "handshake"
            }
            AdversarialAttempt::AdrLevel012ConsentBypass => "router_nack",
        }
    }
}

/// One planted adversary's real-event timing + reachability, derived from the
/// SAME dial sweep the blast-radius side-effect consumes (one instrument, two
/// derivations — AC2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialDetection {
    pub adversary_id: String,
    /// The adversary's SERVED cert fingerprint (wire form) — the distinct
    /// crypto identity witness `reconcile_detections` keys on (L8/D7), never a
    /// bare label. Two detections that collapse to one fingerprint (a blinded
    /// class, or a clone masquerading as two hosts) fail the reconcile.
    pub adversary_fingerprint: String,
    pub attack_class: AdversarialAttempt,
    /// Join instant, nanoseconds on the harness's OWN monotonic `Instant`
    /// base (L4 — a same-process monotonic reading, never a cross-host clock
    /// subtraction, never a frame's wall-clock `timestamp`).
    pub join_ns: u64,
    /// First rejection instant at EITHER surface — `None` iff the adversary
    /// was never rejected (a detection miss; the non-degenerate/identity
    /// reconcile in the drill turns this into a hard failure, never a
    /// silently-dropped sample).
    pub first_rejection_ns: Option<u64>,
    /// Legitimate peers this adversary reached BEFORE its first rejection
    /// isolated it (raw material for `max_blast_radius`).
    pub blast_peers: BTreeSet<String>,
}

impl AdversarialDetection {
    /// `first_rejection_ns − join_ns` in whole seconds; `None` if never
    /// detected. Compressed-loopback events are sub-second, so this is
    /// usually `0` — the raw nanosecond delta (not this rounded value) is
    /// what the non-degenerate-distribution check reads (L5).
    pub fn detection_latency_secs(&self) -> Option<u64> {
        self.first_rejection_ns
            .map(|t| t.saturating_sub(self.join_ns) / 1_000_000_000)
    }

    /// Raw nanosecond detection latency — the non-degenerate-distribution
    /// check reads THIS (not the rounded-to-seconds value, which collapses to
    /// `0` for every sample on a sub-second loopback drill).
    pub fn detection_latency_ns(&self) -> Option<u64> {
        self.first_rejection_ns
            .map(|t| t.saturating_sub(self.join_ns))
    }
}

/// Derived-and-reconciled real-event churn drill report (Story 11.3, replaces
/// the v0.5 `run_scaffold` canned constants). Every numeric field traces to a
/// real measured event on the live mesh — NEVER a literal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnDrillReport {
    pub drill_id: String,
    /// Distinct cert fingerprints (wire form, `sha256:...`) observed across
    /// the planted mesh — one half of the derive-and-reconcile numerator for
    /// "N hosts" (never the literal host count on its own).
    pub host_fingerprints: BTreeSet<String>,
    /// Distinct bound `SocketAddr`s observed across the planted mesh — the
    /// other half of the derive-and-reconcile numerator.
    pub host_addrs: BTreeSet<SocketAddr>,
    pub per_adversary: Vec<AdversarialDetection>,
    /// Fleet median (p50) detection latency across `per_adversary` — NFR-Rel-7
    /// floor ≤3600s (1h), BINDING at v2.0.
    pub detection_latency_median_secs: u64,
    pub detection_latency_p99_secs: u64,
    /// Max distinct legitimate peers any adversary reached before isolation —
    /// NFR-Rel-7 floor ≤5, BINDING at v2.0.
    pub max_blast_radius: u32,
    /// Detection → fleet trust re-establishment — NFR-Rel-7 floor ≤86400s
    /// (24h), BINDING at v2.0.
    pub recovery_secs: u64,
    /// Detection → adversary isolated AND all legitimate peers reachable
    /// again — REPORTED against ≤14400s (4h); advisory-if-breached, NOT a
    /// v2.0 ship-block (D5/F3-ledger). `None` iff the harness could not
    /// construct BOTH independent falsifiers (isolation-blind / re-pin-blind)
    /// — cut by construction rather than shipped un-falsifiable.
    pub rto_secs: Option<u64>,
}

impl ChurnDrillReport {
    /// Derive every field from real events — reuses the `rotation.rs`
    /// percentile engine (per-event samples → distribution → floor check),
    /// the exact pattern `RotationDrillReport::from_per_agent` established.
    pub fn from_real_events(
        drill_id: impl Into<String>,
        host_fingerprints: BTreeSet<String>,
        host_addrs: BTreeSet<SocketAddr>,
        per_adversary: Vec<AdversarialDetection>,
        recovery_ns: u64,
        rto_ns: Option<u64>,
    ) -> Self {
        let detection_samples: Vec<u64> = per_adversary
            .iter()
            .filter_map(|a| a.detection_latency_secs())
            .collect();
        let (detection_p50, detection_p99) = super::rotation::percentiles(&detection_samples);
        let max_blast_radius = per_adversary
            .iter()
            .map(|a| a.blast_peers.len() as u32)
            .max()
            .unwrap_or(0);
        Self {
            drill_id: drill_id.into(),
            host_fingerprints,
            host_addrs,
            per_adversary,
            detection_latency_median_secs: detection_p50,
            detection_latency_p99_secs: detection_p99,
            max_blast_radius,
            recovery_secs: recovery_ns / 1_000_000_000,
            rto_secs: rto_ns.map(|ns| ns / 1_000_000_000),
        }
    }

    /// Derive-and-reconcile: the SMALLER of the two distinct-identity witness
    /// sets (cert fingerprint, bound `SocketAddr`) — a duplicate on EITHER
    /// axis collapses the reconciled count below the literal N (L8/D7). The
    /// ≥25 host floor is asserted against THIS, never a literal `host_count`.
    pub fn distinct_host_count(&self) -> usize {
        self.host_fingerprints.len().min(self.host_addrs.len())
    }

    /// BINDING v2.0 floors — a non-empty detected set AND detection median
    /// ≤3600s AND detection p99 ≤3600s (the stricter tail companion to the
    /// NFR-named median — was previously computed-but-ungated) AND
    /// `max_blast_radius` ≤5 AND `recovery_secs` ≤86400. `rto_secs` is
    /// REPORTED/advisory (F3-ledger) and NEVER gates this predicate. The
    /// non-empty guard closes the vacuous-pass hole: a zero-detection report
    /// can no longer clear the floors by construction.
    pub fn passes_v20_binding_floors(&self) -> bool {
        !self.per_adversary.is_empty()
            && self.detection_latency_median_secs <= 3600
            && self.detection_latency_p99_secs <= 3600
            && self.max_blast_radius <= 5
            && self.recovery_secs <= 86_400
    }

    /// Adversarial-host-identity reflex (L8/D7/AC2): the detected set must
    /// reconcile to exactly `planted` DISTINCT adversary identities — each
    /// detection carries a real rejection AND a distinct cert fingerprint.
    /// A blinded class (drops the count 3→2), a missed detection
    /// (`first_rejection_ns == None`), or two detections collapsing to one
    /// fingerprint all hard-fail here. This is the downstream tooth the
    /// `churn-fault-inject` blind mutation REDS — a count/identity contract,
    /// never `Iterator::filter` semantics.
    pub fn reconcile_detections(&self, planted: usize) -> Result<(), String> {
        if self.per_adversary.len() != planted {
            return Err(format!(
                "detected-count reconcile: {} detections, expected {planted} planted adversaries",
                self.per_adversary.len()
            ));
        }
        if let Some(missed) = self
            .per_adversary
            .iter()
            .find(|d| d.first_rejection_ns.is_none())
        {
            return Err(format!(
                "detection miss: adversary {} ({:?}) was never rejected",
                missed.adversary_id, missed.attack_class
            ));
        }
        let distinct: BTreeSet<&str> = self
            .per_adversary
            .iter()
            .map(|d| d.adversary_fingerprint.as_str())
            .collect();
        if distinct.len() != planted {
            return Err(format!(
                "identity reconcile: {} detections collapse to {} distinct fingerprints (expected {planted})",
                self.per_adversary.len(),
                distinct.len()
            ));
        }
        Ok(())
    }

    /// `true` iff `rto_secs` is present and exceeds the 4h reported floor —
    /// the "⚠️ RTO EXCEEDED" advisory line (v2.0 advisory, binds at v2.5).
    pub fn rto_exceeded_advisory(&self) -> bool {
        self.rto_secs.is_some_and(|s| s > 14_400)
    }
}

/// Markdown rendering for the churn report — appended to
/// `_bmad-output/implementation-artifacts/a2a-churn-report.md`.
pub fn report_to_markdown(report: &ChurnDrillReport) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string_pretty(report)?;
    let rto_line = match report.rto_secs {
        Some(secs) if report.rto_exceeded_advisory() => {
            format!("\n⚠️ RTO EXCEEDED — {secs}s > 14400s (v2.0 advisory; binds at v2.5)\n")
        }
        Some(secs) => format!("\nrto_secs = {secs}s (within the 4h reported floor)\n"),
        None => "\nrto_secs CUT — the harness could not construct independent \
                  isolation-blind + re-pin-blind falsifiers (F3, decide-by-construction)\n"
            .to_string(),
    };
    Ok(format!(
        "\n## Churn drill {}\n\n_Real N≥25 (target 30) mesh, compressed-loopback regression \
         floors — NOT geo/production figures (L5)._\n{rto_line}\n```json\n{json}\n```\n",
        report.drill_id
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(
        id: &str,
        class: AdversarialAttempt,
        join_ns: u64,
        rej_ns: Option<u64>,
        blast: &[&str],
    ) -> AdversarialDetection {
        AdversarialDetection {
            adversary_id: id.into(),
            adversary_fingerprint: format!("sha256:fp-{id}"),
            attack_class: class,
            join_ns,
            first_rejection_ns: rej_ns,
            blast_peers: blast.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn addr(port: u16) -> SocketAddr {
        format!("127.0.0.1:{port}").parse().unwrap()
    }

    #[test]
    fn from_real_events_derives_every_field_from_samples() {
        let fingerprints: BTreeSet<String> = (0..25).map(|i| format!("sha256:{i:064x}")).collect();
        let addrs: BTreeSet<SocketAddr> = (0..25).map(|i| addr(10_000 + i)).collect();
        let report = ChurnDrillReport::from_real_events(
            "t1",
            fingerprints,
            addrs,
            vec![
                det(
                    "a0",
                    AdversarialAttempt::TofuPinSpoofing,
                    0,
                    Some(500_000_000),
                    &["h1", "h2"],
                ),
                det(
                    "a1",
                    AdversarialAttempt::AdrLevel012ConsentBypass,
                    0,
                    Some(300_000_000),
                    &["h3"],
                ),
                det(
                    "a2",
                    AdversarialAttempt::CertRotationRaceExploit,
                    0,
                    Some(700_000_000),
                    &["h1", "h4", "h5"],
                ),
            ],
            2_000_000_000,
            Some(1_000_000_000),
        );
        assert_eq!(report.max_blast_radius, 3);
        assert_eq!(report.recovery_secs, 2);
        assert_eq!(report.rto_secs, Some(1));
        assert_eq!(report.distinct_host_count(), 25);
        assert!(report.passes_v20_binding_floors());
        assert!(!report.rto_exceeded_advisory());
    }

    #[test]
    fn passes_v20_binding_floors_reds_on_blast_radius_breach() {
        let report = ChurnDrillReport::from_real_events(
            "t2",
            BTreeSet::new(),
            BTreeSet::new(),
            vec![det(
                "a0",
                AdversarialAttempt::TofuPinSpoofing,
                0,
                Some(1_000_000_000),
                &["h1", "h2", "h3", "h4", "h5", "h6"],
            )],
            10_000_000_000,
            None,
        );
        assert_eq!(report.max_blast_radius, 6);
        assert!(!report.passes_v20_binding_floors());
    }

    #[test]
    fn passes_v20_binding_floors_reds_on_recovery_breach() {
        let report = ChurnDrillReport::from_real_events(
            "t3",
            BTreeSet::new(),
            BTreeSet::new(),
            vec![det(
                "a0",
                AdversarialAttempt::TofuPinSpoofing,
                0,
                Some(1_000_000_000),
                &["h1"],
            )],
            90_000 * 1_000_000_000, // 90000s > 86400s (24h) floor
            None,
        );
        assert!(!report.passes_v20_binding_floors());
    }

    #[test]
    fn rto_exceeded_advisory_does_not_gate_binding_floors() {
        let report = ChurnDrillReport::from_real_events(
            "t4",
            BTreeSet::new(),
            BTreeSet::new(),
            vec![det(
                "a0",
                AdversarialAttempt::TofuPinSpoofing,
                0,
                Some(1_000_000_000),
                &["h1"],
            )],
            10_000_000_000,               // recovery = 10s — well under the 24h floor
            Some(15_000 * 1_000_000_000), // rto = 15000s > 14400s (4h) floor
        );
        assert!(report.rto_exceeded_advisory());
        assert!(
            report.passes_v20_binding_floors(),
            "an RTO breach MUST NOT red the binding leg (F3-ledger: rto_secs is reported, not binding)"
        );
    }

    #[test]
    fn rto_cut_when_not_separable_is_none_not_a_default_zero() {
        let report = ChurnDrillReport::from_real_events(
            "t5",
            BTreeSet::new(),
            BTreeSet::new(),
            vec![],
            0,
            None,
        );
        assert_eq!(report.rto_secs, None);
        assert!(!report.rto_exceeded_advisory());
    }

    #[test]
    fn distinct_host_count_reconciles_to_the_smaller_witness_set() {
        let mut fps = BTreeSet::new();
        fps.insert("sha256:a".to_string()); // duplicate fingerprint collapses to 1 distinct entry
        let mut addrs = BTreeSet::new();
        addrs.insert(addr(1));
        addrs.insert(addr(2));
        let report = ChurnDrillReport::from_real_events("t6", fps, addrs, vec![], 0, None);
        assert_eq!(
            report.distinct_host_count(),
            1,
            "a duplicate identity on ANY witness axis must collapse the reconciled count"
        );
    }

    #[test]
    fn detection_latency_ns_survives_sub_second_rounding_for_non_degeneracy_checks() {
        let a = det(
            "a0",
            AdversarialAttempt::TofuPinSpoofing,
            0,
            Some(500_000),
            &[],
        );
        // Rounds to 0 seconds (sub-second loopback, L5) but the raw ns delta
        // is preserved for the non-degenerate-distribution check.
        assert_eq!(a.detection_latency_secs(), Some(0));
        assert_eq!(a.detection_latency_ns(), Some(500_000));
    }

    #[test]
    fn attack_class_detection_surface_is_two_surface_per_f_new() {
        assert_eq!(
            AdversarialAttempt::TofuPinSpoofing.detection_surface(),
            "handshake"
        );
        assert_eq!(
            AdversarialAttempt::CertRotationRaceExploit.detection_surface(),
            "handshake"
        );
        assert_eq!(
            AdversarialAttempt::AdrLevel012ConsentBypass.detection_surface(),
            "router_nack"
        );
    }

    #[test]
    fn reconcile_detections_reds_on_count_identity_and_miss() {
        let three = vec![
            det("a0", AdversarialAttempt::TofuPinSpoofing, 0, Some(1), &[]),
            det(
                "a1",
                AdversarialAttempt::AdrLevel012ConsentBypass,
                0,
                Some(2),
                &[],
            ),
            det(
                "a2",
                AdversarialAttempt::CertRotationRaceExploit,
                0,
                Some(3),
                &[],
            ),
        ];
        let ok = ChurnDrillReport::from_real_events(
            "rec-ok",
            BTreeSet::new(),
            BTreeSet::new(),
            three.clone(),
            0,
            None,
        );
        assert!(ok.reconcile_detections(3).is_ok());

        // A blinded class drops the count 3->2 → reconcile REDS (the downstream
        // tooth the churn-fault-inject blind mutation exercises).
        let blinded: Vec<_> = three
            .iter()
            .filter(|d| d.attack_class != AdversarialAttempt::TofuPinSpoofing)
            .cloned()
            .collect();
        let blinded_report = ChurnDrillReport::from_real_events(
            "rec-blind",
            BTreeSet::new(),
            BTreeSet::new(),
            blinded,
            0,
            None,
        );
        assert!(
            blinded_report.reconcile_detections(3).is_err(),
            "3->2 count drop must red"
        );

        // Two detections collapsing to one fingerprint (clone) → reconcile REDS.
        let mut clone = det("a3", AdversarialAttempt::TofuPinSpoofing, 0, Some(4), &[]);
        clone.adversary_fingerprint = three[0].adversary_fingerprint.clone();
        let dup = ChurnDrillReport::from_real_events(
            "rec-dup",
            BTreeSet::new(),
            BTreeSet::new(),
            vec![three[0].clone(), clone],
            0,
            None,
        );
        assert!(
            dup.reconcile_detections(2).is_err(),
            "duplicate fingerprint must red"
        );

        // A detection miss (never rejected) → reconcile REDS.
        let missed = vec![det("a0", AdversarialAttempt::TofuPinSpoofing, 0, None, &[])];
        let miss_report = ChurnDrillReport::from_real_events(
            "rec-miss",
            BTreeSet::new(),
            BTreeSet::new(),
            missed,
            0,
            None,
        );
        assert!(
            miss_report.reconcile_detections(1).is_err(),
            "undetected adversary must red"
        );
    }
}
