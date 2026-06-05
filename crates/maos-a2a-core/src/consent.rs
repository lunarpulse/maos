//! ADR-012 typed-intent A2A consent.
//!
//! Per architecture §7.2: "Per-frame consent (ADR-012 typed-intent). Each
//! Host's manifest declares which intent classes it sends to its peer and
//! which it accepts from its peer. The kernel rejects frames whose typed
//! intent is not in the sender's send-allowlist or the receiver's
//! accept-allowlist with `EIntentDenied`."

use maos_domain::frame::FrameAddress;
use maos_domain::invariants::i8::A2AIntent;
use serde::{Deserialize, Serialize};

/// Pre-frame consent envelope per ADR-012 binding-v0.9.
///
/// Extends the v0.3-β `maos_domain::frame::ConsentEnvelope` with the typed
/// intent class (Story 6.3 fills in the ADR-012 hook). The original
/// `ConsentEnvelope` is extended additively in `maos-domain` — this struct
/// is the typed projection the A2A surface operates over (the conversion
/// to/from the domain envelope is in `adapter.rs` via `From` impl below).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2AConsentEnvelope {
    pub consent_id: [u8; 16],
    pub granter: FrameAddress,
    pub timestamp_ns: u64,
    /// ADR-012 binding-v0.9 — typed-intent class for cross-Host consent.
    pub intent_class: A2AIntent,
    /// When the envelope expires. `None` = no expiry (open-ended consent at v0.5).
    pub valid_until_ns: Option<u64>,
}

impl From<maos_domain::frame::ConsentEnvelope> for A2AConsentEnvelope {
    fn from(env: maos_domain::frame::ConsentEnvelope) -> Self {
        Self {
            consent_id: env.consent_id,
            granter: env.granter,
            timestamp_ns: env.timestamp_ns,
            intent_class: env.intent_class.unwrap_or_else(|| A2AIntent::new("standard")),
            valid_until_ns: env.valid_until_ns,
        }
    }
}

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
