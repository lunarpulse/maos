//! Story 13.5b — maosctl legal-hold list/release operator surface.

#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    use tempfile::TempDir;

    fn fake_maos(dir: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
        let script = dir.path().join("fake-maos");
        let capture = dir.path().join("capture.txt");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s|%s\\n' \"$MAOS_ONE_SHOT\" \"$MAOS_LEGAL_HOLD_PRINCIPAL\" >> \"$MAOS_CAPTURE\"\nif [ \"$MAOS_ONE_SHOT\" = legal-hold-list ]; then printf '[]\\n'; else printf '{\"released\":true}\\n'; fi\n",
        )
        .expect("write fake maos");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake maos");
        (script, capture)
    }

    fn maosctl() -> &'static str {
        env!("CARGO_BIN_EXE_maosctl")
    }

    #[test]
    fn list_and_release_forward_one_shot_contract() {
        let dir = TempDir::new().expect("tempdir");
        let (fake, capture) = fake_maos(&dir);

        let list = Command::new(maosctl())
            .args(["legal-hold", "list"])
            .env("MAOS_BIN_PATH", &fake)
            .env("MAOS_CAPTURE", &capture)
            .output()
            .expect("run legal-hold list");
        assert!(
            list.status.success(),
            "list must parse and forward: {}",
            String::from_utf8_lossy(&list.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&list.stdout).trim(), "[]");

        let release = Command::new(maosctl())
            .args(["legal-hold", "release", "--principal", "held@example.org"])
            .env("MAOS_BIN_PATH", &fake)
            .env("MAOS_CAPTURE", &capture)
            .output()
            .expect("run legal-hold release");
        assert!(
            release.status.success(),
            "release must parse and forward: {}",
            String::from_utf8_lossy(&release.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&release.stdout).trim(),
            "{\"released\":true}"
        );

        assert_eq!(
            std::fs::read_to_string(capture).expect("read capture"),
            "legal-hold-list|\nlegal-hold-release|held@example.org\n"
        );
    }
}
