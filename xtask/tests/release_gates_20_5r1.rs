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
        assert_eq!(classify(tag), (true, format!("{kind}\ntag={tag}")), "{tag}");
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

/// rc.1 rehearsal (dispatch): the ref is a branch, so the tag is derived from the checked-out
/// `[workspace.package].version` and judged by the SAME grammar; a push never derives anything.
#[test]
fn a_dispatch_rehearsal_classifies_the_tag_the_tree_would_carry() {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let script = script(&steps, "Classify the tag (fail closed)");
    for (version, rehearsal, accepted) in [
        ("1.0.0-rc.2", "true", Some("kind=rc\ntag=v1.0.0-rc.2")),
        ("1.0.0", "true", Some("kind=ga\ntag=v1.0.0")),
        ("1.0.0-beta.1", "true", None),
        ("1.0.0-rc.2", "false", None),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let manifest = format!("[workspace.package]\nversion = \"{version}\"\n");
        std::fs::write(dir.path().join("Cargo.toml"), manifest).unwrap();
        let out_file = dir.path().join("github-output");
        std::fs::write(&out_file, "").unwrap();
        let env = [
            ("TAG", "a-branch-name"),
            ("REHEARSAL", rehearsal),
            ("GITHUB_OUTPUT", out_file.to_str().unwrap()),
        ];
        let output = bash(&script, dir.path(), None, &env);
        let written = std::fs::read_to_string(&out_file).unwrap();
        match accepted {
            Some(expected) => {
                assert!(output.status.success(), "{version}: {}", text(&output));
                assert_eq!(written.trim(), expected, "{version}");
            }
            None => {
                assert!(!output.status.success(), "{version}: {}", text(&output));
                assert!(written.is_empty(), "{version}: no kind on a refusal");
            }
        }
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

#[test]
fn the_release_body_is_the_committed_notes_file_with_generated_notes_appended() {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let create = &steps[index_of(&steps, "Create GitHub Release")];
    assert_eq!(
        create["with"]["body_path"].as_str(),
        Some("docs/release/${{ github.ref_name }}.md"),
        "the body must be the committed notes file named for the tag"
    );
    assert_eq!(
        create["with"]["generate_release_notes"].as_bool(),
        Some(true),
        "generated notes stay, appended after the committed body"
    );

    // The action falls back silently when `body_path` is unreadable, so an rc or GA tag is
    // refused earlier by a step that is run here as the script it is (operator ruling
    // 2026-11-12, third set, B1: GA notes carry the perf label, so GA needs its notes).
    let guard = &steps[index_of(&steps, NOTES_GUARD_STEP)];
    assert_eq!(guard["if"].as_str(), Some(RC_OR_GA));
    assert_eq!(
        guard["env"]["TAG"].as_str(),
        Some("${{ steps.tag.outputs.tag }}")
    );
    assert!(index_of(&steps, NOTES_GUARD_STEP) < index_of(&steps, "Create GitHub Release"));
    let run = guard["run"].as_str().unwrap();
    for tag in ["v1.0.0-rc.1", "v1.0.0"] {
        let dir = tempfile::tempdir().unwrap();
        let missing = bash(run, dir.path(), None, &[("TAG", tag)]);
        assert!(!missing.status.success(), "{tag}: {}", text(&missing));
        assert!(
            text(&missing).contains("release refused"),
            "{tag}: {}",
            text(&missing)
        );
        std::fs::create_dir_all(dir.path().join("docs/release")).unwrap();
        std::fs::write(dir.path().join(format!("docs/release/{tag}.md")), "").unwrap();
        let empty = bash(run, dir.path(), None, &[("TAG", tag)]);
        assert!(
            !empty.status.success(),
            "{tag}: an empty notes file must be refused: {}",
            text(&empty)
        );
        std::fs::write(
            dir.path().join(format!("docs/release/{tag}.md")),
            "# notes\n",
        )
        .unwrap();
        let present = bash(run, dir.path(), None, &[("TAG", tag)]);
        assert!(present.status.success(), "{tag}: {}", text(&present));
    }
}

// ---- GLM F6 / F4: the aggregate + ledger step ------------------------------------------

fn aggregate_step(kind: &str) -> (Output, String) {
    aggregate_step_with(kind, "")
}

/// `perf_mode`: "" = the perf-ledger branch exists and `ledger.jsonl` is served; the
/// branch lookup fails with "404" (absent), "403", "ratelimit" or "500"; the branch lookup
/// succeeds and the CONTENTS request fails with "contents404", "contents403" or "contents500".
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
                 403) echo 'gh: Resource not accessible by integration (HTTP 403)' >&2; exit 1 ;;\n\
                 ratelimit) echo 'gh: API rate limit exceeded for installation (HTTP 403)' >&2; exit 1 ;;\n\
                 500) echo 'gh: boom (HTTP 500)' >&2; exit 1 ;;\n\
                 *) echo perftip0123456789abcdef0123456789abcdef01234 ;;\n\
               esac ;;\n\
               *contents/ledger.jsonl*) case \"$PERF_MODE\" in\n\
                 contents404) echo 'gh: Not Found (HTTP 404)' >&2; exit 1 ;;\n\
                 contents403) echo 'gh: Resource not accessible by integration (HTTP 403)' >&2; exit 1 ;;\n\
                 contents500) echo 'gh: boom (HTTP 500)' >&2; exit 1 ;;\n\
               esac ;;\n\
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
fn perf_verdicts_are_judged_for_rc_and_ga_and_both_accept_absent() {
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
    // Operator ruling 2026-11-12 (third set, B1): GA accepts ABSENT too; the GA notes
    // label it (see the label-step vectors below).
    let (output, calls) = aggregate_step("ga");
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        calls.contains("--perf-verdicts") && calls.contains("--perf-absent-ok"),
        "GA accepts ABSENT (B1): {calls}"
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
    // (a) only the BRANCH lookup's HTTP 404 is an absent branch: an empty ledger.
    for kind in ["rc", "ga"] {
        let (output, calls) = aggregate_step_with(kind, "404");
        assert!(output.status.success(), "{kind}: {}", text(&output));
        assert!(text(&output).contains("empty ledger"), "{}", text(&output));
        assert!(calls.contains("--perf-verdicts"), "{calls}");
        assert!(
            !calls.contains("contents/ledger.jsonl"),
            "an absent branch has no contents request: {calls}"
        );
        // (c) every other branch-lookup failure refuses, rate limits included.
        for mode in ["403", "ratelimit", "500"] {
            let (output, calls) = aggregate_step_with(kind, mode);
            assert!(!output.status.success(), "{kind}/{mode}: {}", text(&output));
            assert!(
                text(&output).contains("release refused: cannot read the perf-ledger"),
                "{kind}/{mode}: {}",
                text(&output)
            );
            assert!(!text(&output).contains("empty ledger"), "{}", text(&output));
            assert!(
                !calls.contains("check-perf-verdict"),
                "{kind}/{mode}: {calls}"
            );
            assert!(
                !calls.contains("check-release-precondition"),
                "{kind}/{mode}: {calls}"
            );
        }
    }
}

#[test]
fn a_perf_ledger_branch_that_exists_but_cannot_be_read_always_refuses() {
    // (b) review F1: the branch lookup succeeds and `ledger.jsonl` is 404 at the returned
    // commit. That is a broken ledger, never an absent branch; (d) 403 / 5xx refuse too.
    for kind in ["rc", "ga"] {
        for mode in ["contents404", "contents403", "contents500"] {
            let (output, calls) = aggregate_step_with(kind, mode);
            let shown = text(&output);
            assert!(!output.status.success(), "{kind}/{mode}: {shown}");
            assert!(
                shown.contains("release refused: cannot read the perf-ledger"),
                "{kind}/{mode}: {shown}"
            );
            assert!(
                !shown.contains("empty ledger"),
                "{kind}/{mode}: a missing file at an existing branch is not an empty ledger: {shown}"
            );
            assert!(
                calls.contains(
                    "contents/ledger.jsonl?ref=perftip0123456789abcdef0123456789abcdef01234"
                ),
                "the contents request was made at the logged tip: {calls}"
            );
            assert!(
                !calls.contains("check-perf-verdict"),
                "{kind}/{mode}: {calls}"
            );
            assert!(
                !calls.contains("check-release-precondition"),
                "refused before the precondition: {kind}/{mode}: {calls}"
            );
        }
    }
}

// ---- review F2: the notes' ABSENT label must agree with the computed verdicts ----------

const LABEL_STEP: &str =
    "Require the rc and GA notes perf label to agree with the perf verdicts (rc and GA tags only)";
const NOTES_GUARD_STEP: &str = "Require the release notes file (rc and GA tags only)";
const RC_OR_GA: &str = "steps.tag.outputs.kind == 'rc' || steps.tag.outputs.kind == 'ga'";
const ABSENT_LINE: &str = "Perf verdicts: ABSENT";

/// The `check-perf-verdict --json` shape: `{gate, sha, verdicts: [{metric, verdict, ..}]}`.
fn verdict_report(verdicts: &[&str]) -> String {
    let rows: Vec<serde_json::Value> = verdicts
        .iter()
        .enumerate()
        .map(|(i, verdict)| {
            serde_json::json!({
                "metric": format!("metric-{i}"),
                "verdict": verdict,
                "reason": "r",
                "binding": false
            })
        })
        .collect();
    serde_json::json!({ "gate": "check-perf-verdict", "sha": SHA, "verdicts": rows }).to_string()
}

/// Run the label step as the script it is for `kind` (`rc` | `ga`): `notes` is the content of
/// `docs/release/<tag>.md` (None = no file); `report` the verdict JSON file (None = no file).
fn label_step_for(kind: &str, notes: Option<&str>, report: Option<&str>) -> Output {
    let tag = if kind == "ga" {
        "v1.0.0"
    } else {
        "v1.0.0-rc.1"
    };
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let run = script(&steps, LABEL_STEP);
    let dir = tempfile::tempdir().unwrap();
    if let Some(notes) = notes {
        std::fs::create_dir_all(dir.path().join("docs/release")).unwrap();
        std::fs::write(dir.path().join(format!("docs/release/{tag}.md")), notes).unwrap();
    }
    if let Some(report) = report {
        std::fs::write(dir.path().join("perf-verdicts.json"), report).unwrap();
    }
    bash(&run, dir.path(), None, &[("TAG", tag), ("KIND", kind)])
}

/// The rc label step (`docs/release/v1.0.0-rc.1.md`).
fn label_step(notes: Option<&str>, report: Option<&str>) -> Output {
    label_step_for("rc", notes, report)
}

#[test]
fn the_label_step_is_an_rc_and_ga_step_after_the_verdicts_are_computed_and_before_publication() {
    let release = release();
    let steps = steps(&release, "sign-and-publish");
    let at = index_of(&steps, LABEL_STEP);
    assert_eq!(steps[at]["if"].as_str(), Some(RC_OR_GA));
    assert_eq!(
        steps[at]["env"]["TAG"].as_str(),
        Some("${{ steps.tag.outputs.tag }}")
    );
    assert_eq!(
        steps[at]["env"]["KIND"].as_str(),
        Some("${{ steps.tag.outputs.kind }}"),
        "the step reads the one classification, never re-derives the kind from the tag text"
    );
    assert!(index_of(&steps, "Require successful main discipline aggregate") < at);
    assert!(at < index_of(&steps, "Generate SHA256SUMS and sign"));
    assert!(at < index_of(&steps, "Create GitHub Release"));
}

// ---- operator ruling 2026-11-12, third set, B1: GA accepts ABSENT and its notes carry the label ----

#[test]
fn a_ga_with_the_label_and_an_all_absent_report_passes() {
    let notes = format!("# v1.0.0\n\n{ABSENT_LINE}\n\nmore\n");
    let output = label_step_for(
        "ga",
        Some(&notes),
        Some(&verdict_report(&["ABSENT", "ABSENT", "ABSENT"])),
    );
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("perf label agrees: every metric is ABSENT"),
        "{}",
        text(&output)
    );
    // Trailing whitespace is tolerated exactly as `check-rc-readiness` tolerates it.
    let padded = format!("# v1.0.0\n{ABSENT_LINE}  \n");
    let output = label_step_for(
        "ga",
        Some(&padded),
        Some(&verdict_report(&["ABSENT", "ABSENT"])),
    );
    assert!(output.status.success(), "{}", text(&output));
}

#[test]
fn a_ga_without_the_exact_label_line_is_refused_whatever_the_rows_even_all_pass() {
    for report in [
        verdict_report(&["ABSENT", "ABSENT"]),
        verdict_report(&["PASS", "PASS", "PASS"]),
        verdict_report(&["PASS", "ABSENT"]),
        verdict_report(&["FAIL"]),
        verdict_report(&[]),
        "{".to_string(),
    ] {
        for notes in [
            "# v1.0.0\n",
            "# v1.0.0\nPerf verdicts: PASS\n",
            "- `Perf verdicts: ABSENT`: a bullet quoting the label is not the line\n",
            "Perf verdicts: ABSENT (waived)\n",
            " Perf verdicts: ABSENT\n",
            "",
        ] {
            let output = label_step_for("ga", Some(notes), Some(&report));
            let shown = text(&output);
            assert!(!output.status.success(), "{notes:?} / {report}: {shown}");
            assert!(
                shown.contains(
                    "release refused: GA notes must carry the exact `Perf verdicts: ABSENT` line"
                ),
                "{notes:?} / {report}: {shown}"
            );
        }
    }
}

#[test]
fn a_ga_with_the_label_and_any_real_verdict_is_a_refused_contradiction() {
    let notes = format!("# v1.0.0\n{ABSENT_LINE}\n");
    for verdicts in [
        vec!["PASS", "ABSENT", "ABSENT"],
        vec!["PASS", "PASS", "PASS"],
        vec!["ABSENT", "FAIL", "ABSENT"],
        vec!["ABSENT", "INDETERMINATE"],
    ] {
        let output = label_step_for("ga", Some(&notes), Some(&verdict_report(&verdicts)));
        let shown = text(&output);
        assert!(!output.status.success(), "{verdicts:?}: {shown}");
        assert!(
            shown.contains("release refused: notes say ABSENT but the ledger has PASS/FAIL/INDETERMINATE verdicts: update the notes"),
            "{verdicts:?}: {shown}"
        );
    }
}

#[test]
fn a_ga_with_the_label_and_an_unreadable_or_empty_report_is_refused_not_assumed_absent() {
    let notes = format!("{ABSENT_LINE}\n");
    for report in [
        None,
        Some(""),
        Some("{"),
        Some("[]"),
        Some("{\"verdicts\":[]}"),
    ] {
        let output = label_step_for("ga", Some(&notes), report);
        assert!(!output.status.success(), "{report:?}: {}", text(&output));
        assert!(
            text(&output).contains("release refused"),
            "{report:?}: {}",
            text(&output)
        );
    }
}

#[test]
fn a_ga_with_no_notes_file_is_refused_by_the_label_step_itself() {
    // Unlike an rc (whose missing notes the readiness gate and the notes guard own), the GA
    // label step has no other label enforcer: it refuses a missing notes file whatever the report.
    for report in [
        verdict_report(&["ABSENT", "ABSENT"]),
        verdict_report(&["PASS", "PASS"]),
    ] {
        let output = label_step_for("ga", None, Some(&report));
        let shown = text(&output);
        assert!(!output.status.success(), "{report}: {shown}");
        assert!(
            shown.contains(
                "release refused: GA notes must carry the exact `Perf verdicts: ABSENT` line"
            ),
            "{report}: {shown}"
        );
    }
}

#[test]
fn an_absent_label_must_match_a_report_that_is_entirely_absent() {
    let notes = format!("# notes\n\n{ABSENT_LINE}\n\nmore\n");
    let all_absent = verdict_report(&["ABSENT", "ABSENT", "ABSENT"]);
    let output = label_step(Some(&notes), Some(&all_absent));
    assert!(output.status.success(), "{}", text(&output));

    // Trailing whitespace is tolerated exactly as `check-rc-readiness` tolerates it.
    let padded = format!("# notes\n{ABSENT_LINE}  \n");
    let output = label_step(Some(&padded), Some(&all_absent));
    assert!(output.status.success(), "{}", text(&output));
}

#[test]
fn an_absent_label_with_any_real_verdict_is_a_refused_contradiction() {
    let notes = format!("# notes\n{ABSENT_LINE}\n");
    for verdicts in [
        vec!["PASS", "ABSENT", "ABSENT"],
        vec!["ABSENT", "ABSENT", "PASS"],
        vec!["PASS", "PASS", "PASS"],
        vec!["ABSENT", "FAIL", "ABSENT"],
        vec!["ABSENT", "INDETERMINATE"],
    ] {
        let output = label_step(Some(&notes), Some(&verdict_report(&verdicts)));
        let shown = text(&output);
        assert!(!output.status.success(), "{verdicts:?}: {shown}");
        assert!(
            shown.contains("release refused: notes say ABSENT but the ledger has PASS/FAIL/INDETERMINATE verdicts: update the notes"),
            "{verdicts:?}: {shown}"
        );
        let real = verdicts
            .iter()
            .enumerate()
            .find(|(_, verdict)| **verdict != "ABSENT")
            .map(|(i, verdict)| format!("metric-{i}={verdict}"))
            .unwrap();
        assert!(shown.contains(&real), "the refusal names {real}: {shown}");
    }
}

#[test]
fn without_the_exact_label_line_this_step_has_no_opinion() {
    // The readiness gate refuses a missing or another label; this step must not double up.
    let real = verdict_report(&["PASS", "PASS"]);
    for notes in [
        Some("# notes\nPerf verdicts: PASS\n"),
        Some("# notes\n"),
        Some("- `Perf verdicts: ABSENT`: a bullet quoting the label is not the line\n"),
        Some("Perf verdicts: ABSENT (waived)\n"),
        Some(" Perf verdicts: ABSENT\n"),
    ] {
        let output = label_step(notes, Some(&real));
        assert!(output.status.success(), "{notes:?}: {}", text(&output));
    }
}

#[test]
fn an_absent_label_with_an_unreadable_or_empty_report_is_refused_not_assumed_absent() {
    let notes = format!("{ABSENT_LINE}\n");
    for report in [
        None,
        Some(""),
        Some("{"),
        Some("[]"),
        Some("{\"verdicts\":[]}"),
        Some("{\"verdicts\":[{\"metric\":\"m\"}]}"),
    ] {
        let output = label_step(Some(&notes), report);
        assert!(!output.status.success(), "{report:?}: {}", text(&output));
        assert!(
            text(&output).contains("release refused"),
            "{report:?}: {}",
            text(&output)
        );
    }
    // Missing notes: nothing to reconcile here, and no crash; the notes guard owns that.
    let output = label_step(None, Some(&verdict_report(&["PASS"])));
    assert!(output.status.success(), "{}", text(&output));
}

#[test]
fn the_real_check_perf_verdict_json_is_what_the_label_step_reads() {
    // Producer/consumer round trip: an empty ledger through the real binary is an
    // all-ABSENT report the step accepts under the label and the committed notes carry it.
    let dir = tempfile::tempdir().unwrap();
    let ledger = dir.path().join("ledger.jsonl");
    std::fs::write(&ledger, "").unwrap();
    let produced = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-perf-verdict", "--sha", SHA, "--ledger"])
        .arg(&ledger)
        .arg("--json")
        .current_dir(root())
        .env("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(produced.status.success(), "{}", text(&produced));
    let report = String::from_utf8(produced.stdout).unwrap();
    let notes = std::fs::read_to_string(root().join("docs/release/v1.0.0-rc.1.md")).unwrap();
    let output = label_step(Some(&notes), Some(&report));
    assert!(output.status.success(), "{}", text(&output));
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
