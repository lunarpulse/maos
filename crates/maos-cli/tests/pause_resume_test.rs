#![forbid(unsafe_code)]

//! Story 16-1 / T9 (AC7) — CONTRACT tests for `maosctl pause`/`resume`.
//!
//! At HEAD these verbs spawned a `MAOS_ONE_SHOT` child that wrote "Paused"
//! journal rows while the Spirit kept running — §4, measured; the old
//! `lifecycle_journal_has_pause_and_resume` test pinned exactly that fake.
//! Over the Story 16-1 door the contract has two halves:
//!
//! 1. the WIRE: `POST /v1/spirits/{id}/{pause,resume}` with the bearer from
//!    `control.json`, a declared-zero body — and NO child process; and
//! 2. the MAPPING: every typed response maps to its D-16-1-P exit.
//!
//! The door is the hand-rolled loopback fixture (`support/fixture_door.rs`)
//! because `maos-cli` must not gain a `maos-control` edge — not even a dev
//! edge (`dep_kernel_core_free_test.rs` runs plain `cargo tree -p maos-cli`).

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

const PAUSE_ROUTE: &str = "/v1/spirits/butler/pause";
const RESUME_ROUTE: &str = "/v1/spirits/butler/resume";

/// The verb's first half: exact method, exact path, the `control.json`
/// bearer, a Content-Length-0 body (the server answers 411 to a POST
/// without one), and the 200 body printed verbatim on stdout.
fn assert_bare_post(route: &'static str, args: &[&str]) {
    let door = FixtureDoor::spawn();
    let out = door.run(args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{route}: expected exit 0 — stdout: {}, stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(out.stdout, b"{}\n", "{route}: the 200 body prints verbatim");
    let requests = door.requests();
    assert_eq!(requests.len(), 1, "{route}: exactly one round trip");
    assert_eq!(requests[0].method, "POST", "{route}: wrong method");
    assert_eq!(requests[0].path, route, "{route}: wrong path");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str()),
        "{route}: the bearer must be the token from control.json"
    );
    assert_eq!(
        requests[0].content_length,
        Some(0),
        "{route}: a body-less POST must still declare Content-Length: 0"
    );
    assert!(requests[0].body.is_empty(), "{route}: no body expected");
}

#[test]
fn pause_sends_bare_post_and_prints_the_door_body() {
    assert_bare_post(PAUSE_ROUTE, &["pause", "butler"]);
}

#[test]
fn resume_sends_bare_post_and_prints_the_door_body() {
    assert_bare_post(RESUME_ROUTE, &["resume", "butler"]);
}

/// The load-bearing half: at HEAD `pause`/`resume` spawned a one-shot child
/// (`MAOS_BIN_PATH` pointed at the real `maos`) whose journal writes were
/// the ONLY effect. The door verbs must spawn NOTHING — a child that runs
/// would leave the witness file behind.
#[cfg(unix)]
#[test]
fn pause_and_resume_spawn_no_child_process() {
    let door = FixtureDoor::spawn();
    let (script, witness) = fixture_door::witness_script(door.maos_home(), "pause-resume");
    let script = script.to_str().expect("utf8 script path");

    let out = door.run_with(&[("MAOS_BIN_PATH", script)], &["pause", "butler"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "pause: expected exit 0 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let out = door.run_with(&[("MAOS_BIN_PATH", script)], &["resume", "butler"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "resume: expected exit 0 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    assert_eq!(
        door.request_count(),
        2,
        "both verbs must travel the door, one round trip each"
    );
    assert!(
        !witness.exists(),
        "pause/resume spawned a child — the one-shot path is back"
    );
}

/// The second half: each typed response maps to its D-16-1-P exit. The
/// mapping lives once in `door_client::map_response`; this proves the
/// pause route reaches it.
#[test]
fn pause_maps_typed_responses_to_d16_1_p_exits() {
    assert_typed_matrix(&["pause", "butler"], "POST", PAUSE_ROUTE);
}

#[test]
fn resume_maps_typed_responses_to_d16_1_p_exits() {
    assert_typed_matrix(&["resume", "butler"], "POST", RESUME_ROUTE);
}

/// Discovery failures (AC5) — no live door needed: absent config ⇒ 78
/// naming `maos init`; half-set env pair ⇒ 78; world-readable config ⇒ 78;
/// configured-but-dead endpoint ⇒ 69 `DaemonNotRunning`.
#[test]
fn pause_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&["pause", "butler"]);
}
