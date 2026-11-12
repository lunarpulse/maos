//! Story 20-5 AC5 — `check-rc-readiness`: the tree-state half of the `v1.0.0-rc.N` contract.
//! Every vector drives the real `xtask` binary against a fixture tree (the command reads
//! committed files from the current directory only; it never tags, signs, fetches or writes).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const STORY: &str = "20-5-v1-0-rc-tag-and-lts-rule";

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel)
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap()
}

fn xtask(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("run xtask")
}

const SPRINT: &str = "development_status:
  epic-20: in-progress
  20-1a-registry-publish-install-exit-path: done  # ✅ DONE; open item `ready-for-dev` is history
  20-3a-gate-honesty: done
  20-4-gemini-driver-and-endpoint-pin: done  # backlog earlier
  20-5-v1-0-rc-tag-and-lts-rule: done
  20-7-stable-runner-perf-gates: in-progress
  ops-rc-signed-tag: backlog
  epic-20-retrospective: backlog
";

const NOTES: &str = "# v1.0.0-rc.1\n\nPerf verdicts: ABSENT\n";
const ENV_LINE: &str = "Release environment protection: NOT PROVISIONED";
const WAIVED_KEY: &str = "20-7-stable-runner-perf-gates";

const CHECKLIST: &str = "| Item | Consumer | Behavior | Row | State |
|---|---|---|---|---|
| `MAOS_RELEASE_PUBKEY` | release.yml | hard fail | `ops-provisioning-secrets-and-accounts` | `present` |
| `RELEASE_SIGNING_KEY` | release.yml | hard fail | `ops-provisioning-secrets-and-accounts` | `present` |
| `fuzz-ledger branch permission` | collector | hard fail | `ops-provisioning-secrets-and-accounts` | `present` |
| `release environment protection evidence` | release.yml | publication unprotected | `ops-provisioning-secrets-and-accounts` | `present` |
| `DOCKERHUB_TOKEN` | container.yml | skipped | `ops-provisioning-secrets-and-accounts` | `absent` |
";

const DENY: &str = "[advisories]
ignore = [
    # serde_cbor frozen ABI.
    \"RUSTSEC-2021-0127\",
    { id = \"RUSTSEC-2025-0134\", reason = \"rustls-pemfile\" },
]
";

const DEFERRED: &str = "# Deferred work

- **serde_cbor is archived** (RUSTSEC-2021-0127) — Ownerless and open.
| rustls-pemfile | RUSTSEC-2025-0134 | owner A2A-TLS |
";

/// A fixture tree where every leg passes for `--tag v1.0.0-rc.1`.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"xtask\"]\n[workspace.package]\nversion = \"1.0.0-rc.1\"\n",
    );
    for rel in [
        "crates/maos-spirit-abi/src/lib.rs",
        "BREAKING.md",
        "STABILITY.md",
    ] {
        write(root, rel, &std::fs::read_to_string(repo(rel)).unwrap());
    }
    std::fs::create_dir_all(root.join("crates")).unwrap();
    write(
        root,
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
        SPRINT,
    );
    write(root, "docs/runbooks/provisioning-checklist.md", CHECKLIST);
    write(root, "docs/release/v1.0.0-rc.1.md", NOTES);
    write(root, "deny.toml", DENY);
    write(
        root,
        "_bmad-output/implementation-artifacts/deferred-work.md",
        DEFERRED,
    );
    regenerate_stability(root);
    dir
}

fn regenerate_stability(root: &Path) {
    let out = xtask(root, &["stability-matrix"]);
    assert!(
        out.status.success(),
        "fixture STABILITY.md must regenerate: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn legs(output: &Output) -> serde_json::Map<String, Value> {
    let text = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(text.trim())
        .unwrap_or_else(|e| panic!("stdout must be one JSON report ({e}): {text}"));
    assert_eq!(report["gate"], "check-rc-readiness");
    report["legs"]
        .as_array()
        .expect("legs")
        .iter()
        .map(|leg| {
            (
                leg["leg"].as_str().unwrap().to_string(),
                leg["passed"].clone(),
            )
        })
        .collect()
}

fn check(root: &Path, tag: Option<&str>) -> Output {
    let mut args = vec!["check-rc-readiness", "--json"];
    if let Some(tag) = tag {
        args.extend(["--tag", tag]);
    }
    xtask(root, &args)
}

/// The fixture is ready except for `red_legs`: those legs are red, every other leg is
/// green, the command exits non-zero.
fn assert_red(output: &Output, red_legs: &[&str]) {
    assert!(
        !output.status.success(),
        "{red_legs:?} red must fail the gate; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let legs = legs(output);
    assert_eq!(legs.len(), 6, "six legs: {legs:?}");
    for (leg, passed) in &legs {
        assert_eq!(
            passed,
            &Value::Bool(!red_legs.contains(&leg.as_str())),
            "leg {leg} (only {red_legs:?} may be red): {legs:?}"
        );
    }
}

fn assert_only_red(output: &Output, red_leg: &str) {
    assert_red(output, &[red_leg]);
}

fn detail(output: &Output, leg: &str) -> String {
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    let leg = report["legs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["leg"] == leg)
        .expect("leg");
    leg["detail"].as_str().unwrap().to_string()
}

#[test]
fn an_all_green_tree_passes_with_and_without_a_tag() {
    let dir = fixture();
    for tag in [None, Some("v1.0.0-rc.1")] {
        let output = check(dir.path(), tag);
        assert!(
            output.status.success(),
            "tag {tag:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let legs = legs(&output);
        assert_eq!(legs.len(), 6);
        assert!(
            legs.values().all(|passed| passed == &Value::Bool(true)),
            "{legs:?}"
        );
    }
}

#[test]
fn the_version_leg_demands_an_rc_that_equals_the_tag() {
    let dir = fixture();
    // Tag mismatch against rc.1.
    assert_only_red(&check(dir.path(), Some("v1.0.0-rc.2")), "version");
    assert_only_red(&check(dir.path(), Some("1.0.0-rc.1")), "version");
    // Non-rc versions, with the tag that would match them (20-7 done, so only the version
    // leg can be red; the notes carry the label under every version's own file name).
    let base = read(dir.path(), "Cargo.toml");
    write(
        dir.path(),
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
        &SPRINT.replace(
            &format!("{WAIVED_KEY}: in-progress"),
            &format!("{WAIVED_KEY}: done"),
        ),
    );
    for version in [
        "0.1.0-alpha.1",
        "1.0.0",
        "1.0.0-rc.",
        "1.0.0-rc.1x",
        "1.0.0-rcx",
    ] {
        let text = base.replace("\"1.0.0-rc.1\"", &format!("\"{version}\""));
        write(dir.path(), "Cargo.toml", &text);
        write(dir.path(), &format!("docs/release/v{version}.md"), NOTES);
        regenerate_stability(dir.path());
        let tag = format!("v{version}");
        assert_only_red(&check(dir.path(), None), "version");
        assert_only_red(&check(dir.path(), Some(&tag)), "version");
    }
}

#[test]
fn the_stability_leg_reds_on_a_drifted_stability_md() {
    let dir = fixture();
    let text = read(dir.path(), "STABILITY.md").replace("OPERATOR's responsibility", "SUBSTRATE's");
    write(dir.path(), "STABILITY.md", &text);
    assert_only_red(&check(dir.path(), None), "stability-matrix");
}

#[test]
fn the_epic_20_leg_reds_on_any_story_not_done_and_ignores_provenance_comments() {
    let dir = fixture();
    // The fixture's `done  # … ready-for-dev …` provenance comment still reads `done`.
    for status in ["backlog", "ready-for-dev", "in-progress", "review"] {
        let text = SPRINT.replace(
            "20-3a-gate-honesty: done",
            &format!("20-3a-gate-honesty: {status}"),
        );
        write(
            dir.path(),
            "_bmad-output/implementation-artifacts/sprint-status.yaml",
            &text,
        );
        assert_only_red(&check(dir.path(), None), "epic-20-stories");
    }
    // A missing tracker is red, not a vacuous pass.
    std::fs::remove_file(
        dir.path()
            .join("_bmad-output/implementation-artifacts/sprint-status.yaml"),
    )
    .unwrap();
    assert_only_red(&check(dir.path(), None), "epic-20-stories");
}

#[test]
fn this_storys_own_key_may_be_in_review_without_a_tag_but_never_with_one() {
    let dir = fixture();
    for status in ["in-progress", "review"] {
        let text = SPRINT.replace(
            &format!("{STORY}: done"),
            &format!("{STORY}: {status}  # L2 is the dev's own run"),
        );
        write(
            dir.path(),
            "_bmad-output/implementation-artifacts/sprint-status.yaml",
            &text,
        );
        let without = check(dir.path(), None);
        assert!(
            without.status.success(),
            "{status} without --tag passes: {}",
            String::from_utf8_lossy(&without.stdout)
        );
        assert_only_red(&check(dir.path(), Some("v1.0.0-rc.1")), "epic-20-stories");
    }
    // Another story at `review` is never excused.
    let text = SPRINT.replace("20-3a-gate-honesty: done", "20-3a-gate-honesty: review");
    write(
        dir.path(),
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
        &text,
    );
    assert_only_red(&check(dir.path(), None), "epic-20-stories");
}

#[test]
fn the_provisioning_leg_needs_every_row_present() {
    for row in [
        "`MAOS_RELEASE_PUBKEY`",
        "`RELEASE_SIGNING_KEY`",
        "`fuzz-ledger branch permission`",
        "`release environment protection evidence`",
    ] {
        // Not `present`.
        let dir = fixture();
        let text: String = CHECKLIST
            .lines()
            .map(|line| {
                if line.starts_with(&format!("| {row} |")) {
                    line.replace("`present`", "`absent`")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        write(dir.path(), "docs/runbooks/provisioning-checklist.md", &text);
        assert_only_red(&check(dir.path(), None), "provisioning");
        // Row missing altogether.
        let text: String = CHECKLIST
            .lines()
            .filter(|line| !line.starts_with(&format!("| {row} |")))
            .collect::<Vec<_>>()
            .join("\n");
        write(dir.path(), "docs/runbooks/provisioning-checklist.md", &text);
        assert_only_red(&check(dir.path(), None), "provisioning");
    }
}

#[test]
fn the_deny_leg_reds_on_an_ignored_advisory_without_a_deferred_row() {
    let dir = fixture();
    write(
        dir.path(),
        "deny.toml",
        &DENY.replace("ignore = [\n", "ignore = [\n    \"RUSTSEC-2099-0001\",\n"),
    );
    assert_only_red(&check(dir.path(), None), "deny-ignores");
    // A mention that is not a row (prose outside a bullet or table row) is not a row.
    write(
        dir.path(),
        "_bmad-output/implementation-artifacts/deferred-work.md",
        &format!("{DEFERRED}\nSee RUSTSEC-2099-0001 in passing.\n"),
    );
    assert_only_red(&check(dir.path(), None), "deny-ignores");
    // Review GLM F5: the id must match as a whole token, not as a prefix of a longer id.
    write(
        dir.path(),
        "_bmad-output/implementation-artifacts/deferred-work.md",
        &format!("{DEFERRED}- **other advisory** (RUSTSEC-2099-00012, RUSTSEC-2099-0001x) — Ownerless and open.\n"),
    );
    assert_only_red(&check(dir.path(), None), "deny-ignores");
    write(
        dir.path(),
        "_bmad-output/implementation-artifacts/deferred-work.md",
        &format!("{DEFERRED}- **new advisory** (RUSTSEC-2099-0001) — Ownerless and open.\n"),
    );
    assert!(check(dir.path(), None).status.success());
}

#[test]
fn a_missing_input_file_is_a_loud_error_never_a_pass() {
    let dir = fixture();
    std::fs::remove_file(dir.path().join("deny.toml")).unwrap();
    let output = check(dir.path(), None);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("deny.toml"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_command_never_writes_into_the_tree() {
    let dir = fixture();
    let before = snapshot(dir.path());
    assert!(check(dir.path(), Some("v1.0.0-rc.1")).status.success());
    assert!(!check(dir.path(), Some("v9.9.9-rc.1")).status.success());
    assert_eq!(
        before,
        snapshot(dir.path()),
        "check-rc-readiness must not write"
    );
}

const RC_NOTES: &str = "docs/release/v1.0.0-rc.1.md";

fn with_sprint_status(root: &Path, status: &str) {
    write(
        root,
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
        &SPRINT.replace(
            &format!("{WAIVED_KEY}: in-progress"),
            &format!("{WAIVED_KEY}: {status}"),
        ),
    );
}

#[test]
fn the_perf_label_leg_needs_the_notes_and_the_exact_line() {
    let dir = fixture();
    // No label also switches the 20-7 waiver off, so that key is open as well.
    let red = ["perf-verdict-label", "epic-20-stories"];
    for body in [
        "",
        "# v1.0.0-rc.1\n",
        "Perf verdicts: PASS\n",
        "Perf verdicts: ABSENT.\n",
        "perf verdicts: absent\n",
        " Perf verdicts: ABSENT\n",
    ] {
        write(dir.path(), RC_NOTES, body);
        assert_red(&check(dir.path(), None), &red);
        assert_red(&check(dir.path(), Some("v1.0.0-rc.1")), &red);
    }
    std::fs::remove_file(dir.path().join(RC_NOTES)).unwrap();
    assert_red(&check(dir.path(), None), &red);
    write(dir.path(), RC_NOTES, "x\nPerf verdicts: ABSENT\r\ny\n");
    assert!(check(dir.path(), None).status.success());
}

#[test]
fn the_20_7_waiver_is_in_progress_only_rc_only_and_prints_waived() {
    let dir = fixture();
    for tag in [None, Some("v1.0.0-rc.1")] {
        let output = check(dir.path(), tag);
        assert!(output.status.success(), "{tag:?}");
        assert!(detail(&output, "epic-20-stories").contains("WAIVED"));
        for status in ["backlog", "ready-for-dev", "review"] {
            with_sprint_status(dir.path(), status);
            assert_only_red(&check(dir.path(), tag), "epic-20-stories");
        }
        with_sprint_status(dir.path(), "done");
        let output = check(dir.path(), tag);
        assert!(output.status.success());
        assert!(!detail(&output, "epic-20-stories").contains("WAIVED"));
        with_sprint_status(dir.path(), "in-progress");
    }
    // A GA (or any non-rc) version never gets the waiver, label or not.
    let base = read(dir.path(), "Cargo.toml");
    for version in ["1.0.0", "1.0.0-beta.1"] {
        write(
            dir.path(),
            "Cargo.toml",
            &base.replace("\"1.0.0-rc.1\"", &format!("\"{version}\"")),
        );
        write(dir.path(), &format!("docs/release/v{version}.md"), NOTES);
        regenerate_stability(dir.path());
        let red = ["version", "epic-20-stories"];
        assert_red(&check(dir.path(), None), &red);
        assert_red(&check(dir.path(), Some(&format!("v{version}"))), &red);
    }
}

#[test]
fn an_unprovisioned_release_environment_is_a_labelled_warn() {
    let dir = fixture();
    let absent: String = CHECKLIST
        .lines()
        .map(
            |line| match line.starts_with("| `release environment protection evidence` |") {
                true => line.replace("`present`", "`absent`"),
                false => line.to_string(),
            },
        )
        .collect::<Vec<_>>()
        .join("\n");
    write(
        dir.path(),
        "docs/runbooks/provisioning-checklist.md",
        &absent,
    );
    assert_only_red(&check(dir.path(), None), "provisioning");
    write(dir.path(), RC_NOTES, &format!("{NOTES}{ENV_LINE}\n"));
    for tag in [None, Some("v1.0.0-rc.1")] {
        let output = check(dir.path(), tag);
        assert!(output.status.success(), "{tag:?}");
        assert_eq!(
            detail(&output, "provisioning"),
            "WARN: environment protection not provisioned: claim boundary"
        );
    }
    // The label never excuses another absent row; a `present` row needs no label.
    write(
        dir.path(),
        "docs/runbooks/provisioning-checklist.md",
        &absent.replace("| `present` |", "| `absent` |"),
    );
    assert_only_red(&check(dir.path(), None), "provisioning");
    write(
        dir.path(),
        "docs/runbooks/provisioning-checklist.md",
        CHECKLIST,
    );
    write(dir.path(), RC_NOTES, NOTES);
    let output = check(dir.path(), None);
    assert!(output.status.success());
    assert!(!detail(&output, "provisioning").contains("WARN"));
}

fn snapshot(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().display().to_string(),
                    std::fs::read_to_string(&path).unwrap_or_default(),
                ));
            }
        }
    }
    files.sort();
    files
}
