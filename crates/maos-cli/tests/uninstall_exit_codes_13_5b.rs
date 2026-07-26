//! Story 13.5b — maosctl preserves daemon uninstall terminal codes.

#![forbid(unsafe_code)]

#[cfg(unix)]
#[test]
fn uninstall_forwards_erased_held_not_found_and_failed_codes() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let dir = tempfile::TempDir::new().expect("tempdir");
    let fake = dir.path().join("fake-maos");
    std::fs::write(&fake, "#!/bin/sh\nexit \"$FAKE_EXIT_CODE\"\n").expect("write fake maos");
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
        .expect("chmod fake maos");

    for (terminal, expected) in [("erased", 0), ("held", 3), ("not-found", 4), ("failed", 5)] {
        let output = Command::new(env!("CARGO_BIN_EXE_maosctl"))
            .args(["uninstall", "hello-spirit"])
            .env("MAOS_BIN_PATH", &fake)
            .env("FAKE_EXIT_CODE", expected.to_string())
            .output()
            .expect("run maosctl uninstall");
        assert_eq!(
            output.status.code(),
            Some(expected),
            "maosctl must preserve {terminal} child code; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
