//! Story 18-0 AC6 (and AC2.3's enrolment) — the live `discipline.yml` shape of
//! the `expensive-tests` job, the two proven-red steps and the blocking
//! `cargo-deny` job. Parses the real workflow as configuration (the
//! `release_workflow_15_4.rs` precedent); reads YAML and TOML only.

use std::fs;
use std::path::PathBuf;

use serde_yaml::{Mapping, Value};

/// The `!cancelled()` condition after normalisation (review DD): `${{ }}`
/// stripped and whitespace removed, so `if: "!cancelled()"` and
/// `if: ${{ !cancelled() }}` are the same condition.
const NOT_CANCELLED: &str = "!cancelled()";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn workflow() -> Value {
    let path = root().join(".github/workflows/discipline.yml");
    serde_yaml::from_str(&fs::read_to_string(path).expect("read discipline workflow"))
        .expect("parse discipline workflow")
}

fn mapping(value: &Value) -> &Mapping {
    value.as_mapping().expect("expected mapping")
}

fn get<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    mapping(value).get(Value::String(name.to_string()))
}

fn key<'a>(value: &'a Value, name: &str) -> &'a Value {
    get(value, name).unwrap_or_else(|| panic!("missing key {name}"))
}

fn str_of<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    get(value, name).and_then(Value::as_str)
}

fn steps(job: &Value) -> &[Value] {
    key(job, "steps").as_sequence().expect("job steps")
}

fn step_index(job: &Value, name: &str) -> usize {
    steps(job)
        .iter()
        .position(|step| str_of(step, "name") == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
}

/// `needs` as a list; a scalar `needs: job` is accepted (review DD); a job
/// without `needs` has none.
fn needs(jobs: &Value, job: &str) -> Vec<String> {
    match get(key(jobs, job), "needs") {
        None => Vec::new(),
        Some(Value::String(one)) => vec![one.clone()],
        Some(list) => list
            .as_sequence()
            .expect("needs list or string")
            .iter()
            .map(|need| need.as_str().expect("need string").to_string())
            .collect(),
    }
}

/// An `if:` condition with any `${{ … }}` wrapper and all whitespace removed.
fn condition(step: &Value) -> Option<String> {
    let raw: String = str_of(step, "if")?
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    Some(
        raw.strip_prefix("${{")
            .and_then(|inner| inner.strip_suffix("}}"))
            .unwrap_or(&raw)
            .to_string(),
    )
}

/// True when `run` invokes `--test <name>` as a whole token (review DD: flags
/// such as an added `--locked` do not matter).
fn runs_test(run: &str, name: &str) -> bool {
    let tokens: Vec<&str> = run.split_whitespace().collect();
    tokens.windows(2).any(|w| w == ["--test", name])
}

/// Review 2026-10-25: a step- or job-level `continue-on-error` is the exact
/// masking 18-0 closes; it must be absent (or literally `false`).
fn assert_no_continue_on_error(value: &Value, what: &str) {
    match get(value, "continue-on-error") {
        None | Some(Value::Bool(false)) => {}
        Some(other) => panic!("{what} must not carry continue-on-error (found {other:?})"),
    }
}

#[test]
fn proven_red_steps_run_after_a_red_gate_step() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    for (job_name, step, test) in [
        (
            "check-test-cost",
            "Run the 20-12 AC2 proven-red vectors",
            "check_test_cost_proven_red",
        ),
        (
            "check-test-budget",
            "Run the 20-12 AC1 proven-red vectors",
            "check_test_budget_proven_red",
        ),
    ] {
        let job = key(jobs, job_name);
        assert_no_continue_on_error(job, job_name);
        let step = &steps(job)[step_index(job, step)];
        let run = str_of(step, "run").expect("proven-red step run");
        assert!(runs_test(run, test), "'{run}' runs --test {test}");
        assert_eq!(
            condition(step).as_deref(),
            Some(NOT_CANCELLED),
            "'{run}' must run after a red gate step (18-0 AC6.3), not be skipped by it"
        );
        assert_no_continue_on_error(step, &format!("{job_name} step '{run}'"));
    }
}

#[test]
fn expensive_tests_uploads_its_junit_and_builds_no_fixtures() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let job = key(jobs, "expensive-tests");
    assert_no_continue_on_error(job, "expensive-tests");
    let run = step_index(job, "Run the registered expensive tests");
    assert_no_continue_on_error(&steps(job)[run], "the expensive-tests run step");
    let upload_at = step_index(job, "Upload the expensive-tests junit");
    assert!(upload_at > run, "the junit upload follows the run");
    let upload = &steps(job)[upload_at];
    assert!(
        str_of(upload, "uses").is_some_and(|uses| uses.starts_with("actions/upload-artifact@")),
        "the junit is uploaded with actions/upload-artifact"
    );
    assert_eq!(
        condition(upload).as_deref(),
        Some(NOT_CANCELLED),
        "a red expensive run still uploads its junit"
    );
    let with = key(upload, "with");
    assert_eq!(str_of(with, "name"), Some("expensive-tests-junit"));
    assert_eq!(str_of(with, "if-no-files-found"), Some("error"));

    // The uploaded path is where `--profile expensive` writes its junit.
    let nextest: toml::Value = toml::from_str(
        &fs::read_to_string(root().join(".config/nextest.toml")).expect("read nextest config"),
    )
    .expect("parse nextest config");
    let junit = nextest["profile"]["expensive"]["junit"]["path"]
        .as_str()
        .expect("profile.expensive.junit.path");
    assert_eq!(
        str_of(with, "path"),
        Some(format!("target/nextest/expensive/{junit}").as_str())
    );

    // D-18-0-09 A: no fixture build until a registered row needs one.
    assert!(
        steps(job)
            .iter()
            .filter_map(|step| str_of(step, "run"))
            .all(|run| !run.contains("build-test-fixtures")),
        "expensive-tests builds no fixtures (D-18-0-09)"
    );
}

/// D-18-0-10 A / D-15-4-F (and D-18-0-04's enrolment; D-18-1-11 A for
/// `epic-18-exit`): an unregistered job is enrolled in `aggregate.needs` and
/// in no other job's `needs`.
#[test]
fn unregistered_jobs_are_enrolled_in_aggregate_only() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    for unregistered in ["expensive-tests", "cargo-deny", "epic-18-exit"] {
        assert!(
            needs(jobs, "aggregate").contains(&unregistered.to_string()),
            "{unregistered} is in aggregate.needs"
        );
        for name in mapping(jobs).keys().filter_map(Value::as_str) {
            if name == "aggregate" {
                continue;
            }
            assert!(
                !needs(jobs, name).contains(&unregistered.to_string()),
                "{unregistered} must be enrolled in aggregate.needs only, not in {name}.needs"
            );
        }
    }
}

#[test]
fn cargo_deny_is_its_own_blocking_job_in_aggregate() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let job = key(jobs, "cargo-deny");
    assert_no_continue_on_error(job, "the supply-chain gate (D-18-0-04)");
    assert!(key(job, "timeout-minutes").as_u64().is_some());
    let check = &steps(job)[step_index(job, "cargo deny check")];
    assert!(
        str_of(check, "run").is_some_and(|run| run.starts_with("cargo deny check")),
        "the cargo deny check step runs cargo deny check"
    );
    for step in steps(job)
        .iter()
        .filter(|step| str_of(step, "run").is_some_and(|run| run.contains("cargo deny")))
    {
        assert_no_continue_on_error(step, "a cargo deny step");
    }
    // The advisory job no longer hides it.
    assert!(
        steps(key(jobs, "reproducible-build"))
            .iter()
            .filter_map(|step| str_of(step, "run"))
            .all(|run| !run.contains("cargo deny")),
        "cargo deny no longer runs inside the ADVISORY reproducible-build"
    );
}

const EXIT_LINE_1_STEP: &str =
    "Epic-18 exit line 1 — Butler's proposal through the Inference Port (Story 18-1)";
const EXIT_LINE_1_TWIN_STEP: &str =
    "Epic-18 exit line 1 twin — a planted-violation cassette must exit non-zero (Story 18-1)";

/// Exit line `n`, verbatim from the epic's hermetic exit block (the command
/// before its `# n —` comment).
fn epic_18_exit_line(n: u8) -> String {
    let epic = fs::read_to_string(
        root().join("_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md"),
    )
    .expect("read epic 18");
    let marker = format!("   # {n} —");
    epic.lines()
        .find_map(|line| {
            line.split_once(marker.as_str())
                .map(|(command, _)| command.trim().to_string())
        })
        .unwrap_or_else(|| panic!("epic 18 exit line {n}"))
}

/// Story 18-1 code review TIA-8/TIA-9 (D-18-1-11 A) — `epic-18-exit` keeps
/// its shape: exit line 1 verbatim, the binding witness, a twin that fails
/// when the planted violation exits 0, no masking, the pinned image.
#[test]
fn epic_18_exit_runs_line_1_verbatim_and_its_proven_red_twin() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let job = key(jobs, "epic-18-exit");
    assert_eq!(str_of(job, "runs-on"), Some("ubuntu-24.04"));
    assert_no_continue_on_error(job, "epic-18-exit");
    for step in steps(job) {
        assert_no_continue_on_error(step, "an epic-18-exit step");
    }
    let line = epic_18_exit_line(1);
    let (line_1, twin) = (
        step_index(job, EXIT_LINE_1_STEP),
        step_index(job, EXIT_LINE_1_TWIN_STEP),
    );
    let run = str_of(&steps(job)[line_1], "run").expect("line-1 run");
    assert!(
        run.contains(&line),
        "line 1 runs the epic's exit line verbatim: {line}"
    );
    assert!(
        run.contains("grep -q 'butler inference seam wired' line1.err"),
        "line 1 fails a Butler that never bound the Inference Port (TIA-9)"
    );
    assert!(
        run.contains("exit \"$rc\""),
        "line 1 exits with the command's code"
    );

    let run = str_of(&steps(job)[twin], "run").expect("twin run");
    let planted = line.replace(
        "MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json",
        "MAOS_REPLAY_CASSETTE=\"$twin\"",
    );
    assert!(
        run.contains(&planted),
        "the twin runs line 1 over the planted cassette"
    );
    assert!(
        run.contains("del completion[\"options\"]"),
        "the twin drops `options`"
    );
    let when_zero = run
        .split_once("if [ \"$rc\" -eq 0 ]; then")
        .and_then(|(_, rest)| rest.split_once("fi"))
        .map(|(block, _)| block)
        .expect("the twin checks rc == 0");
    assert!(
        when_zero.contains("exit 1"),
        "the twin exits 1 when the planted violation exits 0"
    );
}

const EXIT_LINE_2_STEP: &str =
    "Epic-18 exit line 2 — maos eval halt replays the Butler halt corpus (Story 18-2)";
const EXIT_LINE_2_TWIN_STEP: &str =
    "Epic-18 exit line 2 twin — the drift-blind cassette must miss overall recall (Story 18-2)";

/// Story 18-2 CR-4 — `epic-18-exit` runs exit line 2 verbatim (keyless,
/// scratch `HOME`), uploads the signed sidecar, and runs the drift-blind
/// twin, which fails the step when it exits 0 or does not name the
/// overall-recall floor (D-18-2-02 B). The nightly re-record leg records the
/// eval cassette (D-18-2-06 A) into the `.live.json` SIBLING, never the
/// committed seed (review ruling D-18-2-R3), and still uploads after a missed
/// live floor.
#[test]
fn epic_18_exit_runs_line_2_verbatim_and_its_drift_blind_twin() {
    let epic = fs::read_to_string(
        root().join("_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md"),
    )
    .expect("read epic 18");
    let line = epic
        .lines()
        .find_map(|line| {
            line.split_once("   # 2 —")
                .map(|(command, _)| command.trim())
        })
        .expect("epic 18 exit line 2");
    assert_eq!(line, "maos eval halt --class butler");

    let workflow = workflow();
    let job = key(key(&workflow, "jobs"), "epic-18-exit");
    let run = str_of(&steps(job)[step_index(job, EXIT_LINE_2_STEP)], "run").expect("line-2 run");
    assert!(run.contains(&format!("{line} > line2.out 2> line2.err")));
    assert!(run.contains("unset MAOS_OLLAMA_URL MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY"));
    assert!(run.contains("export HOME=\"$scratch\""));
    assert!(run.contains("benchmark.signed.json line2.benchmark.signed.json"));
    assert!(
        run.contains("exit \"$rc\""),
        "line 2 exits with the command's code"
    );

    let run = str_of(&steps(job)[step_index(job, EXIT_LINE_2_TWIN_STEP)], "run").expect("twin run");
    assert!(run.contains(&format!(
        "{line} --cassette spirits/butler/tests/fixtures/halt-corpus-v0.3.drift-blind.json"
    )));
    let when_zero = run
        .split_once("if [ \"$rc\" -eq 0 ]; then")
        .and_then(|(_, rest)| rest.split_once("fi"))
        .map(|(block, _)| block)
        .expect("the twin checks rc == 0");
    assert!(
        when_zero.contains("exit 1"),
        "the twin exits 1 when the drift-blind run exits 0"
    );
    assert!(run.contains("grep -q 'halt_recall_overall 0.800 below floor 0.90' line2-twin.err"));

    let upload = steps(job)
        .iter()
        .find(|step| str_of(step, "uses") == Some("actions/upload-artifact@v4"))
        .expect("epic-18-exit uploads");
    let paths = str_of(key(upload, "with"), "path").expect("upload paths");
    assert!(paths
        .lines()
        .any(|p| p.trim() == "line2.benchmark.signed.json"));

    let nightly: Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/journey-nightly.yml"))
            .expect("read journey-nightly"),
    )
    .expect("parse journey-nightly");
    let rerecord = key(key(&nightly, "jobs"), "tier-2-rerecord");
    let record =
        &steps(rerecord)[step_index(rerecord, "Re-record the Butler halt-eval cassette (live)")];
    let env = key(record, "env");
    assert_eq!(str_of(env, "MAOS_INFERENCE_MODE"), Some("record"));
    assert_eq!(
        str_of(env, "MAOS_REPLAY_CASSETTE"),
        Some("crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.live.json")
    );
    assert_eq!(
        str_of(record, "run"),
        Some("target/debug/maos eval halt --class butler")
    );
    let upload = &steps(rerecord)[step_index(rerecord, "Upload refreshed cassettes")];
    assert!(
        step_index(rerecord, "Upload refreshed cassettes")
            > step_index(rerecord, "Re-record the Butler halt-eval cassette (live)")
    );
    assert_eq!(condition(upload).as_deref(), Some(NOT_CANCELLED));

    // Story 18-3 TIA-8 (review R4) — the Researcher re-record step: its
    // `.live.json` sibling, the shipped verb, `!cancelled()` (it runs after a
    // missed Butler floor), before the upload, whose directory carries the
    // `.live.verdicts.json` judge sidecar.
    const RESEARCHER_STEP: &str = "Re-record the Researcher five-metric cassette and judge (live)";
    const LIVE: &str =
        "crates/maos-journey-test/cassettes/j-researcher/distill-corpus-v0.live.json";
    let researcher = &steps(rerecord)[step_index(rerecord, RESEARCHER_STEP)];
    let env = key(researcher, "env");
    assert_eq!(str_of(env, "MAOS_INFERENCE_MODE"), Some("record"));
    assert_eq!(str_of(env, "MAOS_REPLAY_CASSETTE"), Some(LIVE));
    assert_eq!(
        str_of(researcher, "run"),
        Some("target/debug/maos eval distill --class researcher")
    );
    assert_eq!(condition(researcher).as_deref(), Some(NOT_CANCELLED));
    assert!(
        step_index(rerecord, "Upload refreshed cassettes") > step_index(rerecord, RESEARCHER_STEP)
    );
    let uploaded = str_of(key(upload, "with"), "path").expect("upload path");
    assert!(
        "crates/maos-journey-test/cassettes/j-researcher/distill-corpus-v0.live.verdicts.json"
            .starts_with(uploaded.trim()),
        "the upload carries the judge sidecar: {uploaded}"
    );
}

const EXIT_LINE_3_STEP: &str =
    "Epic-18 exit line 3 — Researcher replays through the Inference Port (Story 18-3)";
const EXIT_LINE_3_TWIN_STEP: &str =
    "Epic-18 exit line 3 twin — an empty cassette must exit non-zero (Story 18-3)";
const SURVEY_CASSETTE: &str =
    "MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/survey-distill.json";

/// Story 18-3 CR-3 — `epic-18-exit` runs exit line 3 verbatim (keyless,
/// scratch `HOME`): the binding witness, one `researcher_survey` event with
/// the four survey keys and one completion per cassette entry; its twin runs
/// the same line over an EMPTY cassette and fails when that exits 0 or when
/// stderr's refusal line does not itself name `exhausted` (TIA-3: the replay
/// port prints `CASSETTE EXHAUSTED` on its own). TIA-6 (`deferred-work.md:1248`):
/// maosctl is built and `maosctl spirit benchmark butler` reads the sidecar
/// line 2 wrote, under line 2's scratch `HOME`.
#[test]
fn epic_18_exit_runs_line_3_verbatim_and_its_empty_cassette_twin() {
    let line = epic_18_exit_line(3);
    assert!(
        line.contains(SURVEY_CASSETTE),
        "epic line 3 replays the survey cassette"
    );
    let workflow = workflow();
    let job = key(key(&workflow, "jobs"), "epic-18-exit");
    assert_no_continue_on_error(job, "epic-18-exit");
    for step in steps(job) {
        assert_no_continue_on_error(step, "an epic-18-exit step");
    }
    let (line_3, twin) = (
        step_index(job, EXIT_LINE_3_STEP),
        step_index(job, EXIT_LINE_3_TWIN_STEP),
    );
    assert_eq!(twin, line_3 + 1, "the twin follows line 3");

    let run = str_of(&steps(job)[line_3], "run").expect("line-3 run");
    assert!(run.contains(&format!("{line} > line3.out 2> line3.err")));
    assert!(run.contains("unset MAOS_OLLAMA_URL MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY"));
    assert!(run.contains("scratch=\"$(mktemp -d)\""));
    assert!(run.contains("export HOME=\"$scratch\""));
    assert!(run.contains("grep -q 'researcher inference seam wired' line3.err"));
    assert!(run.contains(
        "entries=\"$(jq '.entries | length' crates/maos-journey-test/cassettes/j-researcher/survey-distill.json)\""
    ));
    for needle in [
        ".event == \"researcher_survey\"",
        "has(\"findings\")",
        "has(\"open_questions\")",
        "has(\"confidence_map\")",
        "has(\"bibliography\")",
        ".completions_consumed == $n",
        "--argjson n \"$entries\"",
    ] {
        assert!(run.contains(needle), "line 3 asserts {needle}");
    }
    assert!(
        run.contains("exit \"$rc\""),
        "line 3 exits with the command's code"
    );

    let run = str_of(&steps(job)[twin], "run").expect("twin run");
    assert!(run.contains(
        "jq '.entries = []' crates/maos-journey-test/cassettes/j-researcher/survey-distill.json > \"$twin\""
    ));
    let empty = line.replace(SURVEY_CASSETTE, "MAOS_REPLAY_CASSETTE=\"$twin\"");
    assert!(
        run.contains(&format!("{empty} > line3-twin.out 2> line3-twin.err")),
        "the twin runs line 3 over the empty cassette"
    );
    let when_zero = run
        .split_once("if [ \"$rc\" -eq 0 ]; then")
        .and_then(|(_, rest)| rest.split_once("fi"))
        .map(|(block, _)| block)
        .expect("the twin checks rc == 0");
    assert!(
        when_zero.contains("exit 1"),
        "the twin exits 1 when the empty cassette exits 0"
    );
    assert!(run.contains("grep -qE 'researcher on_idle pass refused: .*exhausted' line3-twin.err"));
    assert!(
        !run.contains("grep -q 'exhausted' line3-twin.err"),
        "the cause is matched on the refusal line, not anywhere in stderr"
    );

    // TIA-6: maosctl is built and reads line 2's sidecar in line 2's step.
    assert!(steps(job)
        .iter()
        .filter_map(|step| str_of(step, "run"))
        .any(|run| run.contains("cargo build --locked -p maos-cli --bin maosctl")));
    let run = str_of(&steps(job)[step_index(job, EXIT_LINE_2_STEP)], "run").expect("line-2 run");
    let (eval, bench) = (
        run.find("maos eval halt --class butler")
            .expect("line 2 evals"),
        run.find("target/debug/maosctl spirit benchmark butler")
            .expect("line 2 runs maosctl spirit benchmark butler"),
    );
    assert!(
        bench > eval,
        "maosctl reads the sidecar after eval halt wrote it"
    );
    assert!(run.contains("bench_rc=$?"));
    assert!(run.contains("if [ \"$bench_rc\" -ne 0 ]; then\n  rc=1"));

    let upload = steps(job)
        .iter()
        .find(|step| str_of(step, "uses") == Some("actions/upload-artifact@v4"))
        .expect("epic-18-exit uploads");
    let paths: Vec<&str> = str_of(key(upload, "with"), "path")
        .expect("upload paths")
        .lines()
        .map(str::trim)
        .collect();
    for path in [
        "line2.maosctl.out",
        "line2.maosctl.err",
        "line3.out",
        "line3.err",
        "line3-twin.out",
        "line3-twin.err",
    ] {
        assert!(paths.contains(&path), "epic-18-exit uploads {path}");
    }
}

const EXIT_LINE_4_STEP: &str =
    "Epic-18 exit line 4 — 21 compressed days and the variance-led halt (Story 18-4)";
const EXIT_LINE_4_TWIN_STEP: &str =
    "Epic-18 exit line 4 twin — the steady director must never halt (Story 18-4)";
const ACCEPTANCE_FIXTURE: &str = "--acceptance tests/fixtures/j-butler/acceptance-21d.jsonl";

/// Story 18-4 AC1.4 — `epic-18-exit` runs exit line 4 verbatim (keyless,
/// scratch `HOME`): the binding witness, exactly one `belief_variance` halt on
/// a day ≤ 21 at a pooled rate ≥ 0.6, one decided offer and one digest per
/// simulated day; its steady twin runs the same line over the steady fixture
/// and fails on any halt or a short scene (the standing red for line 4).
#[test]
fn epic_18_exit_runs_line_4_verbatim_and_its_steady_twin() {
    let line = epic_18_exit_line(4);
    assert!(line.contains("--clock compressed:21d") && line.contains(ACCEPTANCE_FIXTURE));
    let workflow = workflow();
    let job = key(key(&workflow, "jobs"), "epic-18-exit");
    for step in steps(job) {
        assert_no_continue_on_error(step, "an epic-18-exit step");
    }
    let (line_4, twin) = (
        step_index(job, EXIT_LINE_4_STEP),
        step_index(job, EXIT_LINE_4_TWIN_STEP),
    );
    assert_eq!(twin, line_4 + 1, "the twin follows line 4");
    let run = str_of(&steps(job)[line_4], "run").expect("line-4 run");
    assert!(run.contains(&format!("{line} > line4.out 2> line4.err")));
    assert!(run.contains("unset MAOS_OLLAMA_URL MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY"));
    assert!(run.contains("export HOME=\"$scratch\""));
    assert!(run.contains("grep -q 'butler inference seam wired' line4.err"));
    for needle in [
        ".render == \"halted on belief_variance\"",
        ".day <= 21",
        ".diagnostic.rate >= 0.6",
        "select(.event == \"acceptance_day\")] | length) == ($d - 1)",
        "select(.event == \"morning_digest\")] | length) == $d",
    ] {
        assert!(run.contains(needle), "line 4 asserts {needle}");
    }
    assert!(
        run.contains("exit \"$rc\""),
        "line 4 exits with the command's code"
    );

    let run = str_of(&steps(job)[twin], "run").expect("twin run");
    let steady = line.replace(
        ACCEPTANCE_FIXTURE,
        "--acceptance tests/fixtures/j-butler/acceptance-21d-steady.jsonl",
    );
    assert!(run.contains(&format!("{steady} > line4-twin.out 2> line4-twin.err")));
    for needle in [
        "select(.event == \"halt\")] | length) == 0",
        "select(.event == \"acceptance_day\")] | length) == 21",
    ] {
        assert!(run.contains(needle), "the twin asserts {needle}");
    }
    let upload = steps(job)
        .iter()
        .find(|step| str_of(step, "uses") == Some("actions/upload-artifact@v4"))
        .expect("epic-18-exit uploads");
    let paths = str_of(key(upload, "with"), "path").expect("upload paths");
    for path in ["line4.out", "line4.err", "line4-twin.out", "line4-twin.err"] {
        assert!(paths.lines().any(|p| p.trim() == path), "uploads {path}");
    }
}

/// Story 18-3 AC2.3 / CR-3 — the NFR-Aud-7 job scores the Researcher's
/// frozen corpus through the shipped verb (keyless) and requires `pass ==
/// true`, keeps the maos-eval floor test, and runs the Digest-class gate
/// (AC4.2, D-18-3-08 B).
#[test]
fn nfr_aud_7_runs_maos_eval_distill_for_the_researcher() {
    let workflow = workflow();
    let job = key(
        key(&workflow, "jobs"),
        "nfr-aud-7-distillate-five-metrics-floor",
    );
    assert_no_continue_on_error(job, "nfr-aud-7-distillate-five-metrics-floor");
    let runs: Vec<&str> = steps(job)
        .iter()
        .inspect(|step| assert_no_continue_on_error(step, "an nfr-aud-7 step"))
        .filter_map(|step| str_of(step, "run"))
        .collect();
    assert!(runs
        .iter()
        .any(|run| runs_test(run, "distillate_five_metrics_floor")));
    assert!(runs
        .iter()
        .any(|run| runs_test(run, "five_metric_digest_18_3")));
    assert!(runs
        .iter()
        .any(|run| run.contains("cargo build --locked -p maos-bin --bin maos")));
    let eval = runs
        .iter()
        .find(|run| run.contains("target/debug/maos eval distill --class researcher"))
        .expect("the job runs maos eval distill --class researcher");
    assert!(eval.contains("unset MAOS_OLLAMA_URL MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY"));
    assert!(eval.contains("export HOME=\"$scratch\""));
    assert!(eval.contains(".event == \"eval_distill\""));
    assert!(eval.contains(".pass == true"));
    assert!(eval.contains("exit \"$rc\""));
}
