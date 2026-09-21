//! Story 13.5b — maosctl legal-hold list/release operator surface.
//!
//! ⚠ Story 16-1 re-pointed the `list` half. `legal-hold list` used to spawn
//! `MAOS_ONE_SHOT=legal-hold-list`, a mode that no longer exists: it is one of
//! the two durable readers that now run IN PROCESS against a read-only sqlite
//! connection (D-16-1-A), because a reader has nothing to gain from a daemon
//! and a write-capable open can hold the TL lock past the daemon's
//! `busy_timeout=5000` and trip its panic-on-write-error (Trap 9).
//!
//! So the `list` half no longer asserts "maosctl forwarded an env var to a
//! fake script". It seeds a REAL `legal_holds` table with plain SQL and
//! asserts that `maosctl` reports those rows — and that it spawned NOTHING.
//! That is the E15-A6 question answered the right way round: the test reads
//! the tree through the production read path instead of reading back the
//! answer a fixture handed it.
//!
//! The `release` half still proves the OFFLINE arm end-to-end, because
//! erasure and legal holds must work with no daemon and no `maos init`
//! (FR45/FR65 — AC5's "no endpoint configured ⇒ offline child directly").

#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use tempfile::TempDir;

    /// A stand-in for the `maos` binary that records the one-shot mode it was
    /// asked for. Its whole job is to make a SPAWN observable — including the
    /// spawn that must no longer happen.
    fn fake_maos(dir: &TempDir) -> (PathBuf, PathBuf) {
        let script = dir.path().join("fake-maos");
        let capture = dir.path().join("capture.txt");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s|%s\\n' \"$MAOS_ONE_SHOT\" \"$MAOS_LEGAL_HOLD_PRINCIPAL\" >> \"$MAOS_CAPTURE\"\nprintf '{\"released\":true}\\n'\n",
        )
        .expect("write fake maos");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake maos");
        (script, capture)
    }

    fn maosctl() -> &'static str {
        env!("CARGO_BIN_EXE_maosctl")
    }

    /// A command with an EMPTY scratch home, so no `control.json` is found and
    /// no operator door is configured.
    ///
    /// Story 16-1 / D-16-1-Q and AC5: with no endpoint configured a durable
    /// verb goes OFFLINE directly rather than reporting a missing daemon —
    /// GDPR erasure may not require `maos init`. Inheriting the developer's
    /// `HOME` would instead find a real `control.json` and try the door.
    fn isolated(home: &Path, audit_db: &Path) -> Command {
        let mut command = Command::new(maosctl());
        command
            .env("HOME", home)
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home.join("xdg"))
            .env("MAOS_AUDIT_DB", audit_db);
        command
    }

    /// Seed the `legal_holds` table the production reader reads.
    ///
    /// Plain SQL on purpose: the schema IS the contract between the daemon's
    /// write path and `maos_audit::list_legal_holds_readonly`, so writing it
    /// by hand here means the test would notice a column rename instead of
    /// following it.
    fn seed_holds(db: &Path, rows: &[(&str, &str, Option<&str>, i64)]) {
        std::fs::create_dir_all(db.parent().expect("db has a parent")).expect("create db parent");
        let conn = rusqlite::Connection::open(db).expect("open seed db");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS legal_holds (
                 principal_id TEXT PRIMARY KEY,
                 reason TEXT NOT NULL,
                 case_ref TEXT,
                 requested_at_ns INTEGER NOT NULL
             )",
            [],
        )
        .expect("create legal_holds");
        for (principal, reason, case_ref, at) in rows {
            conn.execute(
                "INSERT INTO legal_holds (principal_id, reason, case_ref, requested_at_ns)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![principal, reason, case_ref, at],
            )
            .expect("insert hold");
        }
    }

    #[test]
    fn list_reads_the_real_holds_table_in_process_and_spawns_nothing() {
        let dir = TempDir::new().expect("tempdir");
        let home = dir.path().join("home");
        let audit_db = home.join("audit").join("transparency.sqlite");
        let (fake, capture) = fake_maos(&dir);
        seed_holds(
            &audit_db,
            &[
                ("held@example.org", "litigation", Some("CASE-7"), 200),
                ("earlier@example.org", "regulator", None, 100),
            ],
        );

        let list = isolated(&home, &audit_db)
            .args(["legal-hold", "list"])
            // Pointed at the fake ON PURPOSE: if maosctl still spawned a
            // child, the capture file below would exist.
            .env("MAOS_BIN_PATH", &fake)
            .env("MAOS_CAPTURE", &capture)
            .output()
            .expect("run legal-hold list");
        assert!(
            list.status.success(),
            "list must succeed against a seeded log: {}",
            String::from_utf8_lossy(&list.stderr)
        );
        let rows: serde_json::Value =
            serde_json::from_slice(&list.stdout).expect("list prints JSON");
        let rows = rows.as_array().expect("a JSON array of holds");
        assert_eq!(rows.len(), 2, "both seeded holds are reported: {rows:?}");
        // Ordered by `requested_at_ns`, which is the adapter's own order — a
        // reader that reported them in insertion order would disagree with the
        // daemon about which hold is oldest.
        assert_eq!(rows[0]["principal_id"], "earlier@example.org");
        assert_eq!(rows[0]["case_ref"], serde_json::Value::Null);
        assert_eq!(rows[1]["principal_id"], "held@example.org");
        assert_eq!(rows[1]["reason"], "litigation");
        assert_eq!(rows[1]["case_ref"], "CASE-7");
        assert!(
            !capture.exists(),
            "the durable reader must run IN PROCESS: a child was spawned, capture = {:?}",
            std::fs::read_to_string(&capture).unwrap_or_default()
        );
    }

    /// The falsifier for the test above: an EMPTY table must read as an empty
    /// list, not as the seeded one and not as an error. Without this, a reader
    /// that ignored its argument and printed a constant would pass.
    #[test]
    fn list_reports_an_empty_list_when_no_hold_is_held() {
        let dir = TempDir::new().expect("tempdir");
        let home = dir.path().join("home");
        let audit_db = home.join("audit").join("transparency.sqlite");
        seed_holds(&audit_db, &[]);

        let list = isolated(&home, &audit_db)
            .args(["legal-hold", "list"])
            .output()
            .expect("run legal-hold list");
        assert!(
            list.status.success(),
            "an empty holds table is not an error: {}",
            String::from_utf8_lossy(&list.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&list.stdout).trim(), "[]");
    }

    #[test]
    fn release_forwards_the_offline_one_shot_contract() {
        let dir = TempDir::new().expect("tempdir");
        let home = dir.path().join("home");
        let audit_db = home.join("audit").join("transparency.sqlite");
        std::fs::create_dir_all(&home).expect("create home");
        let (fake, capture) = fake_maos(&dir);

        let release = isolated(&home, &audit_db)
            .args(["legal-hold", "release", "--principal", "held@example.org"])
            .env("MAOS_BIN_PATH", &fake)
            .env("MAOS_CAPTURE", &capture)
            .output()
            .expect("run legal-hold release");
        assert!(
            release.status.success(),
            "release must reach the offline child: {}",
            String::from_utf8_lossy(&release.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&release.stdout).trim(),
            "{\"released\":true}",
            "maosctl prints the child's terminal report verbatim"
        );
        // ⚠ An EMAIL-shaped principal, which is what every legal hold in this
        // tree actually looks like. Story 16-1 first routed this verb as
        // `POST /v1/legal-holds/{principal}/release`, and `@` is outside the
        // door's path-segment charset — so the principal now travels in the
        // body, and this assertion is what keeps the offline half honest too.
        assert_eq!(
            std::fs::read_to_string(&capture).expect("read capture"),
            "legal-hold-release|held@example.org\n",
            "exactly one spawn, of the surviving legal-hold-release mode"
        );
    }
}
