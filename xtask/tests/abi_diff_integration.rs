//! Integration tests for the cargo-public-api-backed abi-diff gate.
//! Story 1a.5: verifies four soundness-gap fixtures.
//!
//! These tests require `cargo-public-api` installed and the nightly Rust
//! toolchain available. They are excluded from `cargo test --workspace` by
//! being a separate test binary and are intended to run via:
//!   cargo test -p xtask --test abi_diff_integration

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/abi-diff")
}

fn run_public_api(manifest_path: &str) -> String {
    let output = Command::new("cargo")
        .args(["public-api", "-sss", "--manifest-path", manifest_path])
        .output()
        .expect("cargo-public-api not installed; install with: cargo install cargo-public-api --version 0.51.0");
    if !output.status.success() {
        panic!(
            "cargo-public-api failed for {}: {}",
            manifest_path,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).expect("non-utf8 output")
}

fn load_expected(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path))
}

fn assert_matches_expected(fixture: &str, variant: &str) {
    let root = fixtures_root();
    let manifest = root.join(fixture).join(variant).join("Cargo.toml");
    let expected_path = root.join(fixture).join(variant).join("EXPECTED.txt");
    let actual = run_public_api(manifest.to_str().unwrap());
    let expected = load_expected(expected_path.to_str().unwrap());
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "cargo-public-api output for {fixture}/{variant} does not match EXPECTED.txt"
    );
}

fn assert_identical_across_variants(fixture: &str, var_a: &str, var_b: &str) {
    let root = fixtures_root();
    let manifest_a = root.join(fixture).join(var_a).join("Cargo.toml");
    let manifest_b = root.join(fixture).join(var_b).join("Cargo.toml");
    let out_a = run_public_api(manifest_a.to_str().unwrap());
    let out_b = run_public_api(manifest_b.to_str().unwrap());
    assert_eq!(
        out_a.trim(),
        out_b.trim(),
        "cargo-public-api output differs between {fixture}/{var_a} and {fixture}/{var_b} (expected identical)"
    );
}

fn assert_differs_across_variants(fixture: &str, var_a: &str, var_b: &str) {
    let root = fixtures_root();
    let manifest_a = root.join(fixture).join(var_a).join("Cargo.toml");
    let manifest_b = root.join(fixture).join(var_b).join("Cargo.toml");
    let out_a = run_public_api(manifest_a.to_str().unwrap());
    let out_b = run_public_api(manifest_b.to_str().unwrap());
    assert_ne!(
        out_a.trim(),
        out_b.trim(),
        "cargo-public-api output is identical between {fixture}/{var_a} and {fixture}/{var_b} (expected different)"
    );
}

// --- Soundness gap 1: quote-fragility ---
// Two crates with semantically-identical APIs but different whitespace.
// cargo-public-api canonical output should be IDENTICAL (no false positive).

#[test]
fn quote_whitespace_no_false_positive() {
    assert_identical_across_variants("quote-whitespace", "baseline", "modified");
    assert_matches_expected("quote-whitespace", "baseline");
    assert_matches_expected("quote-whitespace", "modified");
}

// --- Soundness gap 2: pub-use reexport ---
// Function moved behind a pub use reexport should still appear in output.

#[test]
fn pub_use_reexport_preserved() {
    // Baseline (fn in lib.rs) and modified (fn behind pub use) should be identical.
    assert_identical_across_variants("pub-use-reexport", "baseline", "modified");
    assert_matches_expected("pub-use-reexport", "baseline");
}

#[test]
fn pub_use_reexport_removal_detected() {
    // Removing the pub use (keeping fn private) should produce different output.
    assert_differs_across_variants("pub-use-reexport", "baseline", "modified-removed");
    // The removed variant should NOT contain bar().
    let root = fixtures_root();
    let manifest = root.join("pub-use-reexport/modified-removed/Cargo.toml");
    let output = run_public_api(manifest.to_str().unwrap());
    assert!(!output.contains("bar"), "bar() should not appear after pub use removal");
}

// --- Soundness gap 3: generic-bound order ---
// cargo-public-api produces deterministic output per variant (no quote! fragility).
// Bound order IS represented in the output (faithful, not false-positive).

#[test]
fn generic_bound_reorder_deterministic() {
    // Each variant produces deterministic output matching its EXPECTED.txt.
    assert_matches_expected("generic-bound-reorder", "baseline");
    assert_matches_expected("generic-bound-reorder", "modified");
    // Bound order IS reflected in output — this is intentional precision,
    // not a false positive. The bespoke walker was fragile because quote!()
    // strings were toolchain-dependent; cargo-public-api is deterministic.
}

// --- Soundness gap 4: inline-mod items ---
// Both inline-mod forms should produce the same public API surface.

#[test]
fn inline_mod_items_visible() {
    assert_identical_across_variants("inline-mod-items", "baseline", "modified");
    assert_matches_expected("inline-mod-items", "baseline");
    // Verify the inline mod's public function is actually visible.
    let root = fixtures_root();
    let manifest = root.join("inline-mod-items/baseline/Cargo.toml");
    let output = run_public_api(manifest.to_str().unwrap());
    assert!(output.contains("foo::bar"), "inline mod pub fn should be visible");
}
