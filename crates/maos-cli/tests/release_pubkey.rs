//! Story 15-4 AC4 — the shipped CLI exposes its embedded release verifier key.

use std::process::Command;

#[test]
fn release_pubkey_prints_exact_lowercase_embedded_key() {
    let output = Command::new(env!("CARGO_BIN_EXE_maosctl"))
        .arg("release-pubkey")
        .output()
        .expect("run maosctl release-pubkey");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let expected = hex::encode(maos_audit::release_verify::RELEASE_PUBKEY);
    assert_eq!(stdout.trim_end(), expected);
    assert_eq!(stdout.trim_end().len(), 64);
    assert!(stdout
        .trim_end()
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
}
