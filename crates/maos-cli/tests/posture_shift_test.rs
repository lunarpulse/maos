#![forbid(unsafe_code)]

//! Story 16-1 / T9 (AC7) — CONTRACT tests for `maosctl posture --shift`.
//!
//! At HEAD the verb spawned `MAOS_ONE_SHOT=posture-shift`, which re-admitted
//! pid 0 into a FRESH PolicyTable read from a CWD-relative manifest and
//! shifted pid 0 while the daemon's butler ran at pid 1 (§4, measured).
//! Over the door the contract is:
//!
//! 1. the WIRE: `POST /v1/spirits/{id}/posture` with `{"posture": ...}` —
//!    the daemon shifts the LIVE table of the Spirit it resolves; and
//! 2. the MAPPING: every typed response maps to its D-16-1-P exit.

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

const ROUTE: &str = "/v1/spirits/butler/posture";

#[test]
fn posture_sends_the_shifted_value_in_the_json_body() {
    let door = FixtureDoor::spawn();
    // Both mapped spellings on one door: the enum→wire mapping is the
    // contract, and two different flags must produce two different bodies.
    for flag in ["cautious", "assistive"] {
        let out = door.run(&["posture", "butler", "--shift", flag]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "--shift {flag}: expected exit 0 — stderr: {}",
            String::from_utf8_lossy(&out.stderr),
        );
    }
    let requests = door.requests();
    assert_eq!(requests.len(), 2, "one round trip per shift");
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, ROUTE);
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str()),
        "the bearer must be the token from control.json"
    );
    assert_eq!(
        requests[0].content_type.as_deref(),
        Some("application/json"),
        "posture travels as JSON"
    );
    let body = requests[0].json_body();
    assert_eq!(
        body.get("posture").and_then(|v| v.as_str()),
        Some("cautious"),
        "first shift must carry cautious — got {body}"
    );
    let body = requests[1].json_body();
    assert_eq!(
        body.get("posture").and_then(|v| v.as_str()),
        Some("assistive"),
        "second shift must carry assistive — got {body}"
    );
}

/// The second half: each typed response maps to its D-16-1-P exit.
#[test]
fn posture_maps_typed_responses_to_d16_1_p_exits() {
    assert_typed_matrix(&["posture", "butler", "--shift", "cautious"], "POST", ROUTE);
}

/// Discovery failures (AC5), shared by every door verb.
#[test]
fn posture_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&["posture", "butler", "--shift", "cautious"]);
}

/// `autonomous` is not a runtime posture (`PostureChoice` has three
/// variants): clap refuses it locally, before any round trip.
#[test]
fn autonomous_posture_rejected_by_clap_without_a_round_trip() {
    let door = FixtureDoor::spawn();
    let out = door.run(&["posture", "butler", "--shift", "autonomous"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        door.request_count(),
        0,
        "a clap-level refusal must not touch the door"
    );
}

/// A spirit id the door's route charset refuses never leaves maosctl
/// (`checked_id` — the client-side twin of the server's validation).
#[test]
fn invalid_spirit_id_refused_locally_without_a_round_trip() {
    let door = FixtureDoor::spawn();
    let out = door.run(&["posture", "bad/id", "--shift", "cautious"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(door.request_count(), 0);
}
