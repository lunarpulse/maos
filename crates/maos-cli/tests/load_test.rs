#![forbid(unsafe_code)]

//! Story 16-6 — CONTRACT tests for `maosctl load <manifest>`.
//!
//! The wire contract (D-16-6-E):
//!
//! 1. `POST /v1/spirits` — a COLLECTION route, not `/v1/spirits/{id}/load`.
//!    A filesystem path can never be a path segment (`validate_id` permits
//!    `[A-Za-z0-9._-]{1,128}`), and the id does not exist until the daemon
//!    has parsed the manifest.
//! 2. the body carries the CANONICAL ABSOLUTE manifest path under
//!    `"manifest"` — a relative path would resolve in the DAEMON's working
//!    directory, which is not the operator's (D-16-1-X).
//! 3. a missing or non-canonicalisable manifest exits 1 BEFORE any round
//!    trip: the operator learns their own path is wrong from their own
//!    machine, not from a 400 the daemon had to be woken up to produce.
//! 4. every typed response maps to its D-16-1-P exit, and the four discovery
//!    failures map to theirs — both through the SHARED drivers, never a
//!    hand-rolled copy of the matrix.

use std::path::Path;
use std::process::Command;

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};

const ROUTE: &str = "/v1/spirits";

const MANIFEST: &str = r#"
[class]
name = "reviewer"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.0.1"
forms = ["rust-inproc"]
trust_tier = "local"
description = "load fixture"
"#;

fn scratch_manifest(dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, MANIFEST).expect("write fixture manifest");
    path
}

/// The wire: collection route, POST, canonical absolute path in the body.
#[test]
fn load_posts_the_canonical_manifest_to_the_collection_route() {
    let door = FixtureDoor::spawn();
    let scratch = door
        .maos_home()
        .parent()
        .expect("fixture MAOS_HOME has a scratch parent");
    let manifest = scratch_manifest(scratch, "reviewer.toml");

    // The filename must resolve from the operator's CWD, then cross the door
    // as the canonical absolute path. An already-absolute argument would not
    // distinguish a deleted `canonical_manifest` call in the client.
    let mut command = Command::new(fixture_door::maosctl());
    command.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        command.env("PATH", path);
    }
    command
        .env("HOME", door.home_dir())
        .env("MAOS_HOME", door.maos_home())
        .env("XDG_DATA_HOME", door.xdg_dir())
        .current_dir(scratch)
        .args(["load", "reviewer.toml"]);
    let out = command.output().expect("spawn maosctl");
    assert!(
        out.status.success(),
        "load against a 200 door must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let requests = door.requests();
    assert_eq!(requests.len(), 1, "exactly one door round trip");
    assert_eq!(requests[0].method, "POST");
    assert_eq!(
        requests[0].path, ROUTE,
        "load is a COLLECTION route — the id does not exist yet"
    );
    assert_eq!(
        requests[0].authorization,
        Some(format!("Bearer {}", door.token()))
    );

    let sent = requests[0].json_body();
    let on_the_wire = sent
        .get("manifest")
        .and_then(serde_json::Value::as_str)
        .expect("the body carries a `manifest` field");
    let expected = std::fs::canonicalize(&manifest).expect("canonicalize fixture");
    assert_eq!(
        Path::new(on_the_wire),
        expected,
        "the manifest must cross the door CANONICAL and ABSOLUTE — the daemon \
         resolves relative paths in its own working directory, not the operator's"
    );
}

/// A manifest the operator cannot name is refused locally, with no daemon
/// involved at all.
#[test]
fn load_refuses_a_missing_manifest_without_touching_the_door() {
    let door = FixtureDoor::spawn();
    let missing = door.maos_home().join("no-such-manifest.toml");

    let out = door.run(&["load", missing.to_str().expect("utf8 path")]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a missing manifest is exit 1, not a door error"
    );
    assert_eq!(
        door.request_count(),
        0,
        "local validation precedes door discovery: nothing was sent"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("manifest file not found"),
        "stderr must name the operator's own mistake; got: {stderr}"
    );
}

/// D-16-1-P, through the shared driver — the whole typed matrix, not a
/// hand-picked subset.
#[test]
fn load_maps_typed_responses_to_d16_1_p_exits() {
    // A standalone scratch dir: the matrix spawns a fresh door (and a fresh
    // scratch HOME) per case, so the manifest must outlive any one of them.
    let manifests = tempfile::TempDir::new().expect("tempdir for manifests");
    let manifest = scratch_manifest(manifests.path(), "reviewer.toml");
    let path = manifest.to_str().expect("utf8 manifest").to_owned();
    assert_typed_matrix(&["load", &path], "POST", ROUTE);
}

/// The four discovery failures. The manifest must EXIST: canonicalisation
/// precedes door discovery, and discovery is what this isolates.
#[test]
fn load_discovery_failures_map_to_typed_exits() {
    let manifests = tempfile::TempDir::new().expect("tempdir for manifests");
    let manifest = scratch_manifest(manifests.path(), "reviewer.toml");
    let path = manifest.to_str().expect("utf8 manifest").to_owned();
    assert_discovery_failures(&["load", &path]);
}
