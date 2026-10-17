//! Execute the shipped workflow scripts; fixtures provide inputs, never a second implementation.
use serde_json::json;
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
fn workflow(name: &str) -> Value {
    serde_yaml::from_str(&fs::read_to_string(root().join(".github/workflows").join(name)).unwrap())
        .unwrap()
}
fn step<'a>(wf: &'a Value, job: &str, name: &str) -> &'a Value {
    wf["jobs"][job]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
}
fn script(wf: &Value, job: &str, name: &str) -> String {
    step(wf, job, name)["run"].as_str().unwrap().to_owned()
}
fn bash(script: &str, dir: &Path, vars: &[(&str, &str)]) -> Output {
    Command::new("/bin/bash")
        .args([
            "--noprofile",
            "--norc",
            "-e",
            "-o",
            "pipefail",
            "-c",
            script,
        ])
        .current_dir(dir)
        .envs(vars.iter().copied())
        .output()
        .unwrap()
}
fn show(label: &str, output: &Output) {
    println!(
        "{label}: rc={}\n{}{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn expand(script: &str, values: &[(&str, &str)]) -> String {
    let re = regex::Regex::new(r"\$\{\{\s*(.*?)\s*\}\}").unwrap();
    let needs_result = regex::Regex::new(r"^needs\.[A-Za-z0-9_-]+\.result$").unwrap();
    re.replace_all(script, |caps: &regex::Captures<'_>| {
        let expression = caps[1].trim();
        values
            .iter()
            .find(|(k, _)| *k == expression)
            .map(|(_, v)| *v)
            .unwrap_or_else(|| {
                // Unspecified job results model the successful baseline, not arbitrary outputs.
                assert!(
                    needs_result.is_match(expression),
                    "unmatched workflow expression: {expression}"
                );
                "success"
            })
            .to_owned()
    })
    .into_owned()
}

#[test]
fn expansion_defaults_only_job_results() {
    assert_eq!(expand("${{ needs.check-unsafe.result }}", &[]), "success");
    assert_eq!(
        expand(
            "${{ steps.body.outputs.status }}",
            &[("steps.body.outputs.status", "absent")]
        ),
        "absent"
    );
    for expression in [
        "needs.check.outputs.result",
        "github.sha",
        "needs.check.result || true",
    ] {
        let panic = std::panic::catch_unwind(|| expand(&format!("${{{{ {expression} }}}}"), &[]))
            .expect_err("unknown expressions must refuse");
        let message = panic.downcast_ref::<String>().unwrap();
        assert!(message.contains(expression));
    }
}

fn inventory(source: &str) -> Vec<String> {
    let mut job_start = 0;
    let lines: Vec<_> = source.lines().collect();
    let mut errors = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if line.starts_with("  ") && !line.starts_with("   ") && trimmed.ends_with(':') {
            job_start = i;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        let advisory = |s: &str| {
            s.contains("ADVISORY — owner:")
                && s.contains("; reason:")
                && s.split("owner:")
                    .nth(1)
                    .is_some_and(|x| !x.split(';').next().unwrap_or("").trim().is_empty())
                && s.split("; reason:")
                    .nth(1)
                    .is_some_and(|x| !x.trim().is_empty())
        };
        if (trimmed.starts_with("continue-on-error:")
            || trimmed.starts_with("- continue-on-error:"))
            && !lines[job_start..=i]
                .iter()
                .any(|l| l.contains('#') && advisory(l))
        {
            errors.push(format!("{}: unlabelled continue-on-error", i + 1));
        }
        if line.contains("|| echo") {
            let prior = i.checked_sub(1).map(|n| lines[n].trim()).unwrap_or("");
            if !prior.starts_with("# rule-11 benign: ")
                && !(prior.starts_with('#') && advisory(prior))
            {
                errors.push(format!("{}: unclassified || echo", i + 1));
            }
        }
    }
    errors
}

#[test]
fn every_workflow_escape_hatch_has_an_owner_or_benign_classification() {
    let mut errors = Vec::new();
    for entry in fs::read_dir(root().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        if !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        errors.extend(
            inventory(&source)
                .into_iter()
                .map(|e| format!("{}:{e}", path.display())),
        );
        // Plant both escape hatches into a COPY of every real workflow. No tracked mutation.
        for plant in [
            "    continue-on-error: true\n",
            "    steps:\n      - run: false || echo hidden\n",
        ] {
            let planted = format!("{source}\n  unlabelled-regrowth:\n{plant}");
            assert!(
                !inventory(&planted).is_empty(),
                "unlabelled regrowth escaped inventory"
            );
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn grep_scans_fail_on_unreadable_input_but_not_no_match() {
    let wf = workflow("discipline.yml");
    for (job, name) in [
        ("reproducible-build", "Grep for nightly-only features"),
        (
            "maos-audit-read-only",
            "Grep for write-open patterns in maos-audit",
        ),
    ] {
        let source = script(&wf, job, name);
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("crates/maos-audit")).unwrap();
        fs::create_dir(dir.path().join("xtask")).unwrap();
        let out = bash(&source, dir.path(), &[]);
        show(&format!("{job} no-match"), &out);
        assert!(out.status.success());
        fs::remove_dir_all(dir.path().join("crates")).unwrap();
        let out = bash(&source, dir.path(), &[]);
        show(&format!("{job} unreadable scan input (must RED)"), &out);
        assert!(
            !out.status.success(),
            "grep infrastructure error was swallowed"
        );
        let old_mask = source.replace("if [ \"$rc\" -ge 2 ]; then exit \"$rc\"; fi", ":");
        let out = bash(&old_mask, dir.path(), &[]);
        show(
            &format!("{job} planted old mask (oracle rejects rc=0)"),
            &out,
        );
        assert!(
            out.status.success(),
            "the planted old mask must reproduce the defect"
        );
    }
}

#[test]
fn cargo_tree_failure_and_default_fault_feature_are_blocking() {
    let wf = workflow("discipline.yml");
    // Every "fault feature absent from the release graph" step, not only J4's:
    // the same invalid `cargo tree --release` once made all three vacuous.
    for (job, step, package, feature) in [
        (
            "check-j4-latency",
            "Verify bench-fault-inject absent from release graph (D3)",
            "maos-bench",
            "bench-fault-inject",
        ),
        (
            "check-wasm-form-equiv",
            "Verify equiv-fault-inject absent from release graph (D7)",
            "maos-wasm-host",
            "equiv-fault-inject",
        ),
        (
            "check-multi-region-slo",
            "slo-fault-inject absent from release feature graph",
            "maos-bench",
            "slo-fault-inject",
        ),
    ] {
        let source = script(&wf, job, step);
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "").unwrap();
        let planted_default = format!("[\"{feature}\"]");
        for (label, name, default, success) in [
            ("clean graph", package, "[]", true),
            (
                "cargo tree failure (must RED)",
                "bogus-package",
                "[]",
                false,
            ),
            (
                "default fault feature (must RED)",
                package,
                planted_default.as_str(),
                false,
            ),
        ] {
            fs::write(dir.path().join("Cargo.toml"), format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[workspace]\n[features]\ndefault = {default}\n{feature} = []\n")).unwrap();
            let out = bash(&source, dir.path(), &[]);
            show(&format!("{job}: {label}"), &out);
            assert_eq!(out.status.success(), success, "{job}: {label}");
            if label.starts_with("default fault") {
                let old_mask = source.replace(&format!(" --invert {package}"), "");
                let out = bash(&old_mask, dir.path(), &[]);
                show("planted non-inverted graph (oracle rejects rc=0)", &out);
                assert!(
                    out.status.success(),
                    "{job}: without inversion Cargo hides the root default feature"
                );
            }
        }
    }
}

#[test]
fn aggregate_executes_js_and_reports_advisory_failure_and_not_run() {
    let wf = workflow("discipline.yml");
    let source = step(&wf, "aggregate", "Post/update PR comment")["with"]["script"]
        .as_str()
        .unwrap();
    let source = expand(
        source,
        &[
            ("needs.determinism-tests.outputs.test_result", "failure"),
            ("needs.check-unsafe.result", "skipped"),
            ("needs.macos-check.result", "skipped"),
            ("needs.macos-scope.outputs.applicable", "false"),
        ],
    );
    let harness = format!("const context = {{repo:{{owner:'o',repo:'r'}},issue:{{number:1}}}}; const github = {{rest:{{issues:{{listComments:async()=>({{data:[]}}),createComment:async x=>console.log(x.body)}}}}}}; (async()=>{{{source}}})().catch(e=>{{console.error(e);process.exit(1)}});");
    let out = Command::new("node")
        .args(["-e", &harness])
        .output()
        .expect("node executes github-script");
    show("aggregate github-script", &out);
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(body.contains("| determinism-tests (advisory)¹ | ⚠️ failure |"));
    assert!(body
        .lines()
        .find(|l| l.starts_with("| check-unsafe |"))
        .unwrap()
        .contains("NOT RUN"));
    assert!(body.contains("NOT APPLICABLE (not a pass)"));
}

#[test]
fn aggregate_skip_predicate_exempts_only_macos_and_summary_says_not_run() {
    let wf = workflow("discipline.yml");
    let source = script(&wf, "aggregate", "Check for failures");
    let summary = script(&wf, "aggregate", "Build results table");
    let fail = step(&wf, "aggregate", "Fail if any gate failed");
    assert_eq!(
        fail["if"].as_str(),
        Some("${{ steps.status.outputs.overall == 'failure' }}")
    );
    for (name, result, expected) in [
        ("check-unsafe", "success", "success"),
        ("check-unsafe", "skipped", "failure"),
        ("macos-check", "skipped", "success"),
        ("macos-check", "failure", "failure"),
        ("check-unsafe", "cancelled", "failure"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let table = dir.path().join("summary");
        let mut needs = json!({name: {"result": result}});
        if name != "macos-check" {
            needs["macos-check"] = json!({"result":"skipped"});
        }
        let needs = needs.to_string();
        let vars = [
            ("NEEDS_JSON", needs.as_str()),
            ("GITHUB_OUTPUT", output.to_str().unwrap()),
            ("GITHUB_STEP_SUMMARY", table.to_str().unwrap()),
        ];
        let out = bash(&source, dir.path(), &vars);
        show(&format!("aggregate {name}={result}"), &out);
        assert!(out.status.success());
        assert!(fs::read_to_string(&output)
            .unwrap()
            .contains(&format!("overall={expected}")));
        if expected == "failure" {
            let out = bash(fail["run"].as_str().unwrap(), dir.path(), &vars);
            show("aggregate final failure step (must RED)", &out);
            assert!(!out.status.success());
        }
        let out = bash(&expand(&summary, &[]), dir.path(), &vars);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        if result == "skipped" {
            assert!(fs::read_to_string(table).unwrap().contains("NOT RUN"));
        }
    }
}

#[test]
fn journal_fetch_failure_is_fatal_and_missing_pr_is_explicitly_unverified() {
    let wf = workflow("journal-append.yml");
    let fetch = script(
        &wf,
        "emit-journal-entry",
        "Fetch PR body for revert detection",
    );
    assert!(
        step(
            &wf,
            "emit-journal-entry",
            "Fetch PR body for revert detection"
        )["if"]
            .is_null(),
        "no-PR path must run and name the missing evidence"
    );
    let dir = tempfile::tempdir().unwrap();
    let outpath = dir.path().join("output");
    let tmp = dir.path().to_str().unwrap();
    let fetch = fetch.replace("/tmp/", &format!("{tmp}/"));
    let out = bash(
        &fetch,
        dir.path(),
        &[
            ("PR_NUMBER", ""),
            ("GITHUB_OUTPUT", outpath.to_str().unwrap()),
        ],
    );
    show("journal missing PR", &out);
    assert!(out.status.success());
    assert!(fs::read_to_string(&outpath)
        .unwrap()
        .contains("revert_detection=unverified"));
    // Real command resolution fails without gh/network; a known PR must not erase that failure.
    let out = bash(
        &fetch,
        dir.path(),
        &[
            ("PR_NUMBER", "123"),
            ("GITHUB_OUTPUT", outpath.to_str().unwrap()),
            ("PATH", "/nonexistent"),
        ],
    );
    show("journal failed body fetch (must RED)", &out);
    assert!(!out.status.success());
    let old_mask = fetch.replace(
        &format!("> {tmp}/pr-body.txt\n"),
        &format!("> {tmp}/pr-body.txt || echo '' > {tmp}/pr-body.txt\n"),
    );
    let out = bash(
        &old_mask,
        dir.path(),
        &[
            ("PR_NUMBER", "123"),
            ("GITHUB_OUTPUT", outpath.to_str().unwrap()),
            ("PATH", "/nonexistent"),
        ],
    );
    show(
        "journal planted body-fetch mask (oracle rejects rc=0)",
        &out,
    );
    assert!(out.status.success());
    let source = script(
        &wf,
        "emit-journal-entry",
        "Run invariant-lock with --write-journal",
    );
    // Journal data is an input; the annotation/display logic comes only from the shipped workflow.
    let row = json!({"sha": "planted-entry", "invariant_ids": ["I1"]});
    fs::create_dir(dir.path().join("journal-output")).unwrap();
    fs::write(
        dir.path().join("journal-output/entry.jsonl"),
        format!("{row}\n"),
    )
    .unwrap();
    let suffix = source
        .split_once("# Persist the evidence boundary on every emitted journal row.\n")
        .expect("journal annotation")
        .1;
    let source = expand(
        suffix,
        &[("steps.body.outputs.revert_detection", "unverified")],
    )
    .replace("/tmp/", &format!("{tmp}/"));
    let out = bash(&source, dir.path(), &[]);
    show("journal entry missing PR", &out);
    assert!(out.status.success());
    let entry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.path().join("journal-output/entry.jsonl")).unwrap(),
    )
    .unwrap();
    assert_eq!(entry["revert_detection"], "unverified");
}

#[test]
fn journal_pr_resolution_accepts_a_missing_trailer_without_masking_git_failure() {
    let wf = workflow("journal-append.yml");
    let source = script(
        &wf,
        "emit-journal-entry",
        "Resolve PR number from merge commit message",
    );
    for (message, expected) in [
        ("direct merge", ""),
        ("Merge (#12) then (#34)", "pr_number=34"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let out = Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let out = Command::new("git")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@local",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                message,
            ])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let output = dir.path().join("output");
        fs::write(&output, "").unwrap();
        let vars = [("GITHUB_OUTPUT", output.to_str().unwrap())];
        let out = bash(
            &expand(&source, &[("steps.sha.outputs.merge_sha", "HEAD")]),
            dir.path(),
            &vars,
        );
        show(&format!("journal PR resolution {message}"), &out);
        assert!(out.status.success());
        assert_eq!(fs::read_to_string(&output).unwrap().trim(), expected);
        let out = bash(
            &expand(&source, &[("steps.sha.outputs.merge_sha", "bad-sha")]),
            dir.path(),
            &vars,
        );
        show("journal bad merge SHA (must RED)", &out);
        assert!(!out.status.success());
    }
}

#[test]
fn j4_budget_and_smoke_jobs_make_no_stable_runner_claim() {
    let registry: toml::Value =
        toml::from_str(&fs::read_to_string(root().join("xtask/gate-registry.toml")).unwrap())
            .unwrap();
    let row = registry["ship_gate"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"].as_str() == Some("check-j4-latency"))
        .unwrap();
    assert_eq!(row["disposition"]["v1_5"].as_str(), Some("advisory"));
    let source =
        fs::read_to_string(root().join("crates/maos-bench/tests/t_10_4c_j4_latency_gate.rs"))
            .unwrap();
    assert!(source.contains("ADVISORY over budget (binding: 20-7)"));
    assert!(!source.contains("WOULD BLOCK at v1.5"));
    let wf = workflow("discipline.yml");
    let j4 = &wf["jobs"]["check-j4-latency"];
    assert!(j4["continue-on-error"].is_null());
    assert!(j4["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .all(|s| s["continue-on-error"].is_null()));
    let source = fs::read_to_string(root().join(".github/workflows/discipline.yml")).unwrap();
    for name in ["founder-loop-bench", "mira-nash-bench"] {
        let section = source
            .split(&format!("  {name}:\n"))
            .nth(1)
            .unwrap()
            .split("\n  #")
            .next()
            .unwrap();
        assert!(section.contains("correctness smoke (`--test`, one iteration);"));
        assert!(section.contains("no timing baseline exists — 20-7 owns regression"));
        assert!(script(
            &wf,
            name,
            wf["jobs"][name]["steps"]
                .as_sequence()
                .unwrap()
                .last()
                .unwrap()["name"]
                .as_str()
                .unwrap()
        )
        .contains("-- --test"));
    }
}
