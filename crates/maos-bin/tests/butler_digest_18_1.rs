#![forbid(unsafe_code)]

//! Story 18-1 AC3 + AC6.1 + AC6.3 (D-18-1-08 A) — the FR17 morning digest on
//! the production path.
//!
//! - Against a seeded Transparency Log, `maos run spirits/butler/manifest.toml
//!   --once` emits `{"event":"morning_digest"}` within 30 s of spawn
//!   (wall-clock; FR17 `functional-requirements.md:51`); every
//!   `source_log_ref` resolves to a real TL frame; the open-halt set equals
//!   the TL's truth set for the window; the four FR17 fields are present.
//! - FR58: on a fresh home with no key and no inference env, `maos init` then
//!   `maos run spirits/butler/manifest.toml --once` exit 0 with a shape-valid
//!   notification and a digest, well inside 5 minutes (NFR-Onb-2).

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

const FR17_BUDGET: Duration = Duration::from_secs(30);
const FR58_BUDGET: Duration = Duration::from_secs(300);
const DAY_NS: u64 = 86_400 * 1_000_000_000;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

fn maos(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(workspace_root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home);
    cmd
}

fn tl_path(home: &Path) -> PathBuf {
    home.join("audit").join("transparency.sqlite")
}

/// Run Butler `--once`, timing spawn → the `morning_digest` stdout line.
/// Returns (exit success, every stdout event, stderr, time to the digest).
fn run_once_timing_digest(home: &Path) -> (bool, Vec<serde_json::Value>, String, Option<Duration>) {
    let spawned = Instant::now();
    let mut child = maos(home)
        .args(["run", "spirits/butler/manifest.toml", "--once"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn maos run");
    let mut stderr_pipe = child.stderr.take().unwrap();
    let stderr_reader = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr_pipe.read_to_string(&mut text);
        text
    });
    let mut events = Vec::new();
    let mut digest_at = None;
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        let Ok(line) = line else { break };
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
            if event["event"] == "morning_digest" && digest_at.is_none() {
                digest_at = Some(spawned.elapsed());
            }
            events.push(event);
        }
    }
    let status = child.wait().expect("wait maos run");
    (
        status.success(),
        events,
        stderr_reader.join().unwrap(),
        digest_at,
    )
}

/// Every TL row as (frame id hex, kind label, timestamp ns).
fn tl_rows(home: &Path) -> Vec<(String, String, u64)> {
    maos_audit::query(&tl_path(home), maos_audit::AuditFilter::default())
        .expect("query the daemon's TL")
        .into_iter()
        .map(|row| (row.frame_id_hex, row.kind, row.timestamp_ns))
        .collect()
}

fn refs(items: &serde_json::Value) -> BTreeSet<String> {
    items
        .as_array()
        .expect("an array")
        .iter()
        .map(|item| {
            item["source_log_ref"]
                .as_str()
                .expect("source_log_ref")
                .to_owned()
        })
        .collect()
}

#[test]
fn the_production_digest_meets_fr17_against_a_seeded_tl() {
    let home = tempfile::tempdir().unwrap();
    let started_ns = now_ns();
    // Seed the daemon's own Transparency Log before boot.
    std::fs::create_dir_all(home.path().join("audit")).unwrap();
    let (seeded_completions, seeded_halts) = {
        let tl = TransparencyLogAdapter::open(&tl_path(home.path()), 0x18_01).unwrap();
        let mut completions = BTreeSet::new();
        let mut halts = BTreeSet::new();
        for i in 0..3 {
            let _ = tl.insert_frame_event(
                FrameKind::TaskComplete,
                1,
                None,
                &format!("seeded task {i}"),
                b"done",
                FrameOrigin::SpiritAuto,
            );
            completions.insert(hex(&tl.last_frame_id()));
            if i % 2 == 0 {
                let _ = tl.insert_frame_event(
                    FrameKind::EpistemicHalt,
                    1,
                    None,
                    &format!("seeded halt {i}"),
                    b"halt",
                    FrameOrigin::SpiritAuto,
                );
                halts.insert(hex(&tl.last_frame_id()));
            }
        }
        (completions, halts)
    };

    let (ok, events, stderr, digest_at) = run_once_timing_digest(home.path());
    let finished_ns = now_ns();
    assert!(ok, "maos run butler --once must exit 0; stderr:\n{stderr}");
    let digest_at = digest_at
        .unwrap_or_else(|| panic!("no morning_digest event on stdout (AC3); stderr:\n{stderr}"));
    assert!(
        digest_at <= FR17_BUDGET,
        "FR17: the digest must render within {FR17_BUDGET:?} of spawn, took {digest_at:?}"
    );

    let digest = &events
        .iter()
        .find(|e| e["event"] == "morning_digest")
        .unwrap()["digest"];
    for field in ["completed", "open_halts", "anomalies", "trust_bar"] {
        assert!(
            digest.get(field).is_some(),
            "FR17 field `{field}` missing: {digest}"
        );
    }

    let rows = tl_rows(home.path());
    let real: BTreeSet<&str> = rows.iter().map(|(id, _, _)| id.as_str()).collect();
    let completed = refs(&digest["completed"]);
    let open_halts = refs(&digest["open_halts"]);
    for cited in completed.iter().chain(&open_halts) {
        assert!(
            real.contains(cited.as_str()),
            "hallucinated source_log_ref {cited}: no such TL frame"
        );
    }
    // The truth set, recomputed from the TL: every halt frame in the 24 h
    // window ending at the render instant (frames the `--once` unload writes
    // after the render are outside it). The render instant must itself be
    // this run's wall clock.
    let rendered_ns = events
        .iter()
        .find(|e| e["event"] == "morning_digest")
        .unwrap()["now_ns"]
        .as_u64()
        .expect("now_ns");
    assert!(
        (started_ns..=finished_ns).contains(&rendered_ns),
        "the digest window must end at the render instant"
    );
    let truth: BTreeSet<String> = rows
        .iter()
        .filter(|(_, kind, ts)| {
            kind == "epistemic.halt" && *ts + DAY_NS >= rendered_ns && *ts <= rendered_ns
        })
        .map(|(id, _, _)| id.clone())
        .collect();
    assert!(seeded_halts.is_subset(&truth));
    assert_eq!(
        open_halts, truth,
        "the digest's open halts must equal the TL's truth set for the window"
    );
    assert!(
        seeded_completions.is_subset(&completed),
        "every seeded completion is cited: {seeded_completions:?} vs {completed:?}"
    );

    let intents: Vec<(String, String)> =
        maos_audit::query(&tl_path(home.path()), maos_audit::AuditFilter::default())
            .expect("query the daemon's TL")
            .into_iter()
            .map(|row| (row.kind, row.intent))
            .collect();
    // Review AA-2 (D-18-1-08 A: "stdout plus a TL frame").
    assert_eq!(
        intents
            .iter()
            .filter(|(kind, intent)| kind == "telemetry.event"
                && intent == butler::MORNING_DIGEST_INTENT)
            .count(),
        1,
        "the rendered digest is one `{}` TL frame: {intents:?}",
        butler::MORNING_DIGEST_INTENT
    );
    // Review ruling D-18-1-R3: the pass's scalar write is recorded as a
    // predicate evaluation (FR17 (d)'s denominator); (c) is an explicit unknown.
    let evaluated = format!("{}belief_variance", butler::INTENT_PREDICATE_EVALUATED);
    assert!(
        intents
            .iter()
            .any(|(kind, intent)| kind == "telemetry.event" && *intent == evaluated),
        "the scalar write must leave an `{evaluated}` frame: {intents:?}"
    );
    assert_eq!(
        digest["anomalies"],
        serde_json::json!(butler::ANOMALY_FEED_UNKNOWN)
    );
    assert_eq!(
        digest["trust_bar_basis"]["day_utc"],
        serde_json::json!(rendered_ns / DAY_NS - 1),
        "(d) covers yesterday's UTC day"
    );
}

#[test]
fn fr58_fresh_home_init_then_run_is_zero_config() {
    let home = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let init = maos(home.path()).arg("init").output().expect("maos init");
    assert!(
        init.status.success(),
        "maos init must exit 0 on a fresh home: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    let (ok, events, stderr, digest_at) = run_once_timing_digest(home.path());
    assert!(
        ok,
        "maos run butler --once must exit 0 with no key and no inference env; stderr:\n{stderr}"
    );
    assert!(digest_at.is_some(), "a morning digest is rendered");
    let notification = &events
        .iter()
        .find(|e| e["event"] == "butler_notification")
        .expect("a Butler notification")["notification"];
    for field in ["pattern", "confidence", "evidence", "options"] {
        assert!(
            notification.get(field).is_some(),
            "shape-valid notification lacks `{field}`"
        );
    }
    assert!(
        started.elapsed() <= FR58_BUDGET,
        "FR58 / NFR-Onb-2: within 5 minutes, took {:?}",
        started.elapsed()
    );
}
