//! ADR-012 typed-intent A2A consent.
//!
//! Per architecture §7.2: "Per-frame consent (ADR-012 typed-intent). Each
//! Host's manifest declares which intent classes it sends to its peer and
//! which it accepts from its peer. The kernel rejects frames whose typed
//! intent is not in the sender's send-allowlist or the receiver's
//! accept-allowlist with `EIntentDenied`."

use maos_domain::invariants::i8::A2AIntent;
use serde::{Deserialize, Serialize};

// Story 8.7 / AC2b — the dead `A2AConsentEnvelope` struct + its
// `From<ConsentEnvelope>` impl were DELETED here (Q5 team consensus, 2-1;
// Winston + Security DELETE, Murat dissent on abi-discipline grounds).
//
// Rationale: the type had ZERO non-test callers (the router enforces over
// `IacFrame.consent_envelope` directly, never via `A2AConsentEnvelope`), and
// its `From` silently coerced a *missing* intent to the `"standard"` privilege
// band (`intent_class: env.intent_class.unwrap_or_else(|| A2AIntent::new("standard"))`)
// — a latent fail-open *privilege elevation* the instant anyone wired the type
// onto the consent path. The team rejected "leave + doc-comment" as a
// decaying-promise / loaded-gun pattern and removed the gun entirely.
//
// This is the ONE ratified exception to AC8's abi-diff Added-only discipline
// (a maos-a2a-core public-surface Removed), flagged for Winston's sign-off at
// review — mirroring the 8.6 `ca_roots` fail-closed flag. (Note: the mechanical
// `abi-diff` discipline gate scans only `maos-spirit-abi`, which is untouched,
// so the gate itself stays GREEN; the Removed is on `maos-a2a-core`'s
// cargo-public-api surface.)

/// Per-peer send/accept allowlists per ADR-012.
///
/// `send_allowlist` is what THIS Host is willing to send TO the peer.
/// `accept_allowlist` is what THIS Host is willing to accept FROM the peer.
/// The two allowlists are direction-asymmetric — a Spirit may be willing to
/// send an intent but unwilling to receive it (or vice versa).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentAllowlists {
    /// Intent classes THIS Host sends to the peer.
    #[serde(default)]
    pub send_allowlist: Vec<A2AIntent>,
    /// Intent classes THIS Host accepts from the peer.
    #[serde(default)]
    pub accept_allowlist: Vec<A2AIntent>,
}

impl ConsentAllowlists {
    pub fn send_admits(&self, intent: &A2AIntent) -> bool {
        self.send_allowlist.iter().any(|i| i == intent)
    }

    pub fn accept_admits(&self, intent: &A2AIntent) -> bool {
        self.accept_allowlist.iter().any(|i| i == intent)
    }
}

/// ADR-012 EIntentDenied — the structured rejection for cross-Host consent
/// failures. Carries enough context for the JSON-RPC NACK encoder + the
/// Approval Decision Log row.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[error("intent {intent} not in {direction:?} allowlist for peer {peer}")]
pub struct EIntentDenied {
    pub peer: String,
    pub intent: String,
    pub direction: AllowlistDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllowlistDirection {
    Send,
    Accept,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(s: &str) -> A2AIntent {
        A2AIntent::new(s)
    }

    #[test]
    fn allowlist_send_admits_match() {
        let a = ConsentAllowlists {
            send_allowlist: vec![intent("diagnosis-handoff:read-only-evidence")],
            accept_allowlist: vec![],
        };
        assert!(a.send_admits(&intent("diagnosis-handoff:read-only-evidence")));
        assert!(!a.send_admits(&intent("code-mutation-directive")));
    }

    #[test]
    fn allowlist_accept_admits_match() {
        let a = ConsentAllowlists {
            send_allowlist: vec![],
            accept_allowlist: vec![intent("rca-summary")],
        };
        assert!(a.accept_admits(&intent("rca-summary")));
        assert!(!a.accept_admits(&intent("code-mutation-directive")));
    }

    #[test]
    fn allowlist_empty_admits_nothing() {
        let a = ConsentAllowlists::default();
        assert!(!a.send_admits(&intent("anything")));
        assert!(!a.accept_admits(&intent("anything")));
    }

    #[test]
    fn allowlist_serde_round_trip() {
        let a = ConsentAllowlists {
            send_allowlist: vec![intent("a"), intent("b")],
            accept_allowlist: vec![intent("c")],
        };
        let json = serde_json::to_string(&a).expect("serialize");
        let back: ConsentAllowlists = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.send_allowlist.len(), 2);
        assert_eq!(back.accept_allowlist.len(), 1);
    }

    #[test]
    fn eintent_denied_display() {
        let e = EIntentDenied {
            peer: "host-b".into(),
            intent: "code-mutation-directive".into(),
            direction: AllowlistDirection::Send,
        };
        assert!(format!("{e}").contains("host-b"));
    }
}
