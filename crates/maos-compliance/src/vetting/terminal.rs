#![forbid(unsafe_code)]

//! Four distinguishable terminal causes (ADV-056-2, AC4).
//!
//! None of these four is a unified audit concept elsewhere today:
//! `registry-yank` has a reusable path (`maos-registry` yank + TL
//! `FrameKind::SpiritRevoked`), `expiry-lapse` is adjacent-but-distinct
//! (ComplianceClaim / cap-token expiry, keyed to a different object — NOT
//! conflated here), and `vetting-revocation` + `operator-local` are net-new.
//! This module renders the four-way distinguishability in one audit taxonomy:
//! each cause is independently observable and labeled, with a **defined
//! precedence** so a planted mislabel (a revocation logged as expiry, a yank
//! logged as operator-local) is detectable.
//!
//! v2.2 disposition is `refuse-at-next-load` only (drain-and-refuse is the
//! named v2.5 slot — honest zero-kernel-Δ), plus a **mandatory journaled
//! observation event** ([`RunningSpiritObservation`]) when the compliance layer
//! detects expiry/revocation while an affected Spirit is running.

/// Why a `public-vetted` promotion has terminated. The four causes are
/// pairwise distinct and each independently observable in the audit surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VettingTerminalCause {
    /// The vetter key or attestation was explicitly revoked (keyring `Revoke`).
    VettingRevocation,
    /// The attestation validity window lapsed (`now >= expires_at`).
    ExpiryLapse,
    /// The registry yanked the package (SignedRevocationList / SpiritRevoked).
    RegistryYank,
    /// The operator locally disabled the promotion (operator-local decision).
    OperatorLocal,
}

impl VettingTerminalCause {
    /// The stable, machine-checkable audit label for this cause. This is the
    /// string the four-way distinguishability leg asserts against — a mislabel
    /// in code reds the gate.
    pub fn audit_label(&self) -> &'static str {
        match self {
            VettingTerminalCause::VettingRevocation => "vetting-revocation",
            VettingTerminalCause::ExpiryLapse => "expiry-lapse",
            VettingTerminalCause::RegistryYank => "registry-yank",
            VettingTerminalCause::OperatorLocal => "operator-local",
        }
    }
}

/// The v2.2 disposition applied when a terminal cause is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalDisposition {
    /// v2.2: the next admission of this Spirit is refused. Running Spirits are
    /// observed-and-journaled only (drain-and-refuse is v2.5).
    RefuseAtNextLoad,
}

/// The signals that can terminate a promotion, gathered from the four
/// independent axes. Each is an orthogonal input; [`classify_terminal_cause`]
/// resolves them to a single authoritative cause under a defined precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalInputs {
    /// The registry yanked this package (`maos-registry` yank / SpiritRevoked).
    pub registry_yanked: bool,
    /// The vetter key or attestation was revoked in the keyring.
    pub vetting_revoked: bool,
    /// The operator locally disabled the promotion.
    pub operator_local_disable: bool,
    /// The attestation's validity window has lapsed (`now >= expires_at`).
    pub attestation_expired: bool,
}

/// Classify the single authoritative terminal cause, or `None` if the promotion
/// is still live.
///
/// Precedence (highest first): registry-yank → vetting-revocation →
/// operator-local → expiry-lapse. The precedence is fixed so that when multiple
/// signals fire the audit surface reports the strongest, and — critically —
/// each cause maps to exactly one label, so a swapped label in this function
/// reds the four-way-distinguishability leg.
pub fn classify_terminal_cause(inputs: &TerminalInputs) -> Option<VettingTerminalCause> {
    if inputs.registry_yanked {
        Some(VettingTerminalCause::RegistryYank)
    } else if inputs.vetting_revoked {
        Some(VettingTerminalCause::VettingRevocation)
    } else if inputs.operator_local_disable {
        Some(VettingTerminalCause::OperatorLocal)
    } else if inputs.attestation_expired {
        Some(VettingTerminalCause::ExpiryLapse)
    } else {
        None
    }
}

/// A journaled observation emitted when the compliance layer detects a terminal
/// cause while an affected Spirit is still running (AC4). v2.2 does NOT drain;
/// it records the observation and refuses the next load.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunningSpiritObservation {
    /// Spirit identity affected.
    pub spirit_id: String,
    /// Spirit version affected.
    pub spirit_version: String,
    /// Manifest exact-hash when the detecting surface has it.
    pub manifest_hash_hex: Option<String>,
    /// The single authoritative terminal cause.
    pub cause: VettingTerminalCause,
    /// The v2.2 disposition (always refuse-at-next-load).
    pub disposition: TerminalDisposition,
    /// Wall-clock the observation was made (ms).
    pub detected_at_unix_ms: u64,
}

impl RunningSpiritObservation {
    /// Construct the v2.2 observation for a detected `cause`.
    pub fn new(
        spirit_id: impl Into<String>,
        spirit_version: impl Into<String>,
        manifest_hash: &[u8; 32],
        cause: VettingTerminalCause,
        detected_at_unix_ms: u64,
    ) -> Self {
        Self {
            spirit_id: spirit_id.into(),
            spirit_version: spirit_version.into(),
            manifest_hash_hex: Some(hex::encode(manifest_hash)),
            cause,
            disposition: TerminalDisposition::RefuseAtNextLoad,
            detected_at_unix_ms,
        }
    }

    /// The labeled audit line (the observable audit surface, AC4).
    pub fn journal_note(&self) -> String {
        format!(
            "vetting-terminal cause={} disposition=refuse-at-next-load spirit={} v{} manifest={}",
            self.cause.audit_label(),
            self.spirit_id,
            self.spirit_version,
            self.manifest_hash_hex.as_deref().unwrap_or("unavailable"),
        )
    }
}

/// Audit sink used by the runtime detector and by production TL adapters.
pub trait TerminalObservationSink: Send + Sync {
    fn journal_terminal_observation(&self, observation: &RunningSpiritObservation);
}

/// Classify live terminal inputs, construct the mandatory running-Spirit
/// observation, and dispatch it through the audit sink.
pub fn observe_running_spirit(
    spirit_id: impl Into<String>,
    spirit_version: impl Into<String>,
    manifest_hash: Option<&[u8; 32]>,
    inputs: &TerminalInputs,
    detected_at_unix_ms: u64,
    sink: &dyn TerminalObservationSink,
) -> Option<RunningSpiritObservation> {
    let cause = classify_terminal_cause(inputs)?;
    let observation = RunningSpiritObservation {
        spirit_id: spirit_id.into(),
        spirit_version: spirit_version.into(),
        manifest_hash_hex: manifest_hash.map(hex::encode),
        cause,
        disposition: TerminalDisposition::RefuseAtNextLoad,
        detected_at_unix_ms,
    };
    sink.journal_terminal_observation(&observation);
    Some(observation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_cause_classifies_in_isolation() {
        assert_eq!(
            classify_terminal_cause(&TerminalInputs {
                registry_yanked: true,
                ..Default::default()
            }),
            Some(VettingTerminalCause::RegistryYank)
        );
        assert_eq!(
            classify_terminal_cause(&TerminalInputs {
                vetting_revoked: true,
                ..Default::default()
            }),
            Some(VettingTerminalCause::VettingRevocation)
        );
        assert_eq!(
            classify_terminal_cause(&TerminalInputs {
                operator_local_disable: true,
                ..Default::default()
            }),
            Some(VettingTerminalCause::OperatorLocal)
        );
        assert_eq!(
            classify_terminal_cause(&TerminalInputs {
                attestation_expired: true,
                ..Default::default()
            }),
            Some(VettingTerminalCause::ExpiryLapse)
        );
    }

    #[test]
    fn live_promotion_has_no_cause() {
        assert_eq!(classify_terminal_cause(&TerminalInputs::default()), None);
    }

    /// The four labels are pairwise distinct (the audit surface can tell them
    /// apart) — a swapped label collapses two and reds here.
    #[test]
    fn four_labels_are_pairwise_distinct() {
        let labels = [
            VettingTerminalCause::VettingRevocation.audit_label(),
            VettingTerminalCause::ExpiryLapse.audit_label(),
            VettingTerminalCause::RegistryYank.audit_label(),
            VettingTerminalCause::OperatorLocal.audit_label(),
        ];
        let uniq: std::collections::BTreeSet<&str> = labels.iter().copied().collect();
        assert_eq!(uniq.len(), 4, "terminal-cause labels collided");
    }

    /// Planted mislabel: a revocation must NOT be reported as expiry.
    #[test]
    fn revocation_is_not_mislabeled_as_expiry() {
        let cause = classify_terminal_cause(&TerminalInputs {
            vetting_revoked: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(cause.audit_label(), "vetting-revocation");
        assert_ne!(cause.audit_label(), "expiry-lapse");
    }

    #[test]
    fn observation_note_carries_labeled_cause() {
        let obs = RunningSpiritObservation::new(
            "vetted-spirit",
            "0.1.0",
            &[0xAB; 32],
            VettingTerminalCause::RegistryYank,
            1_234,
        );
        assert!(obs.journal_note().contains("cause=registry-yank"));
        assert_eq!(obs.disposition, TerminalDisposition::RefuseAtNextLoad);
    }
}
