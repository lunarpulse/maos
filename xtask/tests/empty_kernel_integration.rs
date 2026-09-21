use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "xtask", "--"]);
    cmd
}

#[test]
fn violation_i9_fails() {
    let output = xtask()
        .args([
            "check-empty-kernel",
            "--path",
            "xtask/tests/fixtures/violation-i9",
            "--whitelist",
            "xtask/i9-whitelist.toml",
            "--denylist",
            "xtask/i9-denylist.toml",
            "--exemptions",
            "docs/invariants/i9-exemptions.md",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected failure, got success. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("I9 violation: persistent struct"),
        "expected I9 violation in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("HungryCache"),
        "expected 'HungryCache' in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("violation-i9"),
        "expected fixture path in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_i9_passes() {
    let output = xtask()
        .args([
            "check-empty-kernel",
            "--path",
            "xtask/tests/fixtures/clean-i9",
            "--whitelist",
            "xtask/i9-whitelist.toml",
            "--denylist",
            "xtask/i9-denylist.toml",
            "--exemptions",
            "docs/invariants/i9-exemptions.md",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success, got failure. stderr:\n{stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PASSED"),
        "expected PASSED in stdout, got:\n{stdout}"
    );
}

/// Story 15-1 AC2 residual control (§10) — the RED half. No test anywhere
/// exercised "an `#[i9_exempt]` site is undocumented" through the real gate:
/// `missing_exemption_doc_fires` reimplements the matcher inline (already
/// diverged), and `clean_i9_passes` has no exempt struct at all. This fixture
/// carries an exempt struct whose name is absent from the shipped register, so
/// the gate must fail on exactly that leg.
#[test]
fn exempt_but_undocumented_struct_reds() {
    let output = xtask()
        .args([
            "check-empty-kernel",
            "--path",
            "xtask/tests/fixtures/exempt-undocumented",
            "--whitelist",
            "xtask/i9-whitelist.toml",
            "--denylist",
            "xtask/i9-denylist.toml",
            "--exemptions",
            "docs/invariants/i9-exemptions.md",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected failure for an undocumented #[i9_exempt] site, got success"
    );
    assert!(
        stderr.contains("#[i9_exempt]"),
        "expected the undocumented-exemption violation, got:\n{stderr}"
    );
    assert!(
        stderr.contains("not documented in docs/invariants/i9-exemptions.md"),
        "expected the register cross-check message, got:\n{stderr}"
    );
    assert!(
        stderr.contains("exempt-undocumented"),
        "expected the fixture path in the violation, got:\n{stderr}"
    );
}

/// Story 15-1 AC2 residual control (§10) — the GREEN half, keyed to T4. The
/// fixture reuses the name of a real register entry (`VerifiedImageLock`), so
/// passing depends on Story 15-1's register entry existing: revert T4 alone
/// (delete the entry) and this test reds, which is the proven-red direction
/// for the leg T4 closes.
#[test]
fn exempt_and_documented_struct_passes() {
    let output = xtask()
        .args([
            "check-empty-kernel",
            "--path",
            "xtask/tests/fixtures/exempt-documented",
            "--whitelist",
            "xtask/i9-whitelist.toml",
            "--denylist",
            "xtask/i9-denylist.toml",
            "--exemptions",
            "docs/invariants/i9-exemptions.md",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success for a documented #[i9_exempt] site; if this reds, the \
         VerifiedImageLock register entry (Story 15-1 T4) is gone. stderr:\n{stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PASSED"),
        "expected PASSED in stdout, got:\n{stdout}"
    );
}
