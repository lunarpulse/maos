#![forbid(unsafe_code)]
//! Story 17-8 — the consent envelope's recall policy (D-17-8-01 A, D-17-8-02),
//! the one expiry predicate (AC3.2) and the granter binding shared by A2A intake
//! and participant recall.

use maos_domain::frame::{ConsentEnvelope, ConsentRecall, FrameAddress};
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::log_recall::LogRecallError;
use maos_spirit_abi::identity::{HostId, SpiritId, SpiritRole};

/// Story 17-8 AC5.1 — captured at `1241ce52` before `recall` existed
/// (`17-8-evidence/t0/golden-envelope-1241ce52.txt`): a `Full` envelope
/// must serialize byte-identically, or every pre-17-8 peer sees a new shape.
fn golden_envelopes() -> [(ConsentEnvelope, &'static str); 2] {
    [
        (
            ConsentEnvelope {
                consent_id: [7; 16],
                granter: FrameAddress {
                    spirit_id: SpiritId::from("peer"),
                    host_id: Some(HostId("host-b".into())),
                    role: None,
                },
                timestamp_ns: 1_700_000_000_000_000_000,
                intent_class: Some(A2AIntent::new("diagnosis-handoff:read-only-evidence")),
                valid_until_ns: Some(u64::MAX),
                recall: ConsentRecall::Full,
            },
            r#"{"consent_id":[7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7],"granter":{"spirit_id":"peer","host_id":"host-b","role":null},"timestamp_ns":1700000000000000000,"intent_class":"diagnosis-handoff:read-only-evidence","valid_until_ns":18446744073709551615}"#,
        ),
        (
            ConsentEnvelope {
                consent_id: [0; 16],
                granter: FrameAddress {
                    spirit_id: SpiritId::from("a"),
                    host_id: None,
                    role: None,
                },
                timestamp_ns: 0,
                intent_class: None,
                valid_until_ns: None,
                recall: ConsentRecall::Full,
            },
            r#"{"consent_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"granter":{"spirit_id":"a","host_id":null,"role":null},"timestamp_ns":0,"intent_class":null,"valid_until_ns":null}"#,
        ),
    ]
}

#[test]
fn full_consent_envelope_json_is_byte_identical_to_pre_17_8_golden() {
    for (envelope, golden) in golden_envelopes() {
        assert_eq!(serde_json::to_string(&envelope).unwrap(), golden);
        assert_eq!(
            serde_json::from_str::<ConsentEnvelope>(golden).unwrap(),
            envelope
        );
    }
}

#[test]
fn consent_recall_policy_decodes_unknown_values_to_withheld() {
    let (envelope, golden) = golden_envelopes()[1].clone();
    let with = |recall: &str| {
        let json = format!("{},\"recall\":{recall}}}", &golden[..golden.len() - 1]);
        serde_json::from_str::<ConsentEnvelope>(&json)
            .unwrap()
            .recall
    };
    assert_eq!(with("\"full\""), ConsentRecall::Full);
    assert_eq!(with("\"header_only\""), ConsentRecall::HeaderOnly);
    assert_eq!(with("\"withheld\""), ConsentRecall::Withheld);
    for unknown in ["\"summary\"", "\"Full\"", "7", "null", "{}", "[\"full\"]"] {
        assert_eq!(with(unknown), ConsentRecall::Withheld, "{unknown}");
    }
    let header_only = ConsentEnvelope {
        recall: ConsentRecall::HeaderOnly,
        ..envelope
    };
    let json = serde_json::to_string(&header_only).unwrap();
    assert!(json.ends_with(",\"recall\":\"header_only\"}"), "{json}");
    assert_eq!(
        serde_json::from_str::<ConsentEnvelope>(&json).unwrap(),
        header_only
    );
}

#[test]
fn consent_expiry_is_strict_and_open_ended_never_expires() {
    let (mut envelope, _) = golden_envelopes()[1].clone();
    assert!(!envelope.expired_at(u64::MAX), "None is open-ended");
    for valid_until in [0, 1_000, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX - 1] {
        envelope.valid_until_ns = Some(valid_until);
        assert!(!envelope.expired_at(valid_until), "equality is valid");
        assert!(envelope.expired_at(valid_until + 1));
        assert!(
            envelope.expired_at(u64::MAX),
            "the fail-closed clock expires it"
        );
        if let Some(before) = valid_until.checked_sub(1) {
            assert!(!envelope.expired_at(before));
        }
    }
    envelope.valid_until_ns = Some(u64::MAX);
    assert!(!envelope.expired_at(u64::MAX));
}

#[test]
fn granter_binding_compares_spirit_and_host_never_role() {
    let (envelope, _) = golden_envelopes()[0].clone();
    let mut from = envelope.granter.clone();
    from.role = Some(SpiritRole::Worker);
    assert!(envelope.granted_by(&from));
    from.host_id = None;
    assert!(!envelope.granted_by(&from));
    from.host_id = envelope.granter.host_id.clone();
    from.spirit_id = SpiritId::from("other");
    assert!(!envelope.granted_by(&from));
}

#[test]
fn consent_withheld_display_names_only_the_frame() {
    let display = LogRecallError::ConsentWithheld {
        frame_id: [0xAB; 16],
    }
    .to_string();
    assert!(display.starts_with("E_CONSENT_WITHHELD — frame [171, 171,"));
    for leak in ["pid", "granter", "policy", "len", "header", "expired"] {
        assert!(!display.contains(leak), "{display} leaks {leak}");
    }
}
