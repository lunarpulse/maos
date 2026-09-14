#![forbid(unsafe_code)]

//! Story 16-1 / T9 (AC7) — CONTRACT tests for `maosctl revoke-token`.
//!
//! At HEAD the verb opened a FRESH token ring in a one-shot child and could
//! NEVER succeed — the old tests pinned only the failure paths (§4,
//! measured). Over the door the daemon's live ring answers, and the
//! repeated-revoke answer is typed (D-16-1-V):
//!
//! 1. the WIRE: `POST /v1/tokens/{token_id}/revoke` — no body, no
//!    `--reason` (a flag that changed nothing would be a lie); and
//! 2. the MAPPING: every typed response maps to its D-16-1-P exit, with a
//!    second revoke of the same token arriving as the daemon's typed 409
//!    `revoked` ⇒ exit 1.

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

/// 32-char lowercase hex — the wire shape `CapabilityToken::token_id`
/// renders to.
const TOKEN_ID: &str = "a1b2c3d4e5f60718a9b0c1d2e3f40516";
const ROUTE: &str = "/v1/tokens/a1b2c3d4e5f60718a9b0c1d2e3f40516/revoke";

#[test]
fn revoke_sends_bare_post_to_the_token_route() {
    let door = FixtureDoor::spawn();
    let out = door.run(&["revoke-token", TOKEN_ID]);
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
        requests[0].content_length,
        Some(0),
        "the revoke route carries no body (still declares Content-Length: 0)"
    );
    assert!(requests[0].body.is_empty());
}

/// Hex validation happens BEFORE any round trip: short, uppercase and
/// non-hex ids all refuse locally at exit 2.
#[test]
fn invalid_token_ids_refused_locally_without_a_round_trip() {
    let door = FixtureDoor::spawn();
    for bad in [
        "a1b2c3d4",
        "A1B2C3D4E5F60718A9B0C1D2E3F40516",
        "zzb2c3d4e5f60718a9b0c1d2e3f40516",
        "a1b2c3d4e5f60718a9b0c1d2e3f405160",
    ] {
        let out = door.run(&["revoke-token", bad]);
        assert_eq!(
            out.status.code(),
            Some(2),
            "invalid token id {bad:?} must exit 2 — stderr: {}",
            String::from_utf8_lossy(&out.stderr),
        );
    }
    assert_eq!(
        door.request_count(),
        0,
        "a locally-refused id must not touch the door"
    );
}

/// D-16-1-V: revoking an already-revoked token is the daemon's typed 409
/// `revoked` ⇒ exit 1 — the one-shot's silent second `Ok` is gone.
#[test]
fn repeated_revoke_maps_409_revoked_to_exit_1() {
    let door = FixtureDoor::spawn_replying(409, r#"{"error":"revoked"}"#);
    let out = door.run(&["revoke-token", TOKEN_ID]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a repeated revoke must exit 1 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("revoked") && stderr.contains("HTTP 409"),
        "stderr must name the typed error — got: {stderr}"
    );
    assert_eq!(door.requests()[0].path, ROUTE);
}

/// A never-issued id is the daemon's typed 404 `unknown_token` ⇒ exit 1.
#[test]
fn unknown_token_maps_404_to_exit_1() {
    let door = FixtureDoor::spawn_replying(404, r#"{"error":"unknown_token"}"#);
    let out = door.run(&["revoke-token", TOKEN_ID]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "unknown token must exit 1 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown_token"),
        "stderr must name the typed error — got: {stderr}"
    );
}

/// The second half: each typed response maps to its D-16-1-P exit.
#[test]
fn revoke_maps_typed_responses_to_d16_1_p_exits() {
    assert_typed_matrix(&["revoke-token", TOKEN_ID], "POST", ROUTE);
}

/// Discovery failures (AC5), shared by every door verb.
#[test]
fn revoke_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&["revoke-token", TOKEN_ID]);
}
