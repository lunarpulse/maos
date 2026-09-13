#![forbid(unsafe_code)]

//! Story 11.4b (AC5) — `check-escape-detector` gate.
//!
//! ONE standalone gate cloned from `check_scale_churn.rs` / `check_enterprise_pdp.rs`,
//! with **per-leg verdict independence** — each leg reads its OWN oracle
//! invocation, so one break reds exactly one leg (Murat's one-break-one-red).
//!
//! # The legs (each its own oracle invocation)
//!
//! 1. **no-verdict-invariant** (AC2) — `no_verdict_invariant.rs`: the kernel
//!    sandbox-violation emission type carries NO `malice`/`verdict`/`severity`/
//!    `intent` field. Adding one reds the leg (structural-not-semantic).
//! 2. **out-of-kernel-boundary** (AC1) — the detector crate's NORMAL dependency
//!    closure excludes `maos-kernel-core` (the Story-1a.4 rule; `cargo tree
//!    --edges normal`). A dev-dep is allowed (the maos-audit template) but a
//!    library dep reds.
//! 3. **detection-quality** (AC4) — `correlation_quality_on_structural_rows`:
//!    TP-floor + FP-ceiling measured on the DETECTOR's correlation decision
//!    over real Kernel-origin `SandboxBlock` rows. Emits the MEASURED marker →
//!    green at HEAD on every host (the correlation logic is host-independent).
//! 4. **detection-quality-falsifier** (AC4 anti-canned) —
//!    `fault_inject_stubs_detector_to_canned_tp` (`escape-fault-inject`,
//!    `--ignored`): the stub anomalizes an anticipated kill → FP-avoidance
//!    broke, proving the green metric is the REAL correlation, not a constant.
//! 5. **escape-source-identity-blind** (AC4) — `source_identity_blind.rs`: each
//!    anomaly traces to a real kernel `frame_id`; a synthesized/non-kernel
//!    source is excluded; a replayed read does not inflate the count.
//! 6. **kernel-abi-diff** (AC1/AC5) — `check-kernel-baseline` GREEN @ 23081
//!    (ZERO kernel-core delta — the producer is edge-wired, CATCH-0).
//! 7. **release-graph-absence** (ship-blocker) — `escape-fault-inject` is a
//!    `compile_error!`-guarded dev/CI-only feature: a release build WITH it
//!    MUST fail (the guard fires).
//! 8. **producer-wired-proven-red** (AC3) — the REAL launcher reap: a real
//!    seccomp kill produces a real kind=8 `SandboxBlock` TL row (GREEN,
//!    `real_seccomp_kill_produces_real_tl_row_and_detector_anomaly`) AND the
//!    `escape-fault-inject` falsifier severs `emit_sandbox_block` → NO row
//!    (§A7.3 contrast, `fault_inject_severs_producer_wiring_to_no_row`). Both
//!    marker-gated; a silent seccomp-unavailable skip cannot pass vacuously.
//! 9. **detection-quality-live** (AC4) — the correlation TP-floor/FP-ceiling
//!    measured LIVE over REAL seccomp children (no mock;
//!    `detection_quality_meets_floor_and_ceiling_on_real_seccomp`), the
//!    enforcement tripwire complementing the host-independent leg 3.
//!    Legs 8/9 are advisory on hosts whose kernel blocks seccomp
//!    (environment-unavailable, not a regression); a real per-commit tripwire
//!    on seccomp-capable runners (CI ubuntu-latest).
//!
//! # Phase disposition
//!
//! Advisory at v1.0/v1.5 (a RED oracle emits a WOULD-HAVE-BLOCKED banner but
//! does not fail the aggregate); blocking at v2.0 (AC5 / F6).

use crate::gate_common::{
    dev_enforced_red_blocks, emit_command, is_blocking_at, BindingClass, CURRENT_PHASE,
};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
const GATE_NAME: &str = "check-escape-detector";

fn read_disposition() -> Result<HashMap<String, String>, String> {
    let registry_path = Path::new("xtask/gate-registry.toml");
    let registry: crate::corpus_types::ShipGateRegistry =
        crate::corpus_types::load_toml(registry_path)
            .map_err(|e| format!("cannot read gate-registry.toml: {e}"))?;
    for entry in &registry.ship_gates {
        if entry.name == GATE_NAME {
            if entry.disposition.is_empty() {
                return Err(format!("{GATE_NAME} has an empty disposition"));
            }
            return Ok(entry.disposition.clone());
        }
    }
    Err(format!("{GATE_NAME} not found in gate-registry.toml"))
}

/// Does this host's kernel support seccomp *filtering*?
///
/// `/proc/sys/kernel/seccomp/actions_avail` is materialised only when the kernel
/// carries `CONFIG_SECCOMP_FILTER` — the substrate legs 8 and 9 need. A file
/// probe rather than a `prctl` because this module is `#![forbid(unsafe_code)]`.
///
/// This is a KERNEL-capability fact, and it is NOT sufficient on its own: a
/// container can ship a filtering-capable kernel and still refuse `seccomp(2)`
/// by policy. Measured on the 15-3 dev host: `actions_avail` lists eight
/// actions and `/proc/self/status` reports `Seccomp_filters: 1`, yet a real
/// sandboxed spawn fails `PermissionDenied`. So this probe is reported as
/// CONTEXT (`seccomp_kernel_filtering`) and [`observed_substrate_skip`] decides.
fn seccomp_kernel_filtering() -> bool {
    Path::new("/proc/sys/kernel/seccomp/actions_avail").exists()
}

/// Did the harness observe that this host GENUINELY cannot sandbox?
///
/// `maos-escape-detector`'s `tests/common::skip_if_sandbox_unavailable` prints
/// `SKIP <test>: sandbox unavailable on this host` or `SKIP <test>: sandbox
/// spawn refused by host (…)` for `SandboxUnavailable` / `EPERM` / `ENOSYS`
/// only; every other error panics. That line is therefore an authoritative
/// substrate verdict backed by a REAL spawn attempt against the real kernel —
/// the most real probe available, and strictly stronger than a config read.
///
/// Its own doc records why honouring it cannot mask a capable-host regression:
/// a missing kill surfaces as no marker and a RED leg, never as a spawn
/// refusal, so it never reaches that helper.
///
/// Separating this from `green` is the whole of F3. Before story 15-3
/// [`invoke_cargo_test_marker`] collapsed "seccomp is unavailable here" and
/// "the detector regressed" into a single `green=false` — which is D20's
/// recorded defect: `passed==0 && failed==0` is false on a seccomp-blocked
/// host, so the advisory tail converted a RED oracle into `passed: true`.
fn observed_substrate_skip(output: &str) -> bool {
    output.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("SKIP ")
            && (trimmed.contains("sandbox unavailable")
                || trimmed.contains("sandbox spawn refused"))
    })
}

fn write_step_summary(text: &str) {
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| {
                use std::io::Write;
                write!(f, "{text}")
            });
    }
}

/// One oracle leg's parsed result, carrying its dev-time enforcement class and
/// whether the substrate that leg needs was actually available.
///
/// The two facts travel WITH the counts rather than being folded into `green`
/// (F3; the shape is `check_multi_region_slo`'s `RawLeg`), because a RED leg on
/// a host that cannot sandbox and a RED leg on a host that can are different
/// events and only one of them is a regression.
struct LegResult {
    label: &'static str,
    class: BindingClass,
    substrate_present: bool,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
}

impl LegResult {
    /// A hermetic leg: CI can run it on any host, so its RED is unconditional
    /// ([`BindingClass::Blocking`]) and the substrate question does not arise.
    fn hermetic(label: &'static str, passed: u32, failed: u32, ran: bool, green: bool) -> Self {
        LegResult {
            label,
            class: BindingClass::Blocking,
            substrate_present: true,
            passed,
            failed,
            ran,
            attempted: true,
            green,
        }
    }

    /// A hermetic leg whose oracle is a single boolean rather than a test count.
    fn hermetic_bool(label: &'static str, green: bool) -> Self {
        Self::hermetic(label, u32::from(green), u32::from(!green), true, green)
    }

    /// A leg that needs a seccomp-capable host: [`BindingClass::AdvisorySubstrate`],
    /// so a RED blocks only when the substrate was actually there.
    fn substrate(label: &'static str, run: &MarkerRun) -> Self {
        LegResult {
            label,
            class: BindingClass::AdvisorySubstrate,
            substrate_present: !run.substrate_skip,
            passed: run.passed,
            failed: run.failed,
            ran: run.ran,
            attempted: true,
            green: run.green,
        }
    }

    fn status_word(&self) -> &'static str {
        if self.green {
            "green"
        } else if !self.substrate_present {
            "substrate-absent"
        } else if self.attempted {
            "red"
        } else {
            "skipped"
        }
    }

    /// Whether this leg's RED must hard-fail CI.
    ///
    /// The three-way split F3 requires: a green leg never blocks; a RED
    /// `Blocking` leg always blocks; a RED `AdvisorySubstrate` leg blocks only
    /// when its substrate was present — never silent-green, the caller emits a
    /// WOULD-HAVE-BLOCKED banner for the absent case.
    fn blocks(&self) -> bool {
        !self.green && dev_enforced_red_blocks(self.class, self.substrate_present)
    }
}

/// Invoke a filtered `cargo test -p <pkg> --test <file>` and parse its
/// `test result:` summary. PER-LEG INDEPENDENCE: each leg calls this with its
/// OWN package/file/filter (+ optional feature).
fn invoke_cargo_test(
    pkg: &str,
    test_file: &str,
    name_filter: &str,
    features: Option<&str>,
    ignored: bool,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", pkg, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    cmd.arg("--");
    cmd.arg(name_filter);
    if ignored {
        cmd.arg("--ignored");
    }
    cmd.arg("--nocapture");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({pkg}/{test_file}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let green = output.status.success() && ran && passed >= 1 && failed == 0;
    if !green {
        let tail: String = combined
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "{GATE_NAME}: {pkg}/{test_file} (filter={name_filter:?}, features={features:?}, \
             ignored={ignored}) NOT green (passed={passed}, failed={failed}, ran={ran}, \
             exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// A marker-gated leg's raw observation.
///
/// `substrate_skip` is reported SEPARATELY from `green` so the caller can tell
/// "this host cannot sandbox" from "the detector regressed" — the collapse F3
/// names as the defect.
struct MarkerRun {
    passed: u32,
    failed: u32,
    ran: bool,
    green: bool,
    substrate_skip: bool,
}

impl MarkerRun {
    /// The gate could not even invoke `cargo test`. That is a RED with the
    /// substrate presumed PRESENT: a broken invocation must never be laundered
    /// into an environment excuse.
    fn invocation_failed() -> Self {
        MarkerRun {
            passed: 0,
            failed: 1,
            ran: true,
            green: false,
            substrate_skip: false,
        }
    }
}

/// Invoke a filtered `cargo test` and require a measurement marker in the
/// output. GREEN requires the test to pass AND the marker to be present — a
/// silent skip (e.g. seccomp unavailable on the host) emits no marker, so the
/// leg cannot pass vacuously (the anti-canned discipline).
///
/// The absent-marker case is additionally classified: when the harness printed
/// its genuine-unavailability `SKIP` line the substrate was absent, and the
/// caller keeps the leg advisory instead of reding CI on a host that cannot
/// run it (F3).
fn invoke_cargo_test_marker(
    pkg: &str,
    test_file: &str,
    name_filter: &str,
    marker: &str,
    features: Option<&str>,
    ignored: bool,
) -> Result<MarkerRun, String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", pkg, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    cmd.arg("--");
    cmd.arg(name_filter);
    if ignored {
        cmd.arg("--ignored");
    }
    cmd.arg("--nocapture");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({pkg}/{test_file}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let measured = combined.contains(marker);
    // GREEN requires a REAL measurement (the marker) — not a silent skip.
    let green = output.status.success() && ran && passed >= 1 && failed == 0 && measured;
    let substrate_skip = !green && observed_substrate_skip(&combined);
    if !green {
        eprintln!(
            "{GATE_NAME}: {pkg}/{test_file} (filter={name_filter:?}) NOT green (passed={passed}, \
             failed={failed}, ran={ran}, measured={measured}, substrate_skip={substrate_skip}, \
             kernel_seccomp_filtering={}, exit={}). A silent seccomp-unavailable skip emits no \
             `{marker}` marker; when the harness reports genuine unavailability the leg is \
             advisory, otherwise this is a real RED.",
            seccomp_kernel_filtering(),
            output.status
        );
    }
    Ok(MarkerRun {
        passed,
        failed,
        ran,
        green,
        substrate_skip,
    })
}

// ────────────────────────────── per-leg oracles ──────────────────────────────

/// Leg 1: no-verdict-invariant (AC2).
fn run_no_verdict_invariant_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test(
        "maos-escape-detector",
        "no_verdict_invariant",
        "",
        None,
        false,
    )
    .unwrap_or((0, 1, true, false));
    LegResult::hermetic("no-verdict-invariant", p, f, r, g)
}

/// Leg 2: out-of-kernel-boundary (AC1). The detector's NORMAL dependency
/// closure must exclude `maos-kernel-core` (a dev-dep is allowed; a lib dep reds).
fn run_out_of_kernel_boundary_leg() -> LegResult {
    let output = Command::new("cargo")
        .args(["tree", "-p", "maos-escape-detector", "--edges", "normal"])
        .output();
    let green = match output {
        Ok(o) if o.status.success() => {
            let tree = String::from_utf8_lossy(&o.stdout);
            // The detector's library closure must not contain maos-kernel-core.
            // `--edges normal` excludes dev-deps, so a dev-dep on kernel-core
            // (allowed, the maos-audit template) does NOT trip this.
            !tree.contains("maos-kernel-core")
        }
        _ => false,
    };
    if !green {
        eprintln!(
            "{GATE_NAME}: out-of-kernel-boundary leg RED — maos-kernel-core present in the detector's normal closure"
        );
    }
    LegResult::hermetic_bool("out-of-kernel-boundary", green)
}

/// Leg 3: detection-quality on the correlation decision (AC4). Runs the
/// correlation test (real Kernel-origin rows + manifest correlation) — green at
/// HEAD on every host (the correlation logic is host-independent).
fn run_detection_quality_leg() -> LegResult {
    let run = invoke_cargo_test_marker(
        "maos-escape-detector",
        "detection_quality",
        "correlation_quality_on_structural_rows",
        "ESCAPE-DETECTOR-QUALITY-MEASURED",
        None,
        false,
    )
    .unwrap_or_else(|_| MarkerRun::invocation_failed());
    // Hermetic despite the marker gate: the correlation logic is
    // host-independent, so this leg is green at HEAD on every host.
    LegResult::hermetic(
        "detection-quality",
        run.passed,
        run.failed,
        run.ran,
        run.green,
    )
}

/// Leg 4: detection-quality falsifier (AC4 anti-canned). The `escape-fault-inject`
/// stub anomalizes an anticipated kill → FP-avoidance broke, proving the green
/// metric is the REAL correlation.
fn run_detection_quality_falsifier_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test(
        "maos-escape-detector",
        "detection_quality",
        "fault_inject_stubs_detector_to_canned_tp",
        Some("escape-fault-inject"),
        true,
    )
    .unwrap_or((0, 1, true, false));
    LegResult::hermetic("detection-quality-falsifier", p, f, r, g)
}

/// Leg 5: escape-source-identity-blind + replay-dedup (AC4).
fn run_source_identity_blind_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test(
        "maos-escape-detector",
        "source_identity_blind",
        "",
        None,
        false,
    )
    .unwrap_or((0, 1, true, false));
    LegResult::hermetic("escape-source-identity-blind", p, f, r, g)
}

/// Leg 6: kernel-ABI baseline — ZERO kernel-core delta @ 23081 (CATCH-0).
fn run_kernel_abi_leg() -> LegResult {
    let kernel = crate::check_kernel_baseline::check();
    let green = kernel.as_ref().is_ok_and(|report| report.passed);
    if !green {
        // `run(false)` carried the diagnosis on stderr; `check()` is silent,
        // and a red leg whose only record is a constant names nothing
        // (Story 16-0 review). stderr only — stdout belongs to `--json`.
        let diagnosis = match &kernel {
            Ok(report) => crate::check_kernel_baseline::failure_detail(report),
            Err(error) => format!("kernel baseline check errored: {error}"),
        };
        eprintln!("kernel-abi-diff RED — {diagnosis}");
    }
    LegResult::hermetic_bool("kernel-abi-diff", green)
}

/// Leg 7: release-graph-absence (ship-blocker). `escape-fault-inject` is a
/// `compile_error!`-guarded dev/CI-only feature. A release build WITH it MUST
/// fail (the guard fires). GREEN = the build errored citing the feature.
fn run_release_graph_absence_leg() -> LegResult {
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "maos-escape-detector",
            "--features",
            "escape-fault-inject",
        ])
        .output();
    let (green, note) = match output {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let fired = !o.status.success() && stderr.contains("escape-fault-inject");
            (
                fired,
                if fired {
                    "compile_error fired (ship-blocker OK)"
                } else {
                    "release build did NOT fail with the escape-fault-inject compile_error — ship-blocker BROKEN"
                },
            )
        }
        Err(_) => (false, "failed to invoke cargo build"),
    };
    if green {
        eprintln!("{GATE_NAME}: release-graph-absence leg green — {note}");
    } else {
        eprintln!("{GATE_NAME}: release-graph-absence leg RED — {note}");
    }
    LegResult::hermetic_bool("release-graph-absence", green)
}

/// Leg 8: producer-wired-proven-red (AC3). The REAL launcher reap producing a
/// real kind=8 TL row (GREEN direction) AND the `escape-fault-inject` falsifier
/// that severs `emit_sandbox_block` → NO row (RED direction) — the §A7.3
/// contrast proving the row comes from the real wiring, not a canned fixture.
/// Both sub-invocations are MARKER-gated: a silent seccomp-unavailable skip
/// emits no marker, so the leg cannot pass vacuously. Advisory on hosts whose
/// kernel blocks seccomp; a real per-commit tripwire on seccomp-capable runners.
fn run_producer_wired_proven_red_leg() -> LegResult {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut ran = false;
    let mut green = true;
    // Substrate absence is a leg-wide claim: BOTH independent sub-invocations
    // must explicitly report genuine seccomp unavailability. One unavailable
    // run cannot launder the other run's ordinary failure into an environment
    // excuse. An invocation error likewise forces this false.
    let mut substrate_skip = true;
    // Sub A — GREEN: a real seccomp kill produces a real SandboxBlock TL row.
    match invoke_cargo_test_marker(
        "maos-escape-detector",
        "producer_wired_e2e",
        "real_seccomp_kill_produces_real_tl_row_and_detector_anomaly",
        "ESCAPE-PRODUCER-WIRED-MEASURED",
        None,
        false,
    ) {
        Ok(run) => {
            passed += run.passed;
            failed += run.failed;
            ran |= run.ran;
            green &= run.green;
            substrate_skip &= run.substrate_skip;
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: producer-wired leg error (green producer): {e}");
            failed += 1;
            ran = true;
            green = false;
            substrate_skip = false;
        }
    }
    // Sub B — RED direction (AC3 falsifier, §A7.3): `escape-fault-inject` severs
    // the emit, so the SAME real kill produces NO row. Marker emitted only when
    // a real kill was reaped-and-severed (a genuine contrast vs. Sub A).
    match invoke_cargo_test_marker(
        "maos-escape-detector",
        "producer_wired_e2e",
        "fault_inject_severs_producer_wiring_to_no_row",
        "ESCAPE-PRODUCER-FALSIFIER-MEASURED",
        Some("escape-fault-inject"),
        true,
    ) {
        Ok(run) => {
            passed += run.passed;
            failed += run.failed;
            ran |= run.ran;
            green &= run.green;
            substrate_skip &= run.substrate_skip;
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: producer-wired leg error (fault-inject falsifier): {e}");
            failed += 1;
            ran = true;
            green = false;
            substrate_skip = false;
        }
    }
    if !green {
        eprintln!(
            "{GATE_NAME}: producer-wired-proven-red leg not green (substrate_skip={substrate_skip}) \
             — on hosts that genuinely cannot sandbox this leg is advisory \
             (environment-unavailable); on hosts that CAN it is a real per-commit tripwire \
             (real kill → real row; fault-inject sever → no row) and it BLOCKS."
        );
    }
    LegResult::substrate(
        "producer-wired-proven-red",
        &MarkerRun {
            passed,
            failed,
            ran,
            green,
            substrate_skip,
        },
    )
}

/// Leg 9: detection-quality-live (AC4). The detector's correlation-decision
/// TP-floor/FP-ceiling measured LIVE over REAL seccomp children (no mock) — the
/// enforcement tripwire complementing the host-independent correlation leg
/// (leg 3). MARKER-gated: a silent seccomp-unavailable skip cannot pass
/// vacuously. Advisory on seccomp-blocked hosts; real per-commit tripwire on CI.
fn run_detection_quality_live_leg() -> LegResult {
    let run = invoke_cargo_test_marker(
        "maos-escape-detector",
        "detection_quality",
        "detection_quality_meets_floor_and_ceiling_on_real_seccomp",
        "ESCAPE-DETECTOR-QUALITY-MEASURED",
        None,
        false,
    )
    .unwrap_or_else(|_| MarkerRun::invocation_failed());
    LegResult::substrate("detection-quality-live", &run)
}

/// Sum `passed`/`failed` counts across every `test result:` line in `output`.
fn parse_test_summary(output: &str) -> (u32, u32) {
    let mut passed = 0u32;
    let mut failed = 0u32;
    for line in output.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("test result:") {
            passed += parse_count(rest, "passed");
            failed += parse_count(rest, "failed");
        }
    }
    (passed, failed)
}

fn parse_count(s: &str, key: &str) -> u32 {
    let bytes = s.as_bytes();
    let needle = format!(" {key}");
    let mut total = 0u32;
    let mut from = 0usize;
    while let Some(idx) = s[from..].find(&needle) {
        let start = from + idx;
        let mut b = start;
        while b > 0 && bytes[b - 1].is_ascii_digit() {
            b -= 1;
        }
        if b < start {
            if let Ok(n) = s[b..start].parse::<u32>() {
                total += n;
            }
        }
        from = start + needle.len();
    }
    total
}

fn legs_json(legs: &[LegResult]) -> serde_json::Value {
    serde_json::Value::Array(
        legs.iter()
            .map(|l| {
                serde_json::json!({
                    "label": l.label,
                    "class": match l.class {
                        BindingClass::Blocking => "blocking",
                        BindingClass::AdvisorySubstrate => "advisory-substrate",
                    },
                    "substrate_present": l.substrate_present,
                    "blocks": l.blocks(),
                    "passed": l.passed,
                    "failed": l.failed,
                    "ran": l.ran,
                    "attempted": l.attempted,
                    "green": l.green,
                    "status": l.status_word(),
                })
            })
            .collect(),
    )
}

pub fn run(json: bool) -> Result<(), String> {
    // 1. Read + validate the phase disposition from the registry.
    let disposition = read_disposition()?;
    if !matches!(
        disposition.get("v2_0").map(|s| s.as_str()),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
            disposition.get("v2_0")
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);

    // 2. Per-leg oracles (each its OWN invocation(s) — per-leg independence).
    let legs: Vec<LegResult> = vec![
        run_no_verdict_invariant_leg(),
        run_out_of_kernel_boundary_leg(),
        run_detection_quality_leg(),
        run_detection_quality_falsifier_leg(),
        run_source_identity_blind_leg(),
        run_kernel_abi_leg(),
        run_release_graph_absence_leg(),
        run_producer_wired_proven_red_leg(),
        run_detection_quality_live_leg(),
    ];

    // 3. Vacuous-green guard: a leg that was ATTEMPTED but compiled to ZERO
    //    tests / never reported results is a re-stubbed harness — hard-fail at
    //    every phase. The non-cargo-test legs (out-of-kernel-boundary,
    //    kernel-abi-diff, release-graph-absence) are exempt (they carry their
    //    own green, not a test count).
    let exempt = |label: &str| {
        label == "out-of-kernel-boundary"
            || label == "kernel-abi-diff"
            || label == "release-graph-absence"
    };
    for leg in &legs {
        if !exempt(leg.label) && leg.attempted && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}). \
                 The oracle produced no tests — a re-stubbed harness cannot pass this gate \
                 (anti-canned guard).",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = legs.iter().all(|l| l.green);

    // 4. Dev-time enforcement is decided PER LEG by its binding class (F3),
    //    never by the ship ladder. `blocking_now` is the GA disposition and is
    //    retained for reporting only — keying enforcement off it is exactly the
    //    decay D20 records: on a seccomp-blocked host the live legs returned
    //    `green=false` and the advisory tail converted RED into `passed: true`.
    let blocking_legs: Vec<&LegResult> = legs.iter().filter(|leg| leg.blocks()).collect();

    if oracle_green {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": GATE_NAME,
                    "passed": true,
                    "oracle_green": true,
                    "blocking_now": blocking_now,
                    "ship_phase": CURRENT_PHASE,
                    "disposition": disposition,
                    "legs": legs_json(&legs),
                })
            );
        } else {
            eprintln!(
                "{GATE_NAME}: PASSED — oracle green ({} legs); {} at {}",
                legs.len(),
                if blocking_now { "BLOCKING" } else { "advisory" },
                CURRENT_PHASE,
            );
        }
        return Ok(());
    }

    // Oracle RED — the verdict is the legs' binding classes, not the phase.
    let mut detail = String::new();
    for leg in &legs {
        detail.push_str(&format!(
            "- {} leg: {} passed, {} failed (ran={}, attempted={}, green={}, \
             substrate_present={}, blocks={}, status={})\n",
            leg.label,
            leg.passed,
            leg.failed,
            leg.ran,
            leg.attempted,
            leg.green,
            leg.substrate_present,
            leg.blocks(),
            leg.status_word(),
        ));
    }

    if !blocking_legs.is_empty() {
        let labels: Vec<&str> = blocking_legs.iter().map(|leg| leg.label).collect();
        let msg = format!(
            "{GATE_NAME}: BLOCKING — {} RED leg(s) enforce at HEAD ({}):\n{detail}",
            blocking_legs.len(),
            labels.join(", ")
        );
        emit_command(json, "error", &msg);
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": GATE_NAME,
                    "passed": false,
                    "oracle_green": false,
                    "advisory": false,
                    "blocking_now": blocking_now,
                    "blocking_legs": labels,
                    "seccomp_kernel_filtering": seccomp_kernel_filtering(),
                    "ship_phase": CURRENT_PHASE,
                    "disposition": disposition,
                    "legs": legs_json(&legs),
                })
            );
        } else {
            eprintln!("{msg}");
        }
        return Err(format!(
            "{GATE_NAME}: BLOCKING — {} RED leg(s) enforce at HEAD ({})",
            blocking_legs.len(),
            labels.join(", ")
        ));
    }

    // Every RED leg is `AdvisorySubstrate` with its substrate genuinely ABSENT:
    // WOULD-HAVE-BLOCKED banner, non-failing — never silent-green.
    let banner = format!(
        "## ⚠️ Escape-Detector Gate: WOULD HAVE BLOCKED (substrate absent)\n\
         {detail}\
         - Every RED leg needs a seccomp-capable host and this host reported \
           genuine unavailability, so the RED is environment-unavailable rather \
           than a regression. On a host that CAN sandbox these legs BLOCK at \
           HEAD — dev-time enforcement is the binding class, not the ship phase \
           (kernel seccomp filtering present: {}).\n",
        seccomp_kernel_filtering()
    );
    emit_command(
        json,
        "warning",
        "Escape-detector oracle RED with substrate absent — would block on a seccomp-capable host",
    );
    write_step_summary(&banner);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": true,
                "oracle_green": false,
                "advisory": true,
                "blocking_now": false,
                "substrate_absent": true,
                "seccomp_kernel_filtering": seccomp_kernel_filtering(),
                "ship_phase": CURRENT_PHASE,
                "disposition": disposition,
                "legs": legs_json(&legs),
            })
        );
    } else {
        eprintln!(
            "{GATE_NAME}: PASS (advisory — RED legs' substrate absent, would block on a \
             seccomp-capable host); {}",
            legs.iter()
                .map(|l| format!("{}={}", l.label, l.status_word()))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
