//! Deterministic vectors over the production prepush reader and runner.
// Keep private helpers private while exercising the identical implementation.
#![allow(dead_code)]
include!("../src/prepush.rs");

#[test]
fn reader_follows_env_pins_multiple_binaries_and_nextest() {
    let yaml = "env:\n  CARGO_TERM_COLOR: never\n  COUNT: 3\njobs:\n  process:\n    steps:\n      - name: counts\n        run: |\n          cargo test --locked -p p --test a --test b -- --nocapture 2>&1 | tee log\n          python3 check --expected-count 2 --expected-count \"${COUNT}\" log\n      - name: archive\n        env:\n          NEXTEST_ARCHIVE_PACKAGES: p\n          NEXTEST_ARCHIVE_FEATURES: network,wasm-host\n          NEXTEST_ARCHIVE_TESTS: c\n        run: |\n          cargo nextest archive -p p --features network,wasm-host --test c --archive-file archive.tar.zst\n          cargo-nextest nextest run --archive-file archive.tar.zst --extract-to \"$ROOT\" --workspace-remap \"$ROOT\" -E 'binary_id(p::c)' --color never --test-threads 1 --no-capture 2>&1 | tee log\n          grep -q '4 tests run: 4 passed' log\n          for spec in 'MARK :2'; do echo; done\n";
    let pins = read_pins(yaml).unwrap();
    assert_eq!(pins.len(), 2);
    assert_eq!(pins[0].counts, vec![2, 3]);
    assert_eq!(pins[1].counts, vec![4]);
    assert_eq!(pins[1].markers, vec![("MARK ".into(), 2)]);
    assert!(pins.iter().all(|p| p.ansi_safe));
    assert!(!pins[1].selections[0]
        .args
        .iter()
        .any(|a| a.contains("archive")));
    assert!(pins.iter().all(|p| p.parse_error.is_none()));
    assert!(pins[1].selections[0]
        .args
        .iter()
        .any(|a| a == "network,wasm-host"));
    assert_eq!(
        expand(
            "$COUNT ${COUNT} $COUNTX ${UNKNOWN}",
            &BTreeMap::from([("COUNT".into(), "3".into())])
        ),
        "3 3 $COUNTX ${UNKNOWN}"
    );
}

#[test]
fn ansi_drift_and_unresolved_filters_fail_closed() {
    let yaml = "env:\n  CARGO_TERM_COLOR: always\njobs:\n  count:\n    steps:\n      - run: |\n          cargo test -p p --test a\n          grep -q 'test result: ok\\. 2 passed;' log\n";
    let pins = read_pins(yaml).unwrap();
    assert!(!pins[0].ansi_safe);
    assert_eq!(pins[0].counts, vec![2]);
    assert!(selection("-p p --test $UNKNOWN").is_err());
    assert!(selection("-p p --test a --ignored --include-ignored").is_err());
    for unsupported in [
        "--package p",
        "--filter-expr 'all()'",
        "--all-features",
        "--archive-file=bundle",
    ] {
        assert!(selection(unsupported).is_err(), "{unsupported}");
    }
    assert!(read_pins("jobs: {}\n").is_err());
    let diagnostic = format!(
        "{yaml}      - env:\n          NEXTEST_ARCHIVE_PACKAGES: maos-escape-detector\n          NEXTEST_ARCHIVE_FEATURES: ''\n          NEXTEST_ARCHIVE_TESTS: ''\n        run: |\n          cargo-nextest nextest run --archive-file bundle -E 'binary_id(/^maos-escape-detector(::|$)/)' --color never | tee log\n          grep -q 'error:' log\n"
    );
    let pins = read_pins(&diagnostic).unwrap();
    assert!(pins[1].parse_error.is_none());
    assert!(pins[1].selections.is_empty());
    assert!(pins[1].ansi_safe);
    let mut report = Report::default();
    check_pins(Path::new("."), &pins[1..], &mut report);
    assert_eq!(report.exit_code(), 0);
    let unsafe_diagnostic = diagnostic.replace("--color never |", "--color always |");
    let pins = read_pins(&unsafe_diagnostic).unwrap();
    let mut report = Report::default();
    check_pins(Path::new("."), &pins[1..], &mut report);
    assert_eq!(report.records[0].check, "ci-pins:ansi");
    assert_eq!(report.exit_code(), 1);
    let pinned = diagnostic.replace(
        "grep -q 'error:' log",
        "grep -q 'test result: ok. 1 passed;' log",
    );
    let pins = read_pins(&pinned).unwrap();
    assert!(pins[1]
        .parse_error
        .as_ref()
        .unwrap()
        .contains("unresolved selection"));
    let mut report = Report::default();
    check_pins(Path::new("."), &pins[1..], &mut report);
    assert_eq!(report.records[0].check, "ci-pins:selection");
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn list_counts_use_selected_cases_not_total_and_honor_ignored() {
    let json = r#"{"rust-suites":{"p::a":{"testcases":{"a":{"ignored":false,"filter-match":{"status":"matches"}},"b":{"ignored":true,"filter-match":{"status":"matches"}},"c":{"ignored":false,"filter-match":{"status":"mismatch"}}}}}}"#;
    let mut select = selection("-p p --test a").unwrap();
    assert_eq!(listed_counts(json, &select).unwrap(), vec![1]);
    select.include_ignored = true;
    assert_eq!(listed_counts(json, &select).unwrap(), vec![2]);
    assert!(listed_counts("{}", &select).is_err());
    let ordered = r#"{"rust-suites":{"p::z":{"testcases":{"z":{"ignored":false,"filter-match":{"status":"matches"}}}},"p::a":{"testcases":{"a":{"ignored":false,"filter-match":{"status":"matches"}},"b":{"ignored":false,"filter-match":{"status":"matches"}}}}}}"#;
    assert_eq!(listed_counts(ordered, &select).unwrap(), vec![2, 1]);
    assert!(listed_counts(&json.replace("mismatch", "unknown"), &select).is_err());
}

#[test]
fn marker_sites_ignore_comments_and_assertions_and_expose_dynamic_counts() {
    let file = syn::parse_file(
        r#"
        fn helper() { println!("HELPER value=1"); }
        #[test] fn test() {
            // println!("STATIC comment");
            assert!("STATIC assertion".contains("STATIC"));
            println!("STATIC value=1");
            for _ in 0..8 { println!("LOOP value=1"); }
        }
    "#,
    )
    .unwrap();
    let mut emissions = Emissions::default();
    emissions.visit_file(&file);
    assert_eq!(
        emissions.sites,
        vec![
            ("HELPER value=1".into(), true),
            ("STATIC value=1".into(), false),
            ("LOOP value=1".into(), true)
        ]
    );
}

#[test]
fn command_runner_captures_red_and_unsets_local_ci_environment() {
    let mut report = Report::default();
    let mut red = Command::new("sh");
    red.args(["-c", "printf 'SKIP: substrate unavailable\\n' >&2; exit 7"]);
    report.command("stub", "artifact/pin", &mut red);
    let mut green = Command::new("sh");
    green
        .args([
            "-c",
            "test -z \"${CI+x}\" && test -z \"${GITHUB_ACTIONS+x}\" && echo 'SKIP arbitrary stdout'",
        ])
        .env("CI", "true")
        .env("GITHUB_ACTIONS", "true");
    report.command("local-profile", "environment", &mut green);
    assert_eq!(report.exit_code(), 1);
    // Captured SKIP lines stay countable diagnostics, never check dispositions.
    let statuses: Vec<_> = report.records.iter().map(|r| r.status).collect();
    assert_eq!(statuses, ["red", "pass"]);
    let captured: Vec<_> = report
        .output_diagnostics
        .iter()
        .map(|d| (d.check.as_str(), d.line.as_str()))
        .collect();
    assert_eq!(
        captured,
        [
            ("stub", "SKIP: substrate unavailable"),
            ("local-profile", "SKIP arbitrary stdout")
        ]
    );
    let text = report.text();
    assert!(!text.lines().any(|l| l.starts_with("SKIP ")));
    assert!(
        text.contains("2 checks run, 0 skipped-with-reason, 1 red, 2 captured SKIP output line(s)")
    );
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["records"].as_array().unwrap().len(), 2);
    assert_eq!(
        json["output_diagnostics"][1]["line"],
        "SKIP arbitrary stdout"
    );
    assert_eq!(text.lines().filter(|l| l.starts_with("RED ")).count(), 1);
    let mut missing = command(Path::new("."), "missing-required-prepush-tool", "");
    assert!(report
        .command("required-tool", "tool/pin", &mut missing)
        .is_none());
    assert_eq!(report.records.last().unwrap().status, "red");
    assert!(report.text().contains("required tool"));
}

#[test]
fn red_detail_is_bounded_and_keeps_failure_lines() {
    let mut report = Report::default();
    let mut noisy = Command::new("sh");
    noisy.args([
        "-c",
        "echo 'FAIL [ 1.000s] p::a early'; printf 'error %0600d\\n' 0; i=0; while [ $i -lt 3000 ]; do echo \"passing output $i\"; i=$((i+1)); done; echo 'last line' >&2; exit 3",
    ]);
    report.command("noisy", "artifact", &mut noisy);
    let record = &report.records[0];
    assert_eq!(record.status, "red");
    assert!(record
        .detail
        .starts_with("expected exit 0, actual exit status: 3; "));
    // Early failure lines and the final tail survive; passing output does not.
    assert!(record.detail.contains("FAIL [ 1.000s] p::a early"));
    assert!(record.detail.contains("passing output 2999 last line"));
    assert!(!record.detail.contains("passing output 0 "));
    assert!(record.detail.contains(&"0".repeat(394)));
    assert!(!record.detail.contains(&"0".repeat(395)));
    assert!(
        record.detail.len() < 40 * 400 + 200,
        "{}",
        record.detail.len()
    );
}

#[test]
fn red_detail_names_the_failing_test_among_captured_error_lines() {
    // Workspace nextest (`--success-output immediate`): one early failure, then
    // thousands of passing tests whose captured output carries `::error::` lines.
    let failed =
        "FAIL [  45.123s] maos-bin::one_daemon_one_door_16_1 ac4_cold_swap_is_refused_typed";
    let script = format!(
        "echo '        {failed}'; echo \"thread 'ac4' panicked at crates/a.rs:1:1:\"; i=0; while [ $i -lt 3000 ]; do echo \"        PASS [   0.010s] maos-bin::p t$i\"; echo \"::error::check-x: RED fixture $i\"; i=$((i+1)); done; echo '     Summary [ 300.000s] 3001 tests run: 3000 passed, 1 failed, 0 skipped'; echo 'error: test run failed' >&2; exit 100"
    );
    let mut report = Report::default();
    let mut nextest = Command::new("sh");
    nextest.args(["-c", &script]);
    report.command("workspace-nextest", "junit.xml", &mut nextest);
    let record = &report.records[0];
    assert_eq!(record.status, "red");
    assert!(record
        .detail
        .starts_with("expected exit 0, actual exit status: 100; 5964 lines omitted; "));
    // The detail is whitespace-collapsed: `FAIL [ 45.123s] maos-bin::… ac4_…`.
    let named = failed.replace("  ", " ");
    assert!(record.detail.contains(&named), "{}", record.detail);
    assert!(record.detail.contains("panicked at crates/a.rs:1:1:"));
    assert!(record
        .detail
        .contains("1 failed, 0 skipped error: test run failed"));
    assert!(!record.detail.contains("RED fixture 0 "));
    assert!(
        record.detail.len() < 40 * 400 + 200,
        "{}",
        record.detail.len()
    );
}

#[test]
fn red_detail_keeps_output_order_and_libtest_failures() {
    let red = |script: &str| {
        let mut report = Report::default();
        let mut cmd = Command::new("sh");
        cmd.args(["-c", script]);
        report.command("red", "artifact", &mut cmd);
        assert_eq!(report.records[0].status, "red");
        report.records[0].detail.clone()
    };
    // A rustc header outranks its unranked location, yet must still precede it.
    let detail = red(
        "echo 'error[E0425]: cannot find value `x`'; echo '  --> crates/a/src/lib.rs:7:5'; i=0; while [ $i -lt 30 ]; do echo \"noise $i\"; i=$((i+1)); done; exit 101",
    );
    let header = detail.find("error[E0425]: cannot find value `x`");
    let location = detail.find("--> crates/a/src/lib.rs:7:5");
    assert!(header.unwrap() < location.unwrap(), "{detail}");
    // libtest: the FAILED test line is buried before more than 40 noise lines.
    let detail = red(
        "echo 'running 2 tests'; echo 'test tests::foo ... FAILED'; i=0; while [ $i -lt 60 ]; do echo \"noise $i\"; i=$((i+1)); done; echo 'test result: FAILED. 1 passed; 1 failed; 0 ignored'; exit 101",
    );
    let failed = detail.find("test tests::foo ... FAILED");
    let result = detail.find("test result: FAILED. 1 passed; 1 failed");
    assert!(failed.unwrap() < result.unwrap(), "{detail}");
    assert!(!detail.contains("running 2 tests"), "{detail}");
}

#[test]
fn only_timing_budget_findings_become_advisory_and_stale_rows_stay_red() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("xtask")).unwrap();
    let junit = root.path().join("junit.xml");
    fs::write(&junit, r#"<testsuites><testsuite name="p::a"><testcase name="t" classname="p::a" timestamp="2026-10-27T00:00:00Z" time="5.0"/></testsuite></testsuites>"#).unwrap();
    let stale =
        "[[exception]]\nbinary = \"p::gone\"\nseconds = 50.0\nreason = \"r\"\nowner = \"o\"\n";
    let over = "default_seconds = 1.0\ntotal_seconds = 2.0\n";
    let under = "default_seconds = 9.0\ntotal_seconds = 9.0\n";
    let cases = [
        // Per-binary and suite-total timing findings only.
        (over.to_owned(), "advisory"),
        (format!("{over}{stale}"), "red"),
        (format!("{under}{stale}"), "red"),
        (under.to_owned(), "pass"),
        // Register error: no judgment at all.
        ("total_seconds = 9.0\n".to_owned(), "red"),
    ];
    let mut report = Report::default();
    for (budgets, expected) in cases {
        fs::write(root.path().join("xtask/test-budgets.toml"), &budgets).unwrap();
        let mut budget = Command::new(env!("CARGO_BIN_EXE_xtask"));
        budget
            .current_dir(root.path())
            .args(["check-test-budget", "--json", "--junit"])
            .arg(&junit);
        test_budget(&mut report, "junit", &mut budget);
        let record = report.records.last().unwrap();
        assert_eq!(record.status, expected, "{budgets}: {}", record.detail);
    }
}

#[test]
fn count_pin_drift_is_red_against_listed_selection() {
    const CHILD: &str = "PREPUSH_20_16_STUB_CARGO_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let workflow = "jobs:\n  j:\n    steps:\n      - name: equal\n        run: |\n          cargo test --locked -p p --test a --color never 2>&1 | grep 'test result: ok. 2 passed'\n      - name: drift\n        run: |\n          cargo test --locked -p p --test a --color never 2>&1 | grep 'test result: ok. 3 passed'\n";
        let mut report = Report::default();
        check_pins(Path::new("."), &read_pins(workflow).unwrap(), &mut report);
        println!(
            "RECORDS {}",
            serde_json::to_string(&report.records).unwrap()
        );
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    // The only `cargo` on the child's PATH: lists two selected cases.
    let json = r#"{"rust-suites":{"p::a":{"testcases":{"x":{"ignored":false,"filter-match":{"status":"matches"}},"y":{"ignored":false,"filter-match":{"status":"matches"}}}}}}"#;
    let bin = tempfile::tempdir().unwrap();
    let cargo = bin.path().join("cargo");
    fs::write(
        &cargo,
        format!("#!/bin/sh\n[ \"$1 $2\" = 'nextest list' ] || exit 9\nprintf '%s\\n' '{json}'\n"),
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "count_pin_drift_is_red_against_listed_selection",
            "--nocapture",
        ])
        .env("PATH", bin.path())
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let records = stdout.lines().find_map(|l| l.strip_prefix("RECORDS "));
    let records: Value = serde_json::from_str(records.expect("child ran the pin check")).unwrap();
    let judged: Vec<_> = records
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["check"].as_str().unwrap(),
                r["artifact"].as_str().unwrap(),
                r["status"].as_str().unwrap(),
            )
        })
        .collect();
    let (equal, drift) = (format!("{WORKFLOW}:j/equal"), format!("{WORKFLOW}:j/drift"));
    assert_eq!(
        judged,
        [
            ("ci-pins:count", equal.as_str(), "pass"),
            ("ci-pins:count", drift.as_str(), "red")
        ]
    );
    assert!(records[1]["detail"]
        .as_str()
        .unwrap()
        .contains("expected [3], actual [2]"));
}

#[test]
fn binary_trigger_follows_transitive_local_dependencies() {
    let metadata = serde_json::json!({
        "workspace_root": "/repo",
        "packages": [
            { "name": "maos-bin", "manifest_path": "/repo/crates/maos-bin/Cargo.toml",
              "dependencies": [{ "name": "middle", "rename": "alias", "path": "/repo/crates/middle" }] },
            { "name": "middle", "manifest_path": "/repo/crates/middle/Cargo.toml",
              "dependencies": [{ "name": "leaf", "path": "/repo/crates/leaf" }] },
            { "name": "leaf", "manifest_path": "/repo/crates/leaf/Cargo.toml", "dependencies": [] }
        ]
    });
    let closure = binary_paths(&metadata).unwrap();
    assert!(size_trigger(
        &BTreeSet::from(["crates/leaf/src/lib.rs".into()]),
        &closure
    ));
    assert!(!size_trigger(
        &BTreeSet::from(["crates/unrelated/src/lib.rs".into()]),
        &closure
    ));
    assert!(!docs_trigger(&BTreeSet::from([
        "docs/testing/proof-contract.md".into()
    ])));
}

// Isolated from caller config and any hook-exported repository so setup can
// never write to the enclosing checkout.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=prepush",
            "-c",
            "user.email=prepush@invalid",
        ])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

#[test]
fn changes_union_head_and_working_tree_and_keep_rename_endpoints() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let write = |path: &str, text: &str| fs::write(dir.join(path), text).unwrap();
    git(dir, &["init", "-q"]);
    write("Cargo.lock", "base\n");
    write("committed-old.rs", "fn a() {}\n");
    write("staged-old.rs", "fn b() {}\n");
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "base"]);
    git(dir, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    write("Cargo.lock", "head\n");
    git(dir, &["mv", "committed-old.rs", "committed-new.rs"]);
    git(dir, &["commit", "-qam", "head"]);
    // Working bytes restore base: a net base→working diff would omit Cargo.lock.
    write("Cargo.lock", "base\n");
    git(dir, &["mv", "staged-old.rs", "staged-new.rs"]);
    fs::create_dir(dir.join("docs-site")).unwrap();
    write("docs-site/new.md", "untracked\n");
    let mut report = Report::default();
    changes(dir, &mut report).unwrap();
    let expected = [
        "Cargo.lock",
        "committed-new.rs",
        "committed-old.rs",
        "docs-site/new.md",
        "staged-new.rs",
        "staged-old.rs",
    ];
    assert_eq!(
        report.changed_paths,
        BTreeSet::from(expected.map(String::from))
    );
    assert!(report.base.is_some());
    assert!(size_trigger(&report.changed_paths, &BTreeSet::new()));
    assert!(docs_trigger(&report.changed_paths));
}

fn archive_workflow(commands: &str, tests: &str, features: &str) -> String {
    format!("env:\n  CARGO_TERM_COLOR: always\njobs:\n  archive:\n    steps:\n      - env:\n          NEXTEST_ARCHIVE_PACKAGES: p\n          NEXTEST_ARCHIVE_FEATURES: '{features}'\n          NEXTEST_ARCHIVE_TESTS: '{tests}'\n        run: |\n          cargo nextest archive -p p --features network --test a --test b --archive-file bundle\n{commands}")
}

#[test]
fn archive_commands_bind_ordered_counts_and_exact_family() {
    let commands = "          cargo-nextest nextest run --archive-file bundle -E 'binary_id(p::b)' --color never | tee log\n          cargo-nextest nextest run --archive-file bundle -E 'binary_id(p::a)' --color never | tee -a log\n          python3 check --expected-count 16 --expected-count 3 log\n";
    let pins = read_pins(&archive_workflow(commands, "a b", "network")).unwrap();
    assert_eq!(pins.len(), 1);
    assert!(pins[0].parse_error.is_none());
    // Reusing the producer keeps both build targets, but runtime bindings select
    // only the intended binary even when nextest lists the entire archive.
    let selected = &pins[0].selections[0];
    assert_eq!(
        option_values(&selected.args, "--test"),
        BTreeSet::from(["a", "b"])
    );
    let listed = r#"{"rust-suites":{"p::a":{"testcases":{"a":{"ignored":false,"filter-match":{"status":"mismatch"}}}},"p::b":{"testcases":{"b":{"ignored":false,"filter-match":{"status":"matches"}}}}}}"#;
    assert_eq!(listed_counts(listed, selected).unwrap(), vec![1]);
    assert!(listed_counts(&listed.replace("mismatch", "matches"), selected).is_err());
    let actual = ordered_counts(&pins[0].selections, |s| {
        Ok(if s.binary_ids.contains("p::b") {
            vec![16]
        } else {
            vec![3]
        })
    })
    .unwrap();
    assert_eq!(actual, pins[0].counts);
    for yaml in [
        archive_workflow(commands, "a b", "wasm-host"),
        archive_workflow(commands, "a b c", "network"),
        archive_workflow(
            &commands.replace("binary_id(p::b)", "binary_id(other::b)"),
            "a b",
            "network",
        ),
        archive_workflow(
            &commands.replace("-E 'binary_id(p::b)'", "--test b -E 'binary_id(p::b)'"),
            "a b",
            "network",
        ),
        archive_workflow(
            &commands.replace("-E 'binary_id(p::b)'", "--release -E 'binary_id(p::b)'"),
            "a b",
            "network",
        ),
    ] {
        assert!(read_pins(&yaml)
            .unwrap()
            .iter()
            .any(|p| p.parse_error.is_some()));
    }
}

#[test]
fn count_guards_cannot_disagree_or_change_archive_binary() {
    let commands = "          cargo-nextest nextest run --archive-file bundle -E 'binary_id(p::a)' --color never | tee log\n          grep '3 tests run: 3 passed' log\n          python3 check --expected-count 4 log\n";
    assert!(
        read_pins(&archive_workflow(commands, "a", "network")).unwrap()[0]
            .parse_error
            .is_some()
    );
    let unknown = commands.replace(
        "--expected-count 4",
        "--expected-count 3 --expected-count \"$UNKNOWN\"",
    );
    assert!(
        read_pins(&archive_workflow(&unknown, "a", "network")).unwrap()[0]
            .parse_error
            .is_some()
    );
    let mut selected = selection("-p p --test a").unwrap();
    selected.binary_ids.insert("p::a".into());
    let json = r#"{"rust-suites":{"p::a_suffix":{"testcases":{"x":{"ignored":false,"filter-match":{"status":"matches"}}}}}}"#;
    assert!(listed_counts(json, &selected)
        .unwrap_err()
        .contains("exact binary_id"));
    selected.binary_ids.clear();
    let json = r#"{"rust-suites":{"p::a":{"testcases":{"x":{"ignored":true,"filter-match":{"status":"matches"}},"y":{"ignored":false,"filter-match":{"status":"matches"}}}}}}"#;
    selected.ignored = true;
    assert_eq!(listed_counts(json, &selected).unwrap(), vec![1]);
}

#[test]
fn ansi_guards_are_command_scoped_and_fail_closed() {
    let base = "env:\n  CARGO_TERM_COLOR: always\njobs:\n  a:\n    steps:\n      - run: |\n          cargo test -p p --test a\n          grep -q 'test result: ok\\. 1 passed;' log\n";
    assert!(!read_pins(&format!("{base}          CARGO_TERM_COLOR=never\n")).unwrap()[0].ansi_safe);
    assert!(
        !read_pins(&base.replace(
            "cargo test",
            "echo CARGO_TERM_COLOR=never\n          cargo test"
        ))
        .unwrap()[0]
            .ansi_safe
    );
    assert!(
        read_pins(&base.replace("cargo test", "CARGO_TERM_COLOR=never cargo test")).unwrap()[0]
            .ansi_safe
    );
    assert!(
        !read_pins(&base.replace(
            "cargo test",
            "CARGO_TERM_COLOR=never echo ignored\n          cargo test"
        ))
        .unwrap()[0]
            .ansi_safe
    );
    assert!(
        read_pins(&base.replace(
            "cargo test",
            "export CARGO_TERM_COLOR=never\n          cargo test"
        ))
        .unwrap()[0]
            .ansi_safe
    );
    assert!(
        !read_pins(&base.replace(
            "cargo test",
            "export CARGO_TERM_COLOR=never\n          unset CARGO_TERM_COLOR\n          cargo test"
        ))
        .unwrap()[0]
            .ansi_safe
    );
    assert!(!ansi_safe("--color always | echo --color never", true));
    assert!(!ansi_safe("| echo --color never", false));
    assert!(ansi_safe("--color=never -- --nocapture", false));
    let mixed = archive_workflow("          cargo-nextest nextest run --archive-file bundle -E 'binary_id(p::a)' --color never\n          cargo-nextest nextest run --archive-file bundle -E 'binary_id(p::b)'\n          python3 check --expected-count 1 --expected-count 2 log\n", "a b", "network");
    assert!(!read_pins(&mixed).unwrap()[0].ansi_safe);
}

#[test]
fn dynamic_marker_skips_are_named_missing_and_static_drift_stay_red() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("crates/p/tests");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("a.rs"), r#"fn helper() { println!("HELPER value"); } #[test] fn t() { for _ in 0..8 { println!("SCOPED-VERDICT value"); } println!("STATIC value"); let emit = || println!("CLOSURE value"); emit(); emit(); println!("EQUAL value"); println!("EQUAL value"); }"#).unwrap();
    let pin = Pin {
        location: "marker".into(),
        selections: vec![selection("-p p --test a").unwrap()],
        counts: vec![],
        markers: vec![
            ("SCOPED-VERDICT ".into(), 8),
            ("HELPER ".into(), 2),
            ("STATIC ".into(), 2),
            ("MISSING ".into(), 1),
            ("CLOSURE ".into(), 2),
            ("EQUAL ".into(), 2),
        ],
        ansi_safe: true,
        parse_error: None,
    };
    let mut report = Report::default();
    marker_pins(root.path(), &pin, "workflow/source", &mut report);
    assert_eq!(
        report.records.iter().map(|r| r.status).collect::<Vec<_>>(),
        vec!["skip", "skip", "red", "red", "skip", "pass"]
    );
    assert!(report.text().contains("SCOPED-VERDICT"));
    assert!(report.text().contains("indeterminate locally"));
    assert_eq!(report.exit_code(), 1);
}
