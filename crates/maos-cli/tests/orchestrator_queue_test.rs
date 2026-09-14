#![forbid(unsafe_code)]

//! Story 16-1 / T9 (AC7) — CONTRACT tests for
//! `maosctl orchestrator queue/status`.
//!
//! At HEAD both verbs ran against a FRESH in-process registry: status was
//! ALWAYS `0/32` and queued ids restarted at 1 per process (§4, measured;
//! the old tests asserted Approval-Log rows from the one-shot). Over the
//! door the daemon's own buffer answers:
//!
//! 1. the WIRE: `POST /v1/orchestrator/{spirit}` with `{"text": ...}` for a
//!    queue, `GET /v1/orchestrator/{spirit}` for a status; and
//! 2. the MAPPING: queue maps every typed response to its D-16-1-P exit;
//!    status renders the door's `pending`/`capacity` as `p/c` — never a
//!    constant.

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

const ROUTE: &str = "/v1/orchestrator/butler";

#[test]
fn queue_sends_the_instruction_as_the_text_field() {
    let door = FixtureDoor::spawn();
    let out = door.run(&[
        "orchestrator",
        "queue",
        "--spirit",
        "butler",
        "water the plants",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(out.stdout, b"{}\n", "the 200 body prints verbatim");
    let requests = door.requests();
    assert_eq!(requests.len(), 1, "exactly one round trip");
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
        "the instruction travels as JSON"
    );
    let body = requests[0].json_body();
    assert_eq!(
        body.get("text").and_then(|v| v.as_str()),
        Some("water the plants"),
        "the instruction must travel verbatim as `text` — got {body}"
    );
}

/// Status renders the DOOR's numbers: two different payloads must render
/// two different occupancies, so a registry the test filled and read back
/// (or a constant) cannot pass.
#[test]
fn status_sends_get_and_renders_the_door_occupancy() {
    for (pending, capacity) in [(1_u64, 32_u64), (7, 64)] {
        let door = FixtureDoor::spawn_replying(
            200,
            &format!(r#"{{"pending":{pending},"capacity":{capacity},"queue":[]}}"#),
        );
        let out = door.run(&["orchestrator", "status", "--spirit", "butler"]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "expected exit 0 — stderr: {}",
            String::from_utf8_lossy(&out.stderr),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(&format!("{pending}/{capacity}")),
            "status must render the door's pending/capacity as {pending}/{capacity} — got: {stdout}"
        );
        let requests = door.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET", "status is a read");
        assert_eq!(requests[0].path, ROUTE);
        assert_eq!(
            requests[0].authorization.as_deref(),
            Some(format!("Bearer {}", door.token()).as_str()),
            "the bearer must be the token from control.json"
        );
    }
}

/// The empty-instruction refusal is LOCAL (clap/dispatch, exit 2) — the
/// door must never see it.
#[test]
fn empty_instruction_refused_locally_without_a_round_trip() {
    let door = FixtureDoor::spawn();
    for instruction in ["", "   "] {
        let out = door.run(&["orchestrator", "queue", "--spirit", "butler", instruction]);
        assert_eq!(
            out.status.code(),
            Some(2),
            "empty instruction {instruction:?} must exit 2"
        );
    }
    assert_eq!(
        door.request_count(),
        0,
        "a locally-refused instruction must not touch the door"
    );
}

/// The second half: each typed response maps to its D-16-1-P exit. An
/// unknown spirit is the DAEMON's typed 404 — the client preflight is gone
/// (D-16-1-K).
#[test]
fn queue_maps_typed_responses_to_d16_1_p_exits() {
    assert_typed_matrix(
        &[
            "orchestrator",
            "queue",
            "--spirit",
            "butler",
            "water the plants",
        ],
        "POST",
        ROUTE,
    );
}

/// Discovery failures (AC5), shared by every door verb.
#[test]
fn queue_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&[
        "orchestrator",
        "queue",
        "--spirit",
        "butler",
        "water the plants",
    ]);
}

/// A spirit id the door's route charset refuses never leaves maosctl.
#[test]
fn invalid_spirit_id_refused_locally_without_a_round_trip() {
    let door = FixtureDoor::spawn();
    let out = door.run(&[
        "orchestrator",
        "queue",
        "--spirit",
        "bad/id",
        "water the plants",
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(door.request_count(), 0);
}
