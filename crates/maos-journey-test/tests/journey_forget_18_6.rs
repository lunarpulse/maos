#![forbid(unsafe_code)]
//! Story 18-6 AC1–3: real daemon/door, bound non-`--live` seed-claim replay.
#[path = "support/binaries.rs"]
mod binaries;
#[path = "../../../tests/harness/forget_18_6.rs"]
mod h;
use serde_json::Value;
use std::path::PathBuf;
// ADR-064 refuses replay + `--live`. This process witness instead targets
// the real root's fixed seed-rag-noisy claim: its unique statement fragment
// is the principal token and its claim_id is the canary. Using the statement
// token also reaches the inference adapter's journaled prompt bytes.
// MCP-sourced claims are exercised separately through the root-library
// journey, not relabelled as a process replay.
const PRINCIPAL: &str = "noisy corpus.";
const CANARY: &str = "seed-rag-noisy";
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-researcher/forget-redistill-18-6.json";
const SEED_STATEMENTS: [&str; 3] = [
    "Retrieval augmentation reduces hallucinated citations in literature surveys.",
    "Retrieval augmentation does not reduce hallucinated citations over a noisy corpus.",
    "Self-consistency voting may improve factual recall, though gains appear limited to larger models.",
];
const SEED_PROMPT_HASHES: [&str; 3] = [
    "50a2cca1c109e87bb39db11f5a2eced496a5682379e5ac350a6ae071248dffa9",
    "918e4919e4e227521296cb5c2239af44c75c0efac321c105a6e6664b193511a8",
    "70d5a1ad26c91e35c16636da8188a643c0f657a0c070d2841ea1ea1f9c6e5bd4",
];
const SEED_SUMMARIES: [&str; 3] = [
    "Retrieval augmentation cuts hallucinated citations in literature surveys.",
    "Over a noisy corpus, retrieval augmentation does not cut hallucinated citations.",
    "Self-consistency voting may improve factual recall, but gains appear limited to larger models.",
];
fn is_digest(row: &(String, u32, String, Value)) -> bool {
    row.3.get("source_log_ref").is_some() && row.3.get("redacted").is_none()
}
fn normalize(id: &str) -> String {
    id.replace(':', "")
}

#[test]
fn live_forget_redistills_and_never_resurrects_then_pending_is_typed() {
    let maos = PathBuf::from(binaries::resolve("maos", option_env!("CARGO_BIN_EXE_maos")));
    let ctl = PathBuf::from(binaries::resolve(
        "maosctl",
        option_env!("CARGO_BIN_EXE_maosctl"),
    ));
    let home = tempfile::tempdir().unwrap();
    let mut init = h::command(&maos, home.path());
    init.arg("init");
    h::checked(init);
    let mut butler = h::command(&maos, home.path());
    butler
        .args([
            "run",
            "spirits/butler/manifest.toml",
            "--clock",
            "compressed:4d",
            "--acceptance",
            "tests/fixtures/j-butler/acceptance-21d-steady.jsonl",
        ])
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env(
            "MAOS_REPLAY_CASSETTE",
            "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json",
        );
    h::checked(butler);
    assert!(
        !maos_audit::subject_access_query(&h::db(home.path()), "director-ada")
            .unwrap()
            .is_empty(),
        "Butler wrote indexed Private data"
    );
    let (held_id, controls) = {
        let stores = h::Stores::open(home.path(), 180600);
        // The prior boot's real Butler established pid 1 as an indexed
        // writer. Keep the fixed seed's indexed Private leg at that pid too;
        // the daemon below door-loads its Researcher at a different pid.
        stores.private(1, PRINCIPAL, CANARY);
        let held = stores.frame(8800, "held-director", "held-canary");
        let probes: Vec<Value> = include_str!("../../../tests/corpora/gdpr-cascade-probe-v1.jsonl")
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let mut controls = Vec::new();
        for probe in &probes[..50] {
            let p = probe["control_principal"].as_str().unwrap();
            stores.private(8900, p, "untouched control");
            let source = stores.frame(8900, p, "control-canary");
            controls.push(stores.digest(8900, vec![source], "untouched control summary"));
        }
        (held, controls)
    };
    let before = h::rows(home.path());
    let mut run = h::command(&maos, home.path());
    // A non-inferencing root occupies pid 1 without surveying any seed.
    // Loading Researcher through the real door then gives it a fresh,
    // unindexed pid, with no startup pass racing an unload.
    run.args(["run", "spirits/reviewer/manifest.toml"])
        .env("MAOS_IDLE_FAST", "1")
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", CASSETTE);
    let daemon = h::Daemon::spawn(run);
    h::wait("operator door", &daemon, || {
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "reviewer"])
            .status
            .success()
    });
    let butler_forget = h::ctl(
        &ctl,
        home.path(),
        &["forget", "--principal", "director-ada"],
    );
    assert!(
        butler_forget.status.success(),
        "{}",
        String::from_utf8_lossy(&butler_forget.stderr)
    );
    h::assert_absent(home.path(), "director-ada", "CANARY-forget-18-6-000");
    for args in [
        &["load", "spirits/researcher/manifest.toml"][..],
        &["start", "researcher"][..],
    ] {
        let out = h::ctl(&ctl, home.path(), args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    h::wait("bound seed canary Distillate", &daemon, || {
        h::rows(home.path())
            .iter()
            .any(|r| is_digest(r) && r.3.to_string().contains(CANARY))
    });
    let initial = h::rows(home.path());
    let personal = initial
        .iter()
        .find(|r| is_digest(r) && r.3.to_string().contains(CANARY))
        .unwrap();
    let writer = personal.1;
    assert!(
        initial.iter().any(|r| r.1 == writer
            && r.2 == "researcher:seed_claim"
            && r.3["claim_id"] == CANARY
            && r.3["statement"].as_str().unwrap().contains(PRINCIPAL)),
        "raw fixed seed source carries the principal and canary"
    );
    assert!(
        maos_audit::subject_access_query(&h::db(home.path()), PRINCIPAL)
            .unwrap()
            .iter()
            .all(|r| r.writer_spirit_pid != writer),
        "Researcher is unindexed"
    );
    assert_ne!(writer, 1, "the indexed prior writer must not own this seed");
    assert!(
        maos_audit::subject_access_query(&h::db(home.path()), PRINCIPAL)
            .unwrap()
            .iter()
            .any(|r| r.writer_spirit_pid == 1),
        "the same principal has an indexed Private writer"
    );
    assert!(
        initial
            .iter()
            .any(|r| r.1 == writer && r.2 == "methodology_conflict"),
        "the fixed seed contradiction genuinely halts"
    );
    let requests_before = initial
        .iter()
        .filter(|r| r.2 == "distillate.redistill.requested")
        .count();
    let held = h::ctl(
        &ctl,
        home.path(),
        &[
            "forget",
            "--principal",
            "held-director",
            "--reason",
            "legal-hold",
        ],
    );
    assert_eq!(held.status.code(), Some(3));
    let held_rows = h::rows(home.path());
    let held_hex: String = held_id.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        before.iter().find(|r| r.0 == held_hex),
        held_rows.iter().find(|r| r.0 == held_hex)
    );
    assert!(held_rows.iter().any(|r| r.2 == "principal.forget.held"));
    assert_eq!(
        held_rows
            .iter()
            .filter(|r| r.2 == "distillate.redistill.requested")
            .count(),
        requests_before
    );
    let forgotten = h::ctl(&ctl, home.path(), &["forget", "--principal", PRINCIPAL]);
    assert!(
        forgotten.status.success(),
        "{}",
        String::from_utf8_lossy(&forgotten.stderr)
    );
    let terminal = h::json(&forgotten);
    h::assert_absent(home.path(), PRINCIPAL, CANARY);
    let requests = terminal["forget"]["redistill"].as_array().unwrap();
    assert!(!requests.is_empty());
    let outcomes = terminal["redistill_outcomes"].as_array().unwrap();
    assert_eq!(requests.len(), outcomes.len());
    assert_eq!(terminal["pending_count"], 0);
    assert_eq!(
        terminal["redistilled_count"].as_u64().unwrap() as usize,
        requests.len()
    );
    let after_forget = h::rows(home.path());
    let mut scrubbed = Vec::new();
    for field in [
        "redacted_distillate_frame_ids",
        "redacted_principal_frame_ids",
    ] {
        scrubbed.extend(
            terminal["forget"][field]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| normalize(v.as_str().unwrap())),
        );
    }
    for outcome in outcomes {
        assert_eq!(outcome["outcome"]["status"], "redistilled");
        let id = normalize(
            outcome["outcome"]["new_distillate_frame_id"]
                .as_str()
                .unwrap(),
        );
        let new = after_forget.iter().find(|r| r.0 == id).unwrap();
        assert_eq!(new.1, writer);
        assert_ne!(new.0, personal.0);
        let surviving: Vec<_> = outcome["request"]["surviving_source_log_ref"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| normalize(v.as_str().unwrap()))
            .collect();
        for source in new.3["source_log_ref"].as_array().unwrap() {
            assert!(surviving.contains(&normalize(source.as_str().unwrap())));
        }
        assert!(after_forget.iter().any(|r| r.2 == "distillate.redistilled"
            && r.3["outcome"]["new_distillate_frame_id"]
                == outcome["outcome"]["new_distillate_frame_id"]));
    }
    // No scheduled-repeat process claim: IdleWatchdog fires once per quiet
    // period; a door reload would re-journal built-in constant seed claims.
    // Same-instance next-pass cleanliness is exercised by the root-library
    // MCP witness and the Researcher cache-purge unit test.
    let next = h::rows(home.path());
    h::assert_absent(home.path(), PRINCIPAL, CANARY);
    for row in next
        .iter()
        .filter(|r| r.1 == writer && is_digest(r) && r.3["redacted"] != true)
    {
        let text = row.3.to_string();
        assert!(!text.contains(PRINCIPAL) && !text.contains(CANARY));
        for source in row.3["source_log_ref"].as_array().unwrap() {
            assert!(
                !scrubbed.contains(&normalize(source.as_str().unwrap())),
                "cached scrubbed source resurrected"
            );
        }
    }
    let probes: Vec<Value> = include_str!("../../../tests/corpora/gdpr-cascade-probe-v1.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(probes.len(), 100);
    for probe in &probes {
        h::assert_absent(
            home.path(),
            probe["principal"].as_str().unwrap(),
            probe["canary"].as_str().unwrap(),
        );
        assert_eq!(
            maos_audit::subject_access_query(
                &h::db(home.path()),
                probe["control_principal"].as_str().unwrap()
            )
            .unwrap()
            .len(),
            1
        );
    }
    for id in controls {
        let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            before.iter().find(|r| r.0 == hex),
            next.iter().find(|r| r.0 == hex)
        );
    }
    // Unload the owning writer before the next door cascade.
    let unloaded = h::ctl(&ctl, home.path(), &["unload", "researcher"]);
    assert!(unloaded.status.success());
    {
        let stores = h::Stores::open(home.path(), 180601);
        let raw = stores.frame(writer, "unloaded-director", "unloaded-canary");
        stores.digest(writer, vec![raw], "unloaded-director summary");
    }
    let pending = h::ctl(
        &ctl,
        home.path(),
        &["forget", "--principal", "unloaded-director"],
    );
    assert!(pending.status.success());
    let pending = h::json(&pending);
    h::assert_pending(&pending);
    assert_eq!(pending["pending_count"], 1);
    drop(daemon);
    {
        let stores = h::Stores::open(home.path(), 180602);
        let raw = stores.frame(writer, "offline-director", "offline-canary");
        stores.digest(writer, vec![raw], "offline-director summary");
    }
    let mut offline = h::command(&maos, home.path());
    offline
        .env("MAOS_ONE_SHOT", "forget")
        .env("MAOS_FORGET_PRINCIPAL", "offline-director");
    let offline = h::json(&h::checked(offline));
    h::assert_pending(&offline);
    assert_eq!(offline["pending_count"], 1);
}

#[test]
fn seed_cassette_has_only_real_prompt_hashes() {
    let cassette: Value = serde_json::from_str(include_str!(
        "../cassettes/j-researcher/forget-redistill-18-6.json"
    ))
    .unwrap();
    assert_eq!(cassette["provenance"], "seed");
    // Three initial seed prompts, then the redistill survivors in the
    // DistillateWriter's flattened (reverse) order, then scheduled passes
    // over the surviving seed cache in its original order.
    assert_eq!(cassette["entries"].as_array().unwrap().len(), 133);
    for (i, entry) in cassette["entries"].as_array().unwrap().iter().enumerate() {
        let claim = match i {
            0..=2 => i,
            3 => 2,
            4 => 0,
            _ => [0, 2][(i - 5) % 2],
        };
        let prompt = format!("Summarize this research claim in one sentence for a digest. Preserve any hedges; add no facts.\n\nClaim: {}", SEED_STATEMENTS[claim]);
        assert_eq!(entry["sequence"], i);
        assert_eq!(entry["prompt_len"], prompt.len());
        assert_eq!(entry["prompt_sha256"], SEED_PROMPT_HASHES[claim]);
        assert_eq!(entry["response"]["text"], SEED_SUMMARIES[claim]);
    }
}
