#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 20-14 L2 (AC1, AC3) — the real `maos shell` binds the boot's session principal and one task
//! per turn at the Inference Port mint; the real `maosctl audit cost-reconcile` then reports one
//! row per task and principal instead of pooling both sessions into `host-unallocated`.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maosctl() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(path.is_file(), "build maosctl first: {}", path.display());
    path
}

fn home_env(command: &mut Command, home: &Path) {
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"));
}

/// `printf '@hello-spirit hello\n' | MAOS_SESSION_PRINCIPAL=<principal> MAOS_INFERENCE_MODE=replay
/// MAOS_REPLAY_CASSETTE=… maos shell`, in the isolated `home`.
fn shell(home: &Path, principal: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command.arg("shell").current_dir(root());
    home_env(&mut command, home);
    command
        .env("MAOS_INFERENCE_MODE", "replay")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root().join("crates/maos-journey-test/cassettes/j0/shell-intro.json"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(principal) = principal {
        command.env("MAOS_SESSION_PRINCIPAL", principal);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"@hello-spirit hello\n")
        .unwrap();
    child.wait_with_output().unwrap()
}

/// The current UTC month as `YYYY-MM` (civil-from-days; no date dependency).
fn this_month() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        / 86_400;
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}")
}

fn reconcile(home: &Path, maos_home: bool) -> serde_json::Value {
    let mut command = Command::new(maosctl());
    command
        .args(["audit", "cost-reconcile", "--month"])
        .arg(this_month())
        .args(["--format", "ndjson", "--pricing"])
        .arg(root().join("xtask/provider-pricing.toml"))
        .current_dir(root());
    home_env(&mut command, home);
    if maos_home {
        command.env("MAOS_HOME", home);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "cost-reconcile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(output.stdout.split(|b| *b == b'\n').next().unwrap()).unwrap()
}

fn audit_db(home: &Path) -> PathBuf {
    home.join("xdg/maos/audit/transparency.sqlite")
}

#[test]
fn two_sessions_report_one_row_per_task_and_principal_and_a_legacy_frame_stays_readable() {
    let home = tempfile::tempdir().unwrap();
    for principal in ["alice", "bob"] {
        let out = shell(home.path(), Some(principal));
        assert!(
            out.status.success(),
            "{principal}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    // A schema-1 frame in the same month (a pre-20-14 writer): no task, proxy attribution.
    let log = TransparencyLogAdapter::open(&audit_db(home.path()), 0x20_14).unwrap();
    let _ = log.insert_frame_event(
        FrameKind::CostAttribution,
        9,
        None,
        "cost:inference-attribution",
        br#"{"schema_version":1,"timestamp_ns":1,"spirit_pid":9,"provider":"legacy","model":"seed","principal":{"kind":"Unattributed"},"attribution_source":"write_target_proxy","attribution_confidence":"unknown","dimensions":{"tokens_in":7,"tokens_out":3}}"#,
        FrameOrigin::Kernel,
    );
    drop(log);
    let report = reconcile(home.path(), false);
    let rows = report["rows"].as_array().unwrap();
    let live: Vec<_> = rows.iter().filter(|r| r["provider"] == "replay").collect();
    assert_eq!(live.len(), 2, "{report}");
    let mut principals: Vec<_> = live
        .iter()
        .map(|r| r["principal"].as_str().unwrap())
        .collect();
    principals.sort_unstable();
    assert_eq!(principals, ["alice", "bob"]);
    for row in &live {
        assert_eq!(row["attribution_source"], "session_principal");
        assert_eq!(row["spirit"], "hello-spirit");
        assert_eq!(row["model"], "hand-authored-seed");
        assert_eq!(
            (row["tokens_in"].as_i64(), row["tokens_out"].as_i64()),
            (Some(50), Some(30))
        );
        assert!(
            row["task"].as_str().unwrap().starts_with("shell.turn:"),
            "{row}"
        );
    }
    assert_ne!(live[0]["task"], live[1]["task"], "one task per shell turn");
    assert!(
        rows.iter()
            .all(|r| r["principal"] != "host-unallocated" || r["provider"] == "legacy"),
        "no live row falls into host-unallocated: {report}"
    );
    let legacy = rows.iter().find(|r| r["provider"] == "legacy").unwrap();
    assert_eq!(legacy["task"], "none (schema 1)");
    assert_eq!(legacy["attribution_source"], "write_target_proxy");
}

#[test]
fn without_a_session_principal_the_proxy_attributes_and_a_blank_value_binds_none() {
    for blank in [None, Some(""), Some("   ")] {
        let home = tempfile::tempdir().unwrap();
        let out = shell(home.path(), blank);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report = reconcile(home.path(), false);
        let [row] = report["rows"].as_array().unwrap().as_slice() else {
            panic!("one row: {report}")
        };
        assert_eq!(row["attribution_source"], "write_target_proxy", "{blank:?}");
        assert_eq!(row["principal"], "host-unallocated", "{blank:?}");
        assert!(row["task"].as_str().unwrap().starts_with("shell.turn:"));
    }
}

#[test]
fn a_malformed_session_principal_refuses_the_boot() {
    for bad in ["alice:admin", "ali\u{1}ce", "ali\nce"] {
        let home = tempfile::tempdir().unwrap();
        let out = shell(home.path(), Some(bad));
        assert!(!out.status.success(), "{bad:?} must refuse the boot");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("MAOS_SESSION_PRINCIPAL"), "{stderr}");
    }
}

#[test]
fn the_boot_parse_follows_the_butler_principal_rule() {
    use maos_bin::shell_host::parse_session_principal as parse;
    use std::ffi::OsString;
    assert_eq!(parse(None), Ok(None));
    assert_eq!(parse(Some(OsString::from(" \t"))), Ok(None));
    assert_eq!(
        parse(Some(OsString::from("alice"))),
        Ok(Some("alice".into()))
    );
    assert!(parse(Some(OsString::from("a:b"))).is_err());
    assert!(parse(Some(OsString::from("a\u{7}b"))).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        assert!(parse(Some(OsString::from_vec(vec![0xff, 0xfe]))).is_err());
    }
}

#[test]
fn a_butler_seam_pass_binds_its_own_task_and_the_boot_principal() {
    let home = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .args(["run", "spirits/butler/manifest.toml", "--once"])
        .current_dir(root());
    home_env(&mut command, home.path());
    let output = command
        .env("MAOS_HOME", home.path())
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root().join("crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json"),
        )
        .env("MAOS_SESSION_PRINCIPAL", "carol")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = reconcile(home.path(), true);
    let rows = report["rows"].as_array().unwrap();
    assert!(
        !rows.is_empty(),
        "the Butler pass journals a cost frame: {report}"
    );
    for row in rows {
        assert_eq!(row["principal"], "carol", "{row}");
        assert_eq!(row["attribution_source"], "session_principal", "{row}");
        assert!(row["task"].as_str().unwrap().contains(".pass:"), "{row}");
    }
}

#[test]
fn the_one_shot_hello_run_binds_its_own_task_and_the_boot_principal() {
    let home = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command.current_dir(root());
    home_env(&mut command, home.path());
    let output = command
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .env("MAOS_INFERENCE_MODE", "replay")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root().join("crates/maos-journey-test/cassettes/j0/shell-intro.json"),
        )
        .env("MAOS_SESSION_PRINCIPAL", "dave")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = reconcile(home.path(), false);
    let [row] = report["rows"].as_array().unwrap().as_slice() else {
        panic!("one row: {report}")
    };
    assert_eq!(row["principal"], "dave");
    assert_eq!(row["attribution_source"], "session_principal");
    assert!(
        row["task"]
            .as_str()
            .unwrap()
            .starts_with("hello-spirit.run:"),
        "{row}"
    );
}
