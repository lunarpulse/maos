//! Story 13.6b — the crossing refusal contract on the wire.
//!
//! D-15 measured that `TransportCause` has **zero** occurrences in either A2A
//! crate: before this story no cross-team refusal reason crossed a socket at all,
//! so an emitter could not tell a consent denial from a stale lease from an
//! unreachable state from a flaky connection. These legs hold the new codes to
//! that contract, and hold AC3's envelope/payload weld to its own code so an
//! impersonation refusal can never be read as a roster or grant problem.
//!
//! Each is registered `Blocking` on `check-multi-tenant-loom` with one `#[test]`
//! per `--exact` leg.

use std::sync::Arc;

use maos_a2a_core::router::A2ARouterCore;
use maos_a2a_core::{
    A2AError, A2AJsonRpcResponse, CrossingRefusal, InMemoryTofuPinStore, TofuPinStore,
    CODE_CROSSING_SOURCE_TEAM_UNBOUND, CODE_CROSS_TEAM_CROSSING_REFUSED,
    CODE_TEAM_IDENTITY_MISMATCH, COHORT_INTENT_COLLECTIVE_SHARE,
};
use maos_spirit_abi::identity::HostId;

/// `interpret_response` performs no peer lookup, so an empty peer table is
/// sufficient — and keeps these legs free of TLS/config scaffolding that would
/// obscure what is actually under test.
fn core() -> A2ARouterCore {
    A2ARouterCore::new(
        vec![],
        Arc::new(InMemoryTofuPinStore::new()) as Arc<dyn TofuPinStore>,
    )
}

fn peer() -> HostId {
    HostId("host-b".to_string())
}

fn code_of(response: &A2AJsonRpcResponse) -> i32 {
    match response {
        A2AJsonRpcResponse::Nack(n) => n.error.code,
        _ => panic!("a refusal must be a NACK"),
    }
}

/// Story 13.6b / AC3 — the envelope/payload weld refusal survives the wire under
/// its OWN code and re-materialises as its OWN typed error.
///
/// This leg exists because D-13's attack is invisible to every shipped check: the
/// payload signature verifies (a seed holder can sign under any team), the
/// envelope is honest, and the cohort gate admits. If the refusal collapsed into
/// `-32010` (a LYING envelope) or into the `TransportFailed` catch-all, an
/// impersonation attempt would read as a misconfigured roster or a flaky socket.
///
/// ⚠ 13.6a's impersonation leg may NOT be cited as evidence for AC3: it proves
/// the envelope binding against synthetic frames at a different site.
#[test]
fn crossing_source_team_unbound_survives_the_wire_under_its_own_code() {
    let refusal = CrossingRefusal::SourceTeamUnbound {
        envelope_team: "team-a".to_string(),
        payload_team: "team-b".to_string(),
    };
    let nack = A2ARouterCore::crossing_refusal_nack(7, &refusal);
    let code = code_of(&nack);
    assert_eq!(code, CODE_CROSSING_SOURCE_TEAM_UNBOUND);
    assert_ne!(
        code, CODE_TEAM_IDENTITY_MISMATCH,
        "AC3's weld must never collapse into 13.6a's envelope binding"
    );
    assert_ne!(
        code, CODE_CROSS_TEAM_CROSSING_REFUSED,
        "AC3's weld must never collapse into an honest-but-ungranted refusal"
    );

    match core()
        .interpret_response(&peer(), nack)
        .expect_err("the emitter must see a typed refusal")
    {
        A2AError::CrossingSourceTeamUnbound {
            envelope_team,
            payload_team,
        } => {
            assert_eq!(envelope_team, "team-a");
            assert_eq!(payload_team, "team-b");
        }
        other => panic!("expected CrossingSourceTeamUnbound, got {other:?}"),
    }
}

/// Story 13.6b / AC2 — a consent denial reaches the EMITTER carrying the ordered
/// pair and the intent, not a generic transport failure.
///
/// `TransportCause::ConsentDenied` stays applier-local — the five-cause matrix
/// still holds inside the applier process, which is the only boundary it has ever
/// crossed. This is the wire half that boundary never had.
#[test]
fn crossing_consent_denial_reaches_the_emitter_with_the_ordered_pair() {
    let refusal = CrossingRefusal::ConsentDenied {
        from_team: "team-b".to_string(),
        to_team: "team-a".to_string(),
        intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
    };
    let nack = A2ARouterCore::crossing_refusal_nack(9, &refusal);
    assert_eq!(code_of(&nack), CODE_CROSS_TEAM_CROSSING_REFUSED);
    match core()
        .interpret_response(&peer(), nack)
        .expect_err("a denied crossing must be a typed error at the emitter")
    {
        A2AError::CrossTeamCrossingRefused {
            reason,
            detail,
            from_team,
            to_team,
            intent,
        } => {
            assert_eq!(reason, "crossing_consent_denied");
            assert_eq!(from_team, "team-b");
            assert_eq!(to_team, "team-a");
            assert_eq!(intent, COHORT_INTENT_COLLECTIVE_SHARE);
            assert!(detail.is_empty(), "consent denial has no extra detail");
        }
        other => panic!("expected CrossTeamCrossingRefused, got {other:?}"),
    }
}

/// Story 13.6b / AC2 — denial, staleness, and unavailability stay THREE distinct
/// observable outcomes on the emitter's side of the socket.
///
/// The kernel collapses all of them to the single word `Transport`
/// (`kernel-core/src/memory/mod.rs`, Residual 6 — owned by 13.6, deliberately not
/// fixed here). That erasure must not be reproduced on the wire, so this leg goes
/// red the moment any two reasons are folded together.
#[test]
fn crossing_denial_staleness_and_unavailability_stay_distinguishable() {
    let core = core();
    let peer = peer();
    let refusals = [
        CrossingRefusal::ConsentDenied {
            from_team: "team-b".to_string(),
            to_team: "team-a".to_string(),
            intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
        },
        CrossingRefusal::ConsentStale {
            reason: "lease expired".to_string(),
            from_team: "team-b".to_string(),
            to_team: "team-a".to_string(),
            intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
        },
        CrossingRefusal::StateUnavailable {
            reason: "no consent port".to_string(),
            from_team: "team-b".to_string(),
            to_team: "team-a".to_string(),
            intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
        },
    ];
    let observed: Vec<String> = refusals
        .iter()
        .map(|refusal| {
            let nack = A2ARouterCore::crossing_refusal_nack(1, refusal);
            match core.interpret_response(&peer, nack) {
                Err(A2AError::CrossTeamCrossingRefused {
                    reason,
                    detail,
                    from_team,
                    to_team,
                    intent,
                }) => {
                    assert_eq!(from_team, "team-b");
                    assert_eq!(to_team, "team-a");
                    assert_eq!(intent, COHORT_INTENT_COLLECTIVE_SHARE);
                    if reason != "crossing_consent_denied" {
                        assert!(
                            !detail.is_empty(),
                            "non-denial detail must survive the wire"
                        );
                    }
                    reason
                }
                other => panic!("expected a typed crossing refusal, got {other:?}"),
            }
        })
        .collect();
    assert_eq!(
        observed,
        vec![
            "crossing_consent_denied",
            "crossing_consent_stale",
            "crossing_state_unavailable"
        ],
        "AC2: the emitter must tell denial from staleness from unavailability"
    );
    let unique: std::collections::HashSet<&String> = observed.iter().collect();
    assert_eq!(unique.len(), 3, "three causes must not collapse into fewer");
}
