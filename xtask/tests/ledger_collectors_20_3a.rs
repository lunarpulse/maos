//! Execute the shipped collector/recorder, with real git remotes and jq/python.
#![cfg(unix)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
fn command(cwd: &Path, program: &str, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .expect("execute real program")
}
fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
struct Repo {
    _temp: tempfile::TempDir,
    work: PathBuf,
    remote: PathBuf,
}
impl Repo {
    fn new(kind: &str, provision: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let remote = temp.path().join("remote.git");
        success(command(
            temp.path(),
            "git",
            &["init", "--bare", remote.to_str().unwrap()],
        ));
        let work = temp.path().join("source");
        success(command(
            temp.path(),
            "git",
            &["init", "-b", "main", work.to_str().unwrap()],
        ));
        success(command(
            &work,
            "git",
            &["config", "user.name", "collector test"],
        ));
        success(command(
            &work,
            "git",
            &["config", "user.email", "collector@local"],
        ));
        fs::write(work.join("source.txt"), "source stays checked out\n").unwrap();
        fs::create_dir(work.join("scripts")).unwrap();
        for script in ["collect-ledger.sh", "record-fuzz.py"] {
            fs::copy(
                root().join("scripts").join(script),
                work.join("scripts").join(script),
            )
            .unwrap();
        }
        if provision {
            fs::write(
                work.join(if kind == "fuzz" {
                    "fuzz-ledger.json"
                } else {
                    "rto-evidence.toml"
                }),
                if kind == "fuzz" {
                    "{\"schema_version\":1,\"records\":[]}"
                } else {
                    "evidence = []\n"
                },
            )
            .unwrap();
        }
        success(command(&work, "git", &["add", "."]));
        success(command(
            &work,
            "git",
            &["commit", "-m", "fixture source and operator ledger"],
        ));
        success(command(
            &work,
            "git",
            &["remote", "add", "origin", remote.to_str().unwrap()],
        ));
        success(command(&work, "git", &["push", "origin", "HEAD:main"]));
        if provision {
            success(command(
                &work,
                "git",
                &["push", "origin", &format!("HEAD:refs/heads/{kind}-ledger")],
            ));
        }
        fs::create_dir(work.join("records")).unwrap();
        fs::write(work.join("records/record-test.json"), r#"{"target":"manifest_parser","cpu_seconds":37,"corpus_size":2,"timestamp":"2026-10-15T00:00:00Z"}"#).unwrap();
        fs::write(
            work.join("records/rto-evidence-entry.toml"),
            "[[evidence]]\ndrill_date = \"2026-10-15\"\nrto_seconds = 37\ndrill_success = true\n",
        )
        .unwrap();
        Self {
            _temp: temp,
            work,
            remote,
        }
    }
    fn collect(&self, kind: &str) -> Output {
        command(
            &self.work,
            "bash",
            &["scripts/collect-ledger.sh", kind, "records"],
        )
    }
    fn ledger(&self, kind: &str) -> String {
        let name = if kind == "fuzz" {
            "fuzz-ledger.json"
        } else {
            "rto-evidence.toml"
        };
        let output = success(command(
            &self.work,
            "git",
            &[
                "--git-dir",
                self.remote.to_str().unwrap(),
                "show",
                &format!("{kind}-ledger:{name}"),
            ],
        ));
        String::from_utf8(output.stdout).unwrap()
    }
}
fn red(label: &str, output: &Output) {
    assert!(!output.status.success(), "{label} incorrectly succeeded");
    eprintln!(
        "PLANTED RED {label}: status={}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn collectors_refuse_missing_operator_branch() {
    for kind in ["fuzz", "rto"] {
        let repo = Repo::new(kind, false);
        let output = repo.collect(kind);
        red(&format!("{kind} missing branch"), &output);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("::error::") && error.contains("ops-provisioning-secrets-and-accounts")
        );
        let refs = success(command(
            &repo.work,
            "git",
            &[
                "--git-dir",
                repo.remote.to_str().unwrap(),
                "for-each-ref",
                "--format=%(refname)",
            ],
        ));
        assert!(!String::from_utf8_lossy(&refs.stdout).contains("ledger"));
    }
}
#[test]
fn collectors_refuse_rejected_push() {
    use std::os::unix::fs::PermissionsExt;
    for kind in ["fuzz", "rto"] {
        let repo = Repo::new(kind, true);
        let before = repo.ledger(kind);
        let hook = repo.remote.join("hooks/pre-receive");
        fs::write(
            &hook,
            "#!/bin/sh\necho 'PLANTED pre-receive refusal' >&2\nexit 1\n",
        )
        .unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
        let output = repo.collect(kind);
        red(&format!("{kind} rejected push"), &output);
        assert!(String::from_utf8_lossy(&output.stderr).contains("pre-receive"));
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains("retrying append"),
            "hook refusals must fail immediately"
        );
        assert_eq!(repo.ledger(kind), before);
    }
}
#[test]
fn collectors_refuse_malformed_record() {
    for kind in ["fuzz", "rto"] {
        let repo = Repo::new(kind, true);
        let before = repo.ledger(kind);
        fs::write(
            repo.work.join(if kind == "fuzz" {
                "records/record-test.json"
            } else {
                "records/rto-evidence-entry.toml"
            }),
            "malformed!",
        )
        .unwrap();
        let output = repo.collect(kind);
        red(&format!("{kind} malformed record"), &output);
        assert_eq!(repo.ledger(kind), before);
    }
}
#[test]
fn collector_appends_exactly_one_element() {
    for kind in ["fuzz", "rto"] {
        let repo = Repo::new(kind, true);
        success(repo.collect(kind));
        let ledger = repo.ledger(kind);
        if kind == "fuzz" {
            let value: serde_json::Value = serde_json::from_str(&ledger).unwrap();
            assert_eq!(value["records"].as_array().unwrap().len(), 1);
            assert_eq!(value["records"][0]["cpu_seconds"], 37);
            success(repo.collect(kind));
            assert_eq!(
                repo.ledger(kind),
                ledger,
                "replaying a fuzz record must not duplicate it"
            );
        } else {
            let value: toml::Value = toml::from_str(&ledger).unwrap();
            assert_eq!(value["evidence"].as_array().unwrap().len(), 1);
        }
        let branch = success(command(&repo.work, "git", &["branch", "--show-current"]));
        assert_eq!(String::from_utf8_lossy(&branch.stdout).trim(), "main");
        eprintln!("GREEN {kind}: exactly one measured record appended; source checkout retained");
    }
}
#[test]
fn old_array_plus_object_append_is_proven_red() {
    let repo = Repo::new("fuzz", true);
    let script = repo.work.join("scripts/collect-ledger.sh");
    let source = fs::read_to_string(&script).unwrap();
    assert!(source.contains(".records += [$rec[0]]"));
    fs::write(
        script,
        source.replace(".records += [$rec[0]]", ".records += $rec[0]"),
    )
    .unwrap();
    let output = repo.collect("fuzz");
    red("old jq array + object", &output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be added"));
    let ledger: serde_json::Value = serde_json::from_str(&repo.ledger("fuzz")).unwrap();
    assert!(ledger["records"].as_array().unwrap().is_empty());
}
fn record(temp: &Path) -> Output {
    command(
        temp,
        "python3",
        &[
            root().join("scripts/record-fuzz.py").to_str().unwrap(),
            "--target",
            "manifest_parser",
            "--logs",
            "logs",
            "--corpus",
            "corpus",
            "--workers",
            "2",
            "--tier",
            "T3",
            "--commit",
            "0123456789012345678901234567890123456789",
            "--run-id",
            "test-1",
            "--output",
            "record.json",
        ],
    )
}
#[test]
fn recorder_refuses_missing_worker_counter_and_measures_real_corpus() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("logs")).unwrap();
    fs::create_dir(temp.path().join("corpus")).unwrap();
    fs::write(temp.path().join("corpus/seed"), b"seed").unwrap();
    fs::write(temp.path().join("corpus/discovered"), b"discovered").unwrap();
    fs::write(
        temp.path().join("logs/worker-0.log"),
        "Done 100 runs in 17 second(s)\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("logs/worker-1.log"),
        "worker stopped without libFuzzer counter\n",
    )
    .unwrap();
    let output = record(temp.path());
    red("worker missing counter", &output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("worker 1 missing"));
    assert!(!temp.path().join("record.json").exists());
    fs::write(
        temp.path().join("logs/worker-1.log"),
        "Done 200 runs in 23 second(s)\n",
    )
    .unwrap();
    success(record(temp.path()));
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(temp.path().join("record.json")).unwrap())
            .unwrap();
    assert_eq!(value["cpu_seconds"], 40);
    assert_eq!(value["corpus_size"], 2);
    assert_eq!(value["tier"], "T3");
    eprintln!("GREEN recorder: 17 + 23 = 40 libFuzzer seconds; real corpus count = 2");
}
#[test]
fn workflows_parse_and_gate_missing_ledger_without_skip() {
    for (file, gate, ledger) in [
        ("fuzz-cadence.yml", "check-fuzz-floor", "fuzz-ledger.json"),
        ("rpo-rto-cadence.yml", "check-rto-gate", "rto-evidence.toml"),
    ] {
        let workflow: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(root().join(".github/workflows").join(file)).unwrap(),
        )
        .unwrap();
        let job = &workflow["jobs"][gate];
        assert_eq!(job["if"].as_str(), Some("always()"));
        let steps = job["steps"].as_sequence().unwrap();
        let read = steps
            .iter()
            .find(|step| {
                step["name"].as_str() == Some("Read ledger without replacing current gate code")
            })
            .unwrap();
        let repo = Repo::new(
            if gate == "check-fuzz-floor" {
                "fuzz"
            } else {
                "rto"
            },
            false,
        );
        fs::write(repo.work.join(ledger), "stale main-branch evidence").unwrap();
        success(command(
            &repo.work,
            "bash",
            &["-euo", "pipefail", "-c", read["run"].as_str().unwrap()],
        ));
        assert!(
            !repo.work.join(ledger).exists(),
            "gate must see ABSENT, never stale evidence"
        );
        let run = steps
            .iter()
            .find(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.starts_with("cargo run"))
            })
            .unwrap();
        assert!(run.get("if").is_none(), "gate command may not be skipped");
    }
    let workflow: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/fuzz-prerelease.yml")).unwrap(),
    )
    .unwrap();
    assert_eq!(workflow["jobs"].as_mapping().unwrap().len(), 6);
}

#[test]
fn t3_final_assertion_executes_measured_floor_and_link_results() {
    let workflow: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/fuzz-prerelease.yml")).unwrap(),
    )
    .unwrap();
    let steps = workflow["jobs"]["t3-ledger-collect"]["steps"]
        .as_sequence()
        .unwrap();
    let step = steps
        .iter()
        .find(|step| {
            step["name"].as_str() == Some("Require all five links and at least 24h per target")
        })
        .unwrap();
    let script = step["run"].as_str().unwrap();
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("records")).unwrap();
    let sha = "0123456789012345678901234567890123456789";
    for target in ["manifest_parser", "frame_deser", "fuzz_exec_deps_parse_elf"] {
        for link in 1..=5 {
            let row =
                serde_json::json!({"target":target,"tier":"T3","commit":sha,"cpu_seconds":18000});
            fs::write(
                temp.path()
                    .join(format!("records/record-{target}-{link}.json")),
                row.to_string(),
            )
            .unwrap();
        }
    }
    let run = |seconds_red: bool, result: &str| {
        let results: serde_json::Value = (1..=5)
            .map(|link| {
                (
                    format!("fuzz-link-{link}"),
                    serde_json::json!({"result":result}),
                )
            })
            .collect::<serde_json::Map<String, serde_json::Value>>()
            .into();
        if seconds_red {
            let row = serde_json::json!({"target":"manifest_parser","tier":"T3","commit":sha,"cpu_seconds":0});
            fs::write(
                temp.path().join("records/record-manifest_parser-1.json"),
                row.to_string(),
            )
            .unwrap();
        }
        Command::new("bash")
            .args(["-euo", "pipefail", "-c", script])
            .current_dir(temp.path())
            .env("GITHUB_SHA", sha)
            .env("LINK_RESULTS", results.to_string())
            .output()
            .unwrap()
    };
    let output = success(run(false, "success"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("90000 libFuzzer seconds"));
    let output = run(false, "failure");
    red("T3 failed link", &output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("T3 link failed or did not run"));
    let output = run(true, "success");
    red("T3 measured total below 24h", &output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("only 72000 measured seconds"));
}

#[test]
fn collector_refuses_multiple_json_values_without_changing_ledger() {
    let repo = Repo::new("fuzz", true);
    let before = repo.ledger("fuzz");
    let path = repo.work.join("records/record-test.json");
    let valid = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        format!("{{\"cpu_seconds\":\"malformed\"}}\n{valid}\n"),
    )
    .unwrap();
    let output = repo.collect("fuzz");
    red("two JSON values (malformed first, valid last)", &output);
    assert_eq!(repo.ledger("fuzz"), before);
    fs::write(path, valid).unwrap();
    success(repo.collect("fuzz"));
    let ledger: serde_json::Value = serde_json::from_str(&repo.ledger("fuzz")).unwrap();
    assert_eq!(ledger["records"].as_array().unwrap().len(), 1);
    eprintln!("GREEN single-value validation: exactly one valid record accepted");
}

#[test]
fn nightly_scan_executes_real_tree_and_rejects_planted_inline_override() {
    let workflow: serde_yaml::Value = serde_yaml::from_str(
        &fs::read_to_string(root().join(".github/workflows/discipline.yml")).unwrap(),
    )
    .unwrap();
    let step = workflow["jobs"]["reproducible-build"]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|step| step["name"].as_str() == Some("Grep for nightly-only features"))
        .unwrap();
    let script = step["run"].as_str().unwrap();
    let output = success(command(
        &root(),
        "bash",
        &["-euo", "pipefail", "-c", script],
    ));
    assert!(String::from_utf8_lossy(&output.stdout).contains("No nightly-only features detected."));
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("scripts")).unwrap();
    std::os::unix::fs::symlink(root().join("crates"), temp.path().join("crates")).unwrap();
    std::os::unix::fs::symlink(root().join("xtask"), temp.path().join("xtask")).unwrap();
    let source = fs::read_to_string(root().join("scripts/run-fuzz-link.sh")).unwrap();
    fs::write(
        temp.path().join("scripts/run-fuzz-link.sh"),
        format!("{source}\ncargo +nightly fuzz build\n"),
    )
    .unwrap();
    let output = command(temp.path(), "bash", &["-euo", "pipefail", "-c", script]);
    red("inline nightly override in shipped script copy", &output);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("violation: cargo +nightly referenced")
    );
    eprintln!("GREEN real-tree reproducible-build scan; planted inline override rejected");
}

#[test]
fn concurrent_collectors_refetch_and_preserve_both_records_exactly_once() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    for retries in [false, true] {
        let repo = Repo::new("fuzz", true);
        let second = repo._temp.path().join("second");
        success(command(
            repo._temp.path(),
            "git",
            &[
                "clone",
                "--branch",
                "main",
                repo.remote.to_str().unwrap(),
                second.to_str().unwrap(),
            ],
        ));
        fs::create_dir(second.join("records")).unwrap();
        let mut row: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(repo.work.join("records/record-test.json")).unwrap(),
        )
        .unwrap();
        row["cpu_seconds"] = 59.into();
        fs::write(second.join("records/record-second.json"), row.to_string()).unwrap();
        if !retries {
            let path = repo.work.join("scripts/collect-ledger.sh");
            let source = fs::read_to_string(&path).unwrap();
            let condition = "if [ \"$retry\" != true ] || [ \"$attempt\" -eq 5 ]; then";
            assert!(source.contains(condition));
            fs::write(path, source.replace(condition, "if true; then")).unwrap();
        }
        let entered = repo._temp.path().join("first-push-entered");
        let release = repo._temp.path().join("release-first-push");
        let hook = repo.work.join(".git/hooks/post-commit");
        fs::write(&hook, format!(
            "#!/bin/sh\ntouch '{}'\nfor attempt in $(seq 1 1000); do\n  [ -e '{}' ] && exit 0\n  sleep 0.01\ndone\nexit 1\n",
            entered.display(), release.display(),
        )).unwrap();
        fs::set_permissions(hook, fs::Permissions::from_mode(0o755)).unwrap();
        let child = Command::new("bash")
            .args(["scripts/collect-ledger.sh", "fuzz", "records"])
            .current_dir(&repo.work)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !entered.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            entered.exists(),
            "first collector must reach real git post-commit hook before pushing"
        );
        let second_output = command(
            &second,
            "bash",
            &["scripts/collect-ledger.sh", "fuzz", "records"],
        );
        // Always release the blocked real push, including on a failed second collector.
        fs::write(release, "").unwrap();
        let first_output = child.wait_with_output().unwrap();
        success(second_output);
        if retries {
            let first_output = success(first_output);
            assert!(String::from_utf8_lossy(&first_output.stdout).contains("retrying append"));
            let ledger: serde_json::Value = serde_json::from_str(&repo.ledger("fuzz")).unwrap();
            let rows = ledger["records"].as_array().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(
                rows.iter().filter(|row| row["cpu_seconds"] == 37).count(),
                1
            );
            assert_eq!(
                rows.iter().filter(|row| row["cpu_seconds"] == 59).count(),
                1
            );
            eprintln!(
                "GREEN concurrent collectors: fetch-first retry; both records present exactly once"
            );
        } else {
            red(
                "concurrent append with old no-retry behavior",
                &first_output,
            );
            let error = String::from_utf8_lossy(&first_output.stderr);
            assert!(
                error.contains("[rejected] (fetch first)")
                    || error.contains("[rejected] (non-fast-forward)")
            );
        }
    }
    for file in ["fuzz-cadence.yml", "fuzz-prerelease.yml"] {
        let workflow: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(root().join(".github/workflows").join(file)).unwrap(),
        )
        .unwrap();
        assert!(
            workflow.get("concurrency").is_none(),
            "soaks must not cancel pending release chains"
        );
    }
}
