//! Story 18-4 — exit line 4: the compressed Butler scene, at process level
//! (`env!("CARGO_BIN_EXE_maos")`, a fresh `HOME`, keyless, no nested cargo).
//!
//! - AC1 (D-18-4-01 A, CR-2/CR-3): 21 simulated days replay the acceptance
//!   fixture; the ledger-informed `belief_variance` halt fires on day 14 while
//!   the pooled rate is still ≥ 0.6 (upstream); the steady twin runs 21 days
//!   with no halt; no stdout value is a wall-clock instant.
//! - AC2.3/2.4 (D-18-4-06 A): exactly one `morning_digest` per simulated
//!   day, on the scenario clock, days strictly increasing; one
//!   `butler:acceptance` TL attestation per decided offer, non-personal.
//! - AC3.2 (D-18-4-02 A): the fixture split at day 10 across two processes on
//!   one home halts on the same day; the principal rows are listed by
//!   `subject_access_query`; after `maosctl forget --principal` (the offline
//!   one-shot) a restart restores 0 records.
//! - AC3.3: the root's principal port is fused to its pid; an unauthorized
//!   pid's write is refused `PrincipalWriteUnauthorized`.
//! - CR-2 / D-18-4-06 A: every scene refusal and a malformed
//!   `MAOS_UTC_OFFSET` are typed, non-zero exits.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use butler::AcceptanceFixture;
use maos_spirit_sdk::principal::PrincipalMemoryPort;

const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";
const FIXTURE: &str = "tests/fixtures/j-butler/acceptance-21d.jsonl";
const STEADY: &str = "tests/fixtures/j-butler/acceptance-21d-steady.jsonl";
const PRINCIPAL: &str = "director-ada";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn maos(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(workspace_root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"));
    cmd
}

/// Exit line 4's command, with `fixture` and `clock`.
fn scene(home: &Path, fixture: &Path, clock: &str) -> Output {
    maos(home)
        .args([
            "run",
            "spirits/butler/manifest.toml",
            "--clock",
            clock,
            "--acceptance",
        ])
        .arg(fixture)
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", CASSETTE)
        .output()
        .expect("spawn maos run")
}

fn events(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn named<'a>(events: &'a [serde_json::Value], name: &str) -> Vec<&'a serde_json::Value> {
    events.iter().filter(|e| e["event"] == name).collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn fixture() -> AcceptanceFixture {
    AcceptanceFixture::parse(&std::fs::read_to_string(workspace_root().join(FIXTURE)).unwrap())
        .unwrap()
}

/// Every integer in `value` at or above 10^15 (any epoch-ns instant).
fn instants(value: &serde_json::Value, out: &mut Vec<u64>) {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(n) = n.as_u64().filter(|n| *n >= 1_000_000_000_000_000) {
                out.push(n);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| instants(v, out)),
        serde_json::Value::Object(map) => map.values().for_each(|v| instants(v, out)),
        _ => {}
    }
}

fn attestations(home: &Path) -> Vec<serde_json::Value> {
    maos_audit::query(
        &home.join("audit/transparency.sqlite"),
        maos_audit::AuditFilter::default(),
    )
    .expect("query the scene's TL")
    .into_iter()
    .filter(|e| e.intent == butler::ACCEPTANCE_INTENT)
    .map(|e| serde_json::from_str(&e.payload).expect("attestation payload is JSON"))
    .collect()
}

fn halt_day(events: &[serde_json::Value]) -> Option<i64> {
    named(events, "scene_complete")
        .first()
        .and_then(|e| e["halt_day"].as_i64())
}

/// CR-3 — `on-idle-halt.json` carries one entry per simulated day (21), each
/// with the real hash of the prompt Butler sends for the root's seeded
/// scenario under the D-18-4-03 prompt (no wildcard), `provenance: seed`.
#[test]
fn the_seed_cassette_pins_21_real_hash_entries() {
    use sha2::{Digest, Sha256};
    let cassette: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(workspace_root().join(CASSETTE)).unwrap())
            .unwrap();
    assert_eq!(cassette["provenance"], "seed");
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
    let entries = cassette["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 21, "one completion per simulated day");
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry["sequence"], i);
        assert_eq!(entry["prompt_sha256"], hash.as_str(), "entry {i}");
        assert_eq!(entry["prompt_len"], prompt.len(), "entry {i}");
        let completion = butler::parse_proposal(entry["response"]["text"].as_str().unwrap())
            .expect("each completion is a valid proposal with declared actions");
        assert_eq!(
            completion.options,
            butler::DETERMINISTIC_OPTIONS.map(String::from)
        );
    }
}

/// AC1, AC2.3, AC2.4, AC4.2 (scene half) — exit line 4 verbatim.
#[test]
fn exit_line_4_halts_upstream_on_the_variance_led_rule() {
    let home = tempfile::tempdir().unwrap();
    let output = scene(
        home.path(),
        &workspace_root().join(FIXTURE),
        "compressed:21d",
    );
    assert!(
        output.status.success(),
        "exit line 4 exits 0; stderr:\n{}",
        stderr(&output)
    );
    let events = events(&output);

    let days: Vec<i64> = named(&events, "acceptance_day")
        .iter()
        .map(|e| e["day"].as_i64().unwrap())
        .collect();
    assert_eq!(
        days,
        (1..=13).collect::<Vec<_>>(),
        "one decided day per simulated day"
    );
    let halts = named(&events, "halt");
    assert_eq!(halts.len(), 1, "{events:?}");
    let halt = halts[0];
    assert_eq!(halt["render"], butler::halt_screen_line("belief_variance"));
    assert_eq!(halt["day"], 14, "the variance-led halt fires on day 14");
    assert!(halt["belief_variance"].as_f64().unwrap() > 0.7);
    let rate = halt["diagnostic"]["rate"].as_f64().unwrap();
    assert!(
        rate >= 0.6,
        "upstream of outcome degradation: the rate is still {rate}"
    );
    assert_eq!(halt["diagnostic"]["by_mode"]["shallow"]["rate"], 0.0);
    assert_eq!(halt["diagnostic"]["by_mode"]["deep"]["rate"], 1.0);
    assert_eq!(halt_day(&events), Some(14));
    assert_eq!(named(&events, "drain").len(), 1, "the scene drains");
    assert!(
        named(&events, "acceptance_day")
            .iter()
            .any(|e| e["action"] == "linear_note" && e["effect"] == "refused (mcp unconfigured)"),
        "a linear_note pick is accepted with its effect refused, never written"
    );
    for day in named(&events, "acceptance_day") {
        assert!(day["window"]["offers"].as_u64().unwrap() >= 7, "{day}");
        assert!(
            day["belief_variance"].as_f64().unwrap() <= 0.7,
            "no halt before day 14: {day}"
        );
    }

    // AC2.3 — one digest per simulated day (the halt day included), each on
    // the scenario clock: the fixture's 08:00 local, days strictly rising.
    let fx = fixture();
    let digests: Vec<u64> = named(&events, "morning_digest")
        .iter()
        .map(|e| e["now_ns"].as_u64().unwrap())
        .collect();
    assert_eq!(digests, (1..=14).map(|d| fx.day_ns(d)).collect::<Vec<_>>());
    assert!(named(&events, "morning_digest")
        .iter()
        .all(|e| e["digest"]["utc_offset_secs"] == 9 * 3600));

    // ADR-028 row d — no stdout value is a wall-clock instant.
    let allowed: BTreeSet<u64> = (1..=21)
        .map(|d| fx.day_ns(d))
        .chain(
            [(1..=21).map(|d| fx.day_ns(d) + 15 * 60 * 1_000_000_000)]
                .into_iter()
                .flatten(),
        )
        .collect();
    let mut seen = Vec::new();
    events.iter().for_each(|e| instants(e, &mut seen));
    let wall: Vec<_> = seen.iter().filter(|n| !allowed.contains(n)).collect();
    assert!(wall.is_empty(), "wall-clock values on stdout: {wall:?}");

    // AC2.4 — one non-personal attestation per decided offer.
    let attested = attestations(home.path());
    assert_eq!(attested.len(), days.len(), "{attested:?}");
    for payload in &attested {
        let keys: BTreeSet<&str> = payload
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            BTreeSet::from(["action", "decision_id", "notification_ref", "sim_ns"]),
            "the TL names no work mode, response or option text: {payload}"
        );
    }
}

/// AC1.2 — the steady twin (the CI twin and the AC1 plant's subject).
#[test]
fn the_steady_twin_runs_21_days_without_a_halt() {
    let home = tempfile::tempdir().unwrap();
    let output = scene(
        home.path(),
        &workspace_root().join(STEADY),
        "compressed:21d",
    );
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let events = events(&output);
    assert!(
        named(&events, "halt").is_empty(),
        "the steady director never halts: {events:?}"
    );
    assert_eq!(named(&events, "acceptance_day").len(), 21);
    assert_eq!(named(&events, "morning_digest").len(), 21);
    assert_eq!(halt_day(&events), None);
}

fn split(dir: &Path) -> (PathBuf, PathBuf) {
    let text = std::fs::read_to_string(workspace_root().join(FIXTURE)).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap().to_owned();
    let (mut first, mut second) = (vec![header.clone()], vec![header]);
    for line in lines {
        let day = serde_json::from_str::<serde_json::Value>(line).unwrap()["day"]
            .as_i64()
            .unwrap();
        if day <= 10 { &mut first } else { &mut second }.push(line.to_owned());
    }
    let (a, b) = (dir.join("days-to-10.jsonl"), dir.join("days-from-11.jsonl"));
    std::fs::write(&a, first.join("\n")).unwrap();
    std::fs::write(&b, second.join("\n")).unwrap();
    (a, b)
}

fn restored(events: &[serde_json::Value]) -> serde_json::Value {
    named(events, "acceptance_ledger_restored")
        .first()
        .map(|e| (*e).clone())
        .expect("the record is restored before the first pass")
}

/// AC3.2 (FR31; D-18-4-02 A) — learning survives a restart in the principal
/// namespace, and `forget` erases it.
#[test]
fn a_restart_resumes_from_the_principal_namespace_and_forget_erases_it() {
    let home = tempfile::tempdir().unwrap();
    let parts = tempfile::tempdir().unwrap();
    let (to_10, from_11) = split(parts.path());

    let first = scene(home.path(), &to_10, "compressed:10d");
    assert!(first.status.success(), "{}", stderr(&first));
    let first = events(&first);
    assert_eq!(
        restored(&first)["records"],
        0,
        "a fresh home holds no record"
    );
    assert_eq!(halt_day(&first), None);

    let resumed = scene(home.path(), &from_11, "compressed:21d");
    assert!(resumed.status.success(), "{}", stderr(&resumed));
    let resumed = events(&resumed);
    assert_eq!(
        halt_day(&resumed),
        Some(14),
        "the resumed scene halts on the uninterrupted run's day"
    );
    let restore = restored(&resumed);
    assert_eq!(
        restore["records"], 17,
        "7 history days + days 1..10: {restore}"
    );
    assert_eq!(
        restore["pid"], 1,
        "the root Butler keeps pid 1 across processes"
    );

    let rows =
        maos_audit::subject_access_query(&home.path().join("audit/transparency.sqlite"), PRINCIPAL)
            .expect("subject access query");
    assert!(
        rows.iter()
            .any(|r| r.schema == "butler.acceptance/v1" && r.writer_spirit_pid == 1),
        "the principal rows are listed: {rows:?}"
    );

    let forget = maos(home.path())
        .env("MAOS_ONE_SHOT", "forget")
        .env("MAOS_FORGET_PRINCIPAL", PRINCIPAL)
        .output()
        .expect("maosctl forget's offline arm");
    assert!(forget.status.success(), "forget: {}", stderr(&forget));

    let after = scene(home.path(), &from_11, "compressed:21d");
    assert!(after.status.success(), "{}", stderr(&after));
    let after = events(&after);
    assert_eq!(
        restored(&after)["records"],
        0,
        "RTBF: nothing restores after forget"
    );
    assert_eq!(
        halt_day(&after),
        Some(11),
        "with no record the window is empty: the conflict proxy halts at once"
    );
    for payload in attestations(home.path()) {
        // The action kind (`linear_note`/`snooze`/`ignored`) is the ruled
        // non-personal attestation field (D-18-4-02 A); mode and response are not.
        let text = payload.to_string();
        for personal in ["deep", "shallow", "pick:", "work_mode", "response"] {
            assert!(
                !text.contains(personal),
                "a TL attestation names `{personal}`: {text}"
            );
        }
    }
}

/// AC3.3 — the root's port writes as its fused pid through the kernel's
/// Principal-write enforcement: an unauthorized pid is refused.
#[test]
fn the_principal_port_is_fused_and_unauthorized_pids_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("audit.sqlite");
    let memory = Arc::new(
        maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
                dir.path().join("memory"),
                4 * 1024,
            )),
            Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&db).unwrap()),
            Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db).unwrap()),
            Arc::new(
                maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_with_global_legal_holds(
                    &db, &db, 1,
                )
                .unwrap(),
            ),
        )
        .with_principal_write_enforcement(),
    );
    memory.authorize_principal_writes(1);
    let admitted = maos_bin::butler_root::PidBoundPrincipalPort::new(Arc::downgrade(&memory), 1);
    let intruder = maos_bin::butler_root::PidBoundPrincipalPort::new(Arc::downgrade(&memory), 9);

    admitted
        .write(PRINCIPAL, "butler.acceptance/v1", "ledger", "# record\n")
        .expect("the admitted pid writes");
    assert_eq!(
        admitted
            .read(PRINCIPAL, "butler.acceptance/v1", "ledger")
            .unwrap()
            .as_deref(),
        Some("# record\n")
    );
    let refused = intruder
        .write(PRINCIPAL, "butler.acceptance/v1", "ledger", "# forged\n")
        .expect_err("an unauthorized pid is refused");
    assert!(
        refused.contains("not authorized")
            || refused.contains("Unauthorized")
            || refused.contains("unauthorized"),
        "{refused}"
    );
    assert_eq!(
        intruder
            .read(PRINCIPAL, "butler.acceptance/v1", "ledger")
            .unwrap(),
        None,
        "another pid's store does not hold the record"
    );
    assert!(
        admitted
            .write("a:b", "butler.acceptance/v1", "ledger", "x")
            .is_err(),
        "a principal id with `:` is refused"
    );
}

fn refused(output: &Output, says: &str) {
    assert!(!output.status.success(), "must refuse: {}", stderr(output));
    assert!(
        stderr(output).contains(says),
        "stderr must say `{says}`:\n{}",
        stderr(output)
    );
}

/// CR-2 — every refusal is typed and exits non-zero; D-18-4-06 A — a
/// malformed `MAOS_UTC_OFFSET` refuses the boot.
#[test]
fn scene_refusals_are_typed() {
    let home = tempfile::tempdir().unwrap();
    let fx = workspace_root().join(FIXTURE);
    let run = |args: &[&str]| maos(home.path()).arg("run").args(args).output().unwrap();
    let manifest = "spirits/butler/manifest.toml";
    let fixture = fx.to_str().unwrap();
    refused(
        &run(&[manifest, "--clock", "compressed:21d"]),
        "--clock requires --acceptance",
    );
    refused(
        &run(&[manifest, "--acceptance", fixture]),
        "--acceptance requires --clock",
    );
    refused(
        &run(&[
            manifest,
            "--clock",
            "compressed:21d",
            "--acceptance",
            fixture,
            "--once",
        ]),
        "cannot be combined with --once",
    );
    refused(
        &run(&[
            manifest,
            "--clock",
            "compressed:21d",
            "--acceptance",
            fixture,
            "--live",
        ]),
        "cannot be combined with --live",
    );
    refused(
        &run(&[manifest, "--clock", "21d", "--acceptance", fixture]),
        "malformed clock",
    );
    refused(
        &run(&[
            manifest,
            "--clock",
            "compressed:90m",
            "--acceptance",
            fixture,
        ]),
        "whole simulated days",
    );
    refused(
        &run(&[
            "spirits/researcher/manifest.toml",
            "--clock",
            "compressed:21d",
            "--acceptance",
            fixture,
        ]),
        "is not a Butler manifest",
    );
    let text = std::fs::read_to_string(&fx).unwrap();
    let gap = home.path().join("gap.jsonl");
    std::fs::write(
        &gap,
        text.lines()
            .filter(|l| !l.contains("\"day\":5,"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    refused(
        &run(&[
            manifest,
            "--clock",
            "compressed:21d",
            "--acceptance",
            gap.to_str().unwrap(),
        ]),
        "has no line for day 5",
    );
    let bad = home.path().join("bad.jsonl");
    std::fs::write(&bad, text.replacen("\"pick:1\"", "\"accept\"", 1)).unwrap();
    refused(
        &run(&[
            manifest,
            "--clock",
            "compressed:21d",
            "--acceptance",
            bad.to_str().unwrap(),
        ]),
        "acceptance fixture line 2",
    );
    let offset = maos(home.path())
        .args(["run", manifest, "--once"])
        .env("MAOS_UTC_OFFSET", "+25:00")
        .output()
        .unwrap();
    refused(&offset, "MAOS_UTC_OFFSET");
}
