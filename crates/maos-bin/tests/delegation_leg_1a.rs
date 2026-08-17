#![forbid(unsafe_code)]

//! The delegation leg driven end-to-end in-process (story `j1-crosshost-1a`,
//! AC2.1-2.3 / AC3.1-3.4).
//!
//! This exercises the REAL `maos_bin::delegation::DelegationLeg` against a real
//! `Mailbox` + `IacBusAdapter` + `LoopbackA2ARouter` — emit → route → pump → drain
//! — rather than re-implementing the path in the test. A test that rebuilds the
//! wire proves the test's wire works, not the product's.

use std::sync::Arc;

use maos_bin::delegation::{self, DelegationLeg};
use maos_domain::frame::FramePayload;
use maos_domain::invariants::i8::A2AIntent;
use maos_iac::adapter::metrics::IacRtMetrics;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind as TlFrameKind};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::SpiritRole;
use orchestrator::{Orchestrator, DELEGATION_CONSENT_INTENT};

const TEST_RUN_NONCE: u64 = 0xA11CE;

struct Harness {
    iac: Arc<IacBusAdapter>,
    log: Arc<TransparencyLogAdapter>,
    leg: DelegationLeg,
    emitter: Orchestrator,
}

async fn harness() -> Harness {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let leg = DelegationLeg::install(
        Arc::clone(&mailbox),
        &A2AIntent::new(DELEGATION_CONSENT_INTENT),
    )
    .await
    .expect("the delegation leg installs on an Arc'd mailbox");
    Harness {
        iac: Arc::new(IacBusAdapter::new(mailbox, Arc::clone(&log))),
        log,
        leg,
        emitter: Orchestrator::new(delegation::FROM_SPIRIT),
    }
}

fn delegation_frame(
    emitter: &Orchestrator,
    seq: u64,
    run_nonce: u64,
    goal: &str,
) -> maos_domain::frame::IacFrame {
    let intent = A2AIntent::new(DELEGATION_CONSENT_INTENT);
    emitter
        .assign_frame_remote(
            seq,
            run_nonce,
            delegation::RECIPIENT_SPIRIT,
            SpiritRole::Worker,
            emitter.build_task_assign(goal, "exit 0", None),
            maos_domain::invariants::i13::IntentLineage::new(vec![intent.clone()]),
            delegation::TO_HOST,
            delegation::FROM_HOST,
            intent,
        )
        .expect("test lineage is canonical and non-empty")
}

/// AC3.1-3.2 — the routed frame reaches the consumer's OWN handle and the goal is
/// extracted from the delivered `TaskAssign` payload. If the handle were `_`-bound
/// (the `smoke_orchestrator_fanout_6_2` mistake) the frame would be dropped and
/// `delegate` would report that the frame never arrived.
#[tokio::test]
async fn delegation_round_trips_the_goal_through_the_real_wire() {
    let mut h = harness().await;
    // j1-crosshost-2b AC2.1 — `delegate` now returns a typed `DelegationOutcome`
    // because `DelegationLeg::install` chooses its router. This harness installs the
    // LOOPBACK arm, so the frame never leaves the process and the goal is drained
    // locally: `RehearsedLocally` is the correct outcome here, and a
    // `SentCrossHost` would mean this rung-1 control silently started exercising
    // rung 2's path.
    let outcome = h
        .leg
        .delegate(
            &h.iac,
            delegation_frame(&h.emitter, 1, TEST_RUN_NONCE, "build the thing"),
        )
        .await
        .expect("the frame routes, pumps, and is drained by the consumer");
    assert_eq!(
        outcome,
        maos_bin::delegation::DelegationOutcome::RehearsedLocally {
            goal: "build the thing".to_string()
        }
    );
}

/// AC3.4 — the recursion guard, asserted DIRECTLY: a routed frame produces
/// **exactly one** local delivery, and the pump does not re-emit.
///
/// Without the `to[..].host_id` strip the re-delivered frame re-enters Phase 3 of
/// `Mailbox::deliver`, routes again, lands on the intake sink again, and loops
/// forever. Two observations pin it: the consumer's handle yields one frame and
/// then nothing, and the journal holds exactly one `TaskAssign` row for the
/// delegation (a re-emit would either duplicate the row or spin).
#[tokio::test]
async fn routed_frame_produces_exactly_one_local_delivery_and_no_reemit() {
    let mut h = harness().await;
    h.leg
        .delegate(
            &h.iac,
            delegation_frame(&h.emitter, 1, TEST_RUN_NONCE, "once only"),
        )
        .await
        .expect("first delegation succeeds");

    // `delegate` already drained the one delivery; a second drain must find the
    // handle empty. A looping pump would have queued more.
    let second = h
        .leg
        .delegate(
            &h.iac,
            delegation_frame(&h.emitter, 2, TEST_RUN_NONCE, "second goal"),
        )
        .await
        .expect("a second, distinct delegation also delivers exactly once");
    assert_eq!(
        second,
        maos_bin::delegation::DelegationOutcome::RehearsedLocally {
            goal: "second goal".to_string()
        },
        "each delegation must drain ITS OWN frame — a stale queued copy would return the first goal"
    );

    let rows = h
        .log
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::TaskAssign),
            ..Default::default()
        })
        .expect("journal is queryable");
    assert_eq!(
        rows.len(),
        2,
        "exactly one journaled TaskAssign per delegation — the pump's local \
         re-delivery must NOT write a second row for the same frame (I2 was already \
         satisfied at emit), and must not re-emit"
    );
    for row in rows {
        assert_eq!(
            &row.frame_id[8..],
            &TEST_RUN_NONCE.to_le_bytes(),
            "the run nonce must occupy the high half of every delegation frame ID"
        );
    }
}

/// Two processes restart their local sequence at one. The per-run nonce must
/// keep both frame IDs insertable into the same persistent Transparency Log.
#[tokio::test]
async fn identical_sequences_from_distinct_runs_have_distinct_frame_ids() {
    let mut h = harness().await;
    let first_run_nonce = 0x1111;
    let second_run_nonce = 0x2222;

    h.leg
        .delegate(
            &h.iac,
            delegation_frame(&h.emitter, 1, first_run_nonce, "first run"),
        )
        .await
        .expect("first run inserts sequence one");
    h.leg
        .delegate(
            &h.iac,
            delegation_frame(&h.emitter, 1, second_run_nonce, "second run"),
        )
        .await
        .expect("second run may reuse sequence one without a primary-key collision");

    let rows = h
        .log
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::TaskAssign),
            ..Default::default()
        })
        .expect("journal is queryable");
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].frame_id, rows[1].frame_id);
    assert_eq!(&rows[0].frame_id[..8], &1u64.to_le_bytes());
    assert_eq!(&rows[1].frame_id[..8], &1u64.to_le_bytes());
    let mut high_halves = rows
        .iter()
        .map(|row| row.frame_id[8..].to_vec())
        .collect::<Vec<_>>();
    high_halves.sort();
    assert_eq!(
        high_halves,
        vec![
            first_run_nonce.to_le_bytes().to_vec(),
            second_run_nonce.to_le_bytes().to_vec(),
        ]
    );
}

/// AC3.10 — the completion is the EXISTING `TaskComplete` frame, it lands in the
/// journal, and it reaches the Orchestrator's registered handle. A frame emitted
/// into a void would still "journal" but close no loop.
#[tokio::test]
async fn completion_is_a_real_task_complete_frame_the_orchestrator_receives() {
    let mut h = harness().await;
    h.leg
        .delegate(
            &h.iac,
            delegation_frame(&h.emitter, 1, TEST_RUN_NONCE, "work"),
        )
        .await
        .unwrap();

    let drained = h
        .leg
        .journal_completion(&h.iac, 2, TEST_RUN_NONCE)
        .await
        .expect("the completion frame delivers");
    assert_eq!(
        drained, 1,
        "the TaskComplete frame must reach the Orchestrator's handle"
    );

    let rows = h
        .log
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::TaskComplete),
            ..Default::default()
        })
        .expect("journal is queryable");
    assert_eq!(rows.len(), 1, "exactly one journaled completion");
    assert_eq!(
        &rows[0].frame_id[8..],
        &TEST_RUN_NONCE.to_le_bytes(),
        "completion IDs must carry the same run nonce as assignment IDs"
    );
    let payload: FramePayload =
        serde_json::from_slice(&rows[0].payload_redacted).expect("a typed TaskComplete payload");
    match payload {
        FramePayload::TaskComplete(p) => assert_eq!(p.result, "completed"),
        other => panic!("expected TaskComplete, got {other:?}"),
    }
}

/// AC2.1 — set-once, proven through the production installer: the leg cannot be
/// installed twice on the same mailbox, so nothing can swap the cross-host router
/// after boot.
#[tokio::test]
async fn a_second_delegation_leg_cannot_replace_the_installed_router() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let intent = A2AIntent::new(DELEGATION_CONSENT_INTENT);
    let _first = DelegationLeg::install(Arc::clone(&mailbox), &intent)
        .await
        .expect("first install succeeds");
    let err = match DelegationLeg::install(Arc::clone(&mailbox), &intent).await {
        Err(e) => e,
        Ok(_) => panic!("a second install must be refused — the router is set-once"),
    };
    assert!(
        err.contains("already installed") || err.contains("set-once"),
        "the refusal must name the set-once property; got: {err}"
    );
}
