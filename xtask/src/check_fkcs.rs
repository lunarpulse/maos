#![forbid(unsafe_code)]

//! Story 11.5 — Frozen-Kernel Conformance Suite infrastructure gate.
//!
//! Surface measurement lives in xtask so the oracle can call sibling gate modules
//! directly. `maos-fkcs` remains a dev-only fixture crate for cohort/admission
//! tests and does not depend on xtask.

use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const GATE_NAME: &str = "check-fkcs";
const CURRENT_PHASE: &str = "v1_5";
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];
const BASELINE_FILE: &str = "xtask/fkcs-baseline.toml";

/// Frozen admission-path baseline (literal AC3). Pins a SHA-256 over the
/// declared admission source files so the `admission-path-unmodified` leg
/// measures byte content, not mere file existence. Re-pin `sha256` after any
/// deliberate admission-path change.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct AdmissionBaseline {
    pub files: Vec<String>,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct FkcsBaseline {
    pub frozen_tag: String,
    pub frozen_commit: String,
    pub src_lines: usize,
    pub abi_baseline: String,
    pub host_baseline: String,
    pub frozen_at: String,
    pub ratifier: String,
    pub admission_baseline: AdmissionBaseline,
}

impl FkcsBaseline {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = resolve_workspace_path(path.as_ref())?;
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        toml::from_str(&raw).map_err(|e| format!("failed to parse {}: {e}", path.display()))
    }

    pub fn validate_files_exist(&self) -> Result<(), String> {
        for path in [&self.abi_baseline, &self.host_baseline] {
            let resolved = resolve_workspace_path(Path::new(path))?;
            let metadata = fs::metadata(&resolved)
                .map_err(|e| format!("baseline file {} missing: {e}", resolved.display()))?;
            if metadata.len() == 0 {
                return Err(format!("baseline file {} is empty", resolved.display()));
            }
        }
        Ok(())
    }

    pub fn validate_frozen_tag_src_lines(&self) -> Result<(), String> {
        let paths = git([
            "ls-tree",
            "-r",
            "--name-only",
            &self.frozen_tag,
            "--",
            "crates/maos-kernel-core/src",
        ])?;
        let mut total = 0;
        for path in paths.lines().filter(|path| path.ends_with(".rs")) {
            let object = format!("{}:{path}", self.frozen_tag);
            let source = git(["show", object.as_str()])?;
            total += source.lines().count();
        }
        self.reconcile_src_lines(total)
    }

    pub fn reconcile_src_lines(&self, actual: usize) -> Result<(), String> {
        if self.src_lines != actual {
            return Err(format!(
                "src_lines mismatch: fkcs-baseline.toml pins {}, live gate reports {actual}",
                self.src_lines
            ));
        }
        Ok(())
    }

    /// Validate the current workspace independently of the historical frozen
    /// snapshot. The current kernel line count is governed by
    /// `kernel-core-baseline.toml`; `validate_frozen_tag_src_lines` separately
    /// verifies the snapshot against its frozen revision.
    pub fn validate_live_triple(&self) -> Result<(), String> {
        self.validate_files_exist()?;
        // Use `check()` (no stdout) instead of `run(false)` so `--json` stdout
        // stays parseable — `run` prints a human "PASSED" line to stdout.
        let report = crate::check_kernel_baseline::check()?;
        if !report.passed {
            return Err(format!(
                "kernel-core line count drifted from the pinned baseline ({} pins {}, \
                 live {}); authorize the delta or re-pin the current kernel baseline",
                report.baseline_file, report.pinned_lines, report.actual_lines
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FkcsSurfaceSnapshot {
    pub src_lines: usize,
    pub abi_items: BTreeSet<String>,
    pub host_items: BTreeSet<String>,
}

impl FkcsSurfaceSnapshot {
    pub fn synthetic<A, H, AS, HS>(src_lines: usize, abi_items: A, host_items: H) -> Self
    where
        A: IntoIterator<Item = AS>,
        AS: Into<String>,
        H: IntoIterator<Item = HS>,
        HS: Into<String>,
    {
        Self {
            src_lines,
            abi_items: abi_items.into_iter().map(Into::into).collect(),
            host_items: host_items.into_iter().map(Into::into).collect(),
        }
    }

    /// surface. `validate_live_triple` verifies the current workspace against
    /// its current kernel baseline, while `validate_frozen_tag_src_lines`
    /// verifies the immutable snapshot against its annotated frozen revision.
    ///
    /// GREEN-path measurement used by the diff-oracle leg — it must NOT be a
    /// hardcoded literal. The fault-inject leg takes this real capture and
    /// applies synthetic mutations around it. Spawns `cargo` (needs nightly +
    /// cargo-public-api); a missing toolchain is a hard `Err`, never a pass.
    pub fn capture_from_baselines(baseline: &FkcsBaseline) -> Result<Self, String> {
        baseline.validate_live_triple()?;
        Ok(Self {
            src_lines: count_rs_lines(resolve_workspace_path(Path::new(
                "crates/maos-kernel-core/src",
            ))?)?,
            abi_items: capture_live_abi_surface()?,
            host_items: capture_live_host_surface()?,
        })
    }

    /// Fault-injection helper: clone with a different src-line count. Proves
    /// the oracle detects kernel drift. Synthetic mutation around a real base.
    pub fn with_src_lines(&self, src_lines: usize) -> Self {
        let mut next = self.clone();
        next.src_lines = src_lines;
        next
    }

    /// Fault-injection helper: clone with one ABI item removed. Proves the
    /// oracle detects an ABI removal (breaks additive-only).
    pub fn without_abi_item(&self, item: &str) -> Self {
        let mut next = self.clone();
        next.abi_items.remove(item);
        next
    }

    /// Fault-injection helper: clone with an extra host item. Proves the
    /// oracle detects unauthorized host-surface growth (breaks the closed
    /// allowlist).
    pub fn with_extra_host_item(&self, item: &str) -> Self {
        let mut next = self.clone();
        next.host_items.insert(item.to_string());
        next
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgedSelfReport {
    pub kernel_unchanged: bool,
    pub abi_unchanged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FkcsReport {
    pub kernel_unchanged: bool,
    pub lines_before: usize,
    pub lines_after: usize,
    pub abi_additive_only: bool,
    pub host_closed_allowlist_holds: bool,
    pub ignored_self_report: bool,
}

pub struct FkcsOracle;

impl FkcsOracle {
    pub fn derive(
        before: &FkcsSurfaceSnapshot,
        after: &FkcsSurfaceSnapshot,
        self_report: Option<ForgedSelfReport>,
    ) -> FkcsReport {
        let line_stable = before.src_lines == after.src_lines;
        let abi_additive_only = before.abi_items.is_subset(&after.abi_items);
        let host_closed_allowlist_holds = after.host_items.is_subset(&before.host_items);
        FkcsReport {
            kernel_unchanged: line_stable && abi_additive_only && host_closed_allowlist_holds,
            lines_before: before.src_lines,
            lines_after: after.src_lines,
            abi_additive_only,
            host_closed_allowlist_holds,
            ignored_self_report: self_report.is_some(),
        }
    }

    /// Positive-derivation helper: a frozen surface compared against itself
    /// must derive a fully-green report (kernel_unchanged, abi additive-only,
    /// host closed-allowlist, no forged self-report). Exposed so tests assert
    /// the green path without constructing two captures.
    pub fn derive_positive(surface: &FkcsSurfaceSnapshot) -> FkcsReport {
        Self::derive(surface, surface, None)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    held_advisory_reason: Option<&'static str>,
    /// WHY this leg is RED, in the gate's own JSON.
    ///
    /// Added 2026-08-08 after CI run 31193312117: three legs went red in CI and
    /// green locally, and the gate's output carried no reason at all — so the
    /// failure could not be diagnosed from CI output, only guessed at. Several
    /// legs were discarding a perfectly good `Err(String)` with `Err(_)`. A
    /// control that cannot say why it fired is half a control.
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl LegResult {
    fn status_word(&self) -> &'static str {
        if self.green {
            "green"
        } else if self.attempted {
            "red"
        } else {
            "skipped"
        }
    }
}
/// The one admission-path mismatch currently held advisory.
///
/// The hold is fingerprint-bound: a malformed baseline, unreadable source,
/// changed file set, or any later content drift must block rather than inherit
/// Story 13.4's debt by sharing this leg's label.
const HELD_ADVISORY_ADMISSION_BASELINE_SHA256: &str =
    "dfbbf748707d8891edbbfcbdeacb5a55cf4ed83391cb614febe0f9012d2c6eb2";
const HELD_ADVISORY_ADMISSION_WORKTREE_SHA256: &str =
    "9ccc1399bb42568b89885df71dd574f6f2f2552eebcad8523f3ae1a14222bd51";
const HELD_ADVISORY_ADMISSION_REASON: &str =
    "RED since Story 13.4 (148a33ee) changed the admission path without \
     re-pinning `admission_baseline.sha256` in xtask/fkcs-baseline.toml; hidden \
     until Story 13.6e closed this gate's blocking_now/dev_blocks divergence. \
     Re-pinning is a frozen-kernel-conformance judgement on 13.4's change, not \
     a 13.6e drive-by. \
     OWNER: 14-3-ecosystem-readiness-verification-v2-5-graduation-ledger \
     (decision D8; deadline: before that story leaves `backlog`). \
     RE-HOMED 2026-08-26 by Story 14-0 AC5.3 — this string printed \
     `OWNER: Epic-13 retrospective` for months after the decision register had \
     already moved D8 to 14-3, and nobody told the gate. \"Owned by a \
     retrospective is not an owner\" is that register's own epigraph, and an \
     epic-level roll-up has no story file, no ACs and nobody the tracker can \
     page. \
     TRACKING: deferred-work.md, Story 13.6e section; \
     _bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md (D8).";

fn known_admission_hold(expected: &str, computed: &str) -> Option<&'static str> {
    (expected == HELD_ADVISORY_ADMISSION_BASELINE_SHA256
        && computed == HELD_ADVISORY_ADMISSION_WORKTREE_SHA256)
        .then_some(HELD_ADVISORY_ADMISSION_REASON)
}

pub fn run(json: bool) -> Result<(), String> {
    let disposition = read_disposition()?;
    if !matches!(
        disposition.get("v2_0").map(String::as_str),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be blocking"
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);
    // Option C (Epic 12 retro B1): hermetic gate — the Blocking binding class
    // hard-fails a RED oracle at HEAD regardless of CURRENT_PHASE. Dev-time
    // enforcement is decoupled from the GA ship-phase ladder (`blocking_now` is
    // retained for JSON reporting). See gate_common::BindingClass.
    let dev_blocks = blocking_now
        || crate::gate_common::dev_enforced_red_blocks(
            crate::gate_common::BindingClass::Blocking,
            true,
        );
    let legs = vec![
        run_frozen_tag_consistency_leg(),
        run_diff_oracle_derives_leg(),
        run_negative_control_leg(),
        run_proxy_cohort_leg(),
        run_fault_inject_falsifiers_leg(),
        run_admission_path_unmodified_leg(),
        run_release_graph_absence_leg(),
        run_kernel_abi_leg(),
    ];

    // Detect a vacuous leg (gate integrity: an attempted leg that did not run
    // or produced no pass/fail signal is a defect). When `--json` is set the
    // gate still emits exactly one JSON object so stdout stays parseable; a
    // vacuous leg turns into a non-zero exit AFTER the report is written.
    let vacuous = legs
        .iter()
        .find(|leg| leg.attempted && (!leg.ran || (leg.passed == 0 && leg.failed == 0)));

    let oracle_green = legs.iter().all(|leg| leg.green);
    let held: Vec<&LegResult> = legs
        .iter()
        .filter(|leg| !leg.green && leg.held_advisory_reason.is_some())
        .collect();
    let blocking_reds: Vec<&LegResult> = legs
        .iter()
        .filter(|leg| !leg.green && leg.held_advisory_reason.is_none())
        .collect();
    let gate_passed = vacuous.is_none() && (blocking_reds.is_empty() || !dev_blocks);

    // The hold is LOUD (governing rule, `13-6c…md:154`): banner, owner,
    // tracking entry. Never a silent pass, never a re-canned fixture.
    if !held.is_empty() {
        let detail = held
            .iter()
            .filter_map(|leg| {
                leg.held_advisory_reason
                    .map(|why| format!("- {}: {why}\n", leg.label))
            })
            .collect::<String>();
        let count = held.len();
        let banner = format!(
            "## ⚠️ FKCS Gate: WOULD HAVE BLOCKED — {count} leg(s) HELD ADVISORY\n\
             {detail}- Held, not fixed and not re-pinned. Every other RED leg blocks.\n"
        );
        crate::gate_common::emit_command(json, "warning", &banner.replace('\n', " "));
        eprintln!("{banner}");
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": gate_passed,
                "oracle_green": oracle_green,
                "advisory": !oracle_green && gate_passed,
                "blocking_now": blocking_now,
                "current_phase": CURRENT_PHASE,
                "disposition": disposition,
                "legs": legs,
                "vacuous_leg": vacuous.map(|leg| leg.label),
                "held_advisory_legs": held.iter().map(|leg| leg.label).collect::<Vec<_>>(),
                "blocking_red_legs": blocking_reds.iter().map(|leg| leg.label).collect::<Vec<_>>(),
            })
        );
    } else if let Some(leg) = vacuous {
        eprintln!(
            "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={})",
            leg.label, leg.ran, leg.passed, leg.failed
        );
    } else if oracle_green {
        eprintln!("{GATE_NAME}: PASSED — oracle green ({} legs)", legs.len());
    } else {
        eprintln!(
            "{GATE_NAME}: {} ({} held advisory); {}",
            if blocking_reds.is_empty() {
                "PASS — every RED leg is a named, tracked hold"
            } else {
                "RED"
            },
            held.len(),
            legs.iter()
                .map(|leg| format!("{}={}", leg.label, leg.status_word()))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if let Some(leg) = vacuous {
        return Err(format!(
            "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={})",
            leg.label, leg.ran, leg.passed, leg.failed
        ));
    }
    // Story 13.6e (AC2): this gated on `blocking_now` while `gate_passed` above
    // gated on `dev_blocks`, so a RED oracle printed `"passed": false` in JSON
    // and exited 0 — the same one-identifier defect as D-2's two vacuity
    // mechanisms, on a hermetic gate. One identifier, closed here.
    if !blocking_reds.is_empty() && dev_blocks {
        // Print WHY, not just WHICH. CI run 31193312117 reported three red legs
        // with no reason attached, so the failure could only be guessed at from
        // outside; the reasons existed and were being discarded.
        return Err(format!(
            "{GATE_NAME}: BLOCKING — oracle RED: {}\n{}",
            blocking_reds
                .iter()
                .map(|leg| leg.label)
                .collect::<Vec<_>>()
                .join(", "),
            blocking_reds
                .iter()
                .map(|leg| format!(
                    "  - {}: {}",
                    leg.label,
                    leg.detail.as_deref().unwrap_or("(no detail recorded)")
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    Ok(())
}

fn run_frozen_tag_consistency_leg() -> LegResult {
    let outcome = frozen_tag_consistency();
    let green = outcome.is_ok();
    LegResult {
        label: "frozen-tag-consistency",
        passed: u32::from(green),
        failed: u32::from(!green),
        ran: true,
        attempted: true,
        green,
        held_advisory_reason: None,
        detail: outcome.err(),
    }
}

fn run_diff_oracle_derives_leg() -> LegResult {
    // GREEN path: capture the REAL current surfaces and compare the snapshot
    // against itself. A frozen surface must derive kernel_unchanged=true, and
    // a forged (contradictory) self-report must be ignored, not trusted. No
    // hardcoded literals — `real_surface` spawns cargo-public-api.
    let (green, ran, detail) = match real_surface() {
        Ok(surface) => {
            let positive = FkcsOracle::derive_positive(&surface);
            let forged = FkcsOracle::derive(
                &surface,
                &surface,
                Some(ForgedSelfReport {
                    kernel_unchanged: false,
                    abi_unchanged: false,
                }),
            );
            let green =
                positive.kernel_unchanged && forged.kernel_unchanged && forged.ignored_self_report;
            let detail = (!green).then(|| {
                format!(
                    "oracle derivation disagreed: positive.kernel_unchanged={}, \
                     forged.kernel_unchanged={}, forged.ignored_self_report={}",
                    positive.kernel_unchanged, forged.kernel_unchanged, forged.ignored_self_report
                )
            });
            (green, true, detail)
        }
        Err(error) => (
            false,
            true,
            Some(format!("surface capture failed: {error}")),
        ),
    };
    LegResult {
        label: "diff-oracle-derives",
        passed: u32::from(green),
        failed: u32::from(!green),
        ran,
        attempted: true,
        green,
        held_advisory_reason: None,
        detail,
    }
}

fn run_negative_control_leg() -> LegResult {
    cargo_test_leg(
        "negative-control-rejects",
        "maos-fkcs",
        "fkcs_contract",
        "negative_control_rejects",
    )
}

fn run_proxy_cohort_leg() -> LegResult {
    cargo_test_leg(
        "proxy-cohort-fkcs-score",
        "maos-fkcs",
        "fkcs_contract",
        "cohort_green",
    )
}

fn run_fault_inject_falsifiers_leg() -> LegResult {
    // RED falsifiers: take the REAL captured baseline and apply synthetic
    // mutations (line drift, ABI removal, host growth). The oracle MUST red
    // each one. Synthetic mutation around a real captured baseline — not a
    // hardcoded synthetic snapshot.
    let (green, ran, detail) = match real_surface() {
        Ok(base) => {
            let kernel_fault = base.with_src_lines(base.src_lines + 1);
            let abi_fault = match base.abi_items.iter().next().cloned() {
                Some(item) => base.without_abi_item(&item),
                None => base.clone(),
            };
            let host_fault = base.with_extra_host_item("maos_host::UnauthorizedSurface");
            let kernel_red = !FkcsOracle::derive(&base, &kernel_fault, None).kernel_unchanged;
            let abi_red = !FkcsOracle::derive(&base, &abi_fault, None).kernel_unchanged;
            let host_red = !FkcsOracle::derive(&base, &host_fault, None).kernel_unchanged;
            let green = kernel_red && abi_red && host_red;
            // Name the falsifier that failed to fire — a falsifier that does not
            // red on its own planted fault is the null control this leg exists
            // to prevent, and "false" alone does not say which one.
            let detail = (!green).then(|| {
                format!(
                    "falsifier(s) did not fire: kernel_drift_red={kernel_red}, \
                     abi_removal_red={abi_red}, host_growth_red={host_red}"
                )
            });
            (green, true, detail)
        }
        Err(error) => (
            false,
            true,
            Some(format!("surface capture failed: {error}")),
        ),
    };
    LegResult {
        label: "fault-inject-falsifiers",
        passed: u32::from(green),
        failed: u32::from(!green),
        ran,
        attempted: true,
        green,
        held_advisory_reason: None,
        detail,
    }
}

fn run_admission_path_unmodified_leg() -> LegResult {
    // Content semantics (literal AC3): the admission path is pinned by SHA-256
    // over the declared source files. Only the exact, recorded Story 13.4
    // baseline/current mismatch is held; every other failure blocks.
    let (green, held_advisory_reason, detail) = match FkcsBaseline::load_from_file(BASELINE_FILE) {
        Ok(baseline) => match admission_path_observed_hash(&baseline) {
            Ok(computed) => {
                let expected = baseline.admission_baseline.sha256.as_str();
                let green = computed == expected;
                let detail = (!green).then(|| {
                    format!("admission-path SHA-256 mismatch: baseline pins {expected}, worktree computes {computed}")
                });
                (
                    green,
                    known_admission_hold(expected, computed.as_str()),
                    detail,
                )
            }
            Err(error) => (
                false,
                None,
                Some(format!("cannot hash the admission path: {error}")),
            ),
        },
        Err(error) => (
            false,
            None,
            Some(format!("cannot load {BASELINE_FILE}: {error}")),
        ),
    };
    LegResult {
        label: "admission-path-unmodified",
        passed: u32::from(green),
        failed: u32::from(!green),
        ran: true,
        attempted: true,
        green,
        held_advisory_reason,
        detail,
    }
}

fn run_release_graph_absence_leg() -> LegResult {
    let output = Command::new("cargo")
        .args(["tree", "-p", "maos-bin", "--edges", "normal"])
        .output();
    let mut detail = None;
    let green = match output {
        Ok(out) if out.status.success() => {
            let tree = String::from_utf8_lossy(&out.stdout);
            // Exact crate-name match: strip cargo-tree drawing chars, take the
            // first whitespace token (the crate name), compare exactly to
            // `maos-fkcs`. A substring `contains` would false-positive on
            // `maos-fkcs-tests` or a path comment.
            !tree.lines().any(|line| {
                let stripped =
                    line.trim_start_matches(|c: char| c.is_whitespace() || "│├└─".contains(c));
                stripped
                    .split_whitespace()
                    .next()
                    .map(|name| name == "maos-fkcs")
                    .unwrap_or(false)
            })
        }
        Ok(out) => {
            detail = Some(format!(
                "`cargo tree -p maos-bin` exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
            false
        }
        Err(error) => {
            detail = Some(format!("cannot spawn `cargo tree`: {error}"));
            false
        }
    };
    if !green && detail.is_none() {
        detail = Some(
            "dev-only fixture crate `maos-fkcs` is reachable from the `maos-bin` \
             release dependency graph"
                .to_string(),
        );
    }
    LegResult {
        label: "release-graph-absence",
        passed: u32::from(green),
        failed: u32::from(!green),
        ran: true,
        attempted: true,
        green,
        held_advisory_reason: None,
        detail,
    }
}

fn run_kernel_abi_leg() -> LegResult {
    let outcome = crate::check_kernel_baseline::check();
    let green = outcome
        .as_ref()
        .map(|report| report.passed)
        .unwrap_or(false);
    let detail = match &outcome {
        Ok(report) if !report.passed => Some(format!(
            "kernel-core line count drifted: {} pins {}, live {}",
            report.baseline_file, report.pinned_lines, report.actual_lines
        )),
        Ok(_) => None,
        Err(error) => Some(format!("kernel baseline check failed: {error}")),
    };
    LegResult {
        label: "kernel-abi-diff",
        passed: u32::from(green),
        failed: u32::from(!green),
        ran: true,
        attempted: true,
        green,
        held_advisory_reason: None,
        detail,
    }
}

fn cargo_test_leg(label: &'static str, pkg: &str, test_file: &str, filter: &str) -> LegResult {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "test",
        "--locked",
        "-p",
        pkg,
        "--test",
        test_file,
        "--",
        filter,
        "--nocapture",
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output();
    let (passed, failed, ran, green, detail) = match output {
        Ok(out) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let (passed, failed) = parse_test_summary(&combined);
            let ran = combined
                .lines()
                .any(|line| line.trim().starts_with("test result:"));
            let green = out.status.success() && ran && passed >= 1 && failed == 0;
            // On RED keep the tail of the real cargo transcript: the assertion
            // that fired is the only thing that explains the leg. Bounded so a
            // runaway build log cannot swamp the gate's JSON.
            let detail = (!green).then(|| {
                let tail: Vec<&str> = combined.lines().rev().take(20).collect();
                let tail = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
                format!(
                    "cargo test -p {pkg} --test {test_file} -- {filter} => \
                     exit={:?}, ran={ran}, passed={passed}, failed={failed}\n{tail}",
                    out.status.code()
                )
            });
            (passed, failed, ran, green, detail)
        }
        Err(error) => (
            0,
            1,
            true,
            false,
            Some(format!("cannot spawn cargo test for `{label}`: {error}")),
        ),
    };
    LegResult {
        label,
        passed,
        failed,
        ran,
        attempted: true,
        green,
        held_advisory_reason: None,
        detail,
    }
}

fn frozen_tag_consistency() -> Result<(), String> {
    let baseline = FkcsBaseline::load_from_file(BASELINE_FILE)?;
    baseline.validate_live_triple()?;
    // Annotated tag (tag object), not a lightweight ref.
    let tag_type = git(["cat-file", "-t", &baseline.frozen_tag])?;
    if tag_type.trim() != "tag" {
        return Err(format!(
            "{} is not an annotated tag (`git cat-file -t` = {}); freeze requires an annotated tag object",
            baseline.frozen_tag,
            tag_type.trim()
        ));
    }
    // Tag commit matches the pinned freeze.
    let tag_commit = git(["rev-list", "-n", "1", &baseline.frozen_tag])?;
    if tag_commit.trim() != baseline.frozen_commit {
        return Err(format!(
            "{} points at {}, but fkcs-baseline.toml pins {}",
            baseline.frozen_tag,
            tag_commit.trim(),
            baseline.frozen_commit
        ));
    }
    // Reachable from HEAD (frozen commit is an ancestor of HEAD).
    if !commit_reachable_from_head(tag_commit.trim()) {
        return Err(format!(
            "frozen commit {} ({}) is not reachable from HEAD",
            baseline.frozen_tag,
            tag_commit.trim()
        ));
    }
    baseline.validate_frozen_tag_src_lines()?;
    Ok(())
}

fn git<const N: usize>(args: [&str; N]) -> Result<String, String> {
    let out = Command::new("git")
        .current_dir(workspace_root()?)
        .args(args)
        .output()
        .map_err(|e| format!("git invocation failed: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn read_disposition() -> Result<HashMap<String, String>, String> {
    let raw = fs::read_to_string(resolve_workspace_path(Path::new(
        "xtask/gate-registry.toml",
    ))?)
    .map_err(|e| format!("cannot read gate-registry.toml: {e}"))?;
    // Bound the search to the [[ship_gate]] stanza whose `name = "<GATE>"`
    // matches. A stanza opens at a `[[`/`[` table header; the scan resets at
    // every header so a LATER gate's disposition can never be mis-attributed.
    let mut in_target_stanza = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[") || trimmed.starts_with('[') {
            in_target_stanza = false;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name =") {
            in_target_stanza = rest.trim().trim_matches('"') == GATE_NAME;
            continue;
        }
        if in_target_stanza && trimmed.starts_with("disposition =") {
            return parse_inline_disposition(trimmed);
        }
    }
    Err(format!(
        "{GATE_NAME} [[ship_gate]] disposition row not found"
    ))
}

pub fn parse_inline_disposition(line: &str) -> Result<HashMap<String, String>, String> {
    let start = line.find('{').ok_or("disposition missing `{`")?;
    let end = line.rfind('}').ok_or("disposition missing `}`")?;
    let mut out = HashMap::new();
    for part in line[start + 1..end].split(',') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        out.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
    }
    Ok(out)
}

pub fn phase_disposition<'a>(
    disposition: &'a HashMap<String, String>,
    phase: &str,
) -> Option<&'a str> {
    let idx = PHASE_ORDER.iter().position(|p| *p == phase)?;
    for i in (0..=idx).rev() {
        if let Some(d) = disposition.get(PHASE_ORDER[i]) {
            return Some(d.as_str());
        }
    }
    None
}

pub fn is_blocking_at(disposition: &HashMap<String, String>, phase: &str) -> bool {
    matches!(
        phase_disposition(disposition, phase),
        Some("blocking") | Some("blocking-when-present")
    )
}

pub fn read_nonempty_lines(path: &str) -> Result<BTreeSet<String>, String> {
    let resolved = resolve_workspace_path(Path::new(path))?;
    let raw = fs::read_to_string(&resolved)
        .map_err(|e| format!("failed to read {}: {e}", resolved.display()))?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn count_rs_lines(dir: impl AsRef<Path>) -> Result<usize, String> {
    fn walk(path: &Path, total: &mut usize) -> Result<(), String> {
        for entry in fs::read_dir(path).map_err(|e| format!("read_dir {}: {e}", path.display()))? {
            let entry = entry.map_err(|e| format!("dir entry {}: {e}", path.display()))?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, total)?;
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let raw = fs::read_to_string(&path)
                    .map_err(|e| format!("read {}: {e}", path.display()))?;
                *total += raw.lines().count();
            }
        }
        Ok(())
    }
    let mut total = 0;
    walk(dir.as_ref(), &mut total)?;
    Ok(total)
}

fn workspace_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    cwd.ancestors()
        .find(|ancestor| ancestor.join("crates/maos-kernel-core/src").is_dir())
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("failed to find workspace root from {}", cwd.display()))
}

fn resolve_workspace_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return Ok(path.to_path_buf());
    }
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    for ancestor in cwd.ancestors() {
        let candidate = ancestor.join(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "failed to resolve {} from {}",
        path.display(),
        cwd.display()
    ))
}

/// Load the frozen baseline and capture the REAL current surfaces. Shared by
/// the diff-oracle (green) and fault-inject (red) legs so both measure the
/// same real baseline.
fn real_surface() -> Result<FkcsSurfaceSnapshot, String> {
    let baseline = FkcsBaseline::load_from_file(BASELINE_FILE)?;
    FkcsSurfaceSnapshot::capture_from_baselines(&baseline)
}

/// Capture the live ABI surface via `cargo public-api` over `maos-spirit-abi`.
fn capture_live_abi_surface() -> Result<BTreeSet<String>, String> {
    parse_surface_lines(&crate::abi_diff::capture_public_api()?)
}

/// Capture the live host surface via `cargo public-api` over `maos-host`.
fn capture_live_host_surface() -> Result<BTreeSet<String>, String> {
    parse_surface_lines(&crate::check_host_surface::capture_current_surface()?)
}

fn parse_surface_lines(raw: &str) -> Result<BTreeSet<String>, String> {
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn admission_path_observed_hash(baseline: &FkcsBaseline) -> Result<String, String> {
    let files = &baseline.admission_baseline.files;
    if files.is_empty() {
        return Err("admission baseline declares no files".into());
    }
    admission_content_hash(files)
}

/// SHA-256 over the path-stamped concatenation of the declared admission files.
pub fn admission_content_hash(files: &[String]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for file in files {
        let path = resolve_workspace_path(Path::new(file))?;
        let content =
            fs::read(&path).map_err(|e| format!("read admission file {}: {e}", path.display()))?;
        hasher.update(file.as_bytes());
        hasher.update(b"\0");
        hasher.update(&(content.len() as u64).to_le_bytes());
        hasher.update(&content);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// True iff `commit` is reachable from HEAD (an ancestor of HEAD).
fn commit_reachable_from_head(commit: &str) -> bool {
    Command::new("git")
        .args(["merge-base", "--is-ancestor", commit, "HEAD"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn parse_test_summary(output: &str) -> (u32, u32) {
    for line in output.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("test result:") {
            return (
                parse_count(trimmed, "passed"),
                parse_count(trimmed, "failed"),
            );
        }
    }
    (0, 0)
}

fn parse_count(s: &str, key: &str) -> u32 {
    let needle = format!(" {key}");
    let Some(pos) = s.find(&needle) else { return 0 };
    s[..pos]
        .split_whitespace()
        .last()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod review_tests {
    use super::*;

    #[test]
    fn advisory_hold_matches_only_the_recorded_admission_fingerprint() {
        assert_eq!(
            known_admission_hold(
                HELD_ADVISORY_ADMISSION_BASELINE_SHA256,
                HELD_ADVISORY_ADMISSION_WORKTREE_SHA256,
            ),
            Some(HELD_ADVISORY_ADMISSION_REASON)
        );
        assert_eq!(
            known_admission_hold(
                HELD_ADVISORY_ADMISSION_BASELINE_SHA256,
                "later-admission-drift",
            ),
            None
        );
        assert_eq!(
            known_admission_hold(
                "malformed-or-repinned-baseline",
                HELD_ADVISORY_ADMISSION_WORKTREE_SHA256,
            ),
            None
        );
    }
}
