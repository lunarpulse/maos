//! I4 ledger proof: every approval captures actor, target, capability and intent, and a
//! granted approval always has its Approval Decision Log row.
//!
//! Seeded approve/reject decisions (blank actors and failing journals included) drive the
//! production `apply_skill_decision` against a real Transparency Log; the granted targets and
//! the persisted rows are judged by `invariants::i4::check`, as is the adversarial corpus.
//! Plant: apply the decision before calling `journal`, or let `insert_approval_decision`
//! accept a blank actor.

use maos_cli::subcommands::{apply_skill_decision, skill_entries};
use maos_domain::invariants::i4::{check, Observation};
use maos_iac::adapter::transparency_log::TransparencyLogAdapter;
use maos_skill::{discover_skills_detailed, LocalFsSkillQueueStore, SkillAdmissionState};

const SEEDS: [u64; 4] = [4, 0xB0B, 0x1234_5678, 77];
const SKILLS: usize = 8;
const ACTORS: [&str; 5] = ["alice", "bob", "", "   ", "operator-7"];

fn generated(seed: u64) -> Observation {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("skills");
    std::fs::create_dir_all(&root).expect("skills dir");
    for n in 0..SKILLS {
        let src = format!(
            "---\nid = \"skill.s{n}\"\nversion = \"1.0.0\"\nname = \"s{n}\"\ndescription = \"d\"\n---\nBody {n}.\n"
        );
        std::fs::write(root.join(format!("skill.s{n}.md")), src).expect("write skill");
    }
    let discovered = discover_skills_detailed(&[root]);
    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    let log = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).expect("log");
    let mut queue = skill_entries(&discovered.discovered, vec![], &[]).expect("queue");
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut granted = Vec::new();
    for n in 0..SKILLS {
        let id = format!("skill.s{n}");
        for _ in 0..1 + next() % 2 {
            let (approve, journal_fails) = (next() % 3 != 0, next() % 5 == 0);
            let actor = ACTORS[(next() % ACTORS.len() as u64) as usize];
            let pending = queue.show(&id).map(|e| e.state) == Ok(SkillAdmissionState::Pending);
            let outcome =
                apply_skill_decision(&mut queue, &store, &id, approve, actor, None, |row| {
                    if journal_fails {
                        return Err("disk full".to_string());
                    }
                    log.insert_approval_decision(row).map_err(|e| e.to_string())
                });
            if pending && approve && matches!(outcome, Ok(SkillAdmissionState::Admitted)) {
                granted.push(format!("{id}@1.0.0"));
            }
        }
    }
    Observation {
        granted,
        log: log.query_approvals(None).expect("rows"),
    }
}

fn run(corpus: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for (n, line) in corpus.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        match serde_json::from_str::<Observation>(line) {
            Ok(obs) if check(&obs).is_ok() => failures.push(format!("corpus line {n} accepted")),
            Ok(_) => {}
            Err(e) => failures.push(format!("corpus line {n} unparsable: {e}")),
        }
    }
    for seed in SEEDS {
        let obs = generated(seed);
        if obs.granted.is_empty() || obs.log.is_empty() {
            failures.push(format!("seed {seed}: nothing was approved (vacuous)"));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i4_every_approval_has_a_complete_row_and_a_grant_without_a_row_is_refused() {
    let corpus = include_str!("fixtures/invariants/i4_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
