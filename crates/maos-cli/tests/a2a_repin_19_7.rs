#![forbid(unsafe_code)]

//! Story 19-7 (AC3, D-19-7-01/02/03 A) — CONTRACT tests for
//! `maosctl a2a repin list|approve|reject` over the 16-1 door.
//!
//! The wire shapes are duplicated from `maos-control` on purpose
//! (`door_client.rs` module doc: no `maos-control` edge). This file pins the
//! client's half of that duplication from both directions:
//!
//! 1. the WIRE — what `maosctl` puts on the socket: `GET /v1/a2a/repins`, or
//!    `POST /v1/a2a/repins/<peer>` with exactly
//!    `{"approve":bool,"boot_nonce":"<decimal>","actor":"<who>"}`, bearer
//!    from `control.json`; and
//! 2. the MAPPING — every route outcome (200 accepted/rejected, 404
//!    `repin_not_pending`, 404 control-not-installed, 409
//!    `repin_nonce_mismatch|repin_request_expired|repin_refused_by_store`,
//!    401) reaches its D-16-1-P exit with its own remedy line.

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};
use maos_cli::door_client::{RepinDecision, RepinDecisionReply, RepinDecisionRequest};

const PEER: &str = "founder-loop-host";
const ROUTE: &str = "/v1/a2a/repins/founder-loop-host";
const LIST_ROUTE: &str = "/v1/a2a/repins";
/// u64::MAX — above 2^53, so a JSON-number encoding would lose it.
const NONCE: &str = "18446744073709551615";

fn reply_body(decision: RepinDecision, boot_nonce: &str) -> String {
    serde_json::to_string(&RepinDecisionReply {
        peer: PEER.to_owned(),
        boot_nonce: boot_nonce.to_owned(),
        approval_id: "repin-7f3a".to_owned(),
        decision,
    })
    .expect("serialize the fixture reply")
}

fn decide(verb: &str) -> Vec<&str> {
    vec!["a2a", "repin", verb, PEER, "--boot-nonce", NONCE]
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The request half, field for field: `deny_unknown_fields` makes the
/// deserialization itself the exact-key-set assertion.
fn assert_decision_request(
    door: &FixtureDoor,
    expected: &RepinDecisionRequest,
) -> serde_json::Value {
    let requests = door.requests();
    assert_eq!(requests.len(), 1, "exactly one round trip");
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, ROUTE);
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str()),
        "the bearer must be the token from control.json"
    );
    assert_eq!(request.content_type.as_deref(), Some("application/json"));
    let parsed: RepinDecisionRequest = serde_json::from_slice(&request.body)
        .expect("body is exactly the RepinDecisionRequest shape");
    assert_eq!(&parsed, expected);
    request.json_body()
}

#[test]
fn approve_posts_the_decision_with_a_decimal_string_nonce() {
    let body = reply_body(RepinDecision::Accepted, NONCE);
    let door = FixtureDoor::spawn_replying(200, &body);
    let mut args = decide("approve");
    args.extend_from_slice(&["--actor", "lunarpulse"]);
    let out = door.run(&args);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(
        out.stdout,
        format!("{body}\n").into_bytes(),
        "200 prints verbatim"
    );
    let json = assert_decision_request(
        &door,
        &RepinDecisionRequest {
            approve: true,
            boot_nonce: NONCE.to_owned(),
            actor: "lunarpulse".to_owned(),
        },
    );
    assert!(
        json["boot_nonce"].is_string(),
        "boot_nonce travels as a decimal STRING — got {json}"
    );
}

/// Reject sends `approve:false`; with no `--actor` and no `$USER` (the
/// fixture clears the env) the actor is the `operator` fallback.
#[test]
fn reject_posts_approve_false_with_the_default_actor() {
    let door = FixtureDoor::spawn_replying(200, &reply_body(RepinDecision::Rejected, NONCE));
    let out = door.run(&decide("reject"));
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_decision_request(
        &door,
        &RepinDecisionRequest {
            approve: false,
            boot_nonce: NONCE.to_owned(),
            actor: "operator".to_owned(),
        },
    );
    let out = door.run_with(&[("USER", "alice")], &decide("reject"));
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(
        door.requests()[1].json_body()["actor"],
        "alice",
        "$USER is the second default"
    );
}

#[test]
fn list_gets_the_pending_set_and_prints_it_verbatim() {
    let body = r#"[{"peer":"founder-loop-host","observed_boot_nonce":"42"}]"#;
    let door = FixtureDoor::spawn_replying(200, body);
    let out = door.run(&["a2a", "repin", "list"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(out.stdout, format!("{body}\n").into_bytes());
    let requests = door.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, LIST_ROUTE);
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str())
    );
    assert!(requests[0].body.is_empty(), "a list carries no body");
}

struct Outcome {
    label: &'static str,
    status: u16,
    body: &'static str,
    exit: i32,
    /// Substrings the stderr must carry; the last one is the remedy line
    /// that makes the outcome distinct.
    stderr: &'static [&'static str],
}

const OUTCOMES: &[Outcome] = &[
    Outcome {
        label: "404 repin_not_pending",
        status: 404,
        body: r#"{"error":"repin_not_pending","detail":"no pending detection for founder-loop-host"}"#,
        exit: 1,
        stderr: &[
            "repin_not_pending",
            "HTTP 404",
            "no restart detection is pending",
        ],
    },
    Outcome {
        label: "404 control not installed",
        status: 404,
        body: r#"{"error":"not_found"}"#,
        exit: 1,
        stderr: &["not_found", "HTTP 404", "MAOS_ONE_SHOT=cohort-a2a-daemon"],
    },
    Outcome {
        label: "409 repin_nonce_mismatch",
        status: 409,
        body: r#"{"error":"repin_nonce_mismatch","detail":"observed 42"}"#,
        exit: 1,
        stderr: &[
            "repin_nonce_mismatch",
            "HTTP 409",
            "--boot-nonce is not the pending",
        ],
    },
    Outcome {
        label: "409 repin_request_expired",
        status: 409,
        body: r#"{"error":"repin_request_expired","detail":"detected 301 s ago"}"#,
        exit: 1,
        stderr: &[
            "repin_request_expired",
            "HTTP 409",
            "expired before the decision",
        ],
    },
    Outcome {
        label: "409 repin_refused_by_store",
        status: 409,
        body: r#"{"error":"repin_refused_by_store","detail":"fingerprint collision"}"#,
        exit: 1,
        stderr: &[
            "repin_refused_by_store",
            "HTTP 409",
            "pin store refused the re-pin",
        ],
    },
    Outcome {
        label: "401 bad bearer",
        status: 401,
        body: r#"{"error":"unauthorized"}"#,
        exit: 77,
        stderr: &["HTTP 401", "bearer was rejected"],
    },
];

/// Every route outcome maps to its exit, names its code and carries a
/// remedy line no other outcome prints; approve and reject share the table.
#[test]
fn decision_outcomes_map_to_typed_distinct_exits() {
    for verb in ["approve", "reject"] {
        for outcome in OUTCOMES {
            let door = FixtureDoor::spawn_replying(outcome.status, outcome.body);
            let out = door.run(&decide(verb));
            let err = stderr(&out);
            assert_eq!(
                out.status.code(),
                Some(outcome.exit),
                "[{verb} {}]: stderr: {err}",
                outcome.label
            );
            assert!(
                out.stdout.is_empty(),
                "[{verb} {}]: a refusal prints no body",
                outcome.label
            );
            for needle in outcome.stderr {
                assert!(
                    err.contains(needle),
                    "[{verb} {}]: missing {needle:?} in {err}",
                    outcome.label
                );
            }
            let remedy = outcome.stderr[outcome.stderr.len() - 1];
            for other in OUTCOMES.iter().filter(|other| other.label != outcome.label) {
                let other_remedy = other.stderr[other.stderr.len() - 1];
                assert!(
                    !err.contains(other_remedy),
                    "[{verb} {}]: carries {}'s remedy {other_remedy:?} — not distinct ({remedy:?})",
                    outcome.label,
                    other.label
                );
            }
            assert_eq!(door.request_count(), 1, "[{verb} {}]", outcome.label);
        }
    }
}

/// The list route shares the not-installed 404 (never an empty list) and
/// the 401.
#[test]
fn list_outcomes_map_to_typed_exits() {
    for outcome in OUTCOMES
        .iter()
        .filter(|o| matches!(o.label, "404 control not installed" | "401 bad bearer"))
    {
        let door = FixtureDoor::spawn_replying(outcome.status, outcome.body);
        let out = door.run(&["a2a", "repin", "list"]);
        let err = stderr(&out);
        assert_eq!(
            out.status.code(),
            Some(outcome.exit),
            "[{}]: {err}",
            outcome.label
        );
        for needle in outcome.stderr {
            assert!(
                err.contains(needle),
                "[{}]: missing {needle:?} in {err}",
                outcome.label
            );
        }
    }
}

/// Exit 0 is the operator's evidence that the decision landed: a 200 whose
/// body is not the decision that was sent (other decision, other nonce,
/// other peer, not the shape at all) exits 1 and prints nothing.
#[test]
fn a_200_that_is_not_the_sent_decision_is_refused() {
    let wrong_peer = r#"{"peer":"other-host","boot_nonce":"18446744073709551615","approval_id":"a","decision":"accepted"}"#;
    for (verb, body) in [
        ("approve", reply_body(RepinDecision::Rejected, NONCE)),
        ("reject", reply_body(RepinDecision::Accepted, NONCE)),
        ("approve", reply_body(RepinDecision::Accepted, "42")),
        ("approve", wrong_peer.to_owned()),
        ("approve", "{}".to_owned()),
        ("approve", r#"{"peer":"founder-loop-host","boot_nonce":18446744073709551615,"approval_id":"a","decision":"accepted"}"#.to_owned()),
    ] {
        let door = FixtureDoor::spawn_replying(200, &body);
        let out = door.run(&decide(verb));
        let err = stderr(&out);
        assert_eq!(out.status.code(), Some(1), "[{verb} {body}]: {err}");
        assert!(err.contains("repin_reply_unrecognized"), "[{verb} {body}]: {err}");
        assert!(out.stdout.is_empty(), "[{verb} {body}]: nothing printed as success");
    }
}

/// The shared D-16-1-P table (503 family, protocol errors) reaches both
/// route shapes.
#[test]
fn repin_routes_reach_the_d16_1_p_matrix() {
    assert_typed_matrix(&decide("approve"), "POST", ROUTE);
    assert_typed_matrix(&["a2a", "repin", "list"], "GET", LIST_ROUTE);
}

#[test]
fn repin_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&decide("approve"));
    assert_discovery_failures(&["a2a", "repin", "list"]);
}

/// A peer outside the route charset, a non-decimal or out-of-range nonce,
/// and a missing `--boot-nonce` are refused locally (exit 2), never sent.
#[test]
fn malformed_decisions_are_refused_before_the_round_trip() {
    let door = FixtureDoor::spawn();
    for args in [
        vec!["a2a", "repin", "approve", "bad/peer", "--boot-nonce", "42"],
        vec!["a2a", "repin", "approve", PEER, "--boot-nonce", "0x2a"],
        vec!["a2a", "repin", "approve", PEER, "--boot-nonce", "-1"],
        vec![
            "a2a",
            "repin",
            "approve",
            PEER,
            "--boot-nonce",
            "18446744073709551616",
        ],
        vec!["a2a", "repin", "reject", PEER],
    ] {
        let out = door.run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
    }
    assert_eq!(
        door.request_count(),
        0,
        "a locally refused decision never reaches the door"
    );
}

/// The duplicated shapes, pinned as literal JSON — the bytes the
/// `maos-control` twin must accept and emit.
#[test]
fn wire_shapes_are_pinned_as_literal_json() {
    let request: RepinDecisionRequest =
        serde_json::from_str(r#"{"approve":true,"boot_nonce":"42","actor":"op"}"#)
            .expect("request shape");
    assert_eq!(
        request,
        RepinDecisionRequest {
            approve: true,
            boot_nonce: "42".to_owned(),
            actor: "op".to_owned()
        }
    );
    for refused in [
        r#"{"approve":true,"boot_nonce":42,"actor":"op"}"#,
        r#"{"approve":true,"boot_nonce":"42","actor":"op","fingerprint":"ab"}"#,
        r#"{"approve":true,"boot_nonce":"42"}"#,
    ] {
        assert!(
            serde_json::from_str::<RepinDecisionRequest>(refused).is_err(),
            "{refused}"
        );
    }
    for (literal, decision) in [
        (
            r#"{"peer":"p","boot_nonce":"42","approval_id":"a","decision":"accepted"}"#,
            RepinDecision::Accepted,
        ),
        (
            r#"{"peer":"p","boot_nonce":"42","approval_id":"a","decision":"rejected"}"#,
            RepinDecision::Rejected,
        ),
    ] {
        let reply: RepinDecisionReply = serde_json::from_str(literal).expect("reply shape");
        assert_eq!(reply.decision, decision);
        assert_eq!(
            serde_json::to_string(&reply).expect("re-serialize"),
            literal,
            "field order and spelling"
        );
    }
    assert!(serde_json::from_str::<RepinDecisionReply>(
        r#"{"peer":"p","boot_nonce":"42","approval_id":"a","decision":"Accepted"}"#
    )
    .is_err());
}
