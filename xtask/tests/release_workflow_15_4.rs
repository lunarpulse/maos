//! Story 15-4 AC1 — release workflow artifact-shape regression contract.
//!
//! The pre-story workflow can die on nested artifact directories and can publish
//! a signed manifest over zero binaries. Parse the workflow as configuration so
//! both halves stay closed together.

use std::fs;
use std::path::PathBuf;

use maos_audit::release_verify::DEFAULT_RELEASE_PUBKEY;
use serde_yaml::{Mapping, Value};
use xtask::release_dry_run::ARTIFACTS;

/// The suffixes `release.yml`'s matrix maps its three targets onto. Kept beside the
/// table rather than duplicating the published list: the expectations below are
/// DERIVED from `ARTIFACTS × RELEASE_SUFFIXES`, so adding a binary to the table
/// reds this test until the workflow's own build, upload and publish lists follow.
const RELEASE_SUFFIXES: [&str; 3] = ["linux-amd64", "linux-arm64", "darwin-arm64"];
const RELEASE_TARGETS: &str =
    "x86_64-unknown-linux-gnu,aarch64-unknown-linux-gnu,aarch64-apple-darwin";

fn declared_release_files() -> Vec<String> {
    RELEASE_SUFFIXES
        .iter()
        .flat_map(|suffix| {
            ARTIFACTS
                .iter()
                .map(move |artifact| format!("dist/{}-{suffix}", artifact.binary))
        })
        .collect()
}

fn workflow() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join(".github/workflows/release.yml");
    serde_yaml::from_str(&fs::read_to_string(path).expect("read release workflow"))
        .expect("parse release workflow")
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

fn steps(job: &Value) -> &[Value] {
    key(job, "steps").as_sequence().expect("job steps")
}

fn named_step<'a>(job: &'a Value, name: &str) -> &'a Value {
    steps(job)
        .iter()
        .find(|step| get(step, "name").and_then(Value::as_str) == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
}

#[test]
fn release_workflow_flattens_and_publishes_the_explicit_six_binary_set() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let build = key(jobs, "build");
    let publish = key(jobs, "sign-and-publish");
    let triggers = key(&workflow, "on");
    assert!(
        get(triggers, "workflow_dispatch").is_some(),
        "pre-tag macOS rehearsal must be manually dispatchable"
    );
    assert_eq!(
        key(publish, "if").as_str(),
        Some("github.event_name == 'push'"),
        "manual rehearsals must never publish"
    );

    // Epic-15 retrospective, 2026-09-12 (operator-ratified): the pre-merge
    // rehearsal path. `workflow_dispatch` is unusable until the workflow reaches
    // the default branch, which is the very merge the rehearsal de-risks, so the
    // PR trigger is what gives `aarch64-apple-darwin` its first compile off
    // `main`. Both assertions below are load-bearing in OPPOSITE directions.
    let pull_request =
        get(triggers, "pull_request").expect("pre-merge rehearsal needs a pull_request trigger");
    assert_eq!(
        key(pull_request, "branches")
            .as_sequence()
            .map(|b| { b.iter().filter_map(Value::as_str).collect::<Vec<_>>() }),
        Some(vec!["main"]),
        "the rehearsal trigger is scoped to PRs into main"
    );

    // THE COST CONTROL. Without this gate every pull request into `main` runs
    // the full three-target release build, and `macos-latest` bills ~10x — which
    // is the exact expense D-15-4-F removed macOS from the discipline matrix to
    // avoid. Deleting the `if` leaves the workflow valid and the rehearsal
    // working, so nothing else in this suite would notice: a PR tax is a silent
    // regression, not a broken build. Pin the label by name, because a gate that
    // matches any label is the same as no gate.
    // Normalised on BOTH sides: the YAML is a folded scalar and `cargo fmt`
    // rewrites a `\`-continued Rust literal, so comparing raw text pins the
    // formatter's whitespace rather than the gate.
    const REHEARSAL_GATE: &str = "github.event_name != 'pull_request' || contains(github.event.pull_request.labels.*.name, 'release-rehearsal')";
    let normalise = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        key(build, "if").as_str().map(normalise),
        Some(normalise(REHEARSAL_GATE)),
        "the macOS build must stay OFF unlabelled pull requests, and push/dispatch must stay unaffected"
    );

    assert!(key(build, "timeout-minutes").as_u64().is_some());
    assert!(key(publish, "timeout-minutes").as_u64().is_some());

    let build_command = key(named_step(build, "Build release binaries"), "run")
        .as_str()
        .expect("build command");
    assert!(build_command.contains("-p maos-bin"));
    assert!(build_command.contains("-p maos-cli"));

    let upload_targets: Vec<(&str, &str)> = steps(build)
        .iter()
        .filter(|step| {
            get(step, "uses").and_then(Value::as_str) == Some("actions/upload-artifact@v4")
        })
        .map(|step| {
            let with = key(step, "with");
            (
                key(with, "name").as_str().expect("artifact name"),
                key(with, "path").as_str().expect("artifact path"),
            )
        })
        .collect();
    // Counting the steps is not enough: uploading `maos` twice would satisfy a count
    // while leaving every `maosctl-*` file out of the merged dist, and the first tag
    // would then die in manifest generation.
    let expected_uploads: Vec<(String, String)> = ARTIFACTS
        .iter()
        .map(|artifact| {
            (
                format!("{}-${{{{ matrix.suffix }}}}", artifact.binary),
                format!("dist/{}-${{{{ matrix.suffix }}}}", artifact.binary),
            )
        })
        .collect();
    assert_eq!(
        upload_targets,
        expected_uploads
            .iter()
            .map(|(name, path)| (name.as_str(), path.as_str()))
            .collect::<Vec<_>>(),
        "one upload per declared binary, named and pathed per target"
    );

    let download = steps(publish)
        .iter()
        .find(|step| {
            get(step, "uses").and_then(Value::as_str) == Some("actions/download-artifact@v4")
        })
        .expect("download step");
    let download_with = key(download, "with");
    assert_eq!(key(download_with, "pattern").as_str(), Some("maos*"));
    assert_eq!(key(download_with, "merge-multiple").as_bool(), Some(true));

    let manifest_command = key(named_step(publish, "Generate SHA256SUMS and sign"), "run")
        .as_str()
        .expect("manifest command");
    assert!(manifest_command.contains("release-dry-run --manifest-only"));
    // Without the explicit target set the verb falls back to the host target and
    // refuses the four staged arm64 artifacts as uncovered — the tag dies before signing.
    assert!(
        manifest_command.contains(&format!("--targets {RELEASE_TARGETS}")),
        "manifest command must name all three targets: {manifest_command}"
    );
    assert!(!manifest_command
        .lines()
        .any(|line| line.trim_start().starts_with("sha256sum ")));
    assert!(!manifest_command.contains("maos-*"));

    let release = named_step(publish, "Create GitHub Release");
    let release_with = key(release, "with");
    assert_eq!(
        key(release_with, "fail_on_unmatched_files").as_bool(),
        Some(true)
    );
    let files: Vec<&str> = key(release_with, "files")
        .as_str()
        .expect("release files")
        .lines()
        .collect();
    let mut expected_files = declared_release_files();
    expected_files.push("dist/SHA256SUMS".to_string());
    expected_files.push("dist/SHA256SUMS.sig".to_string());
    assert_eq!(
        files,
        expected_files
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "the published set is derived from ARTIFACTS; a table addition must edit this list"
    );
    // A hyphenated tag is a pre-release and must never be served as `latest`.
    assert_eq!(
        key(release_with, "prerelease").as_str(),
        Some("${{ contains(github.ref_name, '-') }}")
    );

    assert_eq!(
        key(key(publish, "permissions"), "checks").as_str(),
        Some("read")
    );
    let guard = named_step(publish, "Require successful main discipline aggregate");
    assert_eq!(
        key(key(guard, "env"), "GH_TOKEN").as_str(),
        Some("${{ github.token }}")
    );
    let guard_command = key(guard, "run").as_str().expect("guard command");
    assert!(
        guard_command.contains("check-runs?check_name=aggregate&status=completed&filter=latest")
    );
    assert!(guard_command.contains("check-release-precondition"));

    let production_key = "${{ secrets.MAOS_RELEASE_PUBKEY }}";
    assert_eq!(
        key(
            key(named_step(build, "Build release binaries"), "env"),
            "MAOS_RELEASE_PUBKEY"
        )
        .as_str(),
        Some(production_key)
    );
    assert_eq!(
        key(
            key(named_step(publish, "Verify artifacts (self-test)"), "env"),
            "MAOS_RELEASE_PUBKEY"
        )
        .as_str(),
        Some(production_key)
    );
    let key_guard = key(
        named_step(publish, "Refuse bundled development release key"),
        "run",
    )
    .as_str()
    .expect("key guard command");
    assert!(key_guard.contains("dist/maosctl-linux-amd64 release-pubkey"));
    // Derived from the library const, never a second copy of the hex: if the bundled
    // key ever moves, this reds until the workflow guard follows. A guard pinned only
    // by a literal it also contains can go stale with the workflow and fail open.
    let bundled: String = DEFAULT_RELEASE_PUBKEY
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert!(
        key_guard.contains(&bundled),
        "guard must compare against the bundled development key: {key_guard}"
    );
    // The step must be able to REFUSE. A body that only echoes both strings would
    // satisfy a containment check and exit 0, publishing a dev-keyed binary.
    assert!(
        key_guard.contains("exit 1"),
        "guard must refuse: {key_guard}"
    );
    assert!(
        key_guard.contains(&format!("== \"{bundled}\"")),
        "guard must branch on equality with the bundled key: {key_guard}"
    );
}

#[test]
fn discipline_runs_both_linux_release_targets_in_the_aggregate_only() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join(".github/workflows/discipline.yml");
    let workflow: Value =
        serde_yaml::from_str(&fs::read_to_string(path).expect("read discipline workflow"))
            .expect("parse discipline workflow");
    let jobs = key(&workflow, "jobs");
    let dry_run = key(jobs, "release-dry-run");

    assert_eq!(
        key(key(dry_run, "strategy"), "fail-fast").as_bool(),
        Some(false)
    );
    let targets: Vec<&str> = key(key(key(dry_run, "strategy"), "matrix"), "target")
        .as_sequence()
        .expect("target matrix")
        .iter()
        .map(|target| target.as_str().expect("target string"))
        .collect();
    assert_eq!(
        targets,
        vec!["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
    );

    let command = key(named_step(dry_run, "Run release dry-run"), "run")
        .as_str()
        .expect("dry-run command");
    assert!(command.contains("release-dry-run --targets ${{ matrix.target }} --json"));
    assert!(!command.contains("--no-default-features"));

    let sign = named_step(dry_run, "Sign staged manifest with development seed");
    assert_eq!(
        key(key(sign, "env"), "RELEASE_SIGNING_KEY").as_str(),
        Some("794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d")
    );
    let verify = named_step(dry_run, "Verify with shipped maosctl");
    assert_eq!(
        key(verify, "if").as_str(),
        Some("matrix.target != 'aarch64-unknown-linux-gnu'")
    );
    // The SHIPPED binary must do the verifying (D-15-4-E leg 1). A workspace
    // `cargo run -p maos-cli` would pass a containment check while never executing
    // the artifact the release publishes.
    let verify_command = key(verify, "run").as_str().expect("verify command");
    assert!(
        verify_command
            .contains("./dist/maosctl-linux-amd64 install --from-local ./dist --verify-only"),
        "verify must exec the staged binary: {verify_command}"
    );
    let wrong_key = named_step(dry_run, "Reject signature from different seed");
    assert_eq!(
        key(key(wrong_key, "env"), "RELEASE_SIGNING_KEY").as_str(),
        Some("0000000000000000000000000000000000000000000000000000000000000000")
    );
    // The falsifier only proves something if the step INVERTS the expectation: a body
    // replaced by a plain re-sign would leave the control seeing a matching pair only.
    let wrong_key_command = key(wrong_key, "run").as_str().expect("wrong-key command");
    assert!(
        wrong_key_command.contains("install --from-local ./dist --verify-only"),
        "falsifier must re-run the verifier: {wrong_key_command}"
    );
    assert!(
        wrong_key_command.contains("exit 1"),
        "falsifier must fail the job when a forged signature is ACCEPTED: {wrong_key_command}"
    );
    let upload = named_step(dry_run, "Upload dry-run artifacts");
    assert_eq!(key(key(upload, "with"), "retention-days").as_u64(), Some(7));

    // AC2 / D-15-4-F: `aggregate.needs` and NOWHERE ELSE. `release-dry-run` is a
    // non-registered build job with no `gate-registry.toml` disposition and no
    // `EXPECTED_GATES` row (the `check-mock-not-in-release` / `maosctl-smoke`
    // pattern), so enrolling it in `v1-0-ship-gate` would make the ship gate block
    // on an unregistered job — and `check_ship_gate_completeness` iterates
    // `EXPECTED_GATES`, never `needs`, so nothing would red.
    let needs = |gate: &str| -> Vec<String> {
        key(key(jobs, gate), "needs")
            .as_sequence()
            .expect("gate needs")
            .iter()
            .map(|need| need.as_str().expect("need string").to_string())
            .collect()
    };
    assert!(
        needs("aggregate").contains(&"release-dry-run".to_string()),
        "aggregate must block on release-dry-run"
    );
    assert!(
        !needs("v1-0-ship-gate").contains(&"release-dry-run".to_string()),
        "release-dry-run must NOT be enrolled in v1-0-ship-gate"
    );
}
