#![forbid(unsafe_code)]

//! Story 16-1 / T9 (AC7) — CONTRACT tests for `maosctl halt resolve`.
//!
//! At HEAD the verb FABRICATED the halt it resolved (a one-shot planted a
//! pending halt tagged "test" and journaled its own resolution — §4,
//! measured; the old tests asserted those invented Approval-Log rows).
//! Over the door the daemon owns the HaltRegistry, and the contract is:
//!
//! 1. the WIRE: `POST /v1/halts/{halt_id}/resolve` with the spirit id in
//!    the BODY (`spirit_id`/`resolution`/`rationale`) — the flag is
//!    `--kind`, and its `provided-context`/`authorized-override` payloads
//!    travel as `rationale`; and
//! 2. the MAPPING: every typed response maps to its D-16-1-P exit.

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

const HALT_ID: &str = "halt-9f3a2b7c";
const ROUTE: &str = "/v1/halts/halt-9f3a2b7c/resolve";

/// Run one resolve against a fresh 200 door and hand back what hit it.
fn resolve(
    extra_args: &[&str],
) -> (
    std::process::Output,
    Vec<fixture_door::RecordedRequest>,
    &'static str,
) {
    let door = FixtureDoor::spawn();
    let mut args = vec!["halt", "resolve", HALT_ID, "--spirit", "butler"];
    args.extend_from_slice(extra_args);
    let out = door.run(&args);
    (out, door.requests(), door.token())
}

fn assert_accepted(out: &std::process::Output, what: &str) {
    assert_eq!(
        out.status.code(),
        Some(0),
        "{what}: expected exit 0 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn accepted_halt_sends_post_with_spirit_id_in_the_body() {
    let (out, requests, token) = resolve(&["--kind", "accepted-halt"]);
    assert_accepted(&out, "accepted-halt");
    assert_eq!(out.stdout, b"{}\n", "the 200 body prints verbatim");
    assert_eq!(requests.len(), 1, "exactly one round trip");
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, ROUTE);
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {token}").as_str()),
        "the bearer must be the token from control.json"
    );
    assert_eq!(
        requests[0].content_type.as_deref(),
        Some("application/json"),
        "the resolution travels as JSON"
    );
    // The spirit id is a BODY field, not a path segment — the route is
    // keyed by the halt alone.
    let body = requests[0].json_body();
    assert_eq!(
        body.get("spirit_id").and_then(|v| v.as_str()),
        Some("butler"),
        "spirit_id must be in the body — got {body}"
    );
    assert_eq!(
        body.get("resolution").and_then(|v| v.as_str()),
        Some("accepted_halt"),
        "the clap spelling maps to the wire spelling — got {body}"
    );
    assert!(
        body.get("rationale").is_some_and(|v| v.is_null()),
        "accepted-halt carries no rationale — got {body}"
    );
}

/// `--text` is the rationale for a provided-context resolution.
#[test]
fn provided_context_carries_the_text_as_rationale() {
    let (out, requests, _) = resolve(&[
        "--kind",
        "provided-context",
        "--text",
        "the missing context",
    ]);
    assert_accepted(&out, "provided-context");
    let body = requests[0].json_body();
    assert_eq!(
        body.get("resolution").and_then(|v| v.as_str()),
        Some("provided_context"),
        "got {body}"
    );
    assert_eq!(
        body.get("rationale").and_then(|v| v.as_str()),
        Some("the missing context"),
        "--text must travel as rationale — got {body}"
    );
}

/// `--operator-policy` is the rationale for an authorized override.
#[test]
fn authorized_override_carries_the_policy_ref_as_rationale() {
    let (out, requests, _) = resolve(&[
        "--kind",
        "authorized-override",
        "--operator-policy",
        "pol-9-operator",
    ]);
    assert_accepted(&out, "authorized-override");
    let body = requests[0].json_body();
    assert_eq!(
        body.get("resolution").and_then(|v| v.as_str()),
        Some("authorized_override"),
        "got {body}"
    );
    assert_eq!(
        body.get("rationale").and_then(|v| v.as_str()),
        Some("pol-9-operator"),
        "--operator-policy must travel as rationale — got {body}"
    );
}

/// clap's `required_if_eq` refuses provided-context without `--text`
/// LOCALLY (exit 2) — the door must never see a half-formed resolution.
#[test]
fn provided_context_without_text_refused_before_the_round_trip() {
    let door = FixtureDoor::spawn();
    let out = door.run(&[
        "halt",
        "resolve",
        HALT_ID,
        "--spirit",
        "butler",
        "--kind",
        "provided-context",
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        door.request_count(),
        0,
        "a locally-refused resolution must not touch the door"
    );
}

/// The second half: each typed response maps to its D-16-1-P exit. The
/// unknown/already-resolved halt is the DAEMON's typed 404 — the client
/// preflight that used to fabricate an answer is gone (D-16-1-K).
#[test]
fn resolve_maps_typed_responses_to_d16_1_p_exits() {
    assert_typed_matrix(
        &[
            "halt",
            "resolve",
            HALT_ID,
            "--spirit",
            "butler",
            "--kind",
            "accepted-halt",
        ],
        "POST",
        ROUTE,
    );
}

/// Discovery failures (AC5), shared by every door verb.
#[test]
fn resolve_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&[
        "halt",
        "resolve",
        HALT_ID,
        "--spirit",
        "butler",
        "--kind",
        "accepted-halt",
    ]);
}

/// Ids outside the door's route charset (`[A-Za-z0-9._-]{1,128}`) are
/// refused locally for BOTH the halt id and the spirit id.
#[test]
fn invalid_ids_refused_locally_without_a_round_trip() {
    let door = FixtureDoor::spawn();
    for args in [
        vec![
            "halt",
            "resolve",
            "bad/id",
            "--spirit",
            "butler",
            "--kind",
            "accepted-halt",
        ],
        vec![
            "halt",
            "resolve",
            HALT_ID,
            "--spirit",
            "bad/id",
            "--kind",
            "accepted-halt",
        ],
    ] {
        let out = door.run(&args);
        assert_eq!(
            out.status.code(),
            Some(2),
            "invalid id must exit 2 — args {args:?}, stderr: {}",
            String::from_utf8_lossy(&out.stderr),
        );
    }
    assert_eq!(
        door.request_count(),
        0,
        "a locally-refused id must not touch the door"
    );
}
