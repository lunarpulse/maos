#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

//! Execute the real job shell with only cargo replaced by a controlled log
//! producer. This tests the accounting oracle, not container containment.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn run_job(omit: bool, duplicate: bool, skip: bool, cargo_rc: u8) -> Output {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let yaml = std::fs::read_to_string(root.join(".github/workflows/discipline.yml")).unwrap();
    let region = yaml
        .split("  t3-escape-corpus:\n")
        .nth(1)
        .expect("real job");
    let body = region
        .split("        run: |\n")
        .nth(1)
        .expect("real run block");
    let script = body
        .lines()
        .take_while(|line| line.is_empty() || line.starts_with("          "))
        .map(|line| line.strip_prefix("          ").unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let directory = tempfile::tempdir().unwrap();
    let bin = directory.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        "#!/bin/sh\ncat \"$T3_TEST_LOG\"\nexit \"$T3_TEST_RC\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    let fixture = directory.path().join("crates/maos-eval/fixtures");
    std::fs::create_dir_all(fixture.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(root.join("crates/maos-eval/fixtures"), &fixture).unwrap();
    let corpus =
        maos_eval::T3EscapeCorpus::load_from(&fixture.join("t3-escape-corpus-v1")).unwrap();
    let mut lines: Vec<_> = corpus
        .scenarios
        .iter()
        .map(|scenario| {
            format!(
                "T3-ESCAPE-EXECUTED {} block_observed=true kind8=1 measured",
                scenario.scenario_id
            )
        })
        .collect();
    if omit {
        lines.pop();
    }
    if duplicate {
        lines.push(lines[0].clone());
    }
    if skip {
        lines.push("SKIP T3 planted absent substrate".into());
    }
    lines.push("T3-NETWORK-MATRIX socket0600=refused socket0666=reachable".into());
    let log = directory.path().join("input.log");
    std::fs::write(&log, lines.join("\n") + "\n").unwrap();
    Command::new("bash")
        .args(["-e", "-o", "pipefail", "-c", &script])
        .current_dir(directory.path())
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("T3_TEST_LOG", log)
        .env("T3_TEST_RC", cargo_rc.to_string())
        .output()
        .expect("execute actual workflow shell")
}

#[test]
fn real_t3_job_accepts_every_executable_scenario_and_names_the_withdrawal() {
    let output = run_job(false, false, false, 0);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(
        "executed=24/24 withdrawn=1 ['runtime_escape_runtime_version_disclosure_005'] unexecuted=0"
    ));
}

#[test]
fn real_t3_job_rejects_missing_duplicate_skipped_and_failed_runs() {
    for (omit, duplicate, skip, rc) in [
        (true, false, false, 0),
        (false, true, false, 0),
        (false, false, true, 0),
        (false, false, false, 7),
    ] {
        let output = run_job(omit, duplicate, skip, rc);
        println!("T3-JOB-PLANT omit={omit} duplicate={duplicate} skip={skip} cargo_rc={rc} observed_exit={:?}", output.status.code());
        assert!(
            !output.status.success(),
            "job accepted omit={omit} duplicate={duplicate} skip={skip} rc={rc}"
        );
    }
}
