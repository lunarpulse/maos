#![forbid(unsafe_code)]

//! Story 16-1 / T9 (AC7) — CONTRACT tests for
//! `maosctl revocations import/list`.
//!
//! At HEAD the one-shot opened a fresh scheduler (so `matched_count` was
//! always 0), kept no rules past exit, and `revocations list` was one of
//! the two VACUOUS tests the story calls out: it spawned the child, killed
//! it after 500 ms and asserted NOTHING (`revocations_import_test.rs:141-167`
//! at HEAD). Over the door (D-16-1-X):
//!
//! 1. the WIRE: the CRL's own BYTES are the `POST /v1/revocations` body
//!    (`application/octet-stream`) — never a path, which would resolve in
//!    the daemon's cwd; `list` is `GET /v1/revocations`; and
//! 2. the MAPPING: every typed response maps to its D-16-1-P exit.

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

/// Deliberately NOT valid JSON or UTF-8: if the client re-encoded, wrapped
/// or path-ified the CRL, byte equality would catch it.
const CRL_BYTES: &[u8] =
    b"--MAOS-CRL-fixture v1\n\x00\x01\x02\xff\xfe revoked-id-1\n\x80\x81\x82\n";

#[test]
fn import_sends_the_crl_bytes_verbatim() {
    let door = FixtureDoor::spawn();
    let crl_path = door.maos_home().join("fixture.crl");
    std::fs::write(&crl_path, CRL_BYTES).expect("write fixture CRL");

    let out = door.run(&[
        "revocations",
        "import",
        crl_path.to_str().expect("utf8 crl"),
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
    assert_eq!(requests[0].path, "/v1/revocations");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str()),
        "the bearer must be the token from control.json"
    );
    assert_eq!(
        requests[0].content_type.as_deref(),
        Some("application/octet-stream"),
        "D-16-1-X: the CRL travels as raw bytes"
    );
    assert_eq!(
        requests[0].content_length,
        Some(CRL_BYTES.len()),
        "Content-Length must be the CRL's own size"
    );
    // The load-bearing byte assertion: the body IS the file, verbatim.
    assert_eq!(
        requests[0].body, CRL_BYTES,
        "the CRL bytes must reach the door unmodified — a path or a re-encode is a contract break"
    );
}

/// `list` is a GET, and its rendered output names the `crl_id` the door
/// returned (rendered as the hex string D-16-1-X specifies) — not a
/// constant, not an empty list.
#[test]
fn list_sends_get_and_renders_the_door_crl_id() {
    let door = FixtureDoor::spawn_replying(
        200,
        r#"{"applied":[{"crl_id":"7d1f0a9c2e5b4831aa06ccd49f12e7b3","applied_at_ns":1719}]}"#,
    );
    let out = door.run(&["revocations", "list"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("7d1f0a9c2e5b4831aa06ccd49f12e7b3"),
        "the door's crl_id must be rendered — got: {stdout}"
    );
    let requests = door.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/v1/revocations");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str()),
        "the bearer must be the token from control.json"
    );
}

/// A missing CRL file is refused locally (exit 1) before any round trip.
#[test]
fn import_missing_file_refused_locally() {
    let door = FixtureDoor::spawn();
    let missing = door.maos_home().join("absent.crl");
    let out = door.run(&["revocations", "import", missing.to_str().expect("utf8")]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        door.request_count(),
        0,
        "a missing file must not touch the door"
    );
}

/// The second half: each typed response maps to its D-16-1-P exit.
#[test]
fn import_maps_typed_responses_to_d16_1_p_exits() {
    let door = FixtureDoor::spawn();
    let crl_path = door.maos_home().join("fixture.crl");
    std::fs::write(&crl_path, CRL_BYTES).expect("write fixture CRL");
    let args = [
        "revocations",
        "import",
        crl_path.to_str().expect("utf8 crl"),
    ];
    assert_typed_matrix(&args, "POST", "/v1/revocations");
}

/// Discovery failures (AC5), shared by every door verb.
#[test]
fn revocations_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&["revocations", "list"]);
}
