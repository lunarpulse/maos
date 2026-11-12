//! Story 20-5 L1 review corrections (GPT-2, GPT-3; GLM F2, F3, F4, F6): the release job's
//! tag classification, its main-ancestry check, its ledger and aggregate step, and the
//! branch-only hosted recipe are run as the scripts they are — the `run:` text is
//! extracted from the workflow YAML and executed under bash with stand-in `gh`, `cargo`
//! and `xtask` binaries — so a green YAML that never reaches its decision cannot pass.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn workflow(rel: &str) -> serde_yaml::Value {
    serde_yaml::from_str(&std::fs::read_to_string(root().join(rel)).unwrap()).unwrap()
}

fn release() -> serde_yaml::Value {
    workflow(".github/workflows/release.yml")
}

fn recipe() -> serde_yaml::Value {
    workflow(
        "_bmad-output/implementation-artifacts/20-5-v1-0-rc-tag-and-lts-rule-evidence/ci/ci-20-5-verify.yml",
    )
}

fn steps(workflow: &serde_yaml::Value, job: &str) -> Vec<serde_yaml::Value> {
    workflow["jobs"][job]["steps"]
        .as_sequence()
        .unwrap()
        .clone()
}

fn index_of(steps: &[serde_yaml::Value], name: &str) -> usize {
    steps
        .iter()
        .position(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("step {name:?} must exist"))
}

fn script(steps: &[serde_yaml::Value], name: &str) -> String {
    steps[index_of(steps, name)]["run"]
        .as_str()
        .unwrap()
        .to_string()
}

fn executable(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("#!/usr/bin/env bash\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn bash(script: &str, dir: &Path, path_prefix: Option<&Path>, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new("bash");
    command.arg("-c").arg(script).current_dir(dir);
    let path = std::env::var("PATH").unwrap();
    match path_prefix {
        Some(prefix) => command.env("PATH", format!("{}:{path}", prefix.display())),
        None => command.env("PATH", path),
    };
    command.env("RUNNER_TEMP", dir).env("GITHUB_SHA", SHA);
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ---- GLM F2: one fail-closed tag classification ------------------------------------

fn classify(tag: &str) -> (bool, String) {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let script = script(&steps, "Classify the tag (fail closed)");
    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("github-output");
    std::fs::write(&out_file, "").unwrap();
    let output = bash(
        &script,
        dir.path(),
        None,
        &[("TAG", tag), ("GITHUB_OUTPUT", out_file.to_str().unwrap())],
    );
    (
        output.status.success(),
        std::fs::read_to_string(out_file)
            .unwrap()
            .trim()
            .to_string(),
    )
}

#[test]
fn the_tag_grammar_has_one_fail_closed_decision_point() {
    for (tag, kind) in [
        ("v1.0.0-rc.1", "kind=rc"),
        ("v10.20.30-rc.12", "kind=rc"),
        ("v1.0.0", "kind=ga"),
        ("v0.1.0-alpha.1", "kind=alpha"),
    ] {
        assert_eq!(classify(tag), (true, kind.to_string()), "{tag}");
    }
    for tag in [
        "v1.0.0-RC.1",
        "v1.0.0-rc1",
        "v1.0.0-rc",
        "v1.0.0-rc.",
        "v1.0.0-rc.1x",
        "v1.0.0-rc.1-x",
        "v1.0.0-beta.1",
        "v1.0.0-alpha.1",
        "v1.0.0-",
        "v1",
        "v1.0",
        "vfoo",
        "1.0.0",
        "V1.0.0",
        "v1.0.0+build",
    ] {
        assert_eq!(
            classify(tag),
            (false, String::new()),
            "{tag} must be refused with no kind"
        );
    }
}

#[test]
fn every_later_control_reads_the_one_classification() {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let classify = index_of(&steps, "Classify the tag (fail closed)");
    let readiness = &steps[index_of(&steps, "Require rc tree readiness (rc tags only)")];
    assert_eq!(
        readiness["if"].as_str(),
        Some("steps.tag.outputs.kind == 'rc'")
    );
    assert_eq!(steps[classify]["id"].as_str(), Some("tag"));
    let aggregate = &steps[index_of(&steps, "Require successful main discipline aggregate")];
    assert_eq!(
        aggregate["env"]["KIND"].as_str(),
        Some("${{ steps.tag.outputs.kind }}")
    );
    assert!(
        !aggregate["run"].as_str().unwrap().contains("REF_NAME"),
        "the aggregate step must not re-derive the kind from the tag text"
    );
    assert!(classify < index_of(&steps, "Require rc tree readiness (rc tags only)"));
}

// ---- GLM F6 / F4: the aggregate + ledger step ------------------------------------------

fn aggregate_step(kind: &str) -> (Output, String) {
    aggregate_step_with(kind, "")
}

/// `perf_mode`: "" = the perf-ledger branch exists; "404" = absent; "500" = any other failure.
fn aggregate_step_with(kind: &str, perf_mode: &str) -> (Output, String) {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let script = script(&steps, "Require successful main discipline aggregate")
        .replace("${{ github.repository }}", "o/r")
        .replace("${{ github.sha }}", SHA);
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    let log = dir.path().join("calls.log");
    executable(
        &bin.join("gh"),
        &format!(
            "echo \"gh $*\" >> {log}\n\
             case \"$*\" in\n\
               *commits/fuzz-ledger*) echo ledgertip0123456789abcdef0123456789abcdef0123 ;;\n\
               *contents/fuzz-ledger.json*) echo '{{\"schema_version\":1,\"records\":[]}}' ;;\n\
               *branches/perf-ledger*) case \"$PERF_MODE\" in\n\
                 404) echo 'gh: Branch not found (HTTP 404)' >&2; exit 1 ;;\n\
                 500) echo 'gh: boom (HTTP 500)' >&2; exit 1 ;;\n\
                 *) echo perftip0123456789abcdef0123456789abcdef01234 ;;\n\
               esac ;;\n\
               *contents/ledger.jsonl*) ;;\n\
               *check-runs*) echo '{{\"check_runs\":[]}}' ;;\n\
             esac",
            log = log.display()
        ),
    );
    executable(
        &bin.join("cargo"),
        &format!("cat > /dev/null\necho \"cargo $*\" >> {}", log.display()),
    );
    let output = bash(
        &script,
        dir.path(),
        Some(&bin),
        &[("KIND", kind), ("GH_TOKEN", "x"), ("PERF_MODE", perf_mode)],
    );
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    (output, calls)
}

#[test]
fn rc_and_ga_pass_the_ledger_pinned_to_its_logged_tip_and_alpha_does_not() {
    for kind in ["rc", "ga"] {
        let (output, calls) = aggregate_step(kind);
        assert!(output.status.success(), "{kind}: {}", text(&output));
        assert!(calls.contains("--fuzz-ledger"), "{kind}: {calls}");
        assert!(
            calls.contains("contents/fuzz-ledger.json?ref=ledgertip0123456789abcdef0123456789abcdef0123"),
            "the ledger is fetched at the tip commit that was logged, not a moving branch name: {calls}"
        );
        assert!(
            text(&output).contains(
                "fuzz-ledger tip consumed: ledgertip0123456789abcdef0123456789abcdef0123"
            ),
            "the consumed ledger commit is printed: {}",
            text(&output)
        );
    }
    let (output, calls) = aggregate_step("alpha");
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        !calls.contains("--fuzz-ledger") && !calls.contains("fuzz-ledger.json"),
        "{calls}"
    );
}

#[test]
fn perf_verdicts_are_judged_for_rc_and_ga_and_only_a_candidate_accepts_absent() {
    let (output, calls) = aggregate_step("rc");
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        calls.contains(&format!("check-perf-verdict --sha {SHA} --ledger")),
        "{calls}"
    );
    assert!(
        calls.contains("contents/ledger.jsonl?ref=perftip0123456789abcdef0123456789abcdef01234"),
        "{calls}"
    );
    assert!(text(&output)
        .contains("perf-ledger tip consumed: perftip0123456789abcdef0123456789abcdef01234"));
    assert!(
        calls.contains("--perf-verdicts") && calls.contains("--perf-absent-ok"),
        "{calls}"
    );
    let (output, calls) = aggregate_step("ga");
    assert!(output.status.success(), "{}", text(&output));
    assert!(calls.contains("--perf-verdicts"), "{calls}");
    assert!(
        !calls.contains("--perf-absent-ok"),
        "GA needs PASS, never ABSENT: {calls}"
    );
    let (output, calls) = aggregate_step("alpha");
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        !calls.contains("perf"),
        "the alpha line has no perf leg: {calls}"
    );
}

#[test]
fn an_absent_perf_ledger_branch_is_an_empty_ledger_and_any_other_failure_refuses() {
    for kind in ["rc", "ga"] {
        let (output, calls) = aggregate_step_with(kind, "404");
        assert!(output.status.success(), "{kind}: {}", text(&output));
        assert!(text(&output).contains("empty ledger"), "{}", text(&output));
        assert!(calls.contains("--perf-verdicts"), "{calls}");
        let (output, calls) = aggregate_step_with(kind, "500");
        assert!(!output.status.success(), "{kind}: {}", text(&output));
        assert!(!calls.contains("check-release-precondition"), "{calls}");
    }
}

#[test]
fn an_unclassified_kind_is_refused_before_any_call() {
    for kind in ["", "beta", "RC", "rc ga"] {
        let (output, calls) = aggregate_step(kind);
        assert!(!output.status.success(), "{kind:?}: {}", text(&output));
        assert!(
            calls.is_empty(),
            "no gh/cargo call before the refusal: {calls}"
        );
    }
}

#[test]
fn the_aggregate_query_never_hides_a_newer_run_in_progress() {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let run = script(&steps, "Require successful main discipline aggregate");
    assert!(run.contains("check_name=aggregate&filter=latest"), "{run}");
    assert!(
        !run.contains("status=completed"),
        "a status filter would surface an older green run while the newest is still running: {run}"
    );
}

// ---- GLM F3: the tagged commit must be on main ---------------------------------------

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {}", text(&output));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn the_tagged_commit_must_be_an_ancestor_of_origin_main() {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let ancestry = index_of(&steps, "Require the tagged commit to be on main");
    assert_eq!(
        ancestry, 1,
        "the ancestry check is the first step after checkout"
    );
    assert_eq!(
        steps[0]["with"]["fetch-depth"].as_u64(),
        Some(0),
        "the checkout must carry the history the ancestry check reads"
    );
    let script = script(&steps, "Require the tagged commit to be on main");

    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("origin.git");
    let work = dir.path().join("work");
    git(
        dir.path(),
        &["init", "--bare", "-b", "main", origin.to_str().unwrap()],
    );
    git(
        dir.path(),
        &["clone", origin.to_str().unwrap(), work.to_str().unwrap()],
    );
    git(&work, &["checkout", "-b", "main"]);
    std::fs::write(work.join("a"), "a").unwrap();
    git(&work, &["add", "a"]);
    git(&work, &["commit", "-m", "on main"]);
    git(&work, &["push", "origin", "main"]);
    let on_main = git(&work, &["rev-parse", "HEAD"]);
    git(&work, &["checkout", "-b", "side"]);
    std::fs::write(work.join("b"), "b").unwrap();
    git(&work, &["add", "b"]);
    git(&work, &["commit", "-m", "unreviewed side branch"]);
    git(&work, &["push", "origin", "side"]);
    let off_main = git(&work, &["rev-parse", "HEAD"]);

    let ok = bash(&script, &work, None, &[("GITHUB_SHA", &on_main)]);
    assert!(ok.status.success(), "{}", text(&ok));
    let refused = bash(&script, &work, None, &[("GITHUB_SHA", &off_main)]);
    assert!(!refused.status.success(), "{}", text(&refused));
    assert!(
        text(&refused).contains("not an ancestor of origin/main"),
        "{}",
        text(&refused)
    );
}

// ---- GPT-2, GPT-3: the hosted recipe proves what it says ---------------------------------

const REFUSAL_JOB: &str = "precondition-refuses-without-a-t3-chain";
const READINESS_JOB: &str = "rc-readiness-reports-its-legs";
const REFUSAL_STEP: &str = "The leg refuses (the gate's own refusal is the PASS of this step)";
const READINESS_STEP: &str =
    "Report the six legs (a red tree is the expected state until the L2 landing)";

fn run_recipe_step(job: &str, name: &str, fake_xtask: Option<&str>) -> Output {
    let recipe = recipe();
    let steps = steps(&recipe, job);
    let script = script(&steps, name);
    let dir = tempfile::tempdir().unwrap();
    if let Some(body) = fake_xtask {
        executable(&dir.path().join("target/debug/xtask"), body);
    }
    bash(&script, dir.path(), None, &[])
}

#[test]
fn both_recipe_jobs_build_the_gate_in_their_own_mandatory_step() {
    for (job, refusing) in [(REFUSAL_JOB, REFUSAL_STEP), (READINESS_JOB, READINESS_STEP)] {
        let steps = steps(&recipe(), job);
        let build = index_of(&steps, "Build xtask (this step must succeed)");
        assert_eq!(
            steps[build]["run"].as_str(),
            Some("cargo build --locked -p xtask")
        );
        assert!(steps[build].get("continue-on-error").is_none());
        assert!(
            build < index_of(&steps, refusing),
            "{job}: the build precedes the judgment"
        );
    }
}

#[test]
fn the_t3_refusal_step_passes_only_a_genuine_refusal() {
    let genuine = format!(
        "echo 'release precondition failed: no complete T3 fuzz chain for manifest_parser at {SHA} (5 links of one run)' >&2; exit 1"
    );
    let ok = run_recipe_step(REFUSAL_JOB, REFUSAL_STEP, Some(&genuine));
    assert!(ok.status.success(), "{}", text(&ok));

    for (label, body) in [
        ("unsupported option (an old binary)", "echo \"error: unexpected argument '--fuzz-ledger' found\" >&2; exit 2".to_string()),
        ("a pass", "echo ok; exit 0".to_string()),
        ("the refusal text but exit 0", format!("echo 'no complete T3 fuzz chain for manifest_parser at {SHA}'; exit 0")),
        ("the refusal text but a crash status", format!("echo 'no complete T3 fuzz chain for manifest_parser at {SHA}' >&2; exit 101")),
        ("a crash", "echo boom >&2; exit 101".to_string()),
        ("refusal naming another SHA", "echo 'no complete T3 fuzz chain for manifest_parser at deadbeef' >&2; exit 1".to_string()),
        ("exit 1 without the T3 diagnostic", format!("echo 'some other failure at {SHA}' >&2; exit 1")),
        ("aggregate refusal", format!("echo 'release precondition failed: aggregate conclusion is failure ({SHA})' >&2; exit 1")),
    ] {
        let output = run_recipe_step(REFUSAL_JOB, REFUSAL_STEP, Some(&body));
        assert!(!output.status.success(), "{label} must fail the step: {}", text(&output));
    }
    let missing = run_recipe_step(REFUSAL_JOB, REFUSAL_STEP, None);
    assert!(
        !missing.status.success(),
        "no built binary (a compile failure) must fail the step"
    );
}

fn report(legs: &[(&str, bool)], gate: &str, passed: bool) -> String {
    let legs: Vec<String> = legs
        .iter()
        .map(|(leg, ok)| format!("{{\"leg\":\"{leg}\",\"passed\":{ok},\"detail\":\"d\"}}"))
        .collect();
    format!(
        "{{\"gate\":\"{gate}\",\"passed\":{passed},\"legs\":[{}]}}",
        legs.join(",")
    )
}

const SIX: [&str; 6] = [
    "version",
    "stability-matrix",
    "epic-20-stories",
    "provisioning",
    "deny-ignores",
    "perf-verdict-label",
];

fn legs_with(red: &[&str]) -> Vec<(&'static str, bool)> {
    SIX.iter().map(|leg| (*leg, !red.contains(leg))).collect()
}

#[test]
fn the_readiness_step_passes_only_a_genuine_red_report() {
    let fake = |json: &str, status: i32| format!("printf '%s' '{json}'; exit {status}");
    let good = report(
        &legs_with(&["version", "epic-20-stories"]),
        "check-rc-readiness",
        false,
    );
    let ok = run_recipe_step(READINESS_JOB, READINESS_STEP, Some(&fake(&good, 1)));
    assert!(ok.status.success(), "{}", text(&ok));

    let all_green = report(&legs_with(&[]), "check-rc-readiness", true);
    let wrong_gate = report(&legs_with(&["version"]), "other-gate", false);
    let five_legs = report(&legs_with(&["version"])[..5], "check-rc-readiness", false);
    let lying = report(&legs_with(&[]), "check-rc-readiness", false);
    for (label, body) in [
        ("an all-green report (exit 0)", fake(&all_green, 0)),
        (
            "an all-green report with refusal status",
            fake(&all_green, 1),
        ),
        ("empty output", "exit 1".to_string()),
        ("malformed JSON", fake("{not json", 1)),
        (
            "a tool failure",
            "echo 'error: unrecognized subcommand' >&2; exit 2".to_string(),
        ),
        ("a crash", fake(&good, 101)),
        ("a report for another gate", fake(&wrong_gate, 1)),
        ("a report with five legs", fake(&five_legs, 1)),
        (
            "a `passed:false` report whose legs are all green",
            fake(&lying, 1),
        ),
        ("a red report with a success status", fake(&good, 0)),
    ] {
        let output = run_recipe_step(READINESS_JOB, READINESS_STEP, Some(&body));
        assert!(
            !output.status.success(),
            "{label} must fail the step: {}",
            text(&output)
        );
    }
    let missing = run_recipe_step(READINESS_JOB, READINESS_STEP, None);
    assert!(
        !missing.status.success(),
        "no built binary must fail the step"
    );
}
