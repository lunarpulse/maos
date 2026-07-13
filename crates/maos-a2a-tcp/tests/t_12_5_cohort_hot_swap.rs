//! Story 12.5 — real N=8 cert-rotation re-pin and manifest-coherence proof.
//!
//! An in-place Spirit hot-swap does not rotate `boot_nonce` or a host
//! certificate. This test exercises the separate cert-rotation mechanism: a
//! transport rebuild receives new peer pins, and a signed cohort-manifest
//! reissue records the same new fingerprints for the roster gate.

mod support;

use std::sync::Arc;

use ed25519_dalek::SigningKey;
use maos_a2a_core::cohort::CohortManifestGate;
use maos_cohort::{
    CohortAuthority, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    ConsentTuple, InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE, SCHEMA_VERSION,
};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use maos_a2a_core::identity::PeerId;
use support::*;

const HOST_COUNT: usize = 8;

fn names() -> Vec<String> {
    (0..HOST_COUNT).map(host_name).collect()
}

fn all_directed_pairs(n: usize) -> Vec<(usize, usize)> {
    (0..n)
        .flat_map(|from| {
            (0..n)
                .filter(move |to| *to != from)
                .map(move |to| (from, to))
        })
        .collect()
}

fn reissued_manifest(names: &[String], leaves: &[Leaf], authority: &SigningKey) -> CohortManifest {
    let members = names
        .iter()
        .zip(leaves)
        .map(|(host_id, leaf)| CohortMember {
            host_id: host_id.clone(),
            fingerprint: leaf.fingerprint.wire(),
            roles: vec!["worker".into()],
        })
        .collect();
    let tuples: Vec<ConsentTuple> = names
        .iter()
        .map(|peer| ConsentTuple {
            peer: peer.clone(),
            role: "worker".into(),
            intent: "readonly".into(),
        })
        .collect();
    CohortManifest {
        schema_version: SCHEMA_VERSION,
        cohort_id: "story-12-5-n8".into(),
        version: 2,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members,
        consent: ConsentMatrix {
            send: tuples.clone(),
            accept: tuples,
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 30,
        signature: ManifestSignature { sig: String::new() },
    }
    .signed_with(authority)
}

/// §A7 cert-rotation re-pin + coherence reflex: changing one member's served
/// certificate while leaving the old client credential causes a real mTLS
/// refusal; rebuilding with new pins admits all N=8 peers, and the signed
/// reissue records exactly the fingerprints trusted by the rebuilt transports.
#[tokio::test]
#[ignore = "Story 12.5 — check-cohort-mesh owns N=8 re-pin plus reissue coherence"]
async fn t_12_5_n8_rotation_refuses_old_cert_and_admits_reissued_new_cert() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-5-rotation");
    let names = names();
    let old: Vec<Leaf> = (0..HOST_COUNT).map(|_| valid_leaf(&ca, &clock)).collect();
    let new: Vec<Leaf> = (0..HOST_COUNT).map(|_| valid_leaf(&ca, &clock)).collect();
    let authority = SigningKey::from_bytes(&[0x52; 32]);

    // The signed reissue is the roster-coherence half. It is not the mTLS pin
    // enforcement layer, so its values are compared directly to rebuilt pins.
    let reissue = reissued_manifest(&names, &new, &authority);
    let reissue_toml = toml::to_string(&reissue).expect("signed reissue serializes");
    let pinned =
        PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).expect("authority pin");
    let gates: Vec<Option<Arc<dyn CohortManifestGate>>> = names
        .iter()
        .map(|host| {
            let state = CohortManifestState::load(
                HostId(host.clone()),
                &reissue_toml,
                pinned.clone(),
                Arc::new(InMemoryCohortAuditSink::default()),
            )
            .expect("new signed reissue loads");
            Some(Arc::new(state) as Arc<dyn CohortManifestGate>)
        })
        .collect();

    let new_refs: Vec<&Leaf> = new.iter().collect();
    let rebuilt = build_mesh_n_with_gates(
        &clock,
        &ca,
        &names,
        &new_refs,
        &new_refs,
        no_retry(),
        &gates,
    )
    .await;
    let new_pairs = all_directed_pairs(HOST_COUNT);
    let admitted = concurrent_dial_pairs(&rebuilt, &new_pairs, 12_500, IntentClass::Readonly).await;
    assert!(
        admitted.iter().all(|(_, _, result)| result.is_ok()),
        "new certificate must pass both rebuilt TOFU pins and the cohort gate: {admitted:?}"
    );

    for (index, node) in rebuilt.iter().enumerate() {
        // Independent source (NOT the test's own input leaf): read what a PEER's
        // rebuilt TOFU store actually pinned for this node, then reconcile the
        // signed reissue against it. Comparing the reissue field to the same
        // `new[index]` leaf it was constructed from is a tautology (X == X); the
        // pin store is reached through the transport-seeding path instead.
        let observer = &rebuilt[(index + 1) % HOST_COUNT];
        let pinned = observer
            .transport
            .pins()
            .get_pin_sync(&PeerId::new(&node.name))
            .expect("a peer must have pinned this node after the rebuild");
        assert_eq!(
            reissue.members[index].fingerprint,
            pinned.fingerprint.wire(),
            "reissued manifest fingerprint must equal the peer's actually-pinned fp"
        );
        assert_eq!(
            pinned.fingerprint.wire(),
            node.fingerprint.wire(),
            "the peer's pin must match this node's served leaf"
        );
    }

    // A client still presenting the old leaf but expecting the rebuilt peer's
    // new leaf reaches the real TLS verifier and is rejected by the receiver's
    // newly seeded TOFU pins.
    let old_refs: Vec<&Leaf> = old.iter().collect();
    let stale_clients = build_mesh_n(&clock, &ca, &names, &old_refs, &new_refs, no_retry()).await;
    stale_clients[0].transport.set_peer_endpoint(
        &HostId(names[1].clone()),
        format!("tls://{}", rebuilt[1].addr),
    );
    let old_attempt =
        concurrent_dial_pairs(&stale_clients, &[(0, 1)], 99_999, IntentClass::Readonly).await;
    assert!(
        old_attempt[0].2.is_err(),
        "old certificate must be refused after the rebuilt peer pins the new leaf"
    );
}
