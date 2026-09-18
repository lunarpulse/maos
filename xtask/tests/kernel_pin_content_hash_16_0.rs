//! Story 16-0 — the kernel pin gains a file set and a per-file content hash.
//!
//! `check-kernel-baseline` blocks thirteen CI jobs and, before this file, had no
//! test of any kind — no inline `mod tests`, no file in `xtask/tests/`. That is
//! the null control this suite removes.
//!
//! Every vector here runs the REAL `check_at` and mints its pins through the
//! REAL `pin_block`. Nothing re-implements the walk, the framing or the
//! comparison: a test that re-implements the gate proves something about the
//! test, not about the gate CI runs.
//!
//! The tracked tree is NEVER mutated. Vectors that need a changed kernel copy it
//! to a tempdir first — the AC4 reflow alone moves the count 24474 → 24471 and
//! would red `check-kernel-baseline`, `check-fmt` and `t11_t12_chaos_absence`
//! for any concurrent job, and a panic mid-vector would leave the tracked kernel
//! corrupted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use xtask::check_kernel_baseline::{check_at, failure_detail, pin_block, read_pinned, Report};

/// Story 15-4's seccomp commit — the falsifier. 8 insertions / 8 deletions in
/// the only kernel-core file it touched, with `src_lines` at 24474 on BOTH sides
/// and the gate reporting PASSED throughout.
const FALSIFIER_TAG: &str = "kernel-pin-falsifier-15-4";
const FALSIFIER_SHA: &str = "843d53657f2180b56cb374d4e61875f523da7662";
const FALSIFIER_FILE: &str = "security/sandbox/linux.rs";
/// The file set is invariant across the falsifier — 98 files at the parent and
/// at the commit — which is exactly why the SET half cannot carry AC2 and only
/// the per-file hash can.
const PINNED_ENTRIES: usize = 98;

fn workspace_root() -> PathBuf {
    // `cargo test -p xtask` runs with CWD = `xtask/`, not the workspace root.
    let cwd = std::env::current_dir().expect("current_dir is readable");
    cwd.ancestors()
        .find(|ancestor| ancestor.join("crates/maos-kernel-core/src").is_dir())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            panic!(
                "no workspace root above {} — a scratch copy built with a shared \
                 CARGO_TARGET_DIR leaves test binaries whose CARGO_MANIFEST_DIR points \
                 outside the repo; `touch` this file and rebuild before trusting a red",
                cwd.display()
            )
        })
}

fn kernel_src() -> PathBuf {
    workspace_root().join("crates/maos-kernel-core/src")
}

fn baseline_toml() -> PathBuf {
    workspace_root().join("xtask/kernel-core-baseline.toml")
}

/// A baseline file carrying the live `src_lines` and an arbitrary pin block.
fn baseline_with(dir: &Path, block: &str) -> PathBuf {
    let pinned_lines = read_pinned(&baseline_toml()).expect("live baseline parses");
    let path = dir.join("kernel-core-baseline.toml");
    std::fs::write(&path, format!("src_lines = {pinned_lines}\n\n{block}"))
        .expect("write synthetic baseline");
    path
}

/// Recursive copy. Used so no vector ever mutates the tracked tree.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create copy root");
    for entry in std::fs::read_dir(from).expect("read source dir") {
        let entry = entry.expect("dir entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

/// Fails LOUD. An unresolvable falsifier must never be a silent skip — that is
/// the `check_dev_record_completeness.rs:505` `let Ok(..) else { return
/// Vec::new() }` idiom, which turns an unavailable commit into a pass.
fn git_loud(args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .current_dir(workspace_root())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {} could not be invoked: {e}", args.join(" ")));
    assert!(
        output.status.success(),
        "git {} FAILED: {}\n\
         The Story 16-0 AC2 falsifier could not be resolved. Two causes, both fixable:\n\
         (1) the CI checkout is shallow — `.github/workflows/discipline.yml` must carry \
         `with: fetch-depth: 0` on the `check-kernel-baseline` job (precedent: `check-fkcs`);\n\
         (2) the anchor tag is missing — run \
         `git push origin refs/tags/{FALSIFIER_TAG}`.\n\
         This test MUST NOT pass by skipping: the defect class it guards is an edit that a \
         hand-written fixture would not think to make.",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    output.stdout
}

fn git_text(args: &[&str]) -> String {
    String::from_utf8(git_loud(args)).expect("git output is UTF-8")
}

/// Every `Report` any vector in this file produces passes through here. AC3's
/// derivation invariant: a `passed` that is not computed from the findings is a
/// pass that a hold constant could mint.
fn assert_passed_is_derived(report: &Report) {
    let derived = report.changed.is_empty()
        && report.added.is_empty()
        && report.removed.is_empty()
        && report.actual_lines == report.pinned_lines;
    assert_eq!(
        report.passed,
        derived,
        "`passed` is not derived from the findings: passed={} but changed={:?} added={:?} \
         removed={:?} lines={}/{}",
        report.passed,
        report.changed,
        report.added,
        report.removed,
        report.actual_lines,
        report.pinned_lines
    );
}

fn checked(src: &Path, baseline: &Path) -> Report {
    let report = check_at(src, baseline).expect("check_at returns a Report, not an Err");
    assert_passed_is_derived(&report);
    report
}

// ── AC1 / AC2 green control ──────────────────────────────────────────────────

#[test]
fn live_tree_matches_the_committed_pin() {
    let report = checked(&kernel_src(), &baseline_toml());
    assert!(
        report.passed,
        "the committed pin does not describe the live kernel: {}",
        failure_detail(&report)
    );
    assert_eq!(
        report.file_set_entries, PINNED_ENTRIES,
        "the pinned denominator moved; re-pin with `--emit-pin` and record the driver"
    );
    assert_eq!(report.set_hash, report.pinned_set_hash);
}

// ── AC2: proven red against the real commit, not a fixture ───────────────────

#[test]
fn falsifier_tag_is_annotated_and_peels_to_the_pinned_commit() {
    let kind = git_text(&["cat-file", "-t", FALSIFIER_TAG]);
    assert_eq!(
        kind.trim(),
        "tag",
        "`{FALSIFIER_TAG}` must be an ANNOTATED tag — a lightweight tag carries no \
         tagger or message and can be re-pointed without trace (the \
         `check_fkcs.rs:736-750` precedent)"
    );
    let peeled = git_text(&["rev-parse", &format!("{FALSIFIER_TAG}^{{commit}}")]);
    assert_eq!(
        peeled.trim(),
        FALSIFIER_SHA,
        "`{FALSIFIER_TAG}` has MOVED. It anchors Story 16-0's AC2 falsifier; \
         re-point it at {FALSIFIER_SHA} or the proof is gone.\n\
         Note: ancestry of HEAD is deliberately NOT required — after a squash or rebase \
         merge of `recovery-lane` into `main` the commit stops being an ancestor while \
         the tag keeps it and its parent alive."
    );
}

#[test]
fn a_line_neutral_kernel_edit_reds_the_gate_and_names_the_file() {
    let temp = tempfile::tempdir().expect("tempdir");

    // The pin AS TAKEN AT THE PARENT: copy the live tree, drop the parent's real
    // blob in over the one file the falsifier touched, mint through the gate's
    // own `pin_block`. No fixture — the bytes come from git.
    let parent_tree = temp.path().join("parent-src");
    copy_tree(&kernel_src(), &parent_tree);
    let parent_blob = git_loud(&[
        "show",
        &format!("{FALSIFIER_TAG}^:crates/maos-kernel-core/src/{FALSIFIER_FILE}"),
    ]);
    let live_blob = std::fs::read(kernel_src().join(FALSIFIER_FILE)).expect("live blob reads");
    assert_ne!(
        parent_blob, live_blob,
        "the falsifier's parent blob is byte-identical to the live one — the vector \
         would prove nothing"
    );
    assert_eq!(
        parent_blob.iter().filter(|b| **b == b'\n').count(),
        live_blob.iter().filter(|b| **b == b'\n').count(),
        "the falsifier must be LINE-NEUTRAL — that is the whole defect class"
    );
    std::fs::write(parent_tree.join(FALSIFIER_FILE), &parent_blob).expect("plant parent blob");

    let parent_pin = pin_block(&parent_tree).expect("mint the parent's pin");
    let baseline = baseline_with(temp.path(), &parent_pin);

    // The REAL gate, the parent's pin, the LIVE tree.
    let report = checked(&kernel_src(), &baseline);

    assert!(
        !report.passed,
        "the pin taken at {} passed against the live tree — the line count is 24474 on \
         BOTH sides of the falsifier, so only the per-file hash can red this",
        &FALSIFIER_SHA[..8]
    );
    assert_eq!(
        report.actual_lines, report.pinned_lines,
        "the falsifier is line-neutral: the count must still agree, or this vector is \
         proving the OLD instrument"
    );
    assert_eq!(
        report.changed,
        vec![FALSIFIER_FILE.to_string()],
        "exactly one file changed, and it must be NAMED"
    );
    assert!(report.added.is_empty(), "added: {:?}", report.added);
    assert!(report.removed.is_empty(), "removed: {:?}", report.removed);
    assert_eq!(
        report.file_set_entries, PINNED_ENTRIES,
        "the SET is invariant across the falsifier — 98 files both sides"
    );
    assert!(
        failure_detail(&report).contains(FALSIFIER_FILE),
        "the failure message must name {FALSIFIER_FILE}: {}",
        failure_detail(&report)
    );
}

#[test]
fn an_added_file_is_named_not_absorbed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tree = temp.path().join("src");
    copy_tree(&kernel_src(), &tree);
    let baseline = baseline_with(temp.path(), &pin_block(&tree).expect("mint pristine pin"));

    std::fs::write(tree.join("security/sandbox/backdoor.rs"), b"// planted\n")
        .expect("plant a file");

    let report = checked(&tree, &baseline);
    assert!(!report.passed);
    assert_eq!(
        report.added,
        vec!["security/sandbox/backdoor.rs".to_string()]
    );
    assert!(report.changed.is_empty());
    assert!(report.removed.is_empty());
    assert_eq!(report.file_set_entries, PINNED_ENTRIES + 1);
}

#[test]
fn a_deleted_file_is_named_not_absorbed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tree = temp.path().join("src");
    copy_tree(&kernel_src(), &tree);
    let baseline = baseline_with(temp.path(), &pin_block(&tree).expect("mint pristine pin"));

    std::fs::remove_file(tree.join("security/sandbox/t3-image.lock")).expect("delete a file");

    let report = checked(&tree, &baseline);
    assert!(!report.passed);
    assert_eq!(
        report.removed,
        vec!["security/sandbox/t3-image.lock".to_string()],
        "`t3-image.lock` is the signed T3 trust anchor inside the pinned tree; an \
         `.rs`-only set would not have noticed it leaving"
    );
    assert_eq!(
        report.file_set_entries,
        PINNED_ENTRIES - 1,
        "a deletion must shrink the measured denominator by exactly one"
    );
    assert!(report.changed.is_empty());
    assert!(report.added.is_empty());
}

#[test]
fn the_non_rs_trust_anchor_is_inside_the_pinned_set() {
    let pin = pin_block(&kernel_src()).expect("mint");
    assert!(
        pin.contains("\"security/sandbox/t3-image.lock\""),
        "the pinned set must cover EVERY file under the root, any extension — an \
         `.rs`-only set leaves the one signed security artifact in the tree unpinned"
    );
}

#[test]
fn non_regular_entries_are_refused_not_skipped() {
    // The fail-open hole the Story 16-0 review closed: `WalkDir` without
    // `follow_links` reports a symlink as neither dir nor file, and a silent
    // `continue` left a `ln -s`ed source compiled into the kernel yet
    // unpinned and unnamed. The walk must REFUSE, naming the entry.
    // Unix-only: the symlink half needs `symlink(2)`; the CI leg that runs
    // this suite (`check-kernel-baseline`, ubuntu-latest) is unix.
    #[cfg(unix)]
    {
        let temp = tempfile::tempdir().expect("tempdir");
        let tree = temp.path().join("src");
        copy_tree(&kernel_src(), &tree);
        let target = kernel_src().join("api.rs");
        std::os::unix::fs::symlink(&target, tree.join("security/link.rs"))
            .expect("plant a symlink");
        let error = check_at(&tree, &baseline_toml())
            .expect_err("a symlink inside the pinned set must be refused, not skipped");
        assert!(
            error.contains("security/link.rs") && error.contains("symlinked"),
            "the refusal must name the entry and say why: {error}"
        );
        let error =
            pin_block(&tree).expect_err("pin_block must refuse the same walk, not mint around it");
        assert!(error.contains("security/link.rs"), "{error}");
    }
}

#[test]
fn paths_that_cannot_roundtrip_through_toml_are_refused() {
    // `pin_block` writes paths as quoted TOML keys unescaped; a `"` in a
    // filename would mint a block that cannot paste back, bricking
    // `load_kernel_src` after the next re-pin. Refuse at collection.
    let temp = tempfile::tempdir().expect("tempdir");
    let tree = temp.path().join("src");
    copy_tree(&kernel_src(), &tree);
    std::fs::write(tree.join("we\"ird.rs"), b"// quoted name\n").expect("plant");
    let error = check_at(&tree, &baseline_toml())
        .expect_err("a non-roundtrippable path must be refused, not pinned");
    assert!(
        error.contains("we\"ird.rs") && error.contains("TOML"),
        "the refusal must name the path and say why: {error}"
    );
}

#[test]
fn emit_pin_prints_a_parseable_block_and_refuses_json() {
    // AC3 names the CLI surface as THE re-pin path; every other vector calls
    // the library `pin_block`. This drives the real binary so a flag/dispatch
    // wiring regression cannot ship green.
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(workspace_root())
        .args(["check-kernel-baseline", "--emit-pin"])
        .output()
        .expect("xtask runs");
    assert!(
        output.status.success(),
        "--emit-pin failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        stdout.starts_with("[kernel_src]\n"),
        "--emit-pin must print the paste-ready block to stdout, got: {stdout:?}"
    );
    let pinned_lines = read_pinned(&baseline_toml()).expect("live baseline parses");
    let parsed: toml::Value = toml::from_str(&format!("src_lines = {pinned_lines}\n\n{stdout}"))
        .expect("the emitted block pastes back as valid TOML");
    assert_eq!(
        parsed["kernel_src"]["files"]
            .as_table()
            .expect("files table")
            .len(),
        PINNED_ENTRIES,
        "the emitted block carries the full denominator"
    );

    let both = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(workspace_root())
        .args(["check-kernel-baseline", "--emit-pin", "--json"])
        .output()
        .expect("xtask runs");
    assert!(
        !both.status.success() && both.stdout.is_empty(),
        "--emit-pin --json must be refused loudly with clean stdout, not answered with a \
         TOML block on a JSON call: exit {:?}, stdout {:?}",
        both.status.code(),
        String::from_utf8_lossy(&both.stdout)
    );
}

// ── AC3: the gate cannot be silenced, proven by behaviour ────────────────────

#[test]
fn every_one_of_the_pinned_files_reds_a_line_neutral_mutation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tree = temp.path().join("src");
    copy_tree(&kernel_src(), &tree);
    let baseline = baseline_with(temp.path(), &pin_block(&tree).expect("mint pristine pin"));

    let pristine = checked(&tree, &baseline);
    assert!(pristine.passed, "the pristine copy must be green first");

    // The denominator is asserted BEFORE the predicate: a loop that silently
    // iterates over fewer files proves nothing.
    let paths: Vec<String> = pinned_paths(&baseline);
    assert_eq!(
        paths.len(),
        PINNED_ENTRIES,
        "the mutation vector must cover every pinned file"
    );

    let mut proven = 0usize;
    for relative in &paths {
        let path = tree.join(relative);
        let original = std::fs::read(&path).expect("read file under test");
        let Some(index) = original.iter().position(u8::is_ascii_alphanumeric) else {
            panic!("{relative} has no ASCII alphanumeric byte to mutate");
        };
        // Line-neutral by construction: one alphanumeric byte becomes a
        // different alphanumeric byte. UTF-8 stays valid and no newline moves,
        // so EVERY iteration is the 15-4 defect class.
        let mut mutated = original.clone();
        mutated[index] = if original[index] == b'a' { b'b' } else { b'a' };
        std::fs::write(&path, &mutated).expect("plant the mutation");

        let report = checked(&tree, &baseline);
        assert!(
            !report.passed,
            "{relative} was mutated and the gate still PASSED — it is exempt from the \
             comparison, whether by a constant, a `continue` or a `retain`"
        );
        assert_eq!(
            report.actual_lines, report.pinned_lines,
            "{relative}: the mutation was supposed to be line-neutral"
        );
        assert_eq!(report.changed, vec![relative.clone()], "{relative}");
        assert!(report.added.is_empty(), "{relative}");
        assert!(report.removed.is_empty(), "{relative}");

        std::fs::write(&path, &original).expect("restore");
        assert!(
            checked(&tree, &baseline).passed,
            "{relative} did not restore cleanly"
        );
        proven += 1;
    }
    assert_eq!(
        proven, PINNED_ENTRIES,
        "every pinned file must have been proven"
    );
}

/// Every path in the pin, read back through a real TOML parse.
fn pinned_paths(baseline: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(baseline).expect("read baseline");
    let parsed: toml::Value = toml::from_str(&text).expect("baseline parses as TOML");
    parsed["kernel_src"]["files"]
        .as_table()
        .expect("[kernel_src.files] is a table")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn the_gate_carries_no_hold_or_waiver_constant() {
    // SECONDARY tripwire only. Renaming the constant defeats it, which is why
    // the exhaustive mutation vector above is the binding control.
    let source =
        std::fs::read_to_string(workspace_root().join("xtask/src/check_kernel_baseline.rs"))
            .expect("read the gate source");
    for needle in ["HELD_", "ADVISORY", "KNOWN_", "WAIVER"] {
        assert!(
            !source.contains(needle),
            "`{needle}` appears in the gate. §9 measures exactly this pattern holding the \
             FKCS admission hash RED since Story 13.4: a gate whose red can be silenced by \
             editing a constant in the gate is not a control."
        );
    }
}

// ── AC4: formatting does not move the hash, proven by a vector that can fail ─

#[test]
fn a_reflow_moves_the_hash_and_rustfmt_restores_it() {
    // `rustfmt` the binary, not `cargo fmt` — the tempdir copy has no
    // `Cargo.toml`. The `check-kernel-baseline` CI job installs `stable` with no
    // `components:` line, so this must fail LOUD rather than skip.
    let version = Command::new("rustfmt").arg("--version").output();
    let version = match version {
        Ok(output) if output.status.success() => output,
        other => panic!(
            "rustfmt is not available ({other:?}). The `check-kernel-baseline` job must \
             install `components: rustfmt` (precedent: the `check-fmt` job). An absent \
             toolchain component MUST NOT be a silent pass."
        ),
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let tree = temp.path().join("src");
    copy_tree(&kernel_src(), &tree);

    // H0 over the pristine COPY against the COMMITTED pin. This is what proves
    // the hasher is re-rootable, and therefore that paths are stamped
    // root-relative rather than resolved through the workspace.
    let h0 = checked(&tree, &baseline_toml());
    assert!(
        h0.passed,
        "the pristine tempdir copy does not match the committed pin — the hash is not \
         re-rootable: {}",
        failure_detail(&h0)
    );

    // No `#[rustfmt::skip]` in this file. The tree's only one is in
    // `security/sandbox/linux.rs:25` — introduced by the very commit AC2 uses as
    // its falsifier — and rustfmt would not restore a reflow there.
    let victim = tree.join("security/sandbox/unsupported.rs");
    let pristine = std::fs::read_to_string(&victim).expect("read the reflow victim");
    assert!(
        !pristine.contains("rustfmt::skip"),
        "the reflow victim must not carry `#[rustfmt::skip]` or it fails for the wrong reason"
    );
    // Join a multi-line signature onto one line. This is reflow, not mangling:
    // the joined line is 107 chars against rustfmt's default `max_width = 100`
    // (there is no `rustfmt.toml` in this repo), so rustfmt re-splits it back to
    // exactly the original three-line parameter list. Collapsing BLANK lines
    // would NOT work — rustfmt preserves author blank lines and only caps runs,
    // so that edit is not reversible and the vector would fail for the wrong
    // reason.
    let reflowed = pristine.replace(
        "pub fn spawn_sandboxed(\n    _spec: &SandboxSpec,\n    _command: &mut Command,\n) -> Result<SandboxedChild, SpawnError> {",
        "pub fn spawn_sandboxed(_spec: &SandboxSpec, _command: &mut Command) -> Result<SandboxedChild, SpawnError> {",
    );
    assert_ne!(
        reflowed,
        pristine,
        "the reflow found nothing to join in {} — the signature it targets has moved, and \
         a vector that silently changes nothing is the null control this story exists to \
         refuse",
        victim.display()
    );
    std::fs::write(&victim, &reflowed).expect("reflow");

    // The POSITIVE half. Without it the vector passes on a hasher that ignores
    // its input — and the epic's original AC4 was exactly that null control,
    // because `cargo fmt --all -- --check` already exits 0 at this baseline.
    let h1 = checked(&tree, &baseline_toml());
    assert!(
        !h1.passed,
        "a reflow did not move the hash — the hasher is not reading its input"
    );
    assert_eq!(
        h1.changed,
        vec!["security/sandbox/unsupported.rs".to_string()]
    );
    assert_ne!(h1.set_hash, h0.set_hash);
    assert!(
        h1.added.is_empty() && h1.removed.is_empty(),
        "the reflow must change exactly one file: added {:?} removed {:?}",
        h1.added,
        h1.removed
    );
    assert_ne!(
        h1.actual_lines, h0.actual_lines,
        "and note the reflow moves the COUNT too, which is why it must never touch the \
         tracked tree"
    );

    let formatted = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .arg(&victim)
        .status()
        .expect("rustfmt runs");
    assert!(
        formatted.success(),
        "rustfmt {} failed ({:?})",
        victim.display(),
        String::from_utf8_lossy(&version.stdout).trim()
    );

    let h2 = checked(&tree, &baseline_toml());
    assert!(
        h2.passed,
        "rustfmt did not restore the tree to the pinned bytes: {}",
        failure_detail(&h2)
    );
    assert_eq!(
        h2.set_hash, h0.set_hash,
        "rustfmt is deterministic and idempotent, so on a tree held at its fixed point \
         the content hash is a stable function of content"
    );
}

#[test]
fn the_reflow_vector_left_the_tracked_tree_untouched() {
    // Paired with the vector above: if any test in this file ever writes to the
    // tracked kernel, the committed pin stops describing it.
    let report = checked(&kernel_src(), &baseline_toml());
    assert!(
        report.passed,
        "a test in this file mutated the TRACKED kernel tree: {}",
        failure_detail(&report)
    );
}

// ── AC5: determinism, the parser invariants, and the blast radius ────────────

#[test]
fn the_hash_is_independent_of_directory_creation_order() {
    // `read_dir` order is a function of the filesystem's directory-hash state:
    // it differs across machines, across a fresh clone, and after file churn.
    // Addition is commutative so the line-count SUM survived that for three
    // epics. SHA-256 does not.
    //
    // What actually NORMALIZES order here is the `BTreeMap` in `check_at` and
    // `pin_block` — the explicit sort in `collect_files` is not observable
    // through this vector (delete it and this test still passes). The
    // observable half of the invariant — the emitted map is byte-sorted — is
    // pinned by `the_pin_block_is_emitted_byte_sorted` below, so neither half
    // of the claim rests on a vector that cannot fail.
    let temp = tempfile::tempdir().expect("tempdir");
    let forward = temp.path().join("forward");
    copy_tree(&kernel_src(), &forward);

    // Build a second copy whose entries are created in reverse order at every
    // level, so the two trees have different raw readdir orders.
    let reverse = temp.path().join("reverse");
    copy_tree_reversed(&kernel_src(), &reverse);

    let a = checked(&forward, &baseline_toml());
    let b = checked(&reverse, &baseline_toml());
    assert_eq!(
        a.set_hash, b.set_hash,
        "the hash depends on directory order — it must sort byte-order on the \
         root-relative path before hashing"
    );
    assert!(a.passed && b.passed);
}

#[test]
fn the_pin_block_is_emitted_byte_sorted() {
    // The observable half of AC5's order invariant: `pin_block`'s output IS
    // the pinned map serialized, so its file lines must appear in byte
    // (`LC_ALL=C`) order. This fails if the map stops normalizing through the
    // `BTreeMap` (e.g. a swap to `HashMap`) or the emission order changes.
    let block = pin_block(&kernel_src()).expect("mint");
    let paths: Vec<&str> = block
        .lines()
        .filter_map(|line| line.split_once("\" = \""))
        .map(|(path, _)| path.trim_start_matches('"'))
        .collect();
    assert_eq!(
        paths.len(),
        PINNED_ENTRIES,
        "the emitted block carries the full denominator"
    );
    let mut sorted = paths.clone();
    sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    assert_eq!(
        paths, sorted,
        "pin_block must emit the map LC_ALL=C (byte) sorted"
    );
}

fn copy_tree_reversed(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create copy root");
    let mut entries: Vec<_> = std::fs::read_dir(from)
        .expect("read source dir")
        .map(|e| e.expect("dir entry"))
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    for entry in entries {
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree_reversed(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

#[test]
fn a_mismatch_is_ok_passed_false_and_never_err() {
    // Not a style note. `check_cohort_mesh.rs:696` is `if !check()?.passed`,
    // `check_multi_tenant_loom.rs:1889` is `check()?` and
    // `check_epic_close_coherence.rs:124` is `read_pinned(..)?` — all three
    // report correctly on `Ok(passed: false)`, and ONLY an `Err` aborts them
    // before they emit their own JSON.
    let temp = tempfile::tempdir().expect("tempdir");
    let tree = temp.path().join("src");
    copy_tree(&kernel_src(), &tree);
    let baseline = baseline_with(temp.path(), &pin_block(&tree).expect("mint"));
    let victim = tree.join("api.rs");
    let original = std::fs::read(&victim).expect("read");
    let mut mutated = original.clone();
    let index = mutated
        .iter()
        .position(u8::is_ascii_alphanumeric)
        .expect("alphanumeric byte");
    mutated[index] = if original[index] == b'a' { b'b' } else { b'a' };
    std::fs::write(&victim, mutated).expect("plant");

    let outcome = check_at(&tree, &baseline);
    let report = outcome.expect("a hash mismatch must be Ok(passed: false), NOT Err");
    assert!(!report.passed);
    assert_eq!(report.changed, vec!["api.rs".to_string()]);
    assert!(
        report.added.is_empty() && report.removed.is_empty(),
        "the mismatch must change exactly one file: added {:?} removed {:?}",
        report.added,
        report.removed
    );
    assert_eq!(
        report.file_set_entries, PINNED_ENTRIES,
        "the set is intact — only content moved"
    );
}

#[test]
fn an_empty_tree_is_a_failure_never_a_pass() {
    let temp = tempfile::tempdir().expect("tempdir");
    let empty = temp.path().join("empty-src");
    std::fs::create_dir_all(&empty).expect("create empty root");
    let error = check_at(&empty, &baseline_toml())
        .expect_err("hashing zero files must never produce a passing Report");
    assert!(
        error.contains("0 files"),
        "the refusal must say so out loud: {error}"
    );
}

#[test]
fn io_and_parse_failures_stay_err() {
    let temp = tempfile::tempdir().expect("tempdir");
    let absent = temp.path().join("kernel-core-baseline.toml");
    check_at(&kernel_src(), &absent).expect_err("a missing baseline file is an Err");

    let no_table = temp.path().join("no-table.toml");
    std::fs::write(&no_table, "src_lines = 24474\n").expect("write");
    let error = check_at(&kernel_src(), &no_table)
        .expect_err("a baseline with no [kernel_src] table is an Err");
    assert!(error.contains("[kernel_src]"), "{error}");
}

#[test]
fn a_self_inconsistent_pin_is_a_parse_error_not_a_policy_red() {
    let temp = tempfile::tempdir().expect("tempdir");
    let block = pin_block(&kernel_src()).expect("mint");
    let tampered: String = block
        .lines()
        .map(|line| {
            if line.starts_with("set_hash = ") {
                "set_hash = \"0000000000000000000000000000000000000000000000000000000000000000\""
                    .to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let baseline = baseline_with(temp.path(), &tampered);
    let error = check_at(&kernel_src(), &baseline)
        .expect_err("a set_hash that disagrees with its own map is a broken instrument");
    assert!(error.contains("set_hash"), "{error}");
}

#[test]
fn no_key_before_src_lines_begins_with_src_lines() {
    // D-16-0-C, pinned in BOTH directions. `read_pinned` takes the FIRST
    // `src_lines`-prefixed line and hard-errors on a near-miss; the a2a-tcp
    // guard `.expect()`s on the same shape and would PANIC a kernel-adjacent
    // test. So: no 16-0 key carries the prefix, AND the table lands after the
    // assignment. Either alone is sufficient; both are pinned so a later editor
    // cannot re-open it by moving one line.
    let text = std::fs::read_to_string(baseline_toml()).expect("read baseline");
    let mut seen_assignment = false;
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("src_lines = ") {
            seen_assignment = true;
            continue;
        }
        assert!(
            !trimmed.starts_with("src_lines"),
            "line {} begins with `src_lines` but is not the assignment: {trimmed}",
            number + 1
        );
        assert!(
            seen_assignment,
            "line {} is a non-comment key BEFORE `src_lines = ` ({trimmed}) — the three \
             hand-rolled line scanners take the first match, so this would hijack the pin",
            number + 1
        );
    }
    assert!(seen_assignment, "no `src_lines = ` assignment found");
}

#[test]
fn the_three_other_readers_still_resolve_the_pin() {
    assert_eq!(
        read_pinned(&baseline_toml()).expect("read_pinned"),
        24628,
        "the xtask reader"
    );

    let text = std::fs::read_to_string(baseline_toml()).expect("read baseline");

    // `t11_t12_chaos_absence.rs:196-203` — `starts_with` + `rsplit('=')` +
    // `.expect()`. Replayed here because that crate cannot take an `xtask`
    // dependency (`check-service-boundary` forbids it), so the shape is pinned
    // from this side instead.
    let a2a: usize = text
        .lines()
        .map(str::trim)
        .find(|l| !l.starts_with('#') && l.starts_with("src_lines"))
        .and_then(|l| l.rsplit('=').next())
        .map(str::trim)
        .and_then(|v| v.parse().ok())
        .expect("the a2a-tcp guard's parse still resolves");
    assert_eq!(a2a, 24628, "the a2a-tcp reader");

    // `fkcs_oracle.rs:170` — exact `strip_prefix("src_lines = ")`.
    let oracle: usize = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("src_lines = "))
        .expect("the fkcs oracle's parse still resolves")
        .parse()
        .expect("parses");
    assert_eq!(oracle, 24628, "the fkcs oracle reader");
}

#[test]
fn the_pin_is_readable_by_a_real_toml_parser() {
    // AC1's precondition. The file was invalid TOML at line 29 for three epics
    // (seven HISTORY lines that lost their `#`), so `[kernel_src]` was
    // unimplementable until Story 16-0 repaired them. The repair was
    // line-count-neutral; Story 16-5's ratification ledger now places it at line 496.
    let text = std::fs::read_to_string(baseline_toml()).expect("read baseline");
    let parsed: toml::Value = toml::from_str(&text).expect("the baseline file must be valid TOML");
    assert_eq!(parsed["src_lines"].as_integer(), Some(24628));

    let numbered = text
        .lines()
        .position(|line| line.trim_start().starts_with("src_lines = "))
        .expect("src_lines assignment exists");
    assert_eq!(
        numbered + 1,
        505,
        "`src_lines` moved off line 505 — every `kernel-core-baseline.toml:505` citation \
         in the tree resolves against that line"
    );

    let files = parsed["kernel_src"]["files"]
        .as_table()
        .expect("[kernel_src.files]");
    assert_eq!(files.len(), PINNED_ENTRIES);
    let sorted: Vec<&String> = files.keys().collect();
    let mut expected = sorted.clone();
    expected.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    assert_eq!(
        sorted, expected,
        "the pinned map must be stored LC_ALL=C sorted"
    );
}

#[test]
fn the_digests_are_distinct_per_file() {
    // A framing bug that ignored the path or the content would collapse digests.
    let block = pin_block(&kernel_src()).expect("mint");
    let map: BTreeMap<&str, &str> = block
        .lines()
        .filter_map(|line| line.split_once("\" = \""))
        .map(|(path, digest)| (path.trim_start_matches('"'), digest.trim_end_matches('"')))
        .collect();
    assert_eq!(map.len(), PINNED_ENTRIES);
    let unique: std::collections::BTreeSet<&&str> = map.values().collect();
    assert_eq!(
        unique.len(),
        PINNED_ENTRIES,
        "two pinned files share a digest — the framing is not separating path from content"
    );
}

#[test]
fn a_composite_gates_json_stdout_is_machine_clean() {
    // `deferred-work.md:637`, closed here. The obvious fix was NOT the fix: this
    // gate's own `--json` was already clean, because `check_kernel_baseline::run`
    // prints the human PASSED line only in the non-json `else if` branch. The
    // pollution came from SIX composite legs that called `run(false)` — which
    // prints that line to STDOUT on the green path — from inside their own
    // `--json` payload. They now call `check()` (`check_fkcs.rs:95-97` shape).
    //
    // Exit status is deliberately not asserted: the contract being proven is
    // that stdout PARSES, whether the gate is green or red.
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(workspace_root())
        .args(["check-escape-detector", "--json"])
        .output()
        .expect("xtask runs");
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        !stdout.trim().is_empty(),
        "check-escape-detector produced an EMPTY report (exit {:?}) — its own startup \
         failure, not this suite's; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|e| {
        panic!(
            "check-escape-detector --json stdout does not parse ({e}). A leg is printing a \
             human line to stdout inside the JSON payload:\n{stdout}"
        )
    });
    assert!(
        !stdout.contains("check-kernel-baseline: PASSED"),
        "the kernel gate's human PASSED line leaked into a composite gate's JSON payload"
    );
}
