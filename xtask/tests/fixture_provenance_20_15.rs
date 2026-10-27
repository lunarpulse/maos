//! Story 20-15 D03: real provenance helpers and manifest transport, without a compiler.

#[path = "../../tests/harness/fixtures.rs"]
mod fixture_harness;
#[allow(dead_code)]
#[path = "../src/build_test_fixtures.rs"]
mod producer;

use fixture_harness::VerificationMode;
use producer::{parse_manifest, write_manifest, Entry};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn path_arg(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

/// Drop every inherited `GIT_*` variable — including `GIT_DIR`, `GIT_WORK_TREE`,
/// `GIT_INDEX_FILE`, `GIT_COMMON_DIR`, `GIT_OBJECT_DIRECTORY` and `GIT_NAMESPACE`
/// that a hook may export — so a temp-repo command never touches an outer repo.
fn scrub_git(command: &mut Command) -> &mut Command {
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = scrub_git(&mut Command::new("git"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn commit(root: &Path) {
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture Test",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture source",
        ],
    );
}

fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "--quiet"]);
    std::fs::write(dir.path().join(".gitignore"), "/target/\n").unwrap();
    std::fs::write(dir.path().join("source.rs"), "fn main() {}\n").unwrap();
    commit(dir.path());
    dir
}

/// Use the same explicit transport policy, capture and sealing as `run`.
fn produce(root: &Path) -> Entry {
    produce_with_mode(root, true).unwrap()
}

fn produce_with_mode(root: &Path, transport: bool) -> Result<Entry, String> {
    let tree = producer::clean_tree(root, transport)?;
    let path = root.join("target/fixture");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"fixture bytes").unwrap();
    Ok(Entry {
        artifact: producer::artifact(root, &path, &tree)?,
        tree,
        path,
        built_at: 0,
        inputs: vec![root.join("source.rs")],
    })
}

fn consumer_entry(root: &Path, entry: &Entry) -> fixture_harness::Entry {
    let manifest = root.join("target/manifest.json");
    write_manifest(
        &manifest,
        &BTreeMap::from([("fixture".to_owned(), entry.clone())]),
    )
    .unwrap();
    fixture_harness::parse_manifest_entry(
        &std::fs::read_to_string(&manifest).unwrap(),
        "fixture",
        &manifest,
    )
    .unwrap()
    .unwrap()
}

fn future() -> SystemTime {
    SystemTime::now() + Duration::from_secs(3600)
}

#[test]
fn schema_two_entries_merge_into_three_without_inventing_provenance() {
    let dir = repository();
    let manifest = dir.path().join("target/manifest.json");
    std::fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    let old = "{\n  \"schema\": 2,\n  \"fixtures\": {\n    \"old\": {\"built_at\":1,\"inputs\":[],\"path\":\"/old\"}\n  }\n}\n";
    std::fs::write(&manifest, old).unwrap();
    assert!(fixture_harness::parse_manifest_entry(old, "old", &manifest)
        .unwrap()
        .is_some());
    let built = produce(dir.path());
    consumer_entry(dir.path(), &built);
    let text = std::fs::read_to_string(&manifest).unwrap();
    assert!(text.contains("\"schema\": 3"));
    let merged = parse_manifest(&text).unwrap();
    assert!(merged["old"].tree.is_none() && merged["old"].artifact.is_none());
    assert_eq!(merged["fixture"], built);
    assert!(
        fixture_harness::parse_manifest_entry(&text, "old", &manifest)
            .unwrap()
            .is_some()
    );
}

#[test]
fn invalid_schema_and_partial_provenance_never_become_local_entries() {
    let dir = repository();
    let built = produce(dir.path());
    let manifest = dir.path().join("target/manifest.json");
    consumer_entry(dir.path(), &built);
    let valid = std::fs::read_to_string(&manifest).unwrap();
    for changed in [
        valid.replace("\"schema\": 3", "\"schema\": 99"),
        valid.replace("\"schema\": 3", "\"schema\": 2"),
        valid.replace("\"tree\":", "\"not_tree\":"),
        valid.replace(built.tree.as_ref().unwrap(), "not-a-tree"),
        valid.replace("\"artifact\":", "\"not_artifact\":"),
        valid.replace(built.artifact.as_ref().unwrap(), "not-a-blob"),
    ] {
        assert!(parse_manifest(&changed).is_err(), "{changed}");
        assert!(
            fixture_harness::parse_manifest_entry(&changed, "fixture", &manifest).is_err(),
            "{changed}"
        );
    }
}

#[test]
fn producer_merge_refuses_renamed_provenance_instead_of_rewriting_it_unsealed() {
    let dir = repository();
    let built = produce(dir.path());
    consumer_entry(dir.path(), &built);
    let manifest = dir.path().join("target/manifest.json");
    // Dropping both unknown names would leave a valid-looking local entry.
    let renamed = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("\"tree\":", "\"not_tree\":")
        .replace("\"artifact\":", "\"not_artifact\":");
    std::fs::write(&manifest, &renamed).unwrap();
    let error =
        write_manifest(&manifest, &BTreeMap::from([("next".to_owned(), built)])).unwrap_err();
    assert!(error.contains("unknown field"), "{error}");
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), renamed);
}

#[test]
fn producer_seals_actual_artifact_bytes_and_absolute_paths() {
    let dir = repository();
    let built = produce(dir.path());
    assert!(built.path.is_absolute());
    assert_eq!(
        built.tree.as_deref(),
        Some(git(dir.path(), &["rev-parse", "HEAD^{tree}"]).as_str())
    );
    assert_eq!(
        built.artifact.as_deref(),
        Some(
            git(
                dir.path(),
                &["hash-object", "--no-filters", "--", &path_arg(&built.path),]
            )
            .as_str()
        )
    );
    let consumed = consumer_entry(dir.path(), &built);
    assert!(fixture_harness::verify_entry(
        &consumed,
        &built.path,
        UNIX_EPOCH,
        dir.path(),
        VerificationMode::Transport,
    )
    .is_ok());
}

#[test]
fn producer_does_not_claim_clean_provenance_for_any_dirty_state() {
    for state in ["untracked", "unstaged", "staged"] {
        let dir = repository();
        let tree = producer::clean_tree(dir.path(), true).unwrap().unwrap();
        let path = dir.path().join(if state == "untracked" {
            "new.rs"
        } else {
            "source.rs"
        });
        std::fs::write(path, "changed source").unwrap();
        if state == "staged" {
            git(dir.path(), &["add", "source.rs"]);
        }
        assert_eq!(git(dir.path(), &["rev-parse", "HEAD^{tree}"]), tree);
        let error = produce_with_mode(dir.path(), true).unwrap_err();
        assert!(error.contains("dirty"), "{state}: {error}");
        let built = produce_with_mode(dir.path(), false).unwrap();
        assert!(built.tree.is_none() && built.artifact.is_none(), "{state}");
        let consumed = consumer_entry(dir.path(), &built);
        assert!(fixture_harness::verify_entry(
            &consumed,
            &built.path,
            future(),
            dir.path(),
            VerificationMode::Local,
        )
        .is_ok());
    }
}

#[test]
fn producer_refuses_sources_that_change_after_clean_capture() {
    for committed in [false, true] {
        let dir = repository();
        let built = produce(dir.path());
        std::fs::write(dir.path().join("source.rs"), "changed during build").unwrap();
        if committed {
            commit(dir.path());
        }
        let error = producer::artifact(dir.path(), &built.path, &built.tree).unwrap_err();
        assert!(
            error.contains("dirty") || error.contains("checkout changed during the build"),
            "{error}"
        );
    }
}

#[test]
fn producer_refuses_missing_artifact_instead_of_recording_identity() {
    let dir = repository();
    let built = produce(dir.path());
    std::fs::remove_file(&built.path).unwrap();
    assert!(producer::artifact(dir.path(), &built.path, &built.tree).is_err());
}

#[test]
fn fresh_checkout_accepts_transported_manifest_despite_newer_source_mtimes() {
    let source = repository();
    let built = produce(source.path());
    let artifact = std::fs::read(&built.path).unwrap();
    let manifest = source.path().join("target/manifest.json");
    consumer_entry(source.path(), &built);
    let text = std::fs::read_to_string(&manifest).unwrap();
    let checkout = tempfile::tempdir().unwrap();
    git(
        checkout.path(),
        &[
            "clone",
            "--quiet",
            "--no-hardlinks",
            &path_arg(source.path()),
            ".",
        ],
    );
    assert!(
        std::fs::metadata(checkout.path().join("source.rs"))
            .unwrap()
            .modified()
            .unwrap()
            > UNIX_EPOCH
    );
    // Transport restores the producer's absolute path, not a rewritten alias.
    std::fs::remove_file(&built.path).unwrap();
    std::fs::write(&built.path, artifact).unwrap();
    let consumed = fixture_harness::parse_manifest_entry(&text, "fixture", &manifest)
        .unwrap()
        .unwrap();
    assert!(fixture_harness::verify_entry(
        &consumed,
        &built.path,
        UNIX_EPOCH,
        checkout.path(),
        VerificationMode::Transport,
    )
    .is_ok());
}

#[test]
fn identical_tree_on_another_commit_keeps_provenance_valid() {
    let dir = repository();
    let built = produce(dir.path());
    let consumed = consumer_entry(dir.path(), &built);
    let old_head = git(dir.path(), &["rev-parse", "HEAD"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=Fixture Test",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "same tree",
        ],
    );
    assert_ne!(git(dir.path(), &["rev-parse", "HEAD"]), old_head);
    assert_eq!(producer::clean_tree(dir.path(), true).unwrap(), built.tree);
    assert!(fixture_harness::verify_entry(
        &consumed,
        &built.path,
        UNIX_EPOCH,
        dir.path(),
        VerificationMode::Transport,
    )
    .is_ok());
}

#[test]
fn consumer_rejects_all_dirty_states_even_with_matching_head_and_old_mtimes() {
    // Each porcelain entry keeps its leading status column (` M` unstaged).
    for (state, entry) in [
        ("untracked", "?? new.rs"),
        ("unstaged", " M source.rs"),
        ("staged", "M  source.rs"),
    ] {
        let dir = repository();
        let built = produce(dir.path());
        let consumed = consumer_entry(dir.path(), &built);
        let path = dir.path().join(if state == "untracked" {
            "new.rs"
        } else {
            "source.rs"
        });
        std::fs::write(path, "changed source").unwrap();
        if state == "staged" {
            git(dir.path(), &["add", "source.rs"]);
        }
        let error = fixture_harness::verify_entry(
            &consumed,
            &built.path,
            future(),
            dir.path(),
            VerificationMode::Transport,
        )
        .unwrap_err();
        assert!(error.contains("dirty"), "{state}: {error}");
        assert!(
            error.contains(&format!("git status --porcelain: {entry}")),
            "{state} names {entry:?}: {error}"
        );
        assert_eq!(
            git(dir.path(), &["rev-parse", "HEAD^{tree}"]),
            built.tree.unwrap()
        );
    }
}

#[test]
fn consumer_dirty_refusal_names_eight_entries_and_counts_the_rest() {
    let dir = repository();
    let built = produce(dir.path());
    let consumed = consumer_entry(dir.path(), &built);
    for index in 0..9 {
        std::fs::write(dir.path().join(format!("dirty-{index}.rs")), "new").unwrap();
    }
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Transport,
    )
    .unwrap_err();
    for index in 0..8 {
        assert!(error.contains(&format!("?? dirty-{index}.rs")), "{error}");
    }
    assert!(!error.contains("dirty-8.rs"), "only 8 are named: {error}");
    assert!(error.ends_with("?? dirty-7.rs | … 1 more"), "{error}");
}

#[test]
fn consumer_rejects_stale_tree_without_mtime_fallback() {
    let dir = repository();
    let built = produce(dir.path());
    let consumed = consumer_entry(dir.path(), &built);
    std::fs::write(dir.path().join("source.rs"), "new committed source").unwrap();
    commit(dir.path());
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Transport,
    )
    .unwrap_err();
    assert!(
        error.contains("manifest tree") && error.contains("checkout tree"),
        "{error}"
    );
}

#[test]
fn consumer_checks_artifact_bytes_presence_and_profile_path_identity() {
    let dir = repository();
    let built = produce(dir.path());
    let consumed = consumer_entry(dir.path(), &built);
    let other = dir.path().join("target/other-profile-fixture");
    std::fs::write(&other, b"fixture bytes").unwrap();
    let error = fixture_harness::verify_entry(
        &consumed,
        &other,
        future(),
        dir.path(),
        VerificationMode::Transport,
    )
    .unwrap_err();
    assert!(error.contains("not profile binary"), "{error}");
    std::fs::write(&built.path, b"replaced fixture bytes").unwrap();
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Transport,
    )
    .unwrap_err();
    assert!(error.contains("artifact identity mismatch"), "{error}");
    std::fs::remove_file(&built.path).unwrap();
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Transport,
    )
    .unwrap_err();
    assert!(error.contains("does not exist"), "{error}");
}

#[test]
fn missing_git_checkout_never_falls_back_to_mtime() {
    let dir = repository();
    let built = produce(dir.path());
    let consumed = consumer_entry(dir.path(), &built);
    let not_a_checkout = tempfile::tempdir().unwrap();
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        not_a_checkout.path(),
        VerificationMode::Transport,
    )
    .unwrap_err();
    assert!(
        error.contains("cannot verify fixture provenance"),
        "{error}"
    );
}

#[test]
fn schema_two_local_entries_preserve_mtime_checks_in_dirty_worktrees() {
    let dir = repository();
    let built = produce(dir.path());
    let manifest = dir.path().join("target/manifest.json");
    let old = format!("{{\n  \"schema\": 2,\n  \"fixtures\": {{\n    \"fixture\": {{\"built_at\":0,\"inputs\":[\"{}\"],\"path\":\"{}\"}}\n  }}\n}}\n",
        dir.path().join("source.rs").display(), built.path.display());
    let consumed = fixture_harness::parse_manifest_entry(&old, "fixture", &manifest)
        .unwrap()
        .unwrap();
    std::fs::write(dir.path().join("source.rs"), "local source change").unwrap();
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        UNIX_EPOCH,
        dir.path(),
        VerificationMode::Local,
    )
    .unwrap_err();
    assert!(error.contains("older than its input"), "{error}");
    assert!(fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Local,
    )
    .is_ok());
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Transport,
    )
    .unwrap_err();
    assert!(
        error.contains("requires clean tree and artifact provenance"),
        "{error}"
    );
}

#[test]
fn transport_refuses_both_identities_removed_or_renamed_even_with_old_mtimes() {
    let dir = repository();
    let built = produce(dir.path());
    consumer_entry(dir.path(), &built);
    let manifest = dir.path().join("target/manifest.json");
    let valid = std::fs::read_to_string(&manifest).unwrap();
    let removed = valid
        .replace(
            &format!(",\"tree\":\"{}\"", built.tree.as_ref().unwrap()),
            "",
        )
        .replace(
            &format!(",\"artifact\":\"{}\"", built.artifact.as_ref().unwrap()),
            "",
        );
    let renamed = valid
        .replace("\"tree\":", "\"not_tree\":")
        .replace("\"artifact\":", "\"not_artifact\":");
    for changed in [removed, renamed] {
        match fixture_harness::parse_manifest_entry(&changed, "fixture", &manifest) {
            Ok(Some(consumed)) => {
                // This reference would pass the old local timestamp fallback.
                assert!(fixture_harness::verify_entry(
                    &consumed,
                    &built.path,
                    future(),
                    dir.path(),
                    VerificationMode::Local,
                )
                .is_ok());
                let error = fixture_harness::verify_entry(
                    &consumed,
                    &built.path,
                    future(),
                    dir.path(),
                    VerificationMode::Transport,
                )
                .unwrap_err();
                assert!(
                    error.contains("requires clean tree and artifact provenance"),
                    "{error}"
                );
            }
            Err(_) => {} // Unknown renamed fields are corrupt generated entries.
            Ok(None) => panic!("downgraded fixture entry disappeared"),
        }
    }
}

#[test]
fn local_schema_three_without_provenance_keeps_mtime_checks() {
    let dir = repository();
    let mut built = produce(dir.path());
    built.tree = None;
    built.artifact = None;
    let consumed = consumer_entry(dir.path(), &built);
    std::fs::write(dir.path().join("source.rs"), "local source change").unwrap();
    let error = fixture_harness::verify_entry(
        &consumed,
        &built.path,
        UNIX_EPOCH,
        dir.path(),
        VerificationMode::Local,
    )
    .unwrap_err();
    assert!(error.contains("older than its input"), "{error}");
    assert!(fixture_harness::verify_entry(
        &consumed,
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Local,
    )
    .is_ok());
}

/// Child-mode selector for the `*_child` tests; never set in the parent.
const CHILD: &str = "MAOS_FIXTURE_PROVENANCE_20_15_CHILD";
const RESOLVED: &str = "public fixture resolved: ";

/// Run the child-mode `test` of `exe` with the checkout as cwd; inherited CI,
/// transport and `GIT_*` variables are scrubbed, only `vars` are declared.
fn run_child(exe: &Path, test: &str, selector: &str, root: &Path, vars: &[(&str, &str)]) -> Output {
    let mut child = Command::new(exe);
    scrub_git(&mut child)
        .args(["--exact", test, "--nocapture"])
        .current_dir(root);
    for inherited in ["CI", "GITHUB_ACTIONS", "MAOS_FIXTURE_TRANSPORT"] {
        child.env_remove(inherited);
    }
    child
        .env(CHILD, selector)
        .env("MAOS_FIXTURE_DIR", root.join("target"))
        .envs(vars.iter().copied())
        .output()
        .unwrap()
}

/// Run the real public `fixture` lookup in this test executable.
fn public_fixture(root: &Path, vars: &[(&str, &str)]) -> Output {
    let exe = std::env::current_exe().unwrap();
    run_child(&exe, "public_fixture_child", "1", root, vars)
}

fn accepted(output: &Output, path: &Path) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains(&format!("{RESOLVED}{}", path.display())),
        "{output:?}"
    );
}

fn refused(output: &Output, reason: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success()
            && stderr.contains(reason)
            && stderr.contains(fixture_harness::BUILD_FIXTURES),
        "{output:?}"
    );
}

#[test]
fn public_fixture_child() {
    // No global environment mutation; the parent invokes this exact test.
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    println!(
        "{RESOLVED}{}",
        fixture_harness::fixture("fixture").display()
    );
}

#[test]
fn public_profile_binary_child() {
    // The selector names the binary; the parent invokes this exact test.
    if let Some(name) = std::env::var_os(CHILD) {
        let path = fixture_harness::profile_binary(name.to_str().unwrap());
        println!("{RESOLVED}{}", path.display());
    }
}

#[test]
fn public_profile_binary_requires_entries_for_manifest_owned_binaries_in_transport() {
    let dir = repository();
    // `current_exe` is canonical, so the printed profile path is too.
    let root = &dir.path().canonicalize().unwrap();
    // `profile_binary` resolves `<exe>/../../<name>`: run a copy of this test
    // executable from `<target>/<profile>/deps` inside the ignored target dir.
    let profile = root.join("target/t/debug");
    std::fs::create_dir_all(profile.join("deps")).unwrap();
    let exe = std::env::current_exe().unwrap();
    let copy = profile.join("deps").join(exe.file_name().unwrap());
    // `cp` alone holds the writable descriptor, so no sibling test's fork can
    // leave the copy "text file busy" when it is executed.
    assert!(Command::new("cp")
        .arg(&exe)
        .arg(&copy)
        .status()
        .unwrap()
        .success());
    let other = produce(root);
    consumer_entry(root, &other);
    let transport = [("MAOS_FIXTURE_TRANSPORT", "1")];
    let test = "public_profile_binary_child";
    for name in ["maosctl", "worker-cli-fixture"] {
        let path = profile.join(name);
        std::fs::write(&path, format!("{name} bytes")).unwrap();
        refused(
            &run_child(&copy, test, name, root, &transport),
            &format!("no provenance entry for manifest-owned profile binary `{name}`"),
        );
        // Own-build runs keep the presence-only check for unrecorded binaries.
        accepted(&run_child(&copy, test, name, root, &[]), &path);
        let tree = producer::clean_tree(root, true).unwrap();
        let sealed = Entry {
            artifact: producer::artifact(root, &path, &tree).unwrap(),
            tree,
            path: path.clone(),
            built_at: 0,
            inputs: Vec::new(),
        };
        write_manifest(
            &root.join("target/manifest.json"),
            &BTreeMap::from([(name.to_owned(), sealed)]),
        )
        .unwrap();
        accepted(&run_child(&copy, test, name, root, &transport), &path);
    }
}

#[test]
fn ordinary_ci_own_builds_use_mtimes_until_explicit_transport_is_selected() {
    let dir = repository();
    let root = dir.path();
    // Declared CI is not an input: only the explicit transport bit selects policy.
    let ci = [("CI", "true"), ("GITHUB_ACTIONS", "true")];
    let transport = [("MAOS_FIXTURE_TRANSPORT", "1")];
    let mut own = produce_with_mode(root, false).unwrap();
    assert!(own.tree.is_none() && own.artifact.is_none());
    own.built_at = future().duration_since(UNIX_EPOCH).unwrap().as_secs();
    consumer_entry(root, &own);
    accepted(&public_fixture(root, &ci), &own.path);
    refused(
        &public_fixture(root, &transport),
        "fixture transport requires clean tree and artifact provenance",
    );
    own.built_at = 0;
    consumer_entry(root, &own);
    refused(&public_fixture(root, &ci), "older than its input");
    let sealed = produce_with_mode(root, true).unwrap();
    assert!(sealed.tree.is_some() && sealed.artifact.is_some());
    consumer_entry(root, &sealed);
    accepted(&public_fixture(root, &transport), &sealed.path);
    // Ordinary own-build mode does not even need a Git checkout.
    let unversioned = tempfile::tempdir().unwrap();
    assert!(producer::clean_tree(unversioned.path(), false)
        .unwrap()
        .is_none());
    assert!(producer::clean_tree(unversioned.path(), true).is_err());
}

#[test]
fn incomplete_or_corrupt_manifest_cannot_authorize_an_intact_entry() {
    let dir = repository();
    let built = produce(dir.path());
    consumer_entry(dir.path(), &built);
    let manifest = dir.path().join("target/manifest.json");
    let valid = std::fs::read_to_string(&manifest).unwrap();
    // Every incomplete prefix, including prefixes after a complete entry, fails.
    for (end, _) in valid.trim_end().char_indices() {
        assert!(
            fixture_harness::parse_manifest_entry(&valid[..end], "fixture", &manifest).is_err(),
            "accepted truncated manifest at byte {end}"
        );
    }
    for changed in [
        format!("{valid}garbage"),
        valid.replace("\"schema\": 3,", "\"schema\": 3"),
        valid.replace("\"fixtures\": {", "\"fixtures\": ["),
        valid.replace("\"inputs\":[", "\"inputs\":"),
        valid.replace("],", ","),
        valid.replace(
            "\n  }\n}",
            ",\n    \"broken\": {\"path\":\"/broken\"}\n  }\n}",
        ),
        valid.replace("\"built_at\":0", "\"built_at\":00"),
        valid.replace("\"path\":", "\"path\":\"/duplicate\",\"path\":"),
    ] {
        assert!(
            fixture_harness::parse_manifest_entry(&changed, "fixture", &manifest).is_err(),
            "{changed}"
        );
    }
}

#[test]
fn absent_manifest_owned_profile_entries_fail_only_in_transport_mode() {
    let dir = repository();
    let built = produce(dir.path());
    let manifest = dir.path().join("target/manifest.json");
    let missing = "{\n  \"schema\": 3,\n  \"fixtures\": {\n\n  }\n}\n";
    for name in ["maosctl", "worker-cli-fixture", "worker-probe-fixture"] {
        let entry = fixture_harness::parse_manifest_entry(missing, name, &manifest).unwrap();
        assert!(entry.is_none());
        let error = fixture_harness::verify_profile_entry(
            entry.as_ref(),
            name,
            &built.path,
            future(),
            dir.path(),
            VerificationMode::Transport,
        )
        .unwrap_err();
        assert!(error.contains("no provenance entry"), "{name}: {error}");
        assert!(fixture_harness::verify_profile_entry(
            entry.as_ref(),
            name,
            &built.path,
            future(),
            dir.path(),
            VerificationMode::Local,
        )
        .is_ok());
    }
    // The product's feature-family archive/build identity is not this manifest.
    assert!(fixture_harness::verify_profile_entry(
        None,
        "maos",
        &built.path,
        future(),
        dir.path(),
        VerificationMode::Transport,
    )
    .is_ok());
}
