#![forbid(unsafe_code)]

//! Story 20-3c AC9 (FR62(b), D-20-3c-05 B) — ABI-extension proposals and each
//! status change are journaled as `GovernanceEvent` frames by an operator path
//! (`maosctl governance abi-extension`), queryable as `--frame-kind governance`,
//! and read back by the ratification loader.
//!
//! At `3341f9e4` there was no production writer: the loader's only input was a
//! log the test wrote itself.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn maos() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maos"))
}

fn maosctl() -> PathBuf {
    let path = maos().with_file_name("maosctl");
    assert!(path.is_file(), "build maosctl first: {}", path.display());
    path
}

fn ctl(base: &Path, args: &[&str]) -> Output {
    Command::new(maosctl())
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", base.join("home"))
        .env("MAOS_HOME", base.join("maos-home"))
        .env("XDG_DATA_HOME", base.join("xdg-data"))
        .env("XDG_CONFIG_HOME", base.join("xdg-config"))
        .env("MAOS_MEMORY_ROOT", base.join("memory"))
        .env("MAOS_ARCHIVE_DIR", base.join("archives"))
        .env("MAOS_CRL_PATH", base.join("crl"))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .env("MAOS_BIN_PATH", maos())
        .output()
        .expect("run maosctl")
}

fn propose(base: &Path, status: &str, at: &str) {
    let out = ctl(
        base,
        &[
            "governance",
            "abi-extension",
            "--proposal-id",
            "20-3c-fr62b-witness",
            "--summary",
            "FR62(b) witness proposal",
            "--adr-ref",
            "ADR-045",
            "--status",
            status,
            "--covered-change",
            "IssuanceQuota",
            "--effective-at-ns",
            at,
        ],
    );
    assert!(
        out.status.success(),
        "abi-extension {status}: exit {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn proposals_and_ratifications_are_journaled_queryable_and_loadable() {
    let base = tempfile::tempdir().unwrap();
    for dir in ["home", "xdg-data", "xdg-config"] {
        std::fs::create_dir_all(base.path().join(dir)).unwrap();
    }
    propose(base.path(), "proposed", "10");
    propose(base.path(), "ratified", "20");

    let query = ctl(
        base.path(),
        &[
            "audit",
            "query",
            "--frame-kind",
            "governance",
            "--format",
            "ndjson",
        ],
    );
    assert!(
        query.status.success(),
        "{}",
        String::from_utf8_lossy(&query.stderr)
    );
    let rows: Vec<serde_json::Value> = String::from_utf8_lossy(&query.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let mine: Vec<_> = rows
        .iter()
        .filter(|row| row["intent"] == "governance:abi-extension")
        .collect();
    assert_eq!(mine.len(), 2, "one frame per status: {rows:?}");
    let payload: serde_json::Value =
        serde_json::from_str(mine[1]["payload"].as_str().expect("payload text")).unwrap();
    assert!(payload["recorded_at_ns"].as_u64().is_some_and(|ns| ns > 0));
    assert_eq!(payload["effective_at_ns"], 20);

    let db = base.path().join("maos-home/audit/transparency.sqlite");
    let frames = maos_audit::load_ratification_frames(&db).expect("the loader reads the log");
    assert_eq!(
        frames
            .iter()
            .map(|f| f.proposal_id.as_str())
            .collect::<Vec<_>>(),
        ["20-3c-fr62b-witness"],
        "only the ratified status change loads"
    );
}
