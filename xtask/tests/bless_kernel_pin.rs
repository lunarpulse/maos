#![forbid(unsafe_code)]

//! The kernel-pin leg of `xtask bless` (lean-gates WP-23): the pin is rewritten from the tree on
//! disk, in that direction only, and the gate that reds on a stale pin names the command.

use std::fs;
use std::path::Path;

use xtask::check_kernel_baseline::{bless_pin, check_at, failure_detail};

const PRELUDE: &str = "# data file: values only\nsrc_lines = 1\n\n";

fn plant(root: &Path, baseline: &str) {
    let src = root.join("crates/maos-kernel-core/src");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("lib.rs"), "mod sub;\nfn a() {}\n").unwrap();
    fs::write(src.join("sub/mod.rs"), "fn b() {}\n").unwrap();
    fs::create_dir_all(root.join("xtask")).unwrap();
    fs::write(root.join("xtask/kernel-core-baseline.toml"), baseline).unwrap();
}

fn check(root: &Path) -> xtask::check_kernel_baseline::Report {
    check_at(
        &root.join("crates/maos-kernel-core/src"),
        &root.join("xtask/kernel-core-baseline.toml"),
    )
    .unwrap()
}

fn stale_baseline() -> String {
    format!(
        "{PRELUDE}[kernel_src]\nroot = \"crates/maos-kernel-core/src\"\nset_hash = \"{}\"\n\n[kernel_src.files]\n\"gone.rs\" = \"{}\"\n",
        "0".repeat(64),
        "1".repeat(64)
    )
}

#[test]
fn bless_rewrites_a_stale_pin_so_the_gate_passes_and_keeps_the_prelude() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path();
    // The stale file is not even self-consistent, so only a rewrite from the tree can fix it.
    plant(root, &stale_baseline());

    assert!(bless_pin(root).unwrap(), "a stale pin must be rewritten");

    let blessed = fs::read_to_string(root.join("xtask/kernel-core-baseline.toml")).unwrap();
    assert!(
        blessed.starts_with("# data file: values only\nsrc_lines = 3\n\n[kernel_src]\n"),
        "only the src_lines value changes before the block: {blessed}"
    );
    let report = check(root);
    assert!(report.passed, "{}", failure_detail(&report));
    assert_eq!(report.pinned_lines, 3);
    assert_eq!(report.file_set_entries, 2);
}

#[test]
fn bless_is_idempotent() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path();
    plant(root, &stale_baseline());
    assert!(bless_pin(root).unwrap());
    let first = fs::read(root.join("xtask/kernel-core-baseline.toml")).unwrap();
    assert!(
        !bless_pin(root).unwrap(),
        "a pin in sync must report no change"
    );
    assert_eq!(
        first,
        fs::read(root.join("xtask/kernel-core-baseline.toml")).unwrap()
    );
}

/// The direction is source to pin: an edit to the kernel is followed by the pin, never the
/// reverse, and the stale-pin message names the one command that does it.
#[test]
fn a_kernel_edit_reds_the_gate_naming_bless_and_bless_follows_the_source() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path();
    plant(root, &stale_baseline());
    bless_pin(root).unwrap();

    fs::write(
        root.join("crates/maos-kernel-core/src/lib.rs"),
        "mod sub;\nfn a() {}\nfn extra() {}\n",
    )
    .unwrap();
    let report = check(root);
    assert!(!report.passed);
    assert_eq!(report.changed, vec!["lib.rs".to_string()]);
    assert!(
        failure_detail(&report).contains("cargo run -p xtask -- bless"),
        "{}",
        failure_detail(&report)
    );

    assert!(bless_pin(root).unwrap());
    let report = check(root);
    assert!(report.passed, "{}", failure_detail(&report));
    assert_eq!(report.pinned_lines, 4);
}

#[test]
fn bless_refuses_a_baseline_it_cannot_rewrite_and_writes_nothing() {
    for (label, baseline) in [
        ("no header", PRELUDE.to_string()),
        (
            "two src_lines assignments",
            format!("src_lines = 1\n{PRELUDE}[kernel_src]\n"),
        ),
        ("no src_lines", "[kernel_src]\n".to_string()),
    ] {
        let tree = tempfile::tempdir().unwrap();
        let root = tree.path();
        plant(root, &baseline);
        assert!(bless_pin(root).is_err(), "{label}");
        assert_eq!(
            fs::read_to_string(root.join("xtask/kernel-core-baseline.toml")).unwrap(),
            baseline,
            "{label}: a refused bless must leave the file alone"
        );
    }
}
