#![forbid(unsafe_code)]

//! `check-kernel-baseline` — Story 8.16 (Epic 8 retro §A4); file set and
//! per-file content hash added by Story 16-0 (drift D2, Epic-15 retro action A5).
//!
//! THE single CI-enforced source of truth for the `maos-kernel-core/src` pin.
//! It compares THREE things against `xtask/kernel-core-baseline.toml`:
//!
//! 1. `src_lines` — the physical `.rs` line count under
//!    `crates/maos-kernel-core/src` (unchanged; every HISTORY row and every
//!    story's `kernel_grant` frontmatter cites this figure).
//! 2. the pinned FILE SET — every file under the root, any extension, so an
//!    added or deleted file is NAMED rather than absorbed into a count that
//!    happens to balance.
//! 3. a PER-FILE content hash — so an edit that does not move the line count
//!    (Story 15-4's 8+/8− seccomp reflow of `security/sandbox/linux.rs` is the
//!    worked falsifier) reds the gate and NAMES the file that moved.
//!
//! The aggregate `set_hash` is DERIVED from the sorted per-file map as a cheap
//! headline. It is never the authoritative value: a scalar over N files carries
//! no per-file information and structurally cannot name anything.
//!
//! Why this exists: the Epic-8 live-runtime phase grew the kernel 15505 → 21128
//! across Stories 8.11/8.12, but each story only asserted ITS OWN "byte-identical
//! / +N" locally — the aggregate was never summed, so the records said 16263
//! while reality was 21128. A line count alone then proved too weak: 15-4 landed
//! a real kernel-core edit at a constant 24474 and the gate said PASSED.
//!
//! FOUR readers of the pinned `src_lines`, not one (the pre-16-0 doc comment
//! claimed the a2a-tcp guard read "the SAME toml (no second literal)" — false,
//! and a control whose own doc lies is the cheapest place for the next reader to
//! be misled):
//!
//! * `read_pinned` here — single-sourced for xtask, and
//!   `check-epic-close-coherence` imports it by name (`:54`, `:124`);
//! * `crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs` — a kernel-adjacent
//!   crate that must NOT grow an `xtask` dependency (`check-service-boundary` is
//!   the control that forbids it), so its literal stays;
//! * `xtask/tests/fkcs_oracle.rs` — deliberately re-implements the parse so the
//!   oracle proves something about the file rather than about this function.
//!
//! There are likewise four independent line-counting implementations
//! (here, `check_fkcs`, `t11_t12_chaos_absence`, `fkcs_oracle`). Unifying them
//! is filed to `21-4-one-instrument-and-env-registry`, which owns the
//! one-instrument charter.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub actual_lines: usize,
    pub pinned_lines: usize,
    pub baseline_file: String,
    /// Files hashed on this run. Zero is always a FAILURE, never a pass.
    pub file_set_entries: usize,
    pub set_hash: String,
    pub pinned_set_hash: String,
    /// Present in both sets, digest moved. Sorted, root-relative.
    pub changed: Vec<String>,
    /// On disk, absent from the pin. Sorted, root-relative.
    pub added: Vec<String>,
    /// Pinned, absent from disk. Sorted, root-relative.
    pub removed: Vec<String>,
}

const BASELINE_TOML: &str = "xtask/kernel-core-baseline.toml";
const KERNEL_SRC: &str = "crates/maos-kernel-core/src";

/// Domain separator for a single file's digest. Length-prefixed path and
/// length-prefixed content make the concatenation unambiguous — the
/// `evidence_ledger::worktree_commit_id` framing (`:336-353`).
const FILE_DOMAIN: &[u8] = b"maos-kernel-src-file-v1\0";
/// Domain separator for the derived aggregate over the sorted per-file map.
const SET_DOMAIN: &[u8] = b"maos-kernel-src-set-v1\0";

#[derive(Debug, serde::Deserialize)]
struct Baseline {
    kernel_src: Option<KernelSrc>,
}

#[derive(Debug, serde::Deserialize)]
struct KernelSrc {
    root: String,
    set_hash: String,
    files: BTreeMap<String, String>,
}

pub fn run(json: bool, emit_pin: bool) -> Result<(), String> {
    if emit_pin {
        if json {
            return Err(
                "--emit-pin prints a raw TOML block to stdout and cannot honor --json; \
                 pass one or the other, not both"
                    .into(),
            );
        }
        return emit_pin_block();
    }
    let report = check()?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!(
            "check-kernel-baseline: PASSED (maos-kernel-core/src = {} lines, {} files, pinned {})",
            report.actual_lines, report.file_set_entries, report.pinned_lines
        );
    } else {
        eprintln!(
            "check-kernel-baseline: FAILED — {}",
            failure_detail(&report)
        );
    }

    if !report.passed {
        return Err("kernel-core drifted from the pinned baseline".into());
    }
    Ok(())
}

/// Human-readable drift, naming every file. Also re-emitted by the composite
/// gates that consume this report, so it must stand alone.
pub fn failure_detail(report: &Report) -> String {
    let mut detail = String::new();
    if report.actual_lines != report.pinned_lines {
        detail.push_str(&format!(
            "maos-kernel-core/src is {} lines but {} pins {}. ",
            report.actual_lines, report.baseline_file, report.pinned_lines
        ));
    }
    for (label, files) in [
        ("changed", &report.changed),
        ("added", &report.added),
        ("removed", &report.removed),
    ] {
        if !files.is_empty() {
            detail.push_str(&format!(
                "{label} ({}): {}. ",
                files.len(),
                files.join(", ")
            ));
        }
    }
    detail.push_str(
        "A kernel change requires an AUTHORIZED delta (charter amendment + FLAG-Winston); \
         if intended, re-pin with `cargo run -p xtask -- check-kernel-baseline --emit-pin` \
         and record the measured value, the driver and the story key in the same commit.",
    );
    detail
}

pub fn check() -> Result<Report, String> {
    let root = workspace_root()?;
    check_at(&root.join(KERNEL_SRC), &root.join(BASELINE_TOML))
}

/// Path-injectable seam. `check()` delegates here through workspace-root
/// resolution, which is what makes the gate runnable from a workspace
/// subdirectory (`cargo test -p xtask` runs with CWD = `xtask/`) and what makes
/// the hash RE-ROOTABLE — paths are stamped root-relative and `/`-separated, so
/// a pristine tempdir copy of the tree hashes to the pinned value.
pub fn check_at(src: &Path, baseline: &Path) -> Result<Report, String> {
    let pinned_lines = read_pinned(baseline)?;
    let actual_lines = count_rs_lines(src)?;
    let pin = load_kernel_src(baseline)?;

    let files = collect_files(src)?;
    if files.is_empty() {
        return Err(format!(
            "hashed 0 files under {} — an empty kernel set is a FAILURE, never a pass",
            src.display()
        ));
    }
    let actual: BTreeMap<String, String> = files
        .iter()
        .map(|(path, content)| (path.clone(), file_digest(path, content)))
        .collect();

    // `root` names the tree the map describes. It is compared against the
    // canonical const, NEVER against `src` — comparing it to the argument would
    // destroy re-rootability, which is the property AC4's tempdir vector proves.
    if pin.root != KERNEL_SRC {
        return Err(format!(
            "{}: [kernel_src] root is `{}` but this gate pins `{KERNEL_SRC}`",
            baseline.display(),
            pin.root
        ));
    }

    // The pinned aggregate must agree with the pinned map it is derived from. A
    // self-inconsistent baseline file is a broken instrument, not a policy red,
    // so it is `Err` (the parse class) and not `Ok(passed: false)`.
    let pinned_set_hash = set_hash(&pin.files);
    if pinned_set_hash != pin.set_hash {
        return Err(format!(
            "{}: [kernel_src] set_hash {} does not match its own pinned file map ({}); \
             re-pin with `--emit-pin` rather than hand-editing either half",
            baseline.display(),
            pin.set_hash,
            pinned_set_hash
        ));
    }

    let mut changed = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    for (path, digest) in &actual {
        match pin.files.get(path) {
            Some(pinned) if pinned == digest => {}
            Some(_) => changed.push(path.clone()),
            None => added.push(path.clone()),
        }
    }
    for path in pin.files.keys() {
        if !actual.contains_key(path) {
            removed.push(path.clone());
        }
    }

    Ok(Report {
        passed: changed.is_empty()
            && added.is_empty()
            && removed.is_empty()
            && actual_lines == pinned_lines,
        actual_lines,
        pinned_lines,
        baseline_file: baseline.display().to_string(),
        file_set_entries: actual.len(),
        set_hash: set_hash(&actual),
        pinned_set_hash,
        changed,
        added,
        removed,
    })
}

/// Print the paste-ready `[kernel_src]` block to stdout. The human commits it —
/// there is no in-place rewriter, and no hold, waiver or known-mismatch
/// constant of any kind. `check_corpus::register_corpus` (`:204-242`) is the
/// repo's only baseline-re-pin precedent and this follows it: a gate that can
/// heal itself is not a gate.
fn emit_pin_block() -> Result<(), String> {
    let root = workspace_root()?;
    print!("{}", pin_block(&root.join(KERNEL_SRC))?);
    Ok(())
}

/// The paste-ready `[kernel_src]` block for an arbitrary root.
///
/// Re-rootable on purpose, and NOT merely for `--emit-pin`'s convenience: it is
/// how AC2's falsifier mints "the pin as taken at the parent commit" — copy the
/// tree, drop the parent's blob in, mint through THIS function, then run the
/// real `check_at` against the LIVE tree. Minting the pin any other way would
/// re-implement the framing inside the test, and a test that re-implements the
/// gate proves nothing about the gate CI runs (`fkcs_oracle.rs:160-174` is that
/// shape and `decision_register_gate.rs:13-16` says why it is weak).
pub fn pin_block(src: &Path) -> Result<String, String> {
    let files = collect_files(src)?;
    if files.is_empty() {
        return Err(format!(
            "hashed 0 files under {} — refusing to emit an empty pin",
            src.display()
        ));
    }
    let map: BTreeMap<String, String> = files
        .iter()
        .map(|(path, content)| (path.clone(), file_digest(path, content)))
        .collect();
    let mut block = String::new();
    block.push_str("[kernel_src]\n");
    block.push_str(&format!("root = \"{KERNEL_SRC}\"\n"));
    block.push_str(&format!("set_hash = \"{}\"\n", set_hash(&map)));
    block.push('\n');
    block.push_str("[kernel_src.files]\n");
    for (path, digest) in &map {
        block.push_str(&format!("\"{path}\" = \"{digest}\"\n"));
    }
    Ok(block)
}

/// Parse the single `src_lines = N` key from the baseline toml with a
/// comment-skipping line scan — deliberately NOT `toml::from_str` (which
/// `load_kernel_src` below does use for the `[kernel_src]` table). The file's
/// HISTORY block is `#`-prefixed prose, and a naive
/// regex over the raw text returns `23081` — an `fkcs-baseline.toml` value
/// mentioned in those comments, not the assignment. Comment-skipping line
/// parsing is therefore the only correct reader, and it is deliberately
/// single-sourced here: `check-epic-close-coherence` (Epic-13 retro C1) resolves
/// doc pins against this same function, because two readers of the authoritative
/// pin would be the very drift C1 exists to stop.
///
/// It stays a line scan after Story 16-0. `strip_prefix("src_lines")` takes the
/// FIRST match and does NOT fall through on a near-miss — it hard-errors — and
/// `t11_t12_chaos_absence.rs` `.expect()`s on the same shape, so a key named
/// `src_lines_*` would panic a kernel-adjacent test. That hazard is why no
/// 16-0 key carries an `src_lines` prefix and why `[kernel_src]` lands strictly
/// AFTER the assignment. `toml::from_str` is used only by `load_kernel_src`.
pub fn read_pinned(path: &Path) -> Result<usize, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("src_lines") {
            let val = rest.trim_start().trim_start_matches('=').trim();
            return val
                .parse::<usize>()
                .map_err(|e| format!("parse src_lines `{val}`: {e}"));
        }
    }
    Err(format!(
        "no `src_lines = N` key found in {}",
        path.display()
    ))
}

/// Read the `[kernel_src]` table with a real TOML parse. This is possible only
/// because Story 16-0 comment-repaired the file's seven orphaned prose lines
/// (`:29-35`), which made `tomllib`/`toml::from_str` fail at line 29 for three
/// epics. The repair was line-count-neutral, so `src_lines` is unmoved at `:481`.
fn load_kernel_src(path: &Path) -> Result<KernelSrc, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let baseline: Baseline =
        toml::from_str(&text).map_err(|e| format!("parse {} as toml: {e}", path.display()))?;
    baseline.kernel_src.ok_or_else(|| {
        format!(
            "no `[kernel_src]` table in {} — the file set and per-file content hash are the \
             kernel's drift tripwire; re-pin with `--emit-pin`",
            path.display()
        )
    })
}

/// Every file under `root`, any extension, no ignore list — `t3-image.lock` is
/// the signed T3 image-attestation pin read at runtime by
/// `security/sandbox/t3/image_lock.rs` and is the one security artifact that
/// must not sit unpinned inside the pinned tree.
///
/// The set is read from the FILESYSTEM, never from `git ls-files`: the compiler
/// builds what is on disk, so a `.gitignore`d `mod backdoor;` compiles into the
/// kernel while staying invisible to `--exclude-standard`.
///
/// Sorted byte-order (`LC_ALL=C`) on the root-relative path, because
/// `read_dir` order is a function of the filesystem's directory-hash state and
/// differs across machines and across a fresh clone. Addition is commutative and
/// the line-count sum survived that for three epics; SHA-256 does not.
///
/// Fail-closed: `WalkDir` yields `Result<DirEntry>` and every error propagates.
/// `fs_walk::collect_rs_files` is deliberately NOT reused — its
/// `if let Ok(entries)` swallows IO errors, so an unreadable subdirectory would
/// silently yield a shorter set and a hash over fewer files.
fn collect_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|e| format!("walk {}: {e}", root.display()))?;
        if entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            // A symlink (or fifo/socket/device) under the pinned root: WalkDir
            // without `follow_links` reports it as neither dir nor file, and a
            // silent `continue` here would leave a `ln -s`ed `mod backdoor;`
            // compiled into the kernel yet unpinned and unnamed — the exact
            // fail-open hole the filesystem read (D-16-0-F) exists to close.
            return Err(format!(
                "non-regular entry inside the pinned kernel set (refusing: a symlinked \
                 source compiles into the kernel while staying invisible to the pin): {}",
                entry.path().display()
            ));
        }
        let relative = entry.path().strip_prefix(root).map_err(|e| {
            format!(
                "strip {} from {}: {e}",
                root.display(),
                entry.path().display()
            )
        })?;
        let mut parts = Vec::new();
        for component in relative.components() {
            let part = component
                .as_os_str()
                .to_str()
                .ok_or_else(|| format!("non-UTF-8 path component in {}", entry.path().display()))?;
            parts.push(part);
        }
        let relative = parts.join("/");
        // `pin_block` writes the path into a quoted TOML key unescaped, and a
        // `"`/`\`/control character would mint a block that cannot be pasted
        // back — refuse the exotic path at collection rather than emit an
        // unparsable pin that bricks `load_kernel_src` after the next re-pin.
        if !relative
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'/' | b'_' | b'-'))
        {
            return Err(format!(
                "path `{relative}` carries a character outside [A-Za-z0-9._/-]; refusing to \
                 mint a pin block that would not paste back as valid TOML: {}",
                entry.path().display()
            ));
        }
        let content =
            fs::read(entry.path()).map_err(|e| format!("read {}: {e}", entry.path().display()))?;
        out.push((relative, content));
    }
    // `str`'s Ord is byte-wise, which IS the `LC_ALL=C` order the pinned map is
    // minted in — no locale collation anywhere. `WalkDir::sort_by_file_name` is
    // NOT equivalent: it sorts per-directory by file name, not the full
    // root-relative path. (Order-independence is ultimately guaranteed by the
    // `BTreeMap` in `check_at`/`pin_block`; this sort keeps the walk itself
    // canonical too. `the_pin_block_is_emitted_byte_sorted` pins the observable
    // half of that invariant.)
    out.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    Ok(out)
}

/// SHA-256 over the RAW bytes, framed with a domain separator plus
/// length-prefixed path and length-prefixed content.
///
/// Raw, not line-normalized: `.gitattributes:17` pins `* text=auto eol=lf` in
/// the repository AND on checkout for every platform, so normalizing line
/// endings inside the hash would silently absorb an EOL change that
/// `.gitattributes` exists to prevent — weakening the instrument to fix a hazard
/// already fixed at a better layer. Whitespace-normalizing would be worse still:
/// it would blind the hash to exactly the whitespace-only edits it exists to
/// catch. `fs::read` also removes the UTF-8 error surface `read_to_string`
/// carries.
fn file_digest(relative_path: &str, content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(FILE_DOMAIN);
    hasher.update((relative_path.len() as u64).to_le_bytes());
    hasher.update(relative_path.as_bytes());
    hasher.update((content.len() as u64).to_le_bytes());
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Derived headline over the sorted map. `BTreeMap` iterates in key byte order,
/// so this is order-independent by construction. Never authoritative.
fn set_hash(files: &BTreeMap<String, String>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(SET_DOMAIN);
    for (path, digest) in files {
        hasher.update((path.len() as u64).to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update((digest.len() as u64).to_le_bytes());
        hasher.update(digest.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn workspace_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    cwd.ancestors()
        .find(|ancestor| ancestor.join(KERNEL_SRC).is_dir())
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("failed to find workspace root from {}", cwd.display()))
}

fn count_rs_lines(dir: &Path) -> Result<usize, String> {
    let mut total = 0;
    let entries = fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("dir entry: {e}"))?.path();
        if path.is_dir() {
            total += count_rs_lines(&path)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content =
                fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            total += content.lines().count();
        }
    }
    Ok(total)
}
