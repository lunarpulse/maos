#![forbid(unsafe_code)]

//! Story `j1-crosshost-2c` AC2 — the host discriminator and two-TL reconciliation.
//!
//! `2b` proves the crossing by writing **the same `frame_id`** into both hosts'
//! Transparency Logs. That is what makes reconciliation free — and it is also
//! what makes the two halves otherwise **indistinguishable**: `region` cannot
//! separate two hosts in one jurisdiction (same derived key), `boot_nonce` is
//! per-boot and one `--range 1d` export swept eight, and `attester_pubkey` is
//! bundle-supplied so R-RG1 forbids trusting it.
//!
//! Without the `host` field one host can produce **both halves** of a "two-host"
//! bundle. With it, and with independent per-host roots, two signatures are
//! evidence of two key holders.
//!
//! **Bounded honestly:** the field proves *two keyed identities signed*. It does
//! not prove two machines, two processes, or two operators.

use maos_audit::sealed_export::{
    self, AuditBundle, CoveredWindow, FreshnessMetadata, I11Content, SealedExportError,
};
use maos_audit::AuditEntry;

/// Host A and host B hold **independent** base seeds. Not two welds off one
/// root — the region→team template exists to make keys derivable from one seed,
/// which is the exact property AC2.4 must disprove.
const SEED_A: [u8; 32] = [0xA1; 32];
const SEED_B: [u8; 32] = [0xB2; 32];

fn entry(frame_id_hex: &str, ts: u64) -> AuditEntry {
    AuditEntry {
        frame_id_hex: frame_id_hex.to_string(),
        timestamp_ns: ts,
        spirit_pid: 1,
        boot_nonce: 42,
        capability_token_hex: None,
        kind: "task.assign".to_string(),
        intent: "dev.delegate".to_string(),
        payload: String::new(),
        redaction: None,
    }
}

fn freshness() -> FreshnessMetadata {
    FreshnessMetadata {
        export_timestamp_ns: 2000,
        covered_window: CoveredWindow {
            since_ns: 0,
            until_ns: 2000,
        },
        export_seq: 1,
    }
}

/// A signed half of a two-host run: entries keyed by the *shared* deterministic
/// `frame_id`s, stamped with this host's id, signed with this host's own root.
fn signed_half(host: &str, seed: &[u8; 32], frame_ids: &[&str]) -> AuditBundle {
    let entries = frame_ids
        .iter()
        .enumerate()
        .map(|(i, id)| entry(id, 1_000 + i as u64))
        .collect();
    let unsigned =
        sealed_export::build_bundle(entries, vec![], vec![], freshness()).with_host(host);
    sealed_export::sign_bundle(unsigned, seed).expect("sign half")
}

// ── AC2.1 — the field is additive, signed, and byte-identity-preserving ────

/// The host field must be **bound by the signature**: altering it after signing
/// must fail verification. A label a forger can rewrite is not a control.
#[test]
fn a_post_signing_host_alteration_fails_verification() {
    let mut half = signed_half("host-a", &SEED_A, &["aa11", "bb22"]);
    let key_a = sealed_export::derive_pubkey(&SEED_A);
    sealed_export::verify_bundle(&half, &key_a).expect("the untampered half must verify");

    half.host = Some("host-b".to_string());
    assert!(
        matches!(
            sealed_export::verify_bundle(&half, &key_a),
            Err(SealedExportError::VerificationFailed)
        ),
        "a host field rewritten post-signing must fail verification"
    );

    // And it does not verify under the other host's key either — the forger
    // cannot repair the claim by presenting host B's identity.
    let key_b = sealed_export::derive_pubkey(&SEED_B);
    assert!(sealed_export::verify_bundle(&half, &key_b).is_err());
}

/// **9.2b HARD byte-identity — asserted, not assumed.** A bundle that omits the
/// field must canonicalize to exactly the bytes it did before `host` existed.
/// The golden hash is the one already committed for the region/redaction-absent
/// surface in `trajectory_redaction_test.rs`, recomputed on the same inputs.
#[test]
fn a_host_less_bundle_is_byte_identical_to_the_pre_2c_surface() {
    let e = AuditEntry {
        frame_id_hex: "deadbeef".to_string(),
        timestamp_ns: 1000,
        spirit_pid: 1,
        boot_nonce: 42,
        capability_token_hex: None,
        kind: "test.kind".to_string(),
        intent: "test.intent".to_string(),
        payload: String::new(),
        redaction: None,
    };
    let unsigned = sealed_export::build_bundle(
        vec![e],
        vec!["i12ref".to_string()],
        vec![I11Content {
            source_log_ref: vec!["ref1".to_string()],
            distillation_depth: 0,
        }],
        FreshnessMetadata {
            export_timestamp_ns: 2000,
            covered_window: CoveredWindow {
                since_ns: 0,
                until_ns: 2000,
            },
            export_seq: 1,
        },
    );
    assert!(
        unsigned.host.is_none(),
        "build_bundle must not stamp a host"
    );

    let canonical = sealed_export::canonicalize(&unsigned).expect("canonicalize");
    let canonical_str = String::from_utf8(canonical.clone()).expect("utf8");
    assert!(
        !canonical_str.contains("\"host\""),
        "the canonical bytes must NOT carry a host key when the field is None: {canonical_str}"
    );

    use sha2::{Digest, Sha256};
    // The committed 9.1/9.2b golden for these exact inputs. If `host` were
    // serialized when absent, this changes and every pre-2c bundle stops
    // replaying byte-identically.
    const PRE_2C_GOLDEN_SHA256: &str =
        "6b66873150f63be78f4ccf06bdb1586647990567fc7801d3030b0ac2b00337ba";
    assert_eq!(
        hex::encode(Sha256::digest(&canonical)),
        PRE_2C_GOLDEN_SHA256,
        "the additive host field changed the pre-2c canonical surface: {canonical_str}"
    );

    // Round-trips through JSON without the key, and still verifies.
    let signed = sealed_export::sign_bundle(unsigned, &SEED_A).expect("sign");
    let json = serde_json::to_string(&signed).expect("serialize");
    assert!(!json.contains("\"host\""), "host must be omitted: {json}");
    let reparsed: AuditBundle = serde_json::from_str(&json).expect("a host-less bundle must parse");
    assert!(reparsed.host.is_none());
    sealed_export::verify_bundle(&reparsed, &sealed_export::derive_pubkey(&SEED_A))
        .expect("host-less bundle must still verify");
}

/// AC2.5 — say what the bundle can and cannot discriminate. `region` cannot
/// separate two hosts in one jurisdiction: same region ⇒ **same derived key**.
/// This is the measurement behind the artifact's honesty clause.
#[test]
fn region_cannot_discriminate_two_hosts_but_host_can() {
    let region = maos_domain::region::Region::canonicalize("eu-west-1").expect("region");
    // One shared root, one region: two "different hosts" derive the SAME key.
    // §A6 review 2026-08-18 (P9): this used to compare the identical expression
    // to itself — a vacuous green. The honest assertion is the WELD IDENTITY:
    // the region tag reaches the key material only through
    // `derive_region_signing_seed`, and nothing host-shaped enters it at all.
    let shared = [0x5b; 32];
    assert_eq!(
        sealed_export::derive_region_pubkey(&shared, &region),
        sealed_export::derive_pubkey(&sealed_export::derive_region_signing_seed(&shared, &region)),
        "region is a jurisdiction tag welded into the key material — it has no \
         host input, so it cannot discriminate two hosts under one root"
    );

    // Independent roots produce different keys — that, plus the host field, is
    // what makes "two" more than our word for it.
    assert_ne!(
        sealed_export::derive_pubkey(&SEED_A),
        sealed_export::derive_pubkey(&SEED_B)
    );

    let a = signed_half("host-a", &SEED_A, &["aa11"]);
    let b = signed_half("host-b", &SEED_B, &["aa11"]);
    assert_ne!(
        a.host, b.host,
        "the host field is the only in-bundle discriminator"
    );
    // boot_nonce cannot do it either: both halves carry the same one here, and
    // a real `--range 1d` export sweeps many.
    assert_eq!(a.entries[0].boot_nonce, b.entries[0].boot_nonce);
}

// ── §A6 review 2026-08-18 — the receipt's own bounds ──────────────────────

/// A receipt re-signed with a WIDENED claim scope must be refused: the
/// signature proves authorship of the words, not bounds on them, so the
/// verifier pins the ratified scope instead of trusting the receipt's copy.
#[test]
fn a_receipt_with_a_widened_claim_scope_is_refused() {
    let a = signed_half("host-a", &SEED_A, &["aa11"]);
    let b = signed_half("host-b", &SEED_B, &["aa11"]);
    let join = sealed_export::reconcile_two_host_bundles(
        &a,
        &sealed_export::derive_pubkey(&SEED_A),
        &b,
        &sealed_export::derive_pubkey(&SEED_B),
    )
    .expect("independent halves reconcile");

    let operator_seed = [0x0c; 32];
    let mut receipt = sealed_export::build_two_host_receipt(&operator_seed, &join, 3_000);
    assert!(
        sealed_export::verify_two_host_receipt(
            &receipt,
            &sealed_export::derive_pubkey(&operator_seed)
        )
        .is_ok(),
        "the untampered receipt must verify"
    );

    receipt.claim_scope = "two machines, two operators, fully automated pairing".to_string();
    assert!(
        matches!(
            sealed_export::verify_two_host_receipt(
                &receipt,
                &sealed_export::derive_pubkey(&operator_seed)
            ),
            Err(SealedExportError::UnratifiedClaimScope(_))
        ),
        "a receipt carrying a wider scope than its controls must be refused"
    );

    receipt.claim_scope = sealed_export::TWO_HOST_CLAIM_SCOPE.to_string();
    receipt.schema_version = "maos.two-host-receipt.v999".to_string();
    assert!(
        matches!(
            sealed_export::verify_two_host_receipt(
                &receipt,
                &sealed_export::derive_pubkey(&operator_seed)
            ),
            Err(SealedExportError::UnsupportedReceiptSchema(_))
        ),
        "an unratified schema version must be refused before any signature is parsed"
    );
}

/// A hand-forged blank host claim is no claim: the producer trims and refuses
/// whitespace, but reconciliation reads the ARTIFACT.
#[test]
fn a_blank_host_claim_is_refused_as_no_claim() {
    let a = signed_half("host-a", &SEED_A, &["aa11"]);
    let blank = sealed_export::build_bundle(vec![], vec![], vec![], freshness());
    let blank = sealed_export::sign_bundle(blank.with_host("   "), &SEED_B).expect("sign");
    assert!(
        matches!(
            sealed_export::reconcile_two_host_bundles(
                &a,
                &sealed_export::derive_pubkey(&SEED_A),
                &blank,
                &sealed_export::derive_pubkey(&SEED_B),
            ),
            Err(SealedExportError::MissingHostClaim)
        ),
        "Some(\"\") must not count as a host discriminator"
    );
}

// ── AC2.2 / AC2.3 — the two-bundle verb, reconciled on frame_id ────────────

/// The join key is `frame_id_hex` — `2b` proves both hosts write the same
/// sixteen bytes (`two_host_delegation_2b.rs:533-535`), so the join costs
/// nothing. `correlation_id` is NOT the join key and is not projected.
#[test]
fn reconciliation_joins_on_frame_id_under_two_independent_keys() {
    let a = signed_half("host-a", &SEED_A, &["aa11", "bb22", "cc33"]);
    let b = signed_half("host-b", &SEED_B, &["bb22", "cc33", "dd44"]);

    let join = sealed_export::reconcile_two_host_bundles(
        &a,
        &sealed_export::derive_pubkey(&SEED_A),
        &b,
        &sealed_export::derive_pubkey(&SEED_B),
    )
    .expect("two independently-signed halves must reconcile");

    assert_eq!(join.host_a, "host-a");
    assert_eq!(join.host_b, "host-b");
    assert_eq!(join.shared_frame_ids, vec!["bb22", "cc33"]);
    assert_eq!(join.host_a_only, vec!["aa11"]);
    assert_eq!(join.host_b_only, vec!["dd44"]);
}

/// R-RG1 — reconciliation must never trust the key the artifact carries. A half
/// signed by an unrelated seed still advertises its own `attester_pubkey`, and
/// that must not let it through.
#[test]
fn reconciliation_refuses_a_half_that_nominates_its_own_key() {
    let forger = [0xFF; 32];
    let a = signed_half("host-a", &SEED_A, &["aa11"]);
    let b = signed_half("host-b", &forger, &["aa11"]);

    // The forged half is internally consistent — it verifies under the key it
    // advertises. That is exactly why the verifier must not read it.
    let advertised: [u8; 32] = hex::decode(&b.signature_block.attester_pubkey)
        .expect("hex")
        .try_into()
        .expect("32 bytes");
    sealed_export::verify_bundle(&b, &advertised).expect("self-consistent forgery");

    let err = sealed_export::reconcile_two_host_bundles(
        &a,
        &sealed_export::derive_pubkey(&SEED_A),
        &b,
        &sealed_export::derive_pubkey(&SEED_B),
    )
    .expect_err("a half not signed by host B's published key must be refused");
    assert!(matches!(err, SealedExportError::VerificationFailed));
}

/// AC2.4 — the control the host field exists for: **one seed holder must not be
/// able to produce both halves.** Under one shared root the reconciliation must
/// refuse, because "two identities" collapses to one.
#[test]
fn reconciliation_refuses_two_halves_signed_by_one_root() {
    let one_root = [0x5b; 32];
    let a = signed_half("host-a", &one_root, &["aa11"]);
    let b = signed_half("host-b", &one_root, &["aa11"]);
    let key = sealed_export::derive_pubkey(&one_root);

    // Both halves verify individually and carry distinct host labels — the
    // perfect "two-host" bundle produced by one machine.
    sealed_export::verify_bundle(&a, &key).expect("half a verifies");
    sealed_export::verify_bundle(&b, &key).expect("half b verifies");

    let err = sealed_export::reconcile_two_host_bundles(&a, &key, &b, &key)
        .expect_err("one root cannot attest two identities");
    assert!(matches!(err, SealedExportError::SharedAttesterRoot));
}

/// A host field is REQUIRED on both halves: reconciling two indistinguishable
/// bundles must refuse rather than silently claim two hosts.
#[test]
fn reconciliation_refuses_a_half_with_no_host_claim() {
    let a = signed_half("host-a", &SEED_A, &["aa11"]);
    let b_unsigned =
        sealed_export::build_bundle(vec![entry("aa11", 1000)], vec![], vec![], freshness());
    let b = sealed_export::sign_bundle(b_unsigned, &SEED_B).expect("sign");

    let err = sealed_export::reconcile_two_host_bundles(
        &a,
        &sealed_export::derive_pubkey(&SEED_A),
        &b,
        &sealed_export::derive_pubkey(&SEED_B),
    )
    .expect_err("a bundle with no host claim cannot be half of a two-host run");
    assert!(matches!(err, SealedExportError::MissingHostClaim));
}

/// Two halves claiming the SAME host id are one host, whatever the keys say.
#[test]
fn reconciliation_refuses_two_halves_claiming_one_host() {
    let a = signed_half("host-a", &SEED_A, &["aa11"]);
    let b = signed_half("host-a", &SEED_B, &["aa11"]);
    let err = sealed_export::reconcile_two_host_bundles(
        &a,
        &sealed_export::derive_pubkey(&SEED_A),
        &b,
        &sealed_export::derive_pubkey(&SEED_B),
    )
    .expect_err("identical host claims are not two hosts");
    assert!(matches!(err, SealedExportError::DuplicateHostClaim(_)));
}

/// Halves whose logs share nothing did not witness one run.
#[test]
fn reconciliation_refuses_disjoint_logs() {
    let a = signed_half("host-a", &SEED_A, &["aa11"]);
    let b = signed_half("host-b", &SEED_B, &["dd44"]);
    let err = sealed_export::reconcile_two_host_bundles(
        &a,
        &sealed_export::derive_pubkey(&SEED_A),
        &b,
        &sealed_export::derive_pubkey(&SEED_B),
    )
    .expect_err("no shared frame_id means no shared run");
    assert!(matches!(err, SealedExportError::NoSharedFrames));
}

// ── AC2.2 — the receipt, ported from `build_reattestation_receipt` ─────────

/// The receipt shape *"source X's bundle landed at dest Y"* is exactly the
/// two-host claim. Port it: sign the join, verify it, and refuse a tamper.
#[test]
fn the_two_host_receipt_signs_and_verifies_the_join() {
    let a = signed_half("host-a", &SEED_A, &["aa11", "bb22"]);
    let b = signed_half("host-b", &SEED_B, &["bb22"]);
    let join = sealed_export::reconcile_two_host_bundles(
        &a,
        &sealed_export::derive_pubkey(&SEED_A),
        &b,
        &sealed_export::derive_pubkey(&SEED_B),
    )
    .expect("reconcile");

    let operator_seed = [0x0c; 32];
    let receipt = sealed_export::build_two_host_receipt(&operator_seed, &join, 1_700_000_000);
    let operator_pub = sealed_export::derive_pubkey(&operator_seed);
    sealed_export::verify_two_host_receipt(&receipt, &operator_pub).expect("receipt must verify");

    // The receipt is only as good as what it binds: rewriting either host id,
    // the join, or the attester keys must break it.
    for mutate in [
        (|r: &mut sealed_export::TwoHostRunReceipt| r.host_a = "host-x".into())
            as fn(&mut sealed_export::TwoHostRunReceipt),
        |r: &mut sealed_export::TwoHostRunReceipt| r.host_b = "host-x".into(),
        |r: &mut sealed_export::TwoHostRunReceipt| r.shared_frame_ids.push("ff99".into()),
        |r: &mut sealed_export::TwoHostRunReceipt| r.attester_a = "00".repeat(32),
        |r: &mut sealed_export::TwoHostRunReceipt| r.timestamp_ns += 1,
    ] {
        let mut tampered = receipt.clone();
        mutate(&mut tampered);
        assert!(
            sealed_export::verify_two_host_receipt(&tampered, &operator_pub).is_err(),
            "every field the receipt claims must be under its signature"
        );
    }

    // And the claim it records is the bounded one, in the ratified words.
    assert_eq!(
        sealed_export::TWO_HOST_CLAIM_SCOPE,
        "two keyed identities signed; not two machines, two processes, or two operators"
    );
}
