#![forbid(unsafe_code)]

//! Story 18-3 — the process-level witness (AC1.6, AC4.5, AC5): the real
//! `maos` binary (`env!("CARGO_BIN_EXE_maos")`, no nested cargo) over a
//! scratch `HOME`, a cleared environment and no provider key —
//! `MAOS_OLLAMA_URL` unset, exactly as `epic-18-exit` runs exit line 3.
//!
//! - Exit line 3: the committed strict cassette replays one completion per
//!   seed claim through the kernel adapter, the TL `infer:` frames and the
//!   one `ProviderInfer` issuance name `anthropic` (D-18-3-02 B), the root
//!   prints `researcher_survey`, the seed's contradiction writes an
//!   `epistemic.halt` (D-18-3-09 A) and the distillate is a TL row (AC4.1).
//! - Empty, drifted and short cassettes, the legacy (mode-unset) arm over an
//!   empty cassette and a zero-config `--live` pass exit 1 naming the cause.
//! - Mode unset stays deterministic; an unregistered declared provider
//!   refuses at boot and mints nothing.
//! - A serving root mints per pass and binds a door-loaded Researcher like
//!   the boot arm (D-18-3-03 A, D-18-3-04 A).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

const MANIFEST: &str = "spirits/researcher/manifest.toml";
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-researcher/survey-distill.json";
const BUDGET: Duration = Duration::from_secs(45);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn command(binary: PathBuf, scratch: &Path) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(workspace_root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", scratch)
        .env("MAOS_HOME", scratch.join("home"))
        .env("XDG_DATA_HOME", scratch.join("xdg"));
    cmd
}

fn maos(scratch: &Path) -> Command {
    command(PathBuf::from(env!("CARGO_BIN_EXE_maos")), scratch)
}

/// `maos run <researcher> --once` with `env`.
fn once(scratch: &Path, extra: &[&str], env: &[(&str, &str)]) -> Output {
    maos(scratch)
        .args(["run", MANIFEST, "--once"])
        .args(extra)
        .envs(env.iter().copied())
        .output()
        .expect("run maos")
}

/// Strict explicit replay of `cassette` (exit line 3's environment).
fn replay(scratch: &Path, cassette: &str) -> Output {
    once(
        scratch,
        &[],
        &[
            ("MAOS_INFERENCE_MODE", "replay"),
            ("MAOS_REPLAY_STRICT", "1"),
            ("MAOS_REPLAY_CASSETTE", cassette),
        ],
    )
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn events(out: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Every TL row as (kind, intent, payload), read-only.
fn tl_rows(scratch: &Path) -> Vec<(i64, String, Vec<u8>)> {
    let db = scratch.join("home/audit/transparency.sqlite");
    if !db.exists() {
        return Vec::new();
    }
    let conn = rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open the TL read-only");
    // The daemon creates the table after the file: an early read sees none.
    let Ok(mut statement) =
        conn.prepare("SELECT kind, intent, payload_redacted FROM transparency_log ORDER BY rowid")
    else {
        return Vec::new();
    };
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, Option<Vec<u8>>>(2)?.unwrap_or_default(),
            ))
        })
        .expect("query the TL");
    rows.map(|row| row.expect("TL row")).collect()
}

fn infer_intents(scratch: &Path) -> Vec<String> {
    tl_rows(scratch)
        .into_iter()
        .filter(|(_, intent, _)| intent.starts_with("infer:"))
        .map(|(_, intent, _)| intent)
        .collect()
}

/// `ProviderInfer` issuance audit payloads.
fn provider_infer_issuances(scratch: &Path) -> Vec<String> {
    tl_rows(scratch)
        .into_iter()
        .filter(|(_, intent, _)| intent.starts_with("cap.issue"))
        .map(|(_, _, payload)| String::from_utf8_lossy(&payload).into_owned())
        .filter(|payload| payload.contains("ProviderInfer"))
        .collect()
}

/// A variant of the committed cassette in `dir`.
fn variant(dir: &Path, edit: impl FnOnce(&mut Value)) -> String {
    let mut cassette: Value =
        serde_json::from_str(&std::fs::read_to_string(workspace_root().join(CASSETTE)).unwrap())
            .unwrap();
    edit(&mut cassette);
    let path = dir.join("variant.json");
    std::fs::write(&path, serde_json::to_vec(&cassette).unwrap()).unwrap();
    path.display().to_string()
}

#[test]
fn exit_line_3_replays_one_completion_per_seed_claim_through_the_declared_provider() {
    let scratch = tempfile::tempdir().unwrap();
    let out = replay(scratch.path(), CASSETTE);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(
        err.contains("researcher inference seam wired (provider `anthropic`)"),
        "{err}"
    );
    let survey = events(&out)
        .into_iter()
        .find(|e| e["event"] == "researcher_survey")
        .unwrap_or_else(|| panic!("no researcher_survey event; stderr:\n{err}"));
    for section in [
        "findings",
        "open_questions",
        "confidence_map",
        "bibliography",
    ] {
        assert!(survey.get(section).is_some(), "missing {section}: {survey}");
    }
    let entries = 3;
    assert_eq!(survey["completions_consumed"], entries);
    assert_eq!(survey["findings"].as_array().unwrap().len(), entries);

    // AC5.1 — the request and the one token name `anthropic`.
    let infers = infer_intents(scratch.path());
    assert_eq!(infers.len(), entries, "{infers:?}");
    assert!(
        infers.iter().all(|i| i.starts_with("infer:anthropic->")),
        "{infers:?}"
    );
    let issued = provider_infer_issuances(scratch.path());
    assert_eq!(issued.len(), 1, "one mint for the pass: {issued:?}");
    assert!(issued[0].contains("anthropic"), "{issued:?}");

    // AC4.5 / AC4.1 — the contradiction halts in the kernel; the distillate
    // is written through the I11 writer.
    let rows = tl_rows(scratch.path());
    assert!(
        rows.iter()
            .any(|(kind, intent, _)| *kind == 3 && intent == "methodology_conflict"),
        "no epistemic.halt row for methodology_conflict"
    );
    assert!(
        rows.iter().any(|(kind, _, _)| *kind == 11),
        "no distillate row"
    );
    assert!(events(&out)
        .iter()
        .any(|e| e["event"] == "halt" && e["render"] == "halted on methodology_conflict"));
}

#[test]
fn empty_drifted_and_short_cassettes_exit_1_naming_the_cause() {
    let scratch = tempfile::tempdir().unwrap();
    let cases: [(&str, fn(&mut Value), &str); 3] = [
        (
            "empty",
            |c| c["entries"] = Value::Array(Vec::new()),
            "exhausted",
        ),
        (
            "drifted",
            |c| c["entries"][1]["prompt_sha256"] = "ab".repeat(32).into(),
            "drift",
        ),
        (
            "short",
            |c| {
                c["entries"].as_array_mut().unwrap().pop();
            },
            "exhausted",
        ),
    ];
    for (name, edit, cause) in cases {
        let home = scratch.path().join(name);
        std::fs::create_dir_all(&home).unwrap();
        let cassette = variant(&home, edit);
        let out = replay(&home, &cassette);
        let err = stderr(&out);
        assert_eq!(out.status.code(), Some(1), "{name}: {err}");
        assert!(
            err.contains("researcher on_idle pass refused"),
            "{name}: {err}"
        );
        assert!(err.contains(cause), "{name} must name `{cause}`: {err}");
        assert!(
            !events(&out)
                .iter()
                .any(|e| e["event"] == "researcher_survey"),
            "{name}: a refused pass prints no survey"
        );
    }
}

#[test]
fn the_legacy_cassette_arm_refuses_an_exhausted_cassette_typed() {
    let scratch = tempfile::tempdir().unwrap();
    let cassette = variant(scratch.path(), |c| c["entries"] = Value::Array(Vec::new()));
    let out = once(scratch.path(), &[], &[("MAOS_REPLAY_CASSETTE", &cassette)]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("researcher cassette-replay inference wired"),
        "{err}"
    );
    assert!(err.contains("exhausted"), "{err}");
}

#[test]
fn mode_unset_stays_deterministic_and_touches_no_model() {
    let scratch = tempfile::tempdir().unwrap();
    let out = once(scratch.path(), &[], &[]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("researcher deterministic survey"), "{err}");
    assert!(!events(&out)
        .iter()
        .any(|e| e["event"] == "researcher_survey"));
    assert!(infer_intents(scratch.path()).is_empty());
}

#[test]
fn an_unregistered_declared_provider_refuses_boot_and_mints_nothing() {
    let scratch = tempfile::tempdir().unwrap();
    let out = once(
        scratch.path(),
        &["--live"],
        &[("MAOS_OLLAMA_URL", "http://127.0.0.1:9")],
    );
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains(
            "researcher inference: no inference provider is configured for its declared \
             providers `anthropic`, `openai`"
        ),
        "{err}"
    );
    assert!(provider_infer_issuances(scratch.path()).is_empty());
}

#[test]
fn a_zero_config_live_pass_is_refused_unconfigured() {
    let scratch = tempfile::tempdir().unwrap();
    let out = once(scratch.path(), &["--live"], &[("MAOS_OLLAMA_URL", "skip")]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("researcher inference seam wired (provider `anthropic`)"),
        "{err}"
    );
    assert!(err.contains("researcher on_idle pass refused"), "{err}");
    assert!(
        err.contains("Unconfigured") || err.contains("unconfigured"),
        "{err}"
    );
}

// ── AC5.3 / AC5.4 — the serving root ─────────────────────────────────────

fn maosctl(scratch: &Path, args: &[&str]) -> Output {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli` first",
        path.display()
    );
    command(path, scratch)
        .args(args)
        .output()
        .expect("run maosctl")
}

fn wait_until(what: &str, diagnostics: impl Fn() -> String, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + BUDGET;
    while Instant::now() < deadline {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "timed out after {BUDGET:?} waiting for {what}; stderr:\n{}",
        diagnostics()
    );
}

fn drain(mut pipe: impl Read + Send + 'static) -> Arc<Mutex<String>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&buffer);
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(n) = pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            sink.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    });
    buffer
}

struct Daemon(Option<Child>);

impl Drop for Daemon {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn a_serving_root_mints_per_pass_and_binds_a_door_loaded_researcher() {
    let scratch_dir = tempfile::tempdir().unwrap();
    let scratch = scratch_dir.path();
    std::fs::create_dir_all(scratch.join("home")).unwrap();
    let init = maos(scratch).arg("init").output().expect("maos init");
    assert!(init.status.success(), "{}", stderr(&init));
    // The seed's three completions, twice: the boot pass and the door pass.
    let cassette = variant(scratch, |c| {
        let entries = c["entries"].as_array_mut().unwrap();
        let again = entries.clone();
        entries.extend(again);
    });
    let mut child = maos(scratch)
        .args(["run", MANIFEST])
        .env("MAOS_IDLE_FAST", "1")
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", &cassette)
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the serving root");
    let _stdout = drain(child.stdout.take().unwrap());
    let stderr_buf = drain(child.stderr.take().unwrap());
    let _daemon = Daemon(Some(child));
    let err = || stderr_buf.lock().unwrap().clone();

    wait_until("the boot pass's three inference calls", err, || {
        infer_intents(scratch).len() >= 3
    });
    for args in [
        &["unload", "researcher"][..],
        &["load", MANIFEST][..],
        &["start", "researcher"][..],
    ] {
        let out = maosctl(scratch, args);
        assert!(out.status.success(), "maosctl {args:?}: {}", stderr(&out));
    }
    wait_until("the door-loaded Researcher's inference calls", err, || {
        infer_intents(scratch).len() >= 6
    });
    let infers = infer_intents(scratch);
    assert!(
        infers.iter().all(|i| i.starts_with("infer:anthropic->")),
        "{infers:?}"
    );
    // D-18-3-03 A: one audited issuance per pass (an admission-time token
    // would mint once and expire after 60 s).
    let issued = provider_infer_issuances(scratch);
    assert_eq!(issued.len(), 2, "two passes mint two tokens: {issued:?}");
    assert_eq!(
        err()
            .matches("researcher inference seam wired (provider `anthropic`)")
            .count(),
        2,
        "the boot arm and the door load both bind: {}",
        err()
    );
}
