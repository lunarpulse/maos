//! Story 18-4 — exit line 4: the compressed Butler scene, at process level
//! (`env!("CARGO_BIN_EXE_maos")`, a fresh `HOME`, keyless, no nested cargo).
//!
//! - AC1 (D-18-4-01 A, CR-2/CR-3): 21 simulated days replay the acceptance
//!   fixture; the ledger-informed `belief_variance` halt fires on day 14 while
//!   the pooled rate is still ≥ 0.6 (upstream); the steady twin runs 21 days
//!   with no halt; no stdout value is a wall-clock instant.
//! - AC2.3/2.4 (D-18-4-06 A): exactly one `morning_digest` per simulated
//!   day, on the scenario clock, days strictly increasing, each reading
//!   that day's pass frames (review ruling D-18-4-R5: the halt day's digest
//!   lists the halt and the evaluation); one `butler:acceptance` TL
//!   attestation per decided offer, carrying only `decision_id` + `sim_ns`
//!   (review ruling D-18-4-R1).
//! - AC3.2 (D-18-4-02 A): the fixture split at day 10 across two processes on
//!   one home halts on the same day; the principal rows are listed by
//!   `subject_access_query`; after `maosctl forget --principal` (the offline
//!   one-shot) a restart restores 0 records and its resume is refused (BH-6).
//!   Review ruling D-18-4-R2: a second process resuming after the day-14
//!   halt tunes on the window (`derived_from: acceptance_window`).
//! - AC3.3 (review TIA-1/BH-2): through the ROOT's binder a Butler reaches
//!   only its own pid's record — a second authorised pid's record is
//!   unreachable; an unauthorised pid is refused. Review EC-1/AA-1: a forget
//!   through the door's own call between two decisions resurrects nothing.
//!   Review ruling D-18-4-R3, repaired by 20-3b: a door `unload` + `load`
//!   mid-scene binds the new pid to the same identity and restores its record.
//! - CR-2 / D-18-4-06 A: every scene refusal and a malformed
//!   `MAOS_UTC_OFFSET` are typed, non-zero exits.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    assert_eq!(halt["derived_from"], "acceptance_window");
    // Review TIA-5 — the scene ran through the bound Inference Port.
    assert!(
        stderr(&output).contains("butler inference seam wired"),
        "{}",
        stderr(&output)
    );
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
    let digest_events = named(&events, "morning_digest");
    assert!(digest_events.iter().all(
        |e| e["digest"]["utc_offset_secs"] == 9 * 3600 && e["digest"]["frames"] == "scene_pass"
    ));
    // Review ruling D-18-4-R5 (BH-1) — each digest reads its day's pass:
    // one evaluation a day; the halt day's digest lists the halt and its fire.
    for (day, digest) in digest_events.iter().enumerate() {
        let basis = &digest["digest"]["trust_bar_basis"];
        assert_eq!(basis["evaluations"], 1, "day {}: {digest}", day + 1);
        let halts = digest["digest"]["open_halts"].as_array().unwrap().len();
        assert_eq!(
            halts,
            usize::from(day + 1 == 14),
            "day {}: {digest}",
            day + 1
        );
        assert_eq!(
            basis["fires"],
            u64::from(day + 1 == 14),
            "day {}: {digest}",
            day + 1
        );
    }

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
            BTreeSet::from(["decision_id", "sim_ns"]),
            "the TL names no action, response, work mode or option text: {payload}"
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
    assert!(stderr(&output).contains("butler inference seam wired"));
    let events = events(&output);
    assert!(
        named(&events, "halt").is_empty(),
        "the steady director never halts: {events:?}"
    );
    assert_eq!(named(&events, "acceptance_day").len(), 21);
    assert_eq!(named(&events, "morning_digest").len(), 21);
    assert_eq!(halt_day(&events), None);
}

/// The fixture's header with its days `≤ at` and `> at`.
fn split(dir: &Path, at: i64) -> (PathBuf, PathBuf) {
    let text = std::fs::read_to_string(workspace_root().join(FIXTURE)).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap().to_owned();
    let (mut first, mut second) = (vec![header.clone()], vec![header]);
    for line in lines {
        let day = serde_json::from_str::<serde_json::Value>(line).unwrap()["day"]
            .as_i64()
            .unwrap();
        if day <= at { &mut first } else { &mut second }.push(line.to_owned());
    }
    let (a, b) = (
        dir.join(format!("days-to-{at}.jsonl")),
        dir.join(format!("days-after-{at}.jsonl")),
    );
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
    let (to_10, from_11) = split(parts.path(), 10);

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

    // BH-6 (operator rule 2026-10-26) — the forgotten home holds nothing to
    // resume from: the restart restores 0 records and its resume is refused.
    let after = scene(home.path(), &from_11, "compressed:21d");
    assert!(!after.status.success(), "{}", stderr(&after));
    assert!(
        stderr(&after).contains("the fixture resumes at day 11"),
        "{}",
        stderr(&after)
    );
    assert_eq!(
        restored(&events(&after))["records"],
        0,
        "RTBF: nothing restores after forget"
    );
    // Review ruling D-18-4-R1 — after the forget the TL attestations carry
    // nothing the accept/decline sequence could be rebuilt from.
    for payload in attestations(home.path()) {
        let keys: BTreeSet<&str> = payload
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, BTreeSet::from(["decision_id", "sim_ns"]), "{payload}");
        let text = payload.to_string();
        for personal in [
            "deep",
            "shallow",
            "pick:",
            "linear_note",
            "snooze",
            "ignored",
            "action",
        ] {
            assert!(
                !text.contains(personal),
                "a TL attestation names `{personal}`: {text}"
            );
        }
    }
}

/// Review ruling D-18-4-R2 (EC-3/BH-3) at process level — a home tunes again
/// after its first halt: a second process resuming the day after the day-14
/// halt (one undecided day, tolerated by BH-6) halts on the WINDOW (days
/// 8–13 decided: 4 of 6 accepted), never on the conflict proxy.
#[test]
fn a_resume_after_the_halt_day_tunes_on_the_window() {
    let home = tempfile::tempdir().unwrap();
    let parts = tempfile::tempdir().unwrap();
    let first = scene(
        home.path(),
        &workspace_root().join(FIXTURE),
        "compressed:21d",
    );
    assert!(first.status.success(), "{}", stderr(&first));
    assert_eq!(halt_day(&events(&first)), Some(14));

    let (_, from_15) = split(parts.path(), 14);
    let resumed = scene(home.path(), &from_15, "compressed:21d");
    assert!(resumed.status.success(), "{}", stderr(&resumed));
    let events = events(&resumed);
    let halts = named(&events, "halt");
    assert_eq!(halts.len(), 1, "{events:?}");
    assert_eq!(halts[0]["day"], 15);
    assert_eq!(
        halts[0]["derived_from"], "acceptance_window",
        "day 15 is a window halt, not a proxy halt: {}",
        halts[0]
    );
    assert!((halts[0]["diagnostic"]["rate"].as_f64().unwrap() - 4.0 / 6.0).abs() < 1e-6);

    // A two-day gap (no day 14, no day 15) is still refused, typed.
    let gap = parts.path().join("days-from-16.jsonl");
    let text = std::fs::read_to_string(&from_15).unwrap();
    std::fs::write(
        &gap,
        text.lines()
            .filter(|l| !l.contains("\"day\":15,"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    let refused = scene(home.path(), &gap, "compressed:21d");
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("the fixture resumes at day 16"),
        "{}",
        stderr(&refused)
    );
}

/// A root's memory manager (Principal-write enforcement on) and its TL.
fn root_memory(
    dir: &Path,
) -> (
    Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
    Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
) {
    let db = dir.join("audit.sqlite");
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_with_global_legal_holds(
            &db, &db, 1,
        )
        .unwrap(),
    );
    let memory = Arc::new(
        maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
                dir.join("memory"),
                4 * 1024,
            )),
            Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&db).unwrap()),
            Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db).unwrap()),
            Arc::clone(&tl),
        )
        .with_principal_write_enforcement(),
    );
    (memory, tl)
}

/// The root's `ButlerSeam` with its production acceptance binder.
fn root_seam(
    memory: &Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    clock: &Arc<maos_spirit_sdk::clock::ScenarioClock>,
) -> butler::ButlerSeam {
    let clock = Arc::clone(clock) as Arc<dyn maos_spirit_sdk::clock::Clock>;
    butler::ButlerSeam {
        inference: None,
        mint: Arc::new(|pid, _: &str| {
            Ok(maos_domain::invariants::i1::CapabilityToken::new(
                maos_domain::invariants::i1::TokenId::ZERO,
                pid,
                0,
                [0u8; 64],
            ))
        }),
        digest: Arc::new(butler::MorningDigestRenderer::new(
            PathBuf::new(),
            PathBuf::new(),
            Arc::new(|_, _| {}),
            Arc::clone(&clock),
        )),
        rollover: false,
        clock,
        acceptance: Some(maos_bin::butler_root::acceptance_binder(
            PRINCIPAL.into(),
            Arc::downgrade(memory),
            Arc::downgrade(tl),
        )),
        scene_butler: Arc::default(),
        context: None,
    }
}

/// The root's seeded conflict scenario (one pending notification a pass).
fn conflicted() -> butler::Butler {
    let event = |id: &str, start, end| butler::CalendarEvent {
        id: id.into(),
        title: id.into(),
        start_min: start,
        end_min: end,
        status: butler::EventStatus::Confirmed,
    };
    butler::Butler::with_scenario(butler::ScenarioInput {
        calendar: vec![event("evt-a", 540, 600), event("evt-b", 570, 630)],
        ..Default::default()
    })
}

fn pass(butler: &butler::Butler) {
    use maos_spirit_abi::lifecycle::Spirit as _;
    butler.on_idle(&mut maos_spirit_abi::ctx::Ctx::for_rust_inproc_hook(
        maos_spirit_abi::ctx::CapabilityHandle(0),
        maos_spirit_abi::ctx::MailboxHandle(0),
    ));
    assert!(butler.take_pass_error().is_none(), "the pass is refused");
}

fn record_lines(port: &dyn PrincipalMemoryPort) -> usize {
    port.read(PRINCIPAL, "butler.acceptance/v1", "ledger")
        .unwrap()
        .map_or(0, |md| md.lines().filter(|l| l.starts_with('{')).count())
}

fn history() -> AcceptanceFixture {
    let mut text =
        String::from(r#"{"principal":"director-ada","start":"2026-03-02","utc_offset":"+00:00"}"#);
    for day in -6..=0 {
        text.push_str(&format!(
            "\n{{\"day\":{day},\"work_mode\":\"deep\",\"response\":\"pick:1\"}}"
        ));
    }
    AcceptanceFixture::parse(&text).unwrap()
}

/// AC3.3 (review TIA-1/BH-2) — the ROOT fixes the pid: through the
/// production binder a Butler (whose API names no pid) reads and writes
/// only the record at the pid the root's resolver yields. A second
/// AUTHORISED pid's record is unreachable and untouched; an unauthorised
/// pid's write is refused `PrincipalWriteUnauthorized`.
#[test]
fn the_root_fuses_the_principal_port_and_another_pids_record_is_unreachable() {
    let dir = tempfile::tempdir().unwrap();
    let (memory, tl) = root_memory(dir.path());
    memory.authorize_principal_writes(1);
    memory.authorize_principal_writes(9);
    let other = maos_bin::butler_root::PidBoundPrincipalPort::new(Arc::downgrade(&memory), 9);
    other
        .write(
            PRINCIPAL,
            "butler.acceptance/v1",
            "ledger",
            "# pid 9's record\n",
        )
        .expect("the second authorised pid writes its own record");

    let fx = history();
    let clock = Arc::new(maos_spirit_sdk::clock::ScenarioClock::new(fx.day_ns(1), 0));
    let seam = root_seam(&memory, &tl, &clock);
    let butler = seam
        .attach(conflicted(), || None, Arc::new(|| Some(1)))
        .unwrap();
    butler.seed_acceptance_history(&fx).unwrap();
    assert_eq!(
        butler.acceptance_window().offers,
        7,
        "pid 9's record never reached the window"
    );
    pass(&butler);
    butler.handle_option_pick(1, "deep", None).unwrap();

    let own = maos_bin::butler_root::PidBoundPrincipalPort::new(Arc::downgrade(&memory), 1);
    assert_eq!(
        record_lines(&own),
        8,
        "7 history days + the decision, at pid 1"
    );
    assert_eq!(
        other
            .read(PRINCIPAL, "butler.acceptance/v1", "ledger")
            .unwrap()
            .as_deref(),
        Some("# pid 9's record\n"),
        "the second authorised pid's record is unreachable and untouched"
    );
    let attested: Vec<_> = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter::default())
        .unwrap()
        .into_iter()
        .filter(|f| f.intent == butler::ACCEPTANCE_INTENT)
        .collect();
    assert_eq!(attested.len(), 1);
    assert_eq!(attested[0].spirit_pid, 1, "attested at the root's pid");

    // An unauthorised pid (no admission) is refused, typed.
    let unadmitted = seam
        .attach(conflicted(), || None, Arc::new(|| Some(7)))
        .unwrap();
    let refused = unadmitted
        .seed_acceptance_history(&fx)
        .unwrap_err()
        .to_string();
    assert!(refused.to_lowercase().contains("authoriz"), "{refused}");
    // And an unresolved pid is refused before any read.
    let unresolved = seam
        .attach(conflicted(), || None, Arc::new(|| None))
        .unwrap();
    let refused = unresolved
        .seed_acceptance_history(&fx)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("no admitted pid"), "{refused}");
}

/// Review EC-1/AA-1 — the principal namespace is the record of truth: a
/// forget through the door's own call (`forget_with_reason`) between a pass
/// and its decision leaves only that decision; nothing held in memory is
/// written back.
#[test]
fn a_forget_between_decisions_resurrects_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (memory, tl) = root_memory(dir.path());
    memory.authorize_principal_writes(1);
    let fx = history();
    let clock = Arc::new(maos_spirit_sdk::clock::ScenarioClock::new(fx.day_ns(1), 0));
    let butler = root_seam(&memory, &tl, &clock)
        .attach(conflicted(), || None, Arc::new(|| Some(1)))
        .unwrap();
    butler.seed_acceptance_history(&fx).unwrap();
    pass(&butler);
    butler.handle_option_pick(1, "deep", None).unwrap();
    let port = maos_bin::butler_root::PidBoundPrincipalPort::new(Arc::downgrade(&memory), 1);
    assert_eq!(record_lines(&port), 8);

    clock.set_ns(fx.day_ns(2));
    pass(&butler);
    memory
        .forget_with_reason(PRINCIPAL, None)
        .expect("the door's forget");
    assert_eq!(record_lines(&port), 0, "forget erased the record");
    butler.handle_option_pick(1, "deep", None).unwrap();
    assert_eq!(
        record_lines(&port),
        1,
        "only the post-forget decision is stored; nothing is resurrected"
    );
    assert_eq!(butler.acceptance_window().offers, 0, "day 2 is today");
    clock.set_ns(fx.day_ns(3));
    assert_eq!(
        butler.acceptance_window().offers,
        1,
        "the window holds only day 2"
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

fn maosctl(home: &Path, args: &[&str]) -> Output {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli` first",
        path.display()
    );
    let mut cmd = Command::new(path);
    cmd.current_dir(workspace_root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"));
    cmd.args(args).output().expect("run maosctl")
}

/// D-18-4-R3 / 20-3b: a door reload binds the new pid to the same stable
/// identity. Its eight existing records survive, so the resumed scene
/// reaches the same day-14 acceptance-window halt as an uninterrupted run.
#[test]
fn a_door_reload_mid_scene_binds_the_new_pid_and_restores_the_same_identity() {
    let home = tempfile::tempdir().unwrap();
    let init = maos(home.path()).arg("init").output().expect("maos init");
    assert!(init.status.success(), "{}", stderr(&init));
    let hold = home.path().join("scene.hold");
    std::fs::write(&hold, b"").unwrap();
    let mut child = maos(home.path())
        .args([
            "run",
            "spirits/butler/manifest.toml",
            "--clock",
            "compressed:21d",
        ])
        .arg("--acceptance")
        .arg(workspace_root().join(FIXTURE))
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", CASSETTE)
        .env("MAOS_NOTIFY_DISABLE", "1")
        .env("MAOS_SCENE_HOLD_FILE", &hold)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the scene");
    let (out, err) = (
        drain(child.stdout.take().unwrap()),
        drain(child.stderr.take().unwrap()),
    );
    let deadline = Instant::now() + Duration::from_secs(45);
    while !out.lock().unwrap().contains("\"event\":\"acceptance_day\"") {
        assert!(
            Instant::now() < deadline,
            "day 1 never decided: {}",
            err.lock().unwrap()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    for args in [
        &["unload", "butler"][..],
        &["load", "spirits/butler/manifest.toml"][..],
        &["start", "butler"][..],
    ] {
        let ctl = maosctl(home.path(), args);
        assert!(ctl.status.success(), "maosctl {args:?}: {}", stderr(&ctl));
    }
    std::fs::remove_file(&hold).unwrap();
    let status = child.wait().expect("the scene exits");
    let (stdout, stderr) = (out.lock().unwrap().clone(), err.lock().unwrap().clone());
    assert!(status.success(), "{stderr}");
    assert_eq!(
        stderr.matches("butler inference seam wired").count(),
        2,
        "the boot arm and the door load both bind: {stderr}"
    );
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let restores = named(&events, "acceptance_ledger_restored");
    assert_eq!(restores.len(), 2, "{stdout}");
    assert_eq!(restores[0]["pid"], 1);
    assert_ne!(
        restores[1]["pid"], 1,
        "the door-loaded Butler has a new pid"
    );
    assert_eq!(
        restores[1]["records"], 8,
        "seven seed days and day 1 survive the new pid: {stdout}"
    );
    let halts = named(&events, "halt");
    assert_eq!(halts.len(), 1, "{stdout}");
    assert_eq!(halts[0]["day"], 14);
    assert_eq!(halts[0]["derived_from"], "acceptance_window");
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
    // Review EC-7 — validated on every boot, scene mode included (the
    // scene's own clock then replaces it).
    let offset = maos(home.path())
        .args([
            "run",
            manifest,
            "--clock",
            "compressed:21d",
            "--acceptance",
            fixture,
        ])
        .env("MAOS_UTC_OFFSET", "+25:00")
        .output()
        .unwrap();
    refused(&offset, "MAOS_UTC_OFFSET");
    // Review BH-4/EC-4/TIA-3 — a multibyte unit is a typed refusal, not a panic.
    refused(
        &run(&[
            manifest,
            "--clock",
            "compressed:1é",
            "--acceptance",
            fixture,
        ]),
        "malformed clock",
    );
}

/// FR31 / 20-3b AC4(b): an authorised non-Butler plants valid all-accept
/// Markdown at pid 1, then a fresh boot binds Butler to that recycled pid.
/// Planting pid-based `owner_dirname` must restore seven records, not zero.
#[test]
fn recycled_pid_cannot_poison_butlers_acceptance_restore() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let fx = history();
    let mut forged = butler::AcceptanceLedger::default();
    for day in -6..=0 {
        forged.insert(butler::AcceptanceRecord {
            local_day: fx.local_day(day),
            sim_ns: fx.day_ns(day),
            work_mode: "deep".into(),
            response: "pick:1".into(),
            action: "linear_note".into(),
            accepted: true,
        });
    }
    let markdown = forged.to_markdown()?;
    assert_eq!(butler::AcceptanceLedger::from_markdown(&markdown)?.len(), 7);
    {
        let (memory, _tl) = root_memory(dir.path());
        memory.bind_storage_identity(1, "non-butler")?;
        memory.authorize_principal_writes(1);
        let attacker =
            maos_bin::butler_root::PidBoundPrincipalPort::new(Arc::downgrade(&memory), 1);
        attacker.write(PRINCIPAL, "butler.acceptance/v1", "ledger", &markdown)?;
    }
    let (memory, tl) = root_memory(dir.path());
    memory.bind_storage_identity(1, "butler")?;
    memory.authorize_principal_writes(1);
    let clock = Arc::new(maos_spirit_sdk::clock::ScenarioClock::new(fx.day_ns(1), 0));
    let butler =
        root_seam(&memory, &tl, &clock).attach(conflicted(), || None, Arc::new(|| Some(1)))?;
    // No seed history: this exercises Butler's real restore path without
    // masking an inherited record by replacing its local days.
    let empty_history = AcceptanceFixture::parse(
        "{\"principal\":\"director-ada\",\"start\":\"2026-03-02\",\"utc_offset\":\"+00:00\"}\n{\"day\":1,\"work_mode\":\"deep\",\"response\":\"pick:1\"}",
    )?;
    butler.seed_acceptance_history(&empty_history)?;
    assert_eq!(
        butler.acceptance_window().offers,
        0,
        "Butler must restore zero attacker records"
    );
    let own = maos_bin::butler_root::PidBoundPrincipalPort::new(Arc::downgrade(&memory), 1);
    let restored = own
        .read(PRINCIPAL, "butler.acceptance/v1", "ledger")?
        .ok_or("Butler persisted its empty ledger")?;
    assert_eq!(butler::AcceptanceLedger::from_markdown(&restored)?.len(), 0);
    Ok(())
}
