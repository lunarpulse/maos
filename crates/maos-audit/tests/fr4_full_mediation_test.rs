#![forbid(unsafe_code)]

//! AC2 — FR4 1000-entry fixture: prove 100% mediation across a 1000-call sample.
//!
//! Reads the checked-in fixture `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl`,
//! parses every line as an [`maos_audit::Fr4Entry`], and asserts that all 1000
//! entries carry non-null `capability_token`, non-zero `spirit_pid`, and
//! non-zero `boot_nonce`. The first violation aborts with the offending line
//! number — no silent pass on partial coverage.
//!
//! A companion determinism sub-test re-runs the generator binary, compares
//! the output byte-for-byte against the checked-in fixture, and asserts
//! equality. This gates regeneration drift.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use maos_audit::Fr4Entry;

fn fixture_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join("hello-spirit-1k.jsonl")
}

#[test]
fn test_fr4_full_mediation() {
    let path = fixture_path();
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture {}: {e}", path.display()));

    let mut count = 0usize;
    for (idx, raw) in contents.lines().enumerate() {
        let line_no = idx + 1;
        let entry: Fr4Entry = serde_json::from_str(raw)
            .unwrap_or_else(|e| panic!("FR4 violation at line {line_no}: invalid JSON ({e})"));

        // FR4: capability_token must be present and non-empty (64-char hex of
        // the 32-byte Ed25519 token).
        if entry.capability_token.is_empty() {
            panic!("FR4 violation at line {line_no}: missing field 'capability_token'");
        }
        assert_eq!(
            entry.capability_token.len(),
            64,
            "FR4 violation at line {line_no}: capability_token must be 64-char hex"
        );

        // FR4: spirit_pid must be non-zero (the fixture seeds 1..=5 — zero
        // would mean the seed never bound a Spirit).
        if entry.spirit_pid == 0 {
            panic!("FR4 violation at line {line_no}: missing field 'spirit_pid' (zero)");
        }

        // FR4: boot_nonce must be non-zero (each boot generates a fresh
        // nonce; zero means the kernel never set one).
        if entry.boot_nonce == 0 {
            panic!("FR4 violation at line {line_no}: missing field 'boot_nonce' (zero)");
        }

        // call_type must be a known kind.
        match entry.call_type.as_str() {
            "inference.call" | "capability.invocation" => {}
            other => panic!(
                "FR4 violation at line {line_no}: unknown call_type '{other}'"
            ),
        }

        // timestamp_ns must be non-zero (synthetic fixture uses
        // BASE_TIMESTAMP_NS=1.7e18, so this is just belt-and-suspenders).
        if entry.timestamp_ns == 0 {
            panic!("FR4 violation at line {line_no}: missing field 'timestamp_ns' (zero)");
        }

        count += 1;
    }

    assert_eq!(
        count, 1000,
        "AC2: fixture must contain exactly 1000 mediated entries (got {count})"
    );
}

/// Determinism sub-test: re-run the generator binary, compare to the
/// checked-in fixture byte-for-byte. Fails fast on regeneration drift.
#[test]
fn fixture_is_byte_deterministic() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root resolvable from CARGO_MANIFEST_DIR")
        .to_path_buf();

    let tmp = tempfile::TempDir::new().unwrap();
    let regen_path = tmp.path().join("regen.jsonl");

    // Run the gen_fixture binary via cargo. We use cargo so the build graph
    // re-uses any cached artifacts and the test does not need to know the
    // exact `target/` layout.
    let output = Command::new("cargo")
        .current_dir(&workspace_root)
        .args([
            "run",
            "--quiet",
            "-p",
            "maos-audit",
            "--bin",
            "gen_fixture",
            "--locked",
            "--",
        ])
        .arg(&regen_path)
        .output()
        .expect("invoke gen_fixture via cargo");

    assert!(
        output.status.success(),
        "gen_fixture failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let regenerated = fs::read(&regen_path).expect("read regenerated fixture");
    let checked_in = fs::read(fixture_path()).expect("read checked-in fixture");
    assert_eq!(
        regenerated.len(),
        checked_in.len(),
        "fixture byte length drift: regen={} bytes, checked-in={} bytes — \
         re-run `bash scripts/gen_hello_spirit_fixture.sh` and commit if intentional",
        regenerated.len(),
        checked_in.len()
    );
    assert_eq!(
        regenerated, checked_in,
        "fixture byte drift: gen_fixture produced different bytes than the checked-in file"
    );
}
