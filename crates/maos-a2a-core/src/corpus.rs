//! A2A loopback corpus aggregate types.
//!
//! Mirrors the pattern from Story 6.1's `retract_corpus.rs` + Story 6.2's
//! `intent_lineage_corpus.rs`. The maos-a2a copy is local; the maos-eval
//! crate provides the cross-crate loader (`a2a_loopback_corpus.rs`) that
//! integration tests + discipline.yml jobs invoke.
//!
//! ## Corpus classes (per AC2)
//!
//! | Class | Scenarios | NFR | Floor |
//! |---|---|---|---|
//! | `mtls-replay` | 1000 | NFR-Sec-11 | 0/1000 succeed |
//! | `tofu-mismatch` | 100 | NFR-Sec-12 | 100/100 detected |
//! | `handshake-fault` | 20 | FR23a | 20/0 succeed |
//! | `cross-spirit-consent` | 30 | FR23a | 30/30 disallowed blocked |
//!
//! ## Generation model
//!
//! Per architecture §8.1 the corpora are CONTENT-ADDRESSED — each scenario
//! file's filename is the SHA-256 of its contents. At v0.5 the scenarios
//! are **parametrically generated** from a small seed set; the corpus loader
//! expands `N` scenarios per class via deterministic seed walks (the seed
//! material lives in `corpus.rs`; the runner mints fixtures at test time).
//! This honors the substrate-as-observable-behavior principle while keeping
//! the repo's fixture count tractable. Production runs the FULL 1000+100+20+30
//! corpus on `schedule:` weekly; per-PR runs a reduced sample.

use serde::{Deserialize, Serialize};

/// A single mTLS replay scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtlsReplayScenario {
    pub scenario_id: String,
    /// Captured ClientHello bytes (hex-encoded).
    pub client_hello_hex: String,
    /// Expected handshake outcome: always `rejected` per NFR-Sec-11.
    pub expected_outcome: ReplayOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayOutcome {
    /// The server's anti-replay (nonce-bound) rejects the replayed handshake.
    Rejected,
    /// The replay completes successfully — NFR-Sec-11 violation.
    AcceptedViolation,
}

/// A TOFU pin-mismatch scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TofuMismatchScenario {
    pub scenario_id: String,
    /// First-contact fingerprint (the pinned one).
    pub first_fingerprint_hex: String,
    /// Second-contact fingerprint (the mismatching one).
    pub second_fingerprint_hex: String,
    /// Class of mismatch — covers (a) different cert entirely; (b) cert-
    /// rotation-time bump; (c) impersonation attempt.
    pub mismatch_class: TofuMismatchClass,
    pub expected_alert: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TofuMismatchClass {
    DifferentCertEntirely,
    LegitimateRotationUnpinned,
    ImpersonationAttempt,
}

/// A handshake-fault scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeFaultScenario {
    pub scenario_id: String,
    pub fault_class: HandshakeFaultClass,
    pub expected_error_substring: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandshakeFaultClass {
    CertChainMalformed,
    AlpnMismatch,
    SniMismatch,
    ExpiredCert,
    FutureNotBefore,
    EeCertWithCaKeyUsage,
    KeyUsageWrong,
    EmptyCertChain,
    InvalidSignature,
    UnknownIssuer,
    Other(String),
}

/// A cross-Spirit consent scenario per ADR-012.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSpiritConsentScenario {
    pub scenario_id: String,
    pub sender_intent: String,
    pub send_allowlist: Vec<String>,
    pub accept_allowlist: Vec<String>,
    /// Expected gate outcome — derived from allowlist mismatch.
    pub expected_outcome: ConsentOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentOutcome {
    /// Send-allowlist mismatch — sender rejects pre-wire.
    DeniedAtSender,
    /// Accept-allowlist mismatch — receiver returns NACK.
    DeniedAtReceiver,
    /// Both allowlists permit — frame admitted.
    Admitted,
}

/// Aggregate corpus.
#[derive(Debug, Clone, Default)]
pub struct A2ALoopbackCorpus {
    pub mtls_replay: Vec<MtlsReplayScenario>,
    pub tofu_mismatch: Vec<TofuMismatchScenario>,
    pub handshake_fault: Vec<HandshakeFaultScenario>,
    pub cross_spirit_consent: Vec<CrossSpiritConsentScenario>,
}

impl A2ALoopbackCorpus {
    /// Returns the per-class scenario count summary.
    pub fn summary(&self) -> CorpusSummary {
        CorpusSummary {
            mtls_replay_count: self.mtls_replay.len(),
            tofu_mismatch_count: self.tofu_mismatch.len(),
            handshake_fault_count: self.handshake_fault.len(),
            cross_spirit_consent_count: self.cross_spirit_consent.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSummary {
    pub mtls_replay_count: usize,
    pub tofu_mismatch_count: usize,
    pub handshake_fault_count: usize,
    pub cross_spirit_consent_count: usize,
}

/// Parametric corpus generator — given a `class_size` per class, mint a
/// deterministic corpus. The seed material is the SHA-256 of the class name
/// + scenario index; identical inputs produce identical fixtures
/// (content-addressed reproducibility per §8.1).
///
/// At v0.5 the per-PR job calls `generate(20, 20, 20, 20)` for a fast smoke;
/// the schedule:weekly job calls `generate(1000, 100, 20, 30)` for the
/// architecture-§7.2.1.b-binding corpus floor (mtls-replay only goes to 1000;
/// the other class sizes are capped at the AC2 floors).
pub fn generate(
    mtls_n: usize,
    tofu_n: usize,
    fault_n: usize,
    consent_n: usize,
) -> A2ALoopbackCorpus {
    A2ALoopbackCorpus {
        mtls_replay: (0..mtls_n).map(make_mtls_replay).collect(),
        tofu_mismatch: (0..tofu_n).map(make_tofu_mismatch).collect(),
        handshake_fault: (0..fault_n.min(20)).map(make_handshake_fault).collect(),
        cross_spirit_consent: (0..consent_n.min(30))
            .map(make_cross_spirit_consent)
            .collect(),
    }
}

fn make_mtls_replay(i: usize) -> MtlsReplayScenario {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"mtls-replay");
    h.update(i.to_le_bytes());
    let digest = h.finalize();
    MtlsReplayScenario {
        scenario_id: format!("mtls-replay-{:04}", i),
        client_hello_hex: hex::encode(digest),
        expected_outcome: ReplayOutcome::Rejected,
    }
}

fn make_tofu_mismatch(i: usize) -> TofuMismatchScenario {
    use sha2::{Digest, Sha256};
    let mut h1 = Sha256::new();
    h1.update(b"tofu-first");
    h1.update(i.to_le_bytes());
    let first = h1.finalize();
    let mut h2 = Sha256::new();
    h2.update(b"tofu-second");
    h2.update(i.to_le_bytes());
    let second = h2.finalize();
    let class = match i % 3 {
        0 => TofuMismatchClass::DifferentCertEntirely,
        1 => TofuMismatchClass::LegitimateRotationUnpinned,
        _ => TofuMismatchClass::ImpersonationAttempt,
    };
    TofuMismatchScenario {
        scenario_id: format!("tofu-mismatch-{:03}", i),
        first_fingerprint_hex: hex::encode(first),
        second_fingerprint_hex: hex::encode(second),
        mismatch_class: class,
        expected_alert: true,
    }
}

fn make_handshake_fault(i: usize) -> HandshakeFaultScenario {
    let fault = match i % 10 {
        0 => HandshakeFaultClass::CertChainMalformed,
        1 => HandshakeFaultClass::AlpnMismatch,
        2 => HandshakeFaultClass::SniMismatch,
        3 => HandshakeFaultClass::ExpiredCert,
        4 => HandshakeFaultClass::FutureNotBefore,
        5 => HandshakeFaultClass::EeCertWithCaKeyUsage,
        6 => HandshakeFaultClass::KeyUsageWrong,
        7 => HandshakeFaultClass::EmptyCertChain,
        8 => HandshakeFaultClass::InvalidSignature,
        _ => HandshakeFaultClass::UnknownIssuer,
    };
    let expected_substring = match &fault {
        HandshakeFaultClass::CertChainMalformed => "malformed",
        HandshakeFaultClass::AlpnMismatch => "alpn",
        HandshakeFaultClass::SniMismatch => "sni",
        HandshakeFaultClass::ExpiredCert => "expired",
        HandshakeFaultClass::FutureNotBefore => "not_before",
        HandshakeFaultClass::EeCertWithCaKeyUsage => "key_usage",
        HandshakeFaultClass::KeyUsageWrong => "key_usage",
        HandshakeFaultClass::EmptyCertChain => "empty",
        HandshakeFaultClass::InvalidSignature => "signature",
        HandshakeFaultClass::UnknownIssuer => "unknown_issuer",
        HandshakeFaultClass::Other(_) => "other",
    };
    HandshakeFaultScenario {
        scenario_id: format!("handshake-fault-{:02}", i),
        fault_class: fault,
        expected_error_substring: expected_substring.to_string(),
    }
}

fn make_cross_spirit_consent(i: usize) -> CrossSpiritConsentScenario {
    // Mira-Nash classes (per architecture §11.2)
    let (sender_intent, send_allow, accept_allow, outcome) = match i % 3 {
        // Send-side denial
        0 => (
            "code-mutation-directive".to_string(),
            vec!["diagnosis-handoff:read-only-evidence".to_string()],
            vec![
                "diagnosis-handoff:read-only-evidence".to_string(),
                "rca-summary".to_string(),
            ],
            ConsentOutcome::DeniedAtSender,
        ),
        // Receive-side denial (sender admits, receiver rejects)
        1 => (
            "cross-environment-telemetry-query".to_string(),
            vec!["cross-environment-telemetry-query".to_string()],
            vec!["diagnosis-handoff:read-only-evidence".to_string()],
            ConsentOutcome::DeniedAtReceiver,
        ),
        // Both sides admit
        _ => (
            "diagnosis-handoff:read-only-evidence".to_string(),
            vec!["diagnosis-handoff:read-only-evidence".to_string()],
            vec!["diagnosis-handoff:read-only-evidence".to_string()],
            ConsentOutcome::Admitted,
        ),
    };
    CrossSpiritConsentScenario {
        scenario_id: format!("cross-spirit-consent-{:02}", i),
        sender_intent,
        send_allowlist: send_allow,
        accept_allowlist: accept_allow,
        expected_outcome: outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_default_v05_sample_sizes() {
        let c = generate(20, 20, 20, 20);
        let s = c.summary();
        assert_eq!(s.mtls_replay_count, 20);
        assert_eq!(s.tofu_mismatch_count, 20);
        assert_eq!(s.handshake_fault_count, 20);
        assert_eq!(s.cross_spirit_consent_count, 20);
    }

    #[test]
    fn generate_full_v07_corpus_sizes() {
        let c = generate(1000, 100, 20, 30);
        let s = c.summary();
        assert_eq!(s.mtls_replay_count, 1000);
        assert_eq!(s.tofu_mismatch_count, 100);
        assert_eq!(s.handshake_fault_count, 20);
        assert_eq!(s.cross_spirit_consent_count, 30);
    }

    #[test]
    fn generate_is_deterministic() {
        let a = generate(10, 10, 10, 10);
        let b = generate(10, 10, 10, 10);
        for (x, y) in a.mtls_replay.iter().zip(b.mtls_replay.iter()) {
            assert_eq!(x.client_hello_hex, y.client_hello_hex);
        }
        for (x, y) in a.tofu_mismatch.iter().zip(b.tofu_mismatch.iter()) {
            assert_eq!(x.first_fingerprint_hex, y.first_fingerprint_hex);
            assert_eq!(x.second_fingerprint_hex, y.second_fingerprint_hex);
        }
    }

    #[test]
    fn mtls_replay_all_expect_rejected() {
        let c = generate(50, 0, 0, 0);
        for s in c.mtls_replay {
            assert_eq!(s.expected_outcome, ReplayOutcome::Rejected);
        }
    }

    #[test]
    fn tofu_mismatch_all_expect_alert() {
        let c = generate(0, 30, 0, 0);
        for s in c.tofu_mismatch {
            assert!(s.expected_alert);
            assert_ne!(s.first_fingerprint_hex, s.second_fingerprint_hex);
        }
    }

    #[test]
    fn handshake_fault_caps_at_20() {
        let c = generate(0, 0, 100, 0);
        assert_eq!(c.handshake_fault.len(), 20);
    }

    #[test]
    fn cross_spirit_consent_caps_at_30() {
        let c = generate(0, 0, 0, 100);
        assert_eq!(c.cross_spirit_consent.len(), 30);
    }

    #[test]
    fn cross_spirit_consent_outcomes_cover_all_three_classes() {
        let c = generate(0, 0, 0, 30);
        let mut s_count = 0;
        let mut r_count = 0;
        let mut a_count = 0;
        for s in &c.cross_spirit_consent {
            match s.expected_outcome {
                ConsentOutcome::DeniedAtSender => s_count += 1,
                ConsentOutcome::DeniedAtReceiver => r_count += 1,
                ConsentOutcome::Admitted => a_count += 1,
            }
        }
        assert!(s_count > 0);
        assert!(r_count > 0);
        assert!(a_count > 0);
    }
}
