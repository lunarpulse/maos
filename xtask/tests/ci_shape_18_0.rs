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

fn recall_context_gate_is_complete(run: &str) -> bool {
    run.contains("binary_id(maos-bin::recall_17_3d)")
        && run.contains("18 tests run: 18 passed([,[:space:]]|$)")
        && run.contains("missing '18 tests run: 18 passed'")
        && run.contains("--expected-count 18")
        && run.contains(
            "context_18_5_report_status_and_journaled_kinds_32_33_reach_wasm_handle_frame",
        )
        && run.contains("context_18_5_wasm_guest_cannot_forge_kernel_pressure_or_limit_to_peer")
}

#[test]
fn recall_gate_enrolls_both_context_vectors_and_pins_18_passes() {
    let workflow = workflow();
    let job = key(key(&workflow, "jobs"), "wasm-host-tests");
    let step = &steps(job)[step_index(
        job,
        "In-band recall through the admitted session (Story 17-3d)",
    )];
    let run = str_of(step, "run").expect("recall gate run");
    assert!(recall_context_gate_is_complete(run));
    // Plants: each stale count and each missing context name must independently red.
    for token in [
        "18 tests run: 18 passed([,[:space:]]|$)",
        "missing '18 tests run: 18 passed'",
        "--expected-count 18",
        "context_18_5_report_status_and_journaled_kinds_32_33_reach_wasm_handle_frame",
        "context_18_5_wasm_guest_cannot_forge_kernel_pressure_or_limit_to_peer",
    ] {
        assert!(
            !recall_context_gate_is_complete(&run.replace(token, "REMOVED")),
            "gate must reject missing {token}"
        );
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
        run.contains("grep -q 'butler inference seam wired' \"$RUNNER_TEMP/line1.err\""),
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
    assert!(run.contains(&format!(
        "{line} > \"$RUNNER_TEMP/line2.out\" 2> \"$RUNNER_TEMP/line2.err\""
    )));
    assert!(run.contains("unset MAOS_OLLAMA_URL MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY"));
    assert!(run.contains("export HOME=\"$scratch\""));
    assert!(run.contains("benchmark.signed.json \"$RUNNER_TEMP/line2.benchmark.signed.json\""));
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
    assert!(run.contains(
        "grep -q 'halt_recall_overall 0.800 below floor 0.90' \"$RUNNER_TEMP/line2-twin.err\""
    ));

    let upload = steps(job)
        .iter()
        .find(|step| str_of(step, "uses") == Some("actions/upload-artifact@v4"))
        .expect("epic-18-exit uploads");
    let paths = str_of(key(upload, "with"), "path").expect("upload paths");
    assert!(paths
        .lines()
        .any(|p| p.trim() == "${{ runner.temp }}/line2.benchmark.signed.json"));

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
/// port prints `CASSETTE EXHAUSTED` on its own). TIA-6: producer-built maosctl
/// reads the sidecar line 2 wrote, under line 2's scratch `HOME`.
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
    assert!(run.contains(&format!(
        "{line} > \"$RUNNER_TEMP/line3.out\" 2> \"$RUNNER_TEMP/line3.err\""
    )));
    assert!(run.contains("unset MAOS_OLLAMA_URL MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY"));
    assert!(run.contains("scratch=\"$(mktemp -d)\""));
    assert!(run.contains("export HOME=\"$scratch\""));
    assert!(run.contains("grep -q 'researcher inference seam wired' \"$RUNNER_TEMP/line3.err\""));
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
        "survey_entries=$((entries - 1))",
        "--argjson n \"$survey_entries\"",
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
        run.contains(&format!(
            "{empty} > \"$RUNNER_TEMP/line3-twin.out\" 2> \"$RUNNER_TEMP/line3-twin.err\""
        )),
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
    assert!(run.contains(
        "grep -qE 'researcher on_idle pass refused: .*exhausted' \"$RUNNER_TEMP/line3-twin.err\""
    ));
    assert!(
        !run.contains("grep -q 'exhausted' \"$RUNNER_TEMP/line3-twin.err\""),
        "the cause is matched on the refusal line, not anywhere in stderr"
    );

    // TIA-6: the producer builds maosctl; the consumer requires that executable.
    let producer = key(key(&workflow, "jobs"), "ci-test-producer");
    assert!(steps(producer)
        .iter()
        .filter_map(|step| str_of(step, "run"))
        .any(|run| run.contains("build-test-fixtures") && run.contains("maosctl")));
    let prerequisite = &steps(job)[step_index(job, "Build maosctl")];
    assert_eq!(
        str_of(prerequisite, "run"),
        Some("test -x target/debug/maosctl")
    );
    assert!(
        step_index(job, "Restore exact producer workspace files")
            < step_index(job, "Build maosctl")
    );
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
        "${{ runner.temp }}/line2.maosctl.out",
        "${{ runner.temp }}/line2.maosctl.err",
        "${{ runner.temp }}/line3.out",
        "${{ runner.temp }}/line3.err",
        "${{ runner.temp }}/line3-twin.out",
        "${{ runner.temp }}/line3-twin.err",
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
    assert!(run.contains(&format!(
        "{line} > \"$RUNNER_TEMP/line4.out\" 2> \"$RUNNER_TEMP/line4.err\""
    )));
    assert!(run.contains("unset MAOS_OLLAMA_URL MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY"));
    assert!(run.contains("export HOME=\"$scratch\""));
    assert!(run.contains("grep -q 'butler inference seam wired' \"$RUNNER_TEMP/line4.err\""));
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
    assert!(run.contains(&format!(
        "{steady} > \"$RUNNER_TEMP/line4-twin.out\" 2> \"$RUNNER_TEMP/line4-twin.err\""
    )));
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
    for path in [
        "${{ runner.temp }}/line4.out",
        "${{ runner.temp }}/line4.err",
        "${{ runner.temp }}/line4-twin.out",
        "${{ runner.temp }}/line4-twin.err",
    ] {
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

const ARCHIVE_CONSUMERS: [&str; 5] = [
    "wasm-host-tests",
    "cgroup-ceiling-delegated",
    "cgroup-ceiling-undelegated",
    "epic-17-exit",
    "epic-18-exit",
];

/// Consumers need no Cargo driver at all: even `cargo +stable --locked build`,
/// an absolute or quoted Cargo path (`"$HOME/.cargo/bin/cargo" build`), a command
/// substitution (`$(command -v cargo) build`) or an inline `RUSTC=`/`CARGO=`
/// reassignment must not become an accidental fallback.
fn consumer_refuses_compilation(job: &Value) -> bool {
    let compiler = regex::Regex::new(r#"(?:^|[\s/;'"|&(])(?:cargo|rustc)(?:[\s'")]|$)"#)
        .expect("compiler command regex");
    let reassigned = regex::Regex::new(r#"(?:^|[\s;&|('"])(?:RUSTC|CARGO|CARGO_BUILD_RUSTC)="#)
        .expect("compiler reassignment regex");
    let poisoned = |env: &Value| {
        ["RUSTC", "CARGO_BUILD_RUSTC", "CARGO"]
            .iter()
            .all(|name| str_of(env, name) == Some("/bin/false"))
    };
    get(job, "env").is_some_and(poisoned)
        && steps(job).iter().all(|step| {
            let uses = str_of(step, "uses").unwrap_or("");
            !uses.starts_with("dtolnay/rust-toolchain@")
                && !uses.starts_with("Swatinem/rust-cache@")
                && !uses.starts_with("actions/cache@")
                && str_of(step, "run")
                    .is_none_or(|run| !compiler.is_match(run) && !reassigned.is_match(run))
                && get(step, "env").is_none_or(|env| {
                    ["RUSTC", "CARGO_BUILD_RUSTC", "CARGO"]
                        .iter()
                        .all(|name| str_of(env, name).is_none_or(|value| value == "/bin/false"))
                })
        })
}

fn fixture_transport_is_explicit(workflow: &Value) -> bool {
    fn mode(scope: &Value) -> Option<&str> {
        get(scope, "env").and_then(|env| str_of(env, "MAOS_FIXTURE_TRANSPORT"))
    }
    mode(workflow).is_none()
        && mapping(key(workflow, "jobs")).iter().all(|(name, job)| {
            let name = name.as_str().expect("job name");
            let selected = name == "ci-test-producer" || ARCHIVE_CONSUMERS.contains(&name);
            (if selected {
                mode(job) == Some("1")
            } else {
                mode(job).is_none()
            }) && steps(job).iter().all(|step| mode(step).is_none())
        })
}

#[test]
fn portable_transport_is_opted_in_only_for_the_selected_producer_and_consumers() {
    let workflow = workflow();
    assert!(fixture_transport_is_explicit(&workflow));
    assert!(
        get(key(key(&workflow, "jobs"), "workspace-test-suite"), "env")
            .and_then(|env| get(env, "MAOS_FIXTURE_TRANSPORT"))
            .is_none()
    );
    for name in std::iter::once("ci-test-producer").chain(ARCHIVE_CONSUMERS) {
        let mut missing = workflow.clone();
        missing["jobs"][name]["env"]
            .as_mapping_mut()
            .expect("selected job env")
            .remove(Value::String("MAOS_FIXTURE_TRANSPORT".into()));
        assert!(
            !fixture_transport_is_explicit(&missing),
            "{name} lost opt-in"
        );
        let mut overridden = workflow.clone();
        overridden["jobs"][name]["steps"][0]["env"] =
            serde_yaml::from_str("MAOS_FIXTURE_TRANSPORT: '0'").expect("override plant");
        assert!(
            !fixture_transport_is_explicit(&overridden),
            "{name} disabled transport"
        );
    }
    for scope in ["global", "workspace-job", "workspace-step"] {
        let mut leaked = workflow.clone();
        let target = match scope {
            "global" => &mut leaked,
            "workspace-job" => &mut leaked["jobs"]["workspace-test-suite"],
            _ => &mut leaked["jobs"]["workspace-test-suite"]["steps"][0],
        };
        target["env"] = serde_yaml::from_str("MAOS_FIXTURE_TRANSPORT: '1'").expect("leak plant");
        assert!(
            !fixture_transport_is_explicit(&leaked),
            "{scope} leaked transport"
        );
    }
}

#[test]
fn archive_consumers_refuse_compilation_and_compile_plants() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    for name in ARCHIVE_CONSUMERS {
        let job = key(jobs, name);
        assert!(
            consumer_refuses_compilation(job),
            "{name} must refuse compilation"
        );
        assert_no_continue_on_error(job, name);
        for step in steps(job) {
            assert_no_continue_on_error(step, name);
        }
        // Mutate the actual consumer, not a stand-alone regex example. Each
        // plant keeps the archive wiring and poison environment intact.
        for command in [
            "cargo build --locked -p maos-bin",
            "cargo --locked test -p maos-bin",
            "/usr/bin/cargo +stable run -p xtask -- build-test-fixtures",
            "rustc tests/fixtures/native/cgroup_ceiling.rs",
            "\"$HOME/.cargo/bin/cargo\" build -p maos-bin",
            "$(command -v cargo) build -p maos-bin",
            "'rustc' tests/fixtures/native/cgroup_ceiling.rs",
            "export RUSTC=/opt/toolchain/bin/rustc-1.85",
            "CARGO=/opt/toolchain/bin/driver ./scripts/build.sh",
            "env CARGO_BUILD_RUSTC=/opt/toolchain/bin/rc ./scripts/build.sh",
        ] {
            let mut planted = job.clone();
            planted["steps"]
                .as_sequence_mut()
                .expect("consumer steps")
                .push(
                    serde_yaml::from_str(&format!("run: |\n  {command}\n")).expect("compile plant"),
                );
            assert!(
                !consumer_refuses_compilation(&planted),
                "{name} accepted the compile plant {command}"
            );
        }
        let mut unpoisoned = job.clone();
        unpoisoned["env"]["RUSTC"] = Value::String("rustc".into());
        assert!(!consumer_refuses_compilation(&unpoisoned));
    }
}

/// The consumer compares the version line of the cargo-nextest it extracted —
/// the binary every later step runs — with the producer's record of the packed
/// binary, after extraction and before trusting the bundle.
fn restore_binds_packed_nextest(restore: &str) -> bool {
    let extract =
        "tar -C \"$RUNNER_TEMP/ci-test-tools\" -xf \"$RUNNER_TEMP/ci-test-files/tools.tar\"";
    let version = "nextest = subprocess.check_output([os.environ['RUNNER_TEMP'] + \
                   '/ci-test-tools/cargo-nextest', 'nextest', '--version'], \
                   text=True).splitlines()[0]";
    let (Some(extracted), Some(checked)) =
        (restore.find(extract), restore.find("python3 - <<'PY'"))
    else {
        return false;
    };
    extracted < checked
        && restore.contains(version)
        && restore.contains("nextest=nextest)")
        && !restore.contains("nextest='")
        && restore.contains("assert producer == expected, (producer, expected)")
}

/// Archive consumers replace `cargo test`, which ran every test and killed none:
/// the systemd unit `--timeout` or the job timeout bounds a hang. A consumer
/// remapped to the repo root loads its `.config/nextest.toml` and selects
/// `archive-consumer` (no fail-fast, a slow-timeout that never terminates; the
/// default profile's overrides apply to it too). The native-twin remap is its own
/// workspace without that config — nextest's defaults kill nothing — so it only
/// adds `--no-fail-fast`.
fn consumer_runs_are_complete(job: &Value, config: &toml::Value) -> Result<(), String> {
    let profiles = config.get("profile").ok_or("no [profile] tables")?;
    let profile = profiles
        .get("archive-consumer")
        .ok_or("no [profile.archive-consumer]")?;
    if profile.get("fail-fast").and_then(toml::Value::as_bool) != Some(false) {
        return Err("archive-consumer must set fail-fast = false".into());
    }
    match profile.get("slow-timeout") {
        Some(toml::Value::String(_)) => {}
        Some(toml::Value::Table(table)) if !table.contains_key("terminate-after") => {}
        other => return Err(format!("archive-consumer slow-timeout may kill: {other:?}")),
    }
    for scope in ["default", "archive-consumer"] {
        let Some(scope) = profiles.get(scope) else {
            continue;
        };
        if scope.get("global-timeout").is_some() {
            return Err("a global-timeout kills archive consumers".into());
        }
        let overrides = scope.get("overrides").and_then(toml::Value::as_array);
        if overrides
            .into_iter()
            .flatten()
            .any(|o| o.get("slow-timeout").is_some())
        {
            return Err("an override's slow-timeout reaches archive consumers".into());
        }
    }
    for run in steps(job).iter().filter_map(|step| str_of(step, "run")) {
        for command in run
            .lines()
            .filter(|line| line.contains("cargo-nextest nextest run"))
        {
            let tokens: Vec<_> = command.split_whitespace().collect();
            let has = |pair: [&str; 2]| tokens.windows(2).any(|w| w == pair);
            let complete = if command.contains("--workspace-remap \"$GITHUB_WORKSPACE\" ") {
                has(["--profile", "archive-consumer"])
            } else {
                tokens.contains(&"--no-fail-fast") && !tokens.contains(&"--profile")
            };
            if !complete
                || tokens
                    .iter()
                    .any(|t| matches!(*t, "--fail-fast" | "--max-fail"))
            {
                return Err(format!("consumer may stop early or kill a test: {command}"));
            }
        }
    }
    Ok(())
}

fn plant_runs(job: &Value, from: &str, to: &str) -> Value {
    let mut planted = job.clone();
    for step in planted["steps"].as_sequence_mut().expect("steps") {
        let run = str_of(step, "run").map(|run| run.replace(from, to));
        if let Some(run) = run {
            step["run"] = Value::String(run);
        }
    }
    planted
}

#[test]
fn archive_consumers_run_every_test_without_a_per_test_kill() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let text =
        fs::read_to_string(root().join(".config/nextest.toml")).expect("read nextest config");
    let config: toml::Value = toml::from_str(&text).expect("parse nextest config");
    for name in ARCHIVE_CONSUMERS {
        assert_eq!(
            consumer_runs_are_complete(key(jobs, name), &config),
            Ok(()),
            "{name}"
        );
    }
    // The rootless canary is the one consumer command built outside the workflow.
    let helper = fs::read_to_string(root().join("scripts/run_cgroup_ceiling_17_2.py"))
        .expect("read cgroup helper");
    assert!(helper.contains(
        "\"--extract-overwrite\", \"--color\", \"never\", \"--profile\", \"archive-consumer\","
    ));
    let job = key(jobs, "wasm-host-tests");
    for (from, to) in [
        (
            " --profile archive-consumer --no-capture --test-threads 1 ",
            " --no-capture --test-threads 1 ",
        ),
        (" --color never --no-fail-fast ", " --color never "),
        (
            " --profile archive-consumer ",
            " --profile archive-consumer --fail-fast ",
        ),
    ] {
        let planted = plant_runs(job, from, to);
        assert_ne!(&planted, job, "consumer plant must change the job: {to}");
        assert!(
            consumer_runs_are_complete(&planted, &config).is_err(),
            "{to}"
        );
    }
    for (from, to) in [
        (
            "slow-timeout = { period = \"60s\" }",
            "slow-timeout = { period = \"60s\", terminate-after = 4 }",
        ),
        ("slow-timeout = { period = \"60s\" }\n", ""),
        (
            "[profile.archive-consumer]\nfail-fast = false\n",
            "[profile.archive-consumer]\n",
        ),
        (
            "test-group = \"serial\"\n",
            "test-group = \"serial\"\nslow-timeout = \"1s\"\n",
        ),
    ] {
        let planted = text.replacen(from, to, 1);
        assert_ne!(planted, text, "config plant must change the file: {to}");
        let planted: toml::Value = toml::from_str(&planted).expect("parse planted config");
        assert!(consumer_runs_are_complete(job, &planted).is_err(), "{to}");
    }
}

#[test]
fn archive_transport_is_same_run_same_arch_and_restores_absolute_paths() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let producer = key(jobs, "ci-test-producer");
    assert_eq!(
        key(key(producer, "strategy"), "fail-fast"),
        &Value::Bool(false)
    );
    let include = key(key(key(producer, "strategy"), "matrix"), "include")
        .as_sequence()
        .expect("producer matrix");
    assert_eq!(include.len(), 2, "one producer per architecture");
    for (entry, arch, image) in [
        (&include[0], "x64", "ubuntu-24.04"),
        (&include[1], "arm64", "ubuntu-24.04-arm"),
    ] {
        assert_eq!(str_of(entry, "arch"), Some(arch));
        assert_eq!(str_of(entry, "image"), Some(image));
    }
    assert_eq!(
        str_of(key(producer, "env"), "CARGO_TARGET_DIR"),
        Some("${{ github.workspace }}/target")
    );
    for bundle in ["archives", "files"] {
        let artifact = format!("ci-test-{bundle}-${{{{ matrix.arch }}}}-${{{{ github.sha }}}}");
        let upload = steps(producer)
            .iter()
            .find(|step| {
                str_of(step, "uses") == Some("actions/upload-artifact@v4")
                    && get(step, "with")
                        .is_some_and(|with| str_of(with, "name") == Some(artifact.as_str()))
            })
            .expect("producer upload");
        assert_eq!(
            str_of(key(upload, "with"), "if-no-files-found"),
            Some("error")
        );
        assert_eq!(key(key(upload, "with"), "retention-days").as_u64(), Some(1));
        // A same-run re-run attempt re-uploads the same name; without overwrite it 409s.
        assert_eq!(key(key(upload, "with"), "overwrite").as_bool(), Some(true));
    }
    for name in ARCHIVE_CONSUMERS {
        let job = key(jobs, name);
        assert!(needs(jobs, name)
            .iter()
            .any(|need| need == "ci-test-producer"));
        let arch = if matches!(name, "epic-17-exit" | "epic-18-exit") {
            "x64"
        } else {
            "${{ endsWith(matrix.image, '-arm') && 'arm64' || 'x64' }}"
        };
        let restore_at = step_index(job, "Restore exact producer workspace files");
        for bundle in ["archives", "files"] {
            if name == "epic-18-exit" && bundle == "archives" {
                continue; // This job executes debug binaries, not test binaries.
            }
            let artifact = format!("ci-test-{bundle}-{arch}-${{{{ github.sha }}}}");
            let download_at = steps(job)
                .iter()
                .position(|step| {
                    str_of(step, "uses") == Some("actions/download-artifact@v4")
                        && get(step, "with")
                            .is_some_and(|with| str_of(with, "name") == Some(artifact.as_str()))
                })
                .unwrap_or_else(|| panic!("{name} downloads {artifact}"));
            assert!(download_at < restore_at);
        }
        let restore = str_of(&steps(job)[restore_at], "run").expect("restore command");
        for witness in [
            "GITHUB_SHA",
            "GITHUB_RUN_ID",
            "GITHUB_WORKSPACE",
            "HEAD^{tree}",
            "assert producer == expected",
            "nextest=nextest)",
            "tar -C / -xf",
            "fixtures-debug.tar",
        ] {
            assert!(
                restore.contains(witness),
                "{name} restores identity-bound files: {witness}"
            );
        }
        assert!(
            restore_binds_packed_nextest(restore),
            "{name} compares the extracted cargo-nextest's version"
        );
        // A literal-vs-literal comparison, or reading the version before the
        // binary is extracted, no longer binds the binary that will run.
        let literal = restore.replace("nextest=nextest)", "nextest='0.9.149')");
        let extract =
            "tar -C \"$RUNNER_TEMP/ci-test-tools\" -xf \"$RUNNER_TEMP/ci-test-files/tools.tar\"\n";
        let late = format!("{}{extract}", restore.replacen(extract, "", 1));
        for planted in [literal, late] {
            assert_ne!(
                planted, restore,
                "{name}: restore plant must change the step"
            );
            assert!(!restore_binds_packed_nextest(&planted), "{name}: {planted}");
        }
        for run in steps(job).iter().filter_map(|step| str_of(step, "run")) {
            for command in run
                .lines()
                .filter(|line| line.contains("cargo-nextest nextest run"))
            {
                assert!(command.contains("--archive-file "), "{name}: {command}");
                assert!(
                    command.contains("--extract-to \"$GITHUB_WORKSPACE\""),
                    "{name}: {command}"
                );
                assert!(
                    command.contains("--workspace-remap \"$GITHUB_WORKSPACE"),
                    "{name}: {command}"
                );
                assert!(command.contains("--color never"), "{name}: {command}");
            }
        }
    }
    // The budget remains attached to the workspace job's own junit.
    assert_eq!(needs(jobs, "check-test-budget"), ["workspace-test-suite"]);
    assert!(steps(key(jobs, "workspace-test-suite"))
        .iter()
        .filter_map(|step| str_of(step, "run"))
        .any(|run| run.contains("cargo nextest run") && run.contains("--workspace")));
}

/// Producer families: `(package/features selector, archived suites, archive name)`.
const PRODUCER_FAMILIES: [(&str, &[&str], &str); 7] = [
    (
        "-p maos-bin --features network,wasm-host",
        &[
            "spawned_scope_17_3c",
            "worker_resource_limits_17_2",
            "live_escape_anomaly_17_10",
            "recall_17_3d",
            "participant_recall_17_8",
            "smoke_cli_wrapper_8_12",
        ],
        "maos-bin-network-wasm",
    ),
    (
        "-p maos-bin --features network",
        &[
            "worker_terminal_facts_17_7",
            "cli_wrapper_exhaustion_17_9",
            "observer_live_17_4",
            "observer_activity_corpus_17_4",
        ],
        "maos-bin-network",
    ),
    (
        "-p maos-kernel-core",
        &[
            "cgroup_ceiling_smoke",
            "sandbox_enforcement_linux",
            "bridge_t2_route_17_6",
        ],
        "kernel",
    ),
    ("-p maos-wasm-host", &["t2_sandbox_kill"], "wasm-host-t2"),
    ("-p maos-escape-detector", &[], "escape-detector"),
    ("-p maos-host -p maos-wasm-host", &[], "host-wasm-host"),
    (
        "--manifest-path crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.toml",
        &[],
        "native-twin",
    ),
];

/// Each logical `cargo nextest archive` command (backslash-newline continuations
/// joined) must be exactly one family: its own selector, its own `--test` set and its
/// own `--archive-file`; every family appears once. A suite bound in a sibling
/// family's command is refused even though it still appears in the script.
fn producer_families_are_exact(script: &str) -> Result<(), String> {
    let mut seen = Vec::new();
    for line in script.replace("\\\n", " ").lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if !tokens.windows(2).any(|w| w == ["nextest", "archive"]) {
            continue;
        }
        let mut rest = tokens
            .strip_prefix(&["cargo", "nextest", "archive", "--locked"][..])
            .ok_or_else(|| format!("unlocked archive command: {line}"))?;
        let (mut selector, mut suites, mut archive) = (Vec::new(), Vec::new(), None);
        while let Some((&token, tail)) = rest.split_first() {
            match (token, tail.first()) {
                ("--test", Some(&suite)) => suites.push(suite),
                ("--archive-file", Some(&file)) if archive.is_none() => {
                    archive = Some(file.trim_matches('"'))
                }
                _ => {
                    selector.push(token);
                    rest = tail;
                    continue;
                }
            }
            rest = &tail[1..];
        }
        let (_, want, name) = PRODUCER_FAMILIES
            .iter()
            .find(|(family, _, _)| family.split_whitespace().eq(selector.iter().copied()))
            .ok_or_else(|| format!("unknown archive family {selector:?}: {line}"))?;
        let file = format!("$RUNNER_TEMP/ci-test-archives/{name}.tar.zst");
        if archive != Some(file.as_str()) {
            return Err(format!("{name}: archive {archive:?}, want {file}"));
        }
        let mut want = want.to_vec();
        want.sort_unstable();
        suites.sort_unstable();
        if suites != want {
            return Err(format!("{name}: archives {suites:?}, want {want:?}"));
        }
        if seen.contains(name) {
            return Err(format!("{name}: archived twice"));
        }
        seen.push(*name);
    }
    if seen.len() != PRODUCER_FAMILIES.len() {
        return Err(format!("archive families {seen:?} incomplete"));
    }
    Ok(())
}

/// The pack step rewrites the manifest from exactly `{schema, command, fixtures}`
/// (any other key would be silently dropped), and records the packed
/// cargo-nextest's own `--version` line — pinned to 0.9.149 — not a literal.
fn pack_binds_manifest_and_nextest(pack: &str) -> bool {
    let keys = "assert set(manifest) == {'schema', 'command', 'fixtures'}, sorted(manifest)";
    let version = "nextest = subprocess.check_output([path, 'nextest', '--version'], \
                   text=True).splitlines()[0]";
    matches!(
        (pack.find(keys), pack.find("manifest_path.write_text(")),
        (Some(checked), Some(rewritten)) if checked < rewritten
    ) && pack.contains("archive.add(path, arcname=name, recursive=False)")
        && pack.contains(version)
        && pack.contains("assert nextest.startswith('cargo-nextest 0.9.149 '), nextest")
        && pack.contains("nextest=nextest)")
        && !pack.contains("nextest='")
}

#[test]
fn producer_keeps_package_feature_families_and_nextest_version() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let producer = key(jobs, "ci-test-producer");
    let build = str_of(
        &steps(producer)[step_index(producer, "Build invocation-family nextest archives")],
        "run",
    )
    .expect("archive build commands");
    assert_eq!(producer_families_are_exact(build), Ok(()));
    // Swapping one suite between the network and network+wasm commands keeps every
    // `--test` somewhere in the script; only the per-command binding can refuse it.
    let swapped = build
        .replacen("--test observer_live_17_4", "--test SWAPPED", 1)
        .replacen(
            "--test smoke_cli_wrapper_8_12",
            "--test observer_live_17_4",
            1,
        )
        .replacen("--test SWAPPED", "--test smoke_cli_wrapper_8_12", 1);
    assert_ne!(
        swapped, build,
        "swap vector must change the producer script"
    );
    assert!(producer_families_are_exact(&swapped).is_err());
    assert!(
        !build.contains("--workspace"),
        "package jobs must not gain workspace features"
    );
    let pack = str_of(
        &steps(producer)[step_index(producer, "Pack exact producer fixtures, tools and identity")],
        "run",
    )
    .expect("fixture pack command");
    assert!(pack.contains("for name, entry in manifest['fixtures'].items():"));
    assert!(pack.contains("entry.get('tree') == tree"));
    assert!(pack.contains("['git', 'hash-object', '--no-filters', '--', entry['path']]"));
    assert!(pack.contains("assert artifact == entry['artifact']"));
    assert!(pack.contains("paths.add(pathlib.Path(entry['path']))"));
    assert!(pack.contains("target/tmp/cgroup-ceiling-fixture"));
    assert!(pack_binds_manifest_and_nextest(pack));
    // An extra manifest key the rewrite would drop, or a literal version that
    // names no binary, must both refuse.
    for (from, to) in [
        (
            "assert set(manifest) == {'schema', 'command', 'fixtures'}, sorted(manifest)\n",
            "",
        ),
        ("nextest=nextest)", "nextest='0.9.149')"),
        (
            "assert nextest.startswith('cargo-nextest 0.9.149 '), nextest\n",
            "",
        ),
    ] {
        let planted = pack.replacen(from, to, 1);
        assert_ne!(planted, pack, "pack plant must change the step: {from}");
        assert!(!pack_binds_manifest_and_nextest(&planted), "{from}");
    }
    // AC2.3: one nextest version across the discipline and nightly workflows.
    let nightly: Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/journey-nightly.yml"))
            .expect("read journey-nightly"),
    )
    .expect("parse journey-nightly");
    for job in mapping(jobs)
        .values()
        .chain(mapping(key(&nightly, "jobs")).values())
    {
        for step in steps(job) {
            if let Some(tool) = get(step, "with").and_then(|with| str_of(with, "tool")) {
                if tool.contains("nextest") {
                    assert_eq!(tool, "cargo-nextest@0.9.149");
                }
            }
            assert!(
                !str_of(step, "uses").is_some_and(|uses| uses.ends_with("nextest")),
                "unversioned nextest action"
            );
            if let Some(run) = str_of(step, "run") {
                assert!(
                    !run.contains("cargo install cargo-nextest"),
                    "unversioned nextest install"
                );
            }
        }
    }
}

/// Consumer filters that select a whole archived package family rather than one
/// `--test` suite, with the only family archive that holds those packages.
const WHOLE_FAMILY_FILTERS: [(&str, &str); 3] = [
    (
        "binary_id(/^maos-escape-detector(::|$)/)",
        "escape-detector",
    ),
    (
        "binary_id(/^maos-(host|wasm-host)(::|$)/)",
        "host-wasm-host",
    ),
    ("binary_id(/^equiv-native-twin(::|$)/)", "native-twin"),
];

/// The producer family archive holding `filter`'s binaries: a whole-package filter
/// names its family; every `binary_id(pkg::suite)` term resolves through the family
/// whose selector names `pkg` and whose `--test` set holds `suite`. An unknown term
/// or terms from different families have no archive.
fn family_archive(filter: &str) -> Option<&'static str> {
    if let Some(&(_, name)) = WHOLE_FAMILY_FILTERS.iter().find(|(f, _)| *f == filter) {
        return Some(name);
    }
    let mut names = filter.split(" | ").map(|term| {
        let (package, suite) = term
            .strip_prefix("binary_id(")?
            .strip_suffix(')')?
            .split_once("::")?;
        PRODUCER_FAMILIES
            .iter()
            .find(|(selector, suites, _)| {
                selector.split_whitespace().any(|token| token == package) && suites.contains(&suite)
            })
            .map(|&(_, _, name)| name)
    });
    let first = names.next()??;
    names.all(|name| name == Some(first)).then_some(first)
}

/// Every archive a consumer job runs — each `--archive-file` command and each
/// rootless-canary archive (whose script-side filter is `smoke_cli_wrapper_8_12`) —
/// must be the producer family archive that holds the selected binaries. Returns
/// the family archives consumed.
fn consumer_archives_are_bound(job: &Value) -> Result<Vec<&'static str>, String> {
    let archive = regex::Regex::new(
        r#"--archive-file "\$RUNNER_TEMP/ci-test-archives/([a-z0-9-]+)\.tar\.zst""#,
    )
    .expect("archive regex");
    let filter = regex::Regex::new(r#" -E (?:'([^']+)'|"([^"]+)")"#).expect("filter regex");
    let mut consumed = Vec::new();
    for step in steps(job) {
        let mut bindings = Vec::new();
        if let Some(canary) =
            get(step, "env").and_then(|env| str_of(env, "MAOS_CGROUP_CANARY_ARCHIVE"))
        {
            let file = canary
                .strip_prefix("${{ runner.temp }}/ci-test-archives/")
                .and_then(|file| file.strip_suffix(".tar.zst"));
            bindings.push((file, "binary_id(maos-bin::smoke_cli_wrapper_8_12)"));
        }
        for line in str_of(step, "run")
            .unwrap_or_default()
            .lines()
            .filter(|line| line.contains("--archive-file"))
        {
            let file = archive
                .captures(line)
                .map(|c| c.get(1).expect("archive name").as_str());
            let selected = filter
                .captures(line)
                .and_then(|c| c.get(1).or_else(|| c.get(2)))
                .ok_or_else(|| format!("archive run without -E filter: {line}"))?;
            bindings.push((file, selected.as_str()));
        }
        for (file, selected) in bindings {
            let want = family_archive(selected)
                .ok_or_else(|| format!("no producer family holds {selected}"))?;
            if file != Some(want) {
                return Err(format!("{selected}: archive {file:?}, want {want}"));
            }
            consumed.push(want);
        }
    }
    Ok(consumed)
}

#[test]
fn consumer_archive_files_are_the_families_that_hold_their_suites() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let mut consumed = Vec::new();
    for name in ARCHIVE_CONSUMERS {
        consumed.extend(
            consumer_archives_are_bound(key(jobs, name))
                .unwrap_or_else(|error| panic!("{name}: {error}")),
        );
    }
    for (_, _, family) in PRODUCER_FAMILIES {
        assert!(consumed.contains(&family), "no consumer runs {family}");
    }
    // Pointing a consumer at a sibling family's archive (network+wasm lacks
    // worker_terminal_facts_17_7) keeps every other token and must still red.
    let job = key(jobs, "epic-17-exit");
    let at = step_index(
        job,
        "Epic-17 exit line 4 — retained T3 terminal facts (Story 17-7)",
    );
    let run = str_of(&steps(job)[at], "run").expect("terminal facts run");
    let mut swapped = job.clone();
    swapped["steps"][at]["run"] = Value::String(run.replacen(
        "ci-test-archives/maos-bin-network.tar.zst",
        "ci-test-archives/maos-bin-network-wasm.tar.zst",
        1,
    ));
    assert_ne!(&swapped, job, "swap vector must change the consumer");
    assert!(consumer_archives_are_bound(&swapped).is_err());
}

#[test]
fn wasm_archive_keeps_image_matrix_suite_counts_and_all_runtime_markers() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let job = key(jobs, "wasm-host-tests");
    assert_eq!(key(key(job, "strategy"), "fail-fast"), &Value::Bool(false));
    let images: Vec<_> = key(key(key(job, "strategy"), "matrix"), "image")
        .as_sequence()
        .expect("WASM image matrix")
        .iter()
        .map(|image| image.as_str().expect("image"))
        .collect();
    assert_eq!(
        images,
        [
            "ubuntu-24.04",
            "ubuntu-26.04",
            "ubuntu-24.04-arm",
            "ubuntu-26.04-arm",
        ]
    );
    assert_eq!(str_of(job, "runs-on"), Some("${{ matrix.image }}"));
    for (name, suite, count) in [
        (
            "Real admitted native/WASM verdict, hostile lifting, idle, and SIGSYS gates",
            "spawned_scope_17_3c",
            23,
        ),
        (
            "Live escape-anomaly consumer under real T2 (Story 17-10)",
            "live_escape_anomaly_17_10",
            2,
        ),
        (
            "In-band recall through the admitted session (Story 17-3d)",
            "recall_17_3d",
            18,
        ),
        (
            "Participant recall and consent envelopes, both forms (Story 17-8)",
            "participant_recall_17_8",
            8,
        ),
    ] {
        let step = &steps(job)[step_index(job, name)];
        assert!(get(step, "if").is_none(), "{name} must prove every image");
        let run = str_of(step, "run").expect("runtime proof command");
        for witness in [
            format!("binary_id(maos-bin::{suite})"),
            "run --mode delegated".into(),
            "/ci-test-archives/maos-bin-network-wasm.tar.zst".into(),
            "--no-capture --test-threads 1".into(),
            format!("{count} tests run: {count} passed([,[:space:]]|$)"),
            format!("--expected-count {count}"),
            "check-log --exit-code \"$rc\"".into(),
            "test ${PIPESTATUS[0]} -eq 0".into(),
            "if grep -q '^SKIP[: ]'".into(),
            "exit \"$fail\"".into(),
        ] {
            assert!(run.contains(&witness), "{name} lost {witness}");
        }
        if suite != "spawned_scope_17_3c" {
            assert!(run.contains("--required-test \"$name\""));
            assert!(run.contains("\"${required[@]}\""));
        } else {
            // No required names: an ignored test only shows as nextest-skipped.
            assert!(run.contains(
                "if grep -Eq 'SKIP|(^|[[:space:]])0 tests run:|tests? run: .*, [1-9][0-9]* skipped' \
                 \"$RUNNER_TEMP/spawned-17-3c.log\"; then"
            ));
        }
    }
    let run = str_of(
        &steps(job)[step_index(
            job,
            "Real admitted native/WASM verdict, hostile lifting, idle, and SIGSYS gates",
        )],
        "run",
    )
    .expect("admitted gate");
    for marker in [
        "SCOPED-VERDICT :8",
        "GOVERNED-ATOMIC :10",
        "HOSTILE-LIFTING :2",
        "HOSTILE-LIFTING-CONTROL :1",
        "RUNNER-SIGSYS :2",
        "REPEATED-MAILBOX :1",
        "HOSTILE-PROVENANCE :1",
        "HOSTILE-REFUSALS :1",
        "HOSTILE-FLOODS :1",
        "HOSTILE-DIAGNOSTICS :1",
        "HOSTILE-EXITS :1",
        "HOSTILE-ONCE :1",
        "HOSTILE-IAC-SEND :1",
        "WASM-IAC-SEND :1",
    ] {
        assert!(
            run.contains(&format!("'{marker}'")),
            "missing marker pin {marker}"
        );
    }
    assert!(run.contains("[ \"$got\" -eq \"$want\" ]"));
    let producer = key(jobs, "ci-test-producer");
    let release = str_of(
        &steps(producer)[step_index(producer, "Build the release CLI invocation families")],
        "run",
    )
    .expect("release family build");
    for command in [
        "cargo build --locked --release -p maos-bin -p maos-wasm-host",
        "--features maos-bin/wasm-host --bin maos --bin maos-wasm-runner",
        "cargo build --locked --release -p maos-bin --features wasm-host --bin maos",
        "cargo build --locked --release -p maos-wasm-host --bin maos-wasm-runner",
        "cargo build --locked --release -p maos-cli --bin maosctl",
        "release-epic17.tar",
        "release-wasm-host.tar",
    ] {
        assert!(
            release.contains(command),
            "missing release family {command}"
        );
    }
}

// Archive fixtures refuse a dirty checkout, including untracked files. Logs,
// scene receipts and signed sidecar copies therefore belong outside the tree.
fn consumer_outputs_are_external(job: &Value) -> bool {
    let output = regex::Regex::new(
        r"(?:[A-Za-z0-9_$./-]+\.(?:log|out|err|verdict)|[A-Za-z0-9_$./-]+\.benchmark\.signed\.json)\b",
    )
    .expect("consumer output paths");
    steps(job).iter().all(|step| {
        str_of(step, "run").is_none_or(|run| {
            output
                .find_iter(run)
                .all(|path| path.as_str().starts_with("$RUNNER_TEMP/"))
                && run
                    .lines()
                    .filter(|line| line.contains("--output "))
                    .all(|line| line.contains("--output \"$RUNNER_TEMP/"))
        }) && (str_of(step, "uses") != Some("actions/upload-artifact@v4")
            || get(step, "with")
                .and_then(|with| str_of(with, "path"))
                .is_some_and(|paths| {
                    paths.lines().all(|path| {
                        let path = path.trim();
                        path.starts_with("${{ runner.temp }}/") || path.starts_with("target/")
                    })
                }))
    })
}

#[test]
fn archive_consumer_outputs_preserve_clean_checkout_provenance() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    for name in ARCHIVE_CONSUMERS {
        let job = key(jobs, name);
        assert!(
            consumer_outputs_are_external(job),
            "{name} writes into the checkout"
        );
        for command in [
            "printf proof | tee source-output.log",
            "python3 scripts/epic_17_exit_line1.py --output epic-17-exit-line1",
            "cp sidecar.json line2.benchmark.signed.json",
        ] {
            let mut planted = job.clone();
            planted["steps"]
                .as_sequence_mut()
                .expect("consumer steps")
                .push(serde_yaml::from_str(&format!("run: {command}\n")).expect("output plant"));
            assert!(
                !consumer_outputs_are_external(&planted),
                "{name} accepted {command}"
            );
        }
        let mut planted = job.clone();
        planted["steps"]
            .as_sequence_mut()
            .expect("consumer steps")
            .push(
                serde_yaml::from_str(
                    "uses: actions/upload-artifact@v4\nwith:\n  path: source-output.log\n",
                )
                .expect("upload plant"),
            );
        assert!(
            !consumer_outputs_are_external(&planted),
            "{name} accepted checkout upload"
        );
    }
}

#[test]
fn parsed_workflow_output_is_colorless_without_step_overrides() {
    let workflow = workflow();
    assert_eq!(
        str_of(key(&workflow, "env"), "CARGO_TERM_COLOR"),
        Some("never")
    );
    for job in mapping(key(&workflow, "jobs")).values() {
        for env in get(job, "env")
            .into_iter()
            .chain(steps(job).iter().filter_map(|step| get(step, "env")))
        {
            assert!(str_of(env, "CARGO_TERM_COLOR").is_none_or(|color| color == "never"));
        }
        for run in steps(job).iter().filter_map(|step| str_of(step, "run")) {
            assert!(!run.contains("--color always") && !run.contains("--color=always"));
        }
    }
}
