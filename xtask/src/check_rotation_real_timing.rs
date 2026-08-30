#![forbid(unsafe_code)]

//! Story 14-2 (AC5) — `check-rotation-real-timing` gate.
//!
//! ONE standalone gate cloned from `check_scale_churn.rs`'s shape, with
//! **per-leg verdict independence** — each leg reads its OWN `cargo test`
//! invocation(s), so one break reds exactly one leg. It does NOT append legs
//! to `check-scale-churn` or `check-multi-region-slo`, and neither appends
//! here (F7 / AC5.6 — per-leg independence honoured in both directions).
//!
//! # The legs (AC5.4, each independently red-able)
//!
//! 1. **rotation-floors-3-host** — the live 3-host provision→swap→close
//!    drill: floors via `passes_v15_floors` (which IS `passes_v10_floors`),
//!    zero drops, post-grace probes, marker `ROTATION_HOSTS_ROTATED=3`.
//! 2. **rotation-under-load-10-host** — the 10-host envelope (the epic's
//!    scale-out target) + the CertRotationRaceExploit adversary detection,
//!    marker `ROTATION_HOSTS_ROTATED=10` (the DERIVED witness — AC3.2).
//! 3. **rotation-proven-red** — the THREE falsifier vectors: drop
//!    reachability (exact surviving topology), p99-exceeds-floor, and the
//!    post-grace boundary ported from `scenario_5_3` BEFORE its deletion
//!    (AC6.1.b).
//! 4. **overlap-pin-closure** — AC2.3's four close assertions + during-window
//!    outside-the-set block + collision refusals, the N=3 in-flight window
//!    (drops == 0 under the overlap), and the EXECUTABLE teardown negative
//!    control (drops > 0 — the drop oracle's committed red capability).
//! 5. **kernel-abi-diff** — `check-kernel-baseline` GREEN (ZERO kernel-Δ).
//!
//! # The controls at the gate boundary (mirrors 14-1's ratified set)
//!
//! * **AC5.1 — measurement markers.** Every scale-carrying invocation
//!   requires `ROTATION_HOSTS_ROTATED=<n>` as an EXACT trimmed line carrying
//!   the DERIVED identity witness: a silent skip or an early return emits no
//!   marker and the leg REDS.
//! * **AC5.4 — exact test counts.** Every invocation names one exact test and
//!   passes libtest's `--exact`; `passed == 1` binds the verdict to that test
//!   rather than to substring-filter coincidence.
//! * **AC5.2 — derived enrollment.** Every `#[ignore]`-gated
//!   `t_10_4b_rotation_*` test in the ONE binary the legs invoke must be
//!   covered by a leg filter, or the gate FAILS before running anything.
//! * **AC4.2/AC4.3 — disclosure, transcribed.** The tests' OWN percentile
//!   disclosure lines (`revocation-propagation p50/p99 = … (n=…)`) are
//!   carried VERBATIM to the gate summary and leg JSON; the gate recomputes
//!   nothing and defaults nothing.
//!
//! # Live-oracle posture
//!
//! Every leg test is real (real `127.0.0.1` sockets, real rustls mTLS) and
//! `#[ignore]`-gated — the gate controls execution via `--ignored`; skipped ≠
//! passed. A vacuous leg (attempted but ZERO tests ran) hard-fails at EVERY
//! phase. The `rotation-fault-inject` feature is compiled OUT of release
//! builds by the `cargo check --release --tests --features
//! rotation-fault-inject` guard proof below.
//!
//! # Phase disposition (AC6.3)
//!
//! `BindingClass::Blocking`: a RED oracle hard-fails at HEAD regardless of
//! `CURRENT_PHASE`. The registry must carry `v1_5 = "blocking"` (it has since
//! 10.5 — DOWNGRADING it is the 10.5-AC6 floor-relaxation shape and this
//! gate hard-errors on it) plus the NEW `v2_0` and `v2_2` rungs; at the v2.2
//! ship gate an ABSENT result BLOCKS, never silent-greens.

use crate::gate_common::{
    dev_enforced_red_blocks, emit_command, is_blocking_at, read_disposition, BindingClass,
    CURRENT_PHASE,
};
use std::process::{Command, Stdio};

/// Canonical gate name (matches the registry `[[ship_gate]]` row, the
/// `Commands` variant, and the discipline.yml job).
const GATE_NAME: &str = "check-rotation-real-timing";

const TEST_PACKAGE: &str = "maos-a2a-tcp";
/// The ONLY test binary any leg invokes — the AC5.2 binary binding keys on it.
const TEST_FILE: &str = "t_10_4b_rotation_real_timing";
/// The test directory the AC5.2 enrollment is DERIVED from (never a
/// hand-listed filter set with nothing reconciling it). The scene
/// (`t_14_2_rotation_scene`) is deliberately OUT of scope: AC1.4 forbids it
/// from being a gate-leg oracle for any NFR floor — it narrates in the normal
/// test lane.
const TEST_DIR: &str = "crates/maos-a2a-tcp/tests";
/// AC5.5b — the dev/CI-only fault-injection feature. One mutation invocation
/// enables it at the real verdict-accounting seam; the release ship-blocker
/// separately proves that a release-profile test build refuses it.
const FAULT_INJECT_FEATURE: &str = "rotation-fault-inject";
/// AC5.1 markers: the DERIVED identity witness (AC3.2), never a literal
/// count a test could echo.
const M3: Option<&'static str> = Some("ROTATION_HOSTS_ROTATED=3");
const M10: Option<&'static str> = Some("ROTATION_HOSTS_ROTATED=10");

/// One oracle invocation (AC5.3 per-leg independence): its own name filter,
/// the EXACT number of tests the filter must match (AC5.4), and the
/// measurement marker its test must print (AC5.1).
struct Invocation {
    filter: &'static str,
    /// AC5.5b — the fault-injection feature this invocation enables (the
    /// mutation leg); `None` for clean-path legs.
    features: Option<&'static str>,
    expected_tests: u32,
    marker: Option<&'static str>,
    /// AC4.2 close-pass: scale-carrying drills must publish all three
    /// percentile disclosure lines, each with its `n=`.
    disclose: bool,
}

/// One oracle leg's parsed result (aggregated across its sub-invocations).
struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
    markers: Vec<&'static str>,
    /// The drill's OWN percentile disclosure lines, carried verbatim (AC4.2
    /// third surface; AC4.3 — the gate transcribes, it does not recompute).
    percentiles: Vec<String>,
}

/// Invoke ONE exact oracle test and parse its summary. GREEN requires: exit
/// success, a `test result:` line, exactly one passed test, zero failures, the
/// exact `running 1 test` line, and any required marker as an exact trimmed
/// line.
#[allow(clippy::type_complexity)]
fn invoke_cargo_test(
    invocation: &Invocation,
) -> Result<(u32, u32, bool, bool, Vec<String>), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", TEST_PACKAGE, "--test", TEST_FILE]);
    if let Some(f) = invocation.features {
        cmd.args(["--features", f]);
    }
    cmd.args([
        "--",
        invocation.filter,
        "--exact",
        "--ignored",
        "--nocapture",
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({TEST_PACKAGE}/{TEST_FILE}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    // AC4.2 — the tests' OWN disclosure lines (e.g. `revocation-propagation
    // p50/p99 = 6/8 ms  (n=3)`), carried verbatim. Derived only.
    let percentiles: Vec<String> = combined
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("revocation-propagation p50/p99")
                || t.starts_with("re-handshake")
                || t.starts_with("end-to-end")
                || t.starts_with("post-grace retired-cert accept rate")
        })
        .map(|l| l.trim().to_string())
        .collect();
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let running_exact = combined.contains(&format!("running {} test", invocation.expected_tests));
    let measured = invocation.marker.map_or(true, |m| {
        combined.lines().filter(|l| l.trim() == m).count() as u32 >= invocation.expected_tests
    });
    // AC4.2 close-pass: the three percentile axes must each disclose an
    // `n=` sample count on this invocation's output.
    let disclosed = !invocation.disclose
        || ["revocation-propagation", "re-handshake", "end-to-end"]
            .iter()
            .all(|axis| {
                combined
                    .lines()
                    .any(|l| l.trim().starts_with(axis) && l.contains("(n="))
            });
    let green = output.status.success()
        && ran
        && passed == invocation.expected_tests
        && failed == 0
        && running_exact
        && measured
        && disclosed;
    if !green {
        eprintln!(
            "{GATE_NAME}: {TEST_FILE} (filter={:?}) NOT green (passed={passed}, expected={}, \
             failed={failed}, ran={ran}, running-exact={running_exact}, measured={measured}, \
             exit={}) — see the cargo output above",
            invocation.filter, invocation.expected_tests, output.status
        );
    }
    Ok((passed, failed, ran, green, percentiles))
}

/// Run one leg's invocations and fold them into ONE leg verdict (AND
/// semantics — every invocation must be green for the leg to be green).
fn run_leg(label: &'static str, invocations: &[Invocation]) -> LegResult {
    let (mut passed, mut failed, mut ran, mut green) = (0u32, 0u32, false, true);
    let mut percentiles = Vec::new();
    for invocation in invocations {
        match invoke_cargo_test(invocation) {
            Ok((p, f, r, g, pc)) => {
                (passed, failed, ran, green) = (passed + p, failed + f, ran | r, green & g);
                percentiles.extend(pc);
            }
            Err(e) => {
                eprintln!(
                    "{GATE_NAME}: {label} leg error ({}): {e}",
                    invocation.filter
                );
                (failed, ran, green) = (failed + 1, true, false);
            }
        }
    }
    LegResult {
        label,
        passed,
        failed,
        ran,
        attempted: true,
        green,
        markers: invocations.iter().filter_map(|i| i.marker).collect(),
        percentiles,
    }
}

/// The rotation legs as DATA (AC5.2): the same table drives the runs AND the
/// derived-enrollment reconciliation. One entry per leg — per-leg
/// independence (AC5.6: no cross-wiring into sibling gates).
fn leg_invocations() -> Vec<(&'static str, Vec<Invocation>)> {
    vec![
        (
            "rotation-floors-3-host",
            vec![
                Invocation {
                    filter: "t_10_4b_rotation_real_timing_3_host_drill",
                    features: None,
                    expected_tests: 1,
                    marker: M3,
                    disclose: true,
                },
                // The WIRED markdown surface's own contract (AC6.1.a),
                // ported to the uncharged lane — a floors-leg member.
                Invocation {
                    filter: "t_10_4b_rotation_report_markdown_publishes_sample_counts",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
                // §A6 close-pass (CloseRuntime): the verdict predicate's own
                // contract — the executable red for a lenient-predicate
                // regression.
                Invocation {
                    filter: "t_10_4b_rotation_verdict_predicate_contract",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
            ],
        ),
        (
            "rotation-under-load-10-host",
            vec![Invocation {
                filter: "t_10_4b_rotation_real_timing_10_host_envelope",
                features: None,
                expected_tests: 1,
                marker: M10,
                disclose: true,
            }],
        ),
        (
            "rotation-proven-red",
            vec![
                Invocation {
                    filter: "t_10_4b_rotation_proven_red_drop_reachability",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
                Invocation {
                    filter: "t_10_4b_rotation_proven_red_p99_exceeds_floor",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
                Invocation {
                    filter: "t_10_4b_rotation_proven_red_post_grace_boundary",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
            ],
        ),
        (
            "overlap-pin-closure",
            vec![
                Invocation {
                    filter: "t_10_4b_rotation_overlap_pin_closure",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
                Invocation {
                    filter: "t_10_4b_rotation_under_load_window",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
                // The EXECUTABLE negative control (§A6 Acceptance-5): the
                // drop oracle's `drops > 0` capability, committed beside the
                // overlap's `drops == 0`.
                Invocation {
                    filter: "t_10_4b_rotation_drop_oracle_teardown_control",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
                // AC5.5b mutation leg (close-pass Blind-2): with the feature
                // ON, the blind is characterized — the committed predicate
                // contradicts it on the same outcomes (11.3's mutation shape).
                Invocation {
                    filter: "t_10_4b_rotation_fault_blind_characterized",
                    features: Some(FAULT_INJECT_FEATURE),
                    expected_tests: 1,
                    marker: None,
                    disclose: false,
                },
            ],
        ),
    ]
}

/// AC5.2/AC5.4 — derive every `#[ignore]`-gated rotation test from the whole
/// test directory (`t_10_4b_rotation_*` and `t_14_2_*` files) and require an
/// exact invocation for each test in the gate-owned binary. The scene
/// (`t_14_2_rotation_scene`) is deliberately not ignored and remains in the
/// normal lane per AC1.4.
fn derive_ignored_rotation_tests() -> Result<Vec<(String, String)>, String> {
    let entries = std::fs::read_dir(TEST_DIR)
        .map_err(|e| format!("{GATE_NAME}: cannot walk {TEST_DIR} to derive enrollment: {e}"))?;
    let mut derived = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{GATE_NAME}: {TEST_DIR}: {e}"))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let scoped = (file_name.starts_with("t_10_4b_rotation_")
            || file_name.starts_with("t_14_2_"))
            && file_name.ends_with(".rs");
        if !scoped {
            continue;
        }
        let stem = file_name.trim_end_matches(".rs").to_string();
        let text = std::fs::read_to_string(entry.path())
            .map_err(|e| format!("{GATE_NAME}: cannot read {TEST_DIR}/{file_name}: {e}"))?;
        let (mut pending_ignore, mut in_block) = (false, false);
        for line in text.lines() {
            let mut t = line.trim_start();
            if in_block {
                let Some(end) = t.find("*/") else {
                    continue;
                };
                t = t[end + 2..].trim_start();
                in_block = false;
            }
            while t.starts_with("/*") {
                if let Some(end) = t.find("*/") {
                    t = t[end + 2..].trim_start();
                } else {
                    in_block = true;
                    break;
                }
            }
            if in_block {
                continue;
            }
            if t.starts_with("#[") {
                pending_ignore |= t.starts_with("#[ignore");
                continue;
            }
            let visibility_stripped = t
                .strip_prefix("pub ")
                .or_else(|| {
                    t.strip_prefix("pub(")
                        .and_then(|rest| rest.split_once(") ").map(|(_, tail)| tail))
                })
                .unwrap_or(t);
            let declaration = visibility_stripped
                .strip_prefix("async ")
                .unwrap_or(visibility_stripped);
            if let Some(name) = declaration.strip_prefix("fn ") {
                if pending_ignore && name.starts_with("t_") {
                    let name: String = name
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    derived.push((stem.clone(), name));
                }
                pending_ignore = false;
            } else if !t.is_empty() && !t.starts_with("//") {
                pending_ignore = false;
            }
        }
    }
    derived.sort();
    if derived.is_empty() {
        return Err(format!(
            "{GATE_NAME}: FAIL — the {TEST_DIR} walk derived ZERO ignored rotation tests \
             (AC5.2: an empty derivation is how a filesystem-derived enrollment goes decorative)"
        ));
    }
    Ok(derived)
}

/// AC5.5b — the fault-injection feature's release ship-blocker, PROVEN not
/// probed: `cargo check --release --tests --features rotation-fault-inject`
/// MUST FAIL on the test file's `compile_error!` guard (release profile ⇒
/// `not(debug_assertions)`). §A6 close-pass (CloseTestInfra-3/4/5): the
/// prior `cargo tree --release` probe was an INVALID flag whose ignored
/// exit status let the check green vacuously, and `-e features` text does
/// not even display an empty feature on the root package — a graph-text
/// search could never prove absence. Failure for ANY OTHER reason also
/// fails the gate: unmeasured must fail, not pass.
fn fault_inject_blocked_in_release() -> Result<(), String> {
    let out = Command::new("cargo")
        .args([
            "check",
            "--release",
            "--tests",
            "--features",
            FAULT_INJECT_FEATURE,
            "-p",
            TEST_PACKAGE,
        ])
        .output()
        .map_err(|e| format!("{GATE_NAME}: cannot run cargo check: {e}"))?;
    if out.status.success() {
        return Err(format!(
            "{GATE_NAME}: SHIP-BLOCKER — a release-profile check WITH `{FAULT_INJECT_FEATURE}` \
             SUCCEEDED: the compile_error! guard did not fire (AC5.5b)"
        ));
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !text.contains("rotation-fault-inject is a dev/CI-only") {
        return Err(format!(
            "{GATE_NAME}: SHIP-BLOCKER — the release check failed for an UNRELATED reason \
             (the feature guard's message was not in the output); unmeasured must fail, not \
             pass (AC5.5b)"
        ));
    }
    Ok(())
}

/// Leg 5: kernel-ABI baseline — ZERO kernel-Δ (check-kernel-baseline owns
/// the pin value; this gate never restates it).
fn run_kernel_abi_leg() -> LegResult {
    let green = crate::check_kernel_baseline::check().is_ok_and(|report| report.passed);
    LegResult {
        label: "kernel-abi-diff",
        passed: u32::from(green),
        failed: u32::from(!green),
        ran: true,
        attempted: true,
        green,
        markers: Vec::new(),
        percentiles: Vec::new(),
    }
}

/// Sum `passed`/`failed` counts across every `test result:` line in `output`.
fn parse_test_summary(output: &str) -> (u32, u32) {
    let results = output
        .lines()
        .filter_map(|l| l.trim().strip_prefix("test result:"));
    results.fold((0, 0), |(p, f), r| {
        (p + parse_count(r, "passed"), f + parse_count(r, "failed"))
    })
}

/// Parse the total of `"<n> <key>"` occurrences in `s` (e.g. `8 passed`).
fn parse_count(s: &str, key: &str) -> u32 {
    s.match_indices(key)
        .filter_map(|(i, _)| {
            let tail = s[..i].trim_end();
            let digits = tail.trim_end_matches(|c: char| c.is_ascii_digit());
            tail[digits.len()..].parse::<u32>().ok()
        })
        .sum()
}

/// Build the JSON array of per-leg verdicts for programmatic consumers.
fn legs_json(legs: &[LegResult]) -> serde_json::Value {
    serde_json::Value::Array(
        legs.iter()
            .map(|l| {
                let status = if l.green {
                    "green"
                } else if l.attempted {
                    "red"
                } else {
                    "skipped"
                };
                serde_json::json!({
                    "label": l.label,
                    "passed": l.passed,
                    "failed": l.failed,
                    "ran": l.ran,
                    "attempted": l.attempted,
                    "green": l.green,
                    "status": status,
                    "markers": l.markers,
                    "percentiles": l.percentiles,
                })
            })
            .collect(),
    )
}

pub fn run(json: bool) -> Result<(), String> {
    // 1. Read + VALIDATE the phase disposition (AC6.3 — hard-error on a
    //    registry defect, never trust it): `v1_5` must STAY blocking
    //    (downgrading it is the 10.5-AC6 floor-relaxation shape), and the
    //    v2.2-wave rungs `v2_0` + `v2_2` must exist.
    let disposition = read_disposition(GATE_NAME)?;
    if disposition.get("v1_5").map(|s| s.as_str()) != Some("blocking") {
        return Err(format!(
            "{GATE_NAME}: registry defect — v1_5 disposition must be \"blocking\" (got {:?}); \
             downgrading a live blocking rung is the 10.5-AC6 floor-relaxation shape (AC6.3)",
            disposition.get("v1_5")
        ));
    }
    if disposition.get("v2_0").map(String::as_str) != Some("blocking") {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 must preserve the inherited blocking \
             disposition (got {:?}); an explicit advisory rung is a downgrade (AC6.3)",
            disposition.get("v2_0")
        ));
    }
    // §A6 close-pass (CloseBlind-3): presence alone would let a silent
    // downgrade to advisory pass — validate the VALUE.
    if disposition.get("v2_2").map(|s| s.as_str()) != Some("blocking") {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_2 must be \"blocking\" (got {:?}); the v2.2 \
             ship gate is this story's binding rung and absent-result must BLOCK (AC6.3)",
            disposition.get("v2_2")
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);
    // Option C (Epic-12 retro B1): the Blocking binding class hard-fails a
    // RED oracle at HEAD regardless of CURRENT_PHASE.
    let dev_blocks = blocking_now || dev_enforced_red_blocks(BindingClass::Blocking, true);

    // 2. AC5.5b — the fault-injection feature must be absent from release.
    if let Err(e) = fault_inject_blocked_in_release() {
        emit_command(json, "error", &e);
        return Err(e);
    }

    // 3. AC5.2 — reconcile the derived test file against the leg table
    //    BEFORE any cargo run: a rotation test no leg names fails the gate.
    let derived = derive_ignored_rotation_tests()?;
    let legs_data = leg_invocations();
    for (stem, name) in &derived {
        // Round-2-P4 analogue: a feature-gated derived test is covered only
        // by an invocation that compiles it (features.is_some()).
        let gated = name.starts_with("t_10_4b_rotation_fault_blind");
        let covered = stem == TEST_FILE
            && legs_data.iter().any(|(_, invs)| {
                invs.iter()
                    .any(|i| name == i.filter && (!gated || i.features.is_some()))
            });
        if !covered {
            return Err(format!(
                "{GATE_NAME}: FAIL — `{name}` will never run: no leg names it (AC5.2 derived \
                 enrollment; a hand-listed filter set is a suggestion, not a control)"
            ));
        }
    }

    // 4. Per-leg oracles (each its OWN invocation(s) — per-leg independence).
    let mut legs: Vec<LegResult> = legs_data
        .iter()
        .map(|(label, invocations)| run_leg(label, invocations))
        .collect();
    // 5. Vacuous-green guard (J4/L6 anti-canned): a leg that was ATTEMPTED
    //    but compiled to ZERO tests / never reported results hard-fails at
    //    EVERY phase. kernel-abi-diff is exempt (baseline, not a test count).
    legs.push(run_kernel_abi_leg());
    for leg in &legs {
        if leg.label != "kernel-abi-diff"
            && leg.attempted
            && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}) — a \
                 re-stubbed harness cannot pass this gate (J4 anti-canned guard)",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = legs.iter().all(|l| l.green);

    // 6. Verdict. `dev_blocks` is serialized (AC6.4): the JSON verdict and
    //    the stderr banner cannot contradict each other.
    if oracle_green {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": GATE_NAME,
                    "passed": true,
                    "oracle_green": true,
                    "blocking_now": blocking_now,
                    "dev_blocks": dev_blocks,
                    "current_phase": CURRENT_PHASE,
                    "disposition": disposition,
                    "enrolled_rotation_tests": derived.len(),
                    "legs": legs_json(&legs),
                })
            );
        } else {
            eprintln!(
                "{GATE_NAME}: PASSED — oracle green ({} legs, {} enrolled rotation tests); {} at {}",
                legs.len(),
                derived.len(),
                if dev_blocks { "BLOCKING" } else { "advisory" },
                CURRENT_PHASE,
            );
            // AC4.2 — publish the tests' own disclosure lines beside the
            // banner: verbatim, no values of the gate's own invention.
            for line in legs.iter().flat_map(|l| &l.percentiles) {
                eprintln!("{GATE_NAME}:   {line}");
            }
        }
        return Ok(());
    }

    // Oracle RED — BindingClass::Blocking hard-fails at HEAD regardless of
    // CURRENT_PHASE. No whole-gate advisory tail exists (D20's shape is
    // deleted; per-leg held_advisory_reason is the only lawful hold).
    let detail: String = legs
        .iter()
        .map(|l| {
            let pct: String = l.percentiles.iter().map(|p| format!("  - {p}\n")).collect();
            let (lb, p, f, r, a, g) = (l.label, l.passed, l.failed, l.ran, l.attempted, l.green);
            format!("- {lb} leg: {p} passed, {f} failed (ran={r}, attempted={a}, green={g})\n{pct}")
        })
        .collect();
    let msg = format!("{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE} (binding):\n{detail}");
    emit_command(json, "error", &msg);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": false,
                "oracle_green": false,
                "blocking_now": blocking_now,
                "dev_blocks": dev_blocks,
                "current_phase": CURRENT_PHASE,
                "disposition": disposition,
                "enrolled_rotation_tests": derived.len(),
                "legs": legs_json(&legs),
                "error": &msg,
            })
        );
    } else {
        eprintln!("{msg}");
    }
    Err(format!(
        "{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE}"
    ))
}
