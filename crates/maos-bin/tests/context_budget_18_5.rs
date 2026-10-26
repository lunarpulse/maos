#![forbid(unsafe_code)]
//! ADR-016 process witnesses: strict scene replay and the serving door binder.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/context-budget-18-5.json";
const MANIFEST: &str = "spirits/butler/manifest.toml";
const STEADY: &str = "tests/fixtures/j-butler/acceptance-21d-steady.jsonl";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn command(binary: impl AsRef<Path>, home: &Path) -> Command {
    let mut cmd = Command::new(binary.as_ref());
    cmd.current_dir(root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"));
    cmd
}
fn maos(home: &Path) -> Command {
    command(env!("CARGO_BIN_EXE_maos"), home)
}
fn scene(home: &Path, cassette: &Path, clock: &str) -> Command {
    let mut cmd = maos(home);
    cmd.args(["run", MANIFEST, "--clock", clock, "--acceptance", STEADY])
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", cassette)
        .env("MAOS_NOTIFY_DISABLE", "1");
    cmd
}
fn events(bytes: &[u8]) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}
fn days(events: &[serde_json::Value]) -> Vec<&serde_json::Value> {
    events
        .iter()
        .filter(|e| e["event"] == "acceptance_day" || e["event"] == "halt")
        .collect()
}
fn seed() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(root().join(CASSETTE)).unwrap()).unwrap()
}
fn context_rows(home: &Path) -> Vec<(i64, u32, String, maos_domain::frame::FramePayload)> {
    let conn = rusqlite::Connection::open_with_flags(
        home.join("audit/transparency.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut stmt = conn.prepare("SELECT kind, spirit_pid, intent, payload_redacted FROM transparency_log WHERE kind IN (32, 33) ORDER BY rowid").unwrap();
    let rows = stmt
        .query_map([], |row| {
            let payload: Vec<u8> = row.get(3)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                serde_json::from_slice(&payload).expect("typed ContextEnvelope"),
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    rows
}
fn assert_row(
    row: &(i64, u32, String, maos_domain::frame::FramePayload),
    kind: i64,
    pid: u32,
    used: u32,
) {
    use maos_domain::frame::FramePayload;
    assert_eq!(row.0, kind);
    assert_eq!(row.1, pid);
    let (envelope, threshold) = match &row.3 {
        FramePayload::ContextPressure(e) => (e, 800),
        FramePayload::ContextLimit(e) => (e, 950),
        other => panic!("not a context envelope: {other:?}"),
    };
    assert_eq!(
        row.2, "standard",
        "typed bus delivery uses the frame intent"
    );
    assert_eq!(envelope.spirit_pid, pid);
    assert_eq!(envelope.context_used, used);
    assert_eq!(envelope.context_window_size, 8192);
    assert_eq!(envelope.threshold_permille, threshold);
    assert_eq!(kind, if threshold == 800 { 32 } else { 33 });
}
#[test]
fn context_cassette_pins_real_scene_prompts_and_absolute_usage() {
    use sha2::{Digest, Sha256};
    let event = |id: &str, title: &str, start, end| butler::CalendarEvent {
        id: id.into(),
        title: title.into(),
        start_min: start,
        end_min: end,
        status: butler::EventStatus::Confirmed,
    };
    let scenario = butler::ScenarioInput {
        calendar: vec![
            event("evt-a", "Board review", 540, 600),
            event("evt-b", "Investor call", 570, 630),
        ],
        ..Default::default()
    };
    let prompt =
        butler::proposal_prompt(&scenario, &butler::Butler::new().assess(&scenario)).unwrap();
    let hash = format!("{:x}", Sha256::digest(prompt.as_bytes()));
    let cassette = seed();
    assert_eq!(cassette["provenance"], "seed");
    let entries = cassette["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    for (i, (entry, total)) in entries.iter().zip([4000, 6600, 7800, 8300]).enumerate() {
        assert_eq!(entry["sequence"], i);
        assert_eq!(entry["prompt_sha256"], hash);
        assert_eq!(entry["prompt_len"], prompt.len());
        let usage = &entry["response"]["usage"];
        assert_eq!(
            usage["input_tokens"].as_u64().unwrap() + usage["output_tokens"].as_u64().unwrap(),
            total
        );
        let proposal = butler::parse_proposal(entry["response"]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            proposal.options,
            butler::DETERMINISTIC_OPTIONS.map(String::from)
        );
    }
}
#[test]
fn four_day_process_reports_context_crossings_and_exact_journal_rows() {
    let home = tempfile::tempdir().unwrap();
    let out = scene(home.path(), &root().join(CASSETTE), "compressed:4d")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = events(&out.stdout);
    let days = days(&events);
    assert_eq!(days.len(), 4, "{events:?}");
    for (i, (used, level, crossed)) in [
        (4000, "within", None),
        (6600, "pressure", Some("pressure")),
        (7800, "limit", Some("limit")),
        (8300, "exhausted", Some("exhausted")),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(days[i]["day"], i + 1);
        assert_eq!(
            days[i]["context"],
            serde_json::json!({"used": used, "window": 8192, "level": level, "crossed": crossed})
        );
    }
    let rows = context_rows(home.path());
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_row(&rows[0], 32, rows[0].1, 6600);
    assert_row(&rows[1], 33, rows[0].1, 7800);
    let query = maos(home.path()).args(["audit", "query"]).output().unwrap();
    assert!(
        query.status.success(),
        "{}",
        String::from_utf8_lossy(&query.stderr)
    );
    let listing = String::from_utf8_lossy(&query.stdout);
    assert!(
        listing.contains("context.pressure") && listing.contains("context.limit"),
        "{listing}"
    );
}
fn drain(mut pipe: impl Read + Send + 'static) -> Arc<Mutex<Vec<u8>>> {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&buffer);
    std::thread::spawn(move || {
        let mut chunk = [0; 4096];
        while let Ok(n) = pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            sink.lock().unwrap().extend_from_slice(&chunk[..n]);
        }
    });
    buffer
}
struct Daemon(Child);
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn ctl(home: &Path, args: &[&str]) -> Output {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(binary.is_file(), "build maos-cli before this test");
    command(binary, home).args(args).output().unwrap()
}

/// The real serving door swaps the pid while the scene is held after day 1.
/// Both directions matter: a door reporter crosses pressure; unloading that
/// exhausted incarnation and loading another starts with a Within report.
#[test]
fn door_loaded_butler_reports_for_its_pid_and_reload_starts_within() {
    for (boot_used, door_used, expected_level, expected_crossed) in [
        (4000, 6600, "pressure", Some("pressure")),
        (8300, 4000, "within", None),
    ] {
        let home = tempfile::tempdir().unwrap();
        let init = maos(home.path()).arg("init").output().unwrap();
        assert!(
            init.status.success(),
            "{}",
            String::from_utf8_lossy(&init.stderr)
        );
        let mut cassette = seed();
        let entries = cassette["entries"].as_array_mut().unwrap();
        entries.truncate(2);
        for (entry, total) in entries.iter_mut().zip([boot_used, door_used]) {
            entry["response"]["usage"] =
                serde_json::json!({"input_tokens": total - 80, "output_tokens": 80});
        }
        let cassette_path = home.path().join("door-context.json");
        std::fs::write(&cassette_path, serde_json::to_vec(&cassette).unwrap()).unwrap();
        let hold = home.path().join("scene.hold");
        std::fs::write(&hold, b"").unwrap();
        let mut child = scene(home.path(), &cassette_path, "compressed:2d")
            .env("MAOS_SCENE_HOLD_FILE", &hold)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = drain(child.stdout.take().unwrap());
        let stderr = drain(child.stderr.take().unwrap());
        let mut daemon = Daemon(child);
        let deadline = Instant::now() + Duration::from_secs(45);
        while days(&events(&stdout.lock().unwrap())).is_empty() {
            assert!(
                Instant::now() < deadline,
                "day 1: {}",
                String::from_utf8_lossy(&stderr.lock().unwrap())
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        for args in [
            &["unload", "butler"][..],
            &["load", MANIFEST][..],
            &["start", "butler"][..],
        ] {
            let out = ctl(home.path(), args);
            assert!(
                out.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        std::fs::remove_file(&hold).unwrap();
        loop {
            if let Some(status) = daemon.0.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "{}",
                    String::from_utf8_lossy(&stderr.lock().unwrap())
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "scene did not exit: {}",
                String::from_utf8_lossy(&stderr.lock().unwrap())
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        let ev = events(&stdout.lock().unwrap());
        let reports = days(&ev);
        assert_eq!(reports.len(), 2, "{ev:?}");
        assert_eq!(
            reports[1]["context"],
            serde_json::json!({"used": door_used, "window": 8192, "level": expected_level, "crossed": expected_crossed})
        );
        let restores: Vec<_> = ev
            .iter()
            .filter(|e| e["event"] == "acceptance_ledger_restored")
            .collect();
        assert_eq!(restores.len(), 2, "{ev:?}");
        let boot_pid = restores[0]["pid"].as_u64().unwrap() as u32;
        let door_pid = restores[1]["pid"].as_u64().unwrap() as u32;
        assert_ne!(boot_pid, door_pid);
        let rows = context_rows(home.path());
        if door_used == 6600 {
            assert_eq!(rows.len(), 1, "{rows:?}");
            assert_row(&rows[0], 32, door_pid, 6600);
        } else {
            assert_eq!(rows.len(), 2, "{rows:?}");
            assert_row(&rows[0], 32, boot_pid, 8300);
            assert_row(&rows[1], 33, boot_pid, 8300);
            assert!(rows.iter().all(|row| row.1 != door_pid));
        }
    }
}
