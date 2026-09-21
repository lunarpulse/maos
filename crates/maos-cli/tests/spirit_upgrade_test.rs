#![forbid(unsafe_code)]

//! Story 16-1 / T9 (AC7) — CONTRACT tests for `maosctl spirit upgrade`.
//!
//! ⚠ The HEAD file was one of the story's two VACUOUS tests and is
//! REPLACED, not ported: `spirit_upgrade_parses_with_default_hot_swap_policy`
//! (`spirit_upgrade_test.rs:101-110` at HEAD) ended in
//! `assert!(!stderr.is_empty() || out.status.success(), ...)` — a
//! disjunction that is true for literally every outcome, so the test could
//! not fail short of a spawn error. The one-shot it dispatched exits 101 in
//! debug builds (§4, measured); nothing about the WIRE was observed.
//!
//! Over the door (D-16-1-W) the contract is:
//!
//! 1. the WIRE: `POST /v1/spirits/{id}/upgrade` whose JSON body carries the
//!    CANONICAL ABSOLUTE `target_manifest` (a relative path would resolve
//!    in the daemon's cwd), the `policy`, and the operator's
//!    `attestation`/`vetter_keyring` as body fields — never the daemon's
//!    environment; `--plan` arms the body with `create_plan`,
//!    `from_version` and absolute `candidates`; and
//! 2. the MAPPING: every typed response maps to its D-16-1-P exit, with
//!    `--policy cold-swap` answered by the door's typed 409
//!    `cold_swap_unsupported`, and `--policy migrator` refused LOCALLY
//!    (exit 2 — the kernel multi-hop executor is not on the door surface).

#[path = "support/fixture_door.rs"]
mod fixture_door;

use std::path::Path;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

const ROUTE: &str = "/v1/spirits/butler/upgrade";

const MANIFEST: &str = "\
[name = \"butler-successor\"]
";

/// Write an (empty-but-real) fixture manifest; only its path matters to
/// the wire contract.
fn scratch_manifest(dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, MANIFEST).expect("write fixture manifest");
    path
}

fn canonical(path: &Path) -> String {
    std::fs::canonicalize(path)
        .expect("canonicalize fixture manifest")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn upgrade_sends_absolute_manifest_and_default_policy() {
    let door = FixtureDoor::spawn();
    let successor = scratch_manifest(door.maos_home(), "successor.toml");
    let out = door.run(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        successor.to_str().expect("utf8 manifest"),
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
    let body = requests[0].json_body();
    assert_eq!(
        body.get("target_manifest").and_then(|v| v.as_str()),
        Some(canonical(&successor).as_str()),
        "the manifest must travel CANONICAL ABSOLUTE — got {body}"
    );
    assert_eq!(
        body.get("policy").and_then(|v| v.as_str()),
        Some("hot-swap"),
        "hot-swap is the default policy — got {body}"
    );
    assert!(
        body.get("attestation").is_some_and(|v| v.is_null()),
        "no --attestation ⇒ null — got {body}"
    );
    assert!(
        body.get("vetter_keyring").is_some_and(|v| v.is_null()),
        "no --keyring ⇒ null — got {body}"
    );
    assert!(
        body.get("from_version").is_some_and(|v| v.is_null()),
        "no --plan ⇒ null from_version — got {body}"
    );
    assert!(
        body.get("candidates")
            .is_some_and(|v| v.as_array().is_some_and(|a| a.is_empty())),
        "no --plan ⇒ empty candidates — got {body}"
    );
    assert_eq!(
        body.get("create_plan").and_then(|v| v.as_bool()),
        Some(false),
        "got {body}"
    );
}

/// The vetting artifacts are the OPERATOR's inputs and travel as body
/// fields — the daemon must never read them from its own environment
/// (D-16-1-W).
#[test]
fn attestation_and_keyring_travel_in_the_body() {
    let door = FixtureDoor::spawn();
    let home = door.maos_home();
    let successor = scratch_manifest(home, "successor.toml");
    let attestation = home.join("attestation.cbor");
    let keyring = home.join("keyring.cbor");
    std::fs::write(&attestation, b"cbor").expect("write attestation");
    std::fs::write(&keyring, b"cbor").expect("write keyring");

    let out = door.run(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        successor.to_str().expect("utf8 manifest"),
        "--attestation",
        attestation.to_str().expect("utf8 attestation"),
        "--keyring",
        keyring.to_str().expect("utf8 keyring"),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let body = door.requests()[0].json_body();
    assert_eq!(
        body.get("attestation").and_then(|v| v.as_str()),
        Some(attestation.to_string_lossy().as_ref()),
        "the operator's attestation path must travel verbatim — got {body}"
    );
    assert_eq!(
        body.get("vetter_keyring").and_then(|v| v.as_str()),
        Some(keyring.to_string_lossy().as_ref()),
        "the operator's keyring path must travel verbatim — got {body}"
    );
}

/// `--policy cold-swap` reaches the door (the body names it) and the
/// door's typed refusal maps to exit 1 (D-16-1-W: the cold-swap kernel arm
/// starts an unadmitted successor — the door refuses it, typed).
#[test]
fn cold_swap_reaches_the_door_and_409_cold_swap_unsupported_maps_to_1() {
    let door = FixtureDoor::spawn_replying(409, r#"{"error":"cold_swap_unsupported"}"#);
    let successor = scratch_manifest(door.maos_home(), "successor.toml");
    let out = door.run(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        successor.to_str().expect("utf8 manifest"),
        "--policy",
        "cold-swap",
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "cold-swap refusal must exit 1 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cold_swap_unsupported"),
        "stderr must name the typed error — got: {stderr}"
    );
    let body = door.requests()[0].json_body();
    assert_eq!(
        body.get("policy").and_then(|v| v.as_str()),
        Some("cold-swap"),
        "the policy flag must reach the door — got {body}"
    );
}

/// `--policy migrator` names the kernel's multi-hop executor, which the
/// door does not expose — refused LOCALLY, exit 2, no round trip.
#[test]
fn migrator_policy_refused_locally_without_a_round_trip() {
    let door = FixtureDoor::spawn();
    let successor = scratch_manifest(door.maos_home(), "successor.toml");
    let out = door.run(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        successor.to_str().expect("utf8 manifest"),
        "--policy",
        "migrator",
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        door.request_count(),
        0,
        "a locally-refused policy must not touch the door"
    );
}

/// `--from`/`--candidates` are plan-building inputs: without `--plan` they
/// are a usage error (exit 2), no round trip.
#[test]
fn from_and_candidates_require_plan_locally() {
    let door = FixtureDoor::spawn();
    let successor = scratch_manifest(door.maos_home(), "successor.toml");
    let to = successor.to_str().expect("utf8 manifest").to_owned();
    for extra in [
        vec!["--from", "0.3.0"],
        vec!["--candidates", "c1.toml,c2.toml"],
    ] {
        let mut args = vec!["spirit", "upgrade", "butler", "--to", &to];
        args.extend_from_slice(&extra);
        let out = door.run(&args);
        assert_eq!(
            out.status.code(),
            Some(2),
            "--from/--candidates without --plan must exit 2 — args {args:?}"
        );
    }
    assert_eq!(door.request_count(), 0);
}

/// `--plan` arms the body: `create_plan`, `from_version` and an array of
/// ABSOLUTE candidate paths, in operator order.
#[test]
fn plan_mode_carries_create_plan_from_and_absolute_candidates() {
    let door = FixtureDoor::spawn();
    let home = door.maos_home();
    let successor = scratch_manifest(home, "successor.toml");
    let first = scratch_manifest(home, "cand-1.toml");
    let second = scratch_manifest(home, "cand-2.toml");
    let out = door.run(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        successor.to_str().expect("utf8 manifest"),
        "--plan",
        "--from",
        "0.3.0",
        "--candidates",
        &format!(
            "{},{}",
            first.to_str().expect("utf8 c1"),
            second.to_str().expect("utf8 c2")
        ),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected exit 0 — stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let body = door.requests()[0].json_body();
    assert_eq!(
        body.get("create_plan").and_then(|v| v.as_bool()),
        Some(true),
        "got {body}"
    );
    assert_eq!(
        body.get("from_version").and_then(|v| v.as_str()),
        Some("0.3.0"),
        "got {body}"
    );
    let candidates: Vec<String> = body
        .get("candidates")
        .and_then(|v| v.as_array())
        .map(|array| {
            array
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        candidates,
        vec![canonical(&first), canonical(&second)],
        "candidates must travel canonical absolute, operator order — got {body}"
    );
}

/// A `--to` manifest that does not exist is refused locally (exit 1,
/// canonicalization failure) before any round trip.
#[test]
fn manifest_not_found_refused_locally() {
    let door = FixtureDoor::spawn();
    let missing = door.maos_home().join("absent.toml");
    let out = door.run(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        missing.to_str().expect("utf8 manifest"),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(door.request_count(), 0);
}

/// The second half: each typed response maps to its D-16-1-P exit.
#[test]
fn upgrade_maps_typed_responses_to_d16_1_p_exits() {
    // A standalone scratch dir: the matrix spawns its own doors, so the
    // manifest must outlive any single door's home.
    let manifests = tempfile::TempDir::new().expect("tempdir for manifests");
    let successor = scratch_manifest(manifests.path(), "successor.toml");
    let to = successor.to_str().expect("utf8 manifest").to_owned();
    assert_typed_matrix(&["spirit", "upgrade", "butler", "--to", &to], "POST", ROUTE);
}

/// Discovery failures (AC5). The manifest must still EXIST: local
/// canonicalization precedes door discovery, and the discovery contract is
/// what this test isolates.
#[test]
fn upgrade_discovery_failures_map_to_typed_exits() {
    let manifests = tempfile::TempDir::new().expect("tempdir for manifests");
    let successor = scratch_manifest(manifests.path(), "successor.toml");
    let to = successor.to_str().expect("utf8 manifest").to_owned();
    assert_discovery_failures(&["spirit", "upgrade", "butler", "--to", &to]);
}
