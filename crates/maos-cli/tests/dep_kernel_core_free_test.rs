//! Story 9.1 AC5 — CI assertion: `maos-cli` never depends on `maos-kernel-core`.
//!
//! The charter boundary for Epic 9 requires all audit subcommands to be
//! read-only. A `maos-kernel-core` dependency in the CLI crate's dep tree
//! would violate the kernel-isolation contract. This test enforces the
//! invariant structurally via `cargo tree`.

#[test]
fn maos_cli_dep_tree_excludes_kernel_core() {
    let output = std::process::Command::new("cargo")
        .args(["tree", "-p", "maos-cli", "--prefix", "none"])
        .output()
        .expect("cargo tree must succeed");

    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tree_text = String::from_utf8_lossy(&output.stdout);
    for line in tree_text.lines() {
        assert!(
            !line.contains("maos-kernel-core"),
            "maos-cli MUST NOT depend on maos-kernel-core (Decision B / charter boundary).\n\
             Offending line: {line}"
        );
    }
}
