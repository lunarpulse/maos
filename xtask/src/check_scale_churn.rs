#![forbid(unsafe_code)]

//! Story 11.3 (AC5, D8) + Story 14.1 (AC5, AC6) — `check-scale-churn` gate.
//!
//! ONE standalone gate cloned from `check_multi_region_slo.rs`'s shape, with
//! **per-leg verdict independence** — each leg reads its OWN `cargo test`
//! invocation(s), so one break reds exactly one leg (Murat's
//! one-break-one-red). This gate is DISTINCT from `check-rotation-real-timing`
//! and `check-multi-region-slo` and MUST NOT append legs to either (F7 —
//! their invocations aren't churn-scoped; coupling would mask a break).
//!
//! # The legs (each its own oracle invocation set)
//!
//! 1. **mesh-identity-reconcile** — the compressed N=30 trio (identity
//!    reconcile, duplicate-identity negative control, churn envelope drill)
//!    PLUS the Story-14.1 N=100 envelope: identity reconcile, the duplicate
//!    control at N=100 (**100 clones are not 100 hosts** — its derived
//!    reconciled count is 99 and the marker says so), and the N=100 churn
//!    envelope (10–20% ratio turnover band, AC2.3).
//! 2. **detection-latency** — the clean detection drills at BOTH scales
//!    (adversaries planted INTO the mesh, two surfaces, class-matched
//!    assertions) + the `churn-fault-inject` per-class blind mutations at
//!    BOTH scales (GREEN requires the clean paths AND all 3+3 mutations).
//! 3. **blast-recovery-rto** — the clean blast/recovery/rto drills at BOTH
//!    scales (the blast floor is a RACE at N=100: the adversary is offered
//!    the full derived peer set and offered == reached REDS) + the blast
//!    over-reach, isolation-blind and re-pin-blind falsifiers at BOTH scales
//!    + the offered==reached non-degeneracy mutation at N=100.
//! 4. **kernel-abi-diff** — `check-kernel-baseline` re-pin GREEN at 24472
//!    (ZERO kernel-Δ — this story lives entirely outside kernel-core).
//!
//! # The three controls at the gate boundary (Story 14.1)
//!
//! * **AC5.1 — measurement markers.** The raw green formula can only tell a
//!   test that measured a hundred hosts from a test that compiled and
//!   returned — nothing required measurement at all. Every N=100 invocation
//!   therefore requires the marker as an EXACT trimmed line, at least once
//!   per expected test, carrying the DERIVED reconciled host count
//!   (`SCALE_CHURN_HOSTS_RECONCILED=<n>`,
//!   `check_escape_detector.rs::invoke_cargo_test_marker` shape): a silent
//!   skip or an early return emits no marker and the leg REDS.
//! * **AC5.4 — exact test counts.** libtest name filters are PREFIX
//!   matches, so a filter that silently stops matching one of the per-N
//!   wrappers is invisible to `passed >= 1`. Every invocation declares the
//!   EXACT number of tests its filter must match and must report running
//!   (`check_cohort_mesh.rs` idiom); `passed == expected` replaces
//!   `passed >= 1`.
//! * **AC5.2 — derived enrollment.** The leg filter set is reconciled
//!   against the test directory AT RUN TIME (the
//!   `check_j1_two_host_signed_run.rs::leg_vectors_enrolled` shape): every
//!   `#[ignore]`-gated churn test derived from
//!   `crates/maos-a2a-tcp/tests/{t_11_3_scale_churn*,t_14_1_*}.rs` must be
//!   covered by at least one leg filter AND live in the one binary the legs
//!   actually invoke (`t_11_3_scale_churn` — a name can prefix-match a leg
//!   filter in a binary no leg ever loads), or the gate FAILS before
//!   running anything — a new churn test can never silently never run.
//!
//! # Live-oracle posture
//!
//! Every leg test is real (real `127.0.0.1` sockets, real rustls mTLS, real
//! `A2ARouterCore` NACKs) and `#[ignore]`-gated — the gate controls
//! execution via `--ignored`; skipped ≠ passed. A vacuous leg (attempted but
//! ZERO tests ran) hard-fails at EVERY phase (the J4 anti-canned guard, L6).
//!
//! # Phase disposition
//!
//! Dev-time enforcement is decoupled from the GA ship-phase ladder (Option C,
//! Epic-12 retro B1): this gate's oracle is `BindingClass::Blocking`, so a RED
//! oracle hard-fails at HEAD regardless of `CURRENT_PHASE`. The registry
//! disposition governs only ship-phase JSON reporting (`blocking_now`). The
//! old whole-gate advisory tail (WOULD-HAVE-BLOCKED banner) was STRUCTURALLY
//! UNREACHABLE dead code under that class and was deleted by Story 14-1
//! AC6.1 — never revived (D20's exact shape). `dev_blocks` is serialized in
//! the JSON (AC6.4) so the machine-readable verdict can no longer contradict
//! the stderr banner.

use crate::gate_common::{
    dev_enforced_red_blocks, emit_command, is_blocking_at, read_disposition, BindingClass,
    CURRENT_PHASE,
};
use std::process::{Command, Stdio};

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
const GATE_NAME: &str = "check-scale-churn";

const TEST_PACKAGE: &str = "maos-a2a-tcp";
/// The ONLY test binary any leg invokes — the AC5.2 binary binding keys on it.
const TEST_FILE: &str = "t_11_3_scale_churn";
/// The test directory the AC5.2 enrollment is DERIVED from (never a
/// hand-listed set of filters with nothing reconciling them).
const TEST_DIR: &str = "crates/maos-a2a-tcp/tests";
/// The fault-injection feature the falsifier legs flip on (compiled OUT of
/// the release tree — `t_11_3_scale_churn.rs`'s `compile_error!` guard and
/// the `cargo tree --release` ship-blocker both survive the scale-up, AC5.6).
const FAULT_INJECT_FEATURE: &str = "churn-fault-inject";
/// The AC5.1 full-envelope marker: the test's stdout must carry the DERIVED
/// reconciled host count — exactly 100 (a silent skip or early return emits
/// no marker and the leg REDS).
const M100: Option<&'static str> = Some("SCALE_CHURN_HOSTS_RECONCILED=100");
/// The duplicate-identity control's marker: 100 clones reconcile to 99 —
/// 100 clones are NOT 100 hosts (AC2.2).
const M99: Option<&'static str> = Some("SCALE_CHURN_HOSTS_RECONCILED=99");

/// One oracle invocation (AC5.3 per-leg independence): its own name filter,
/// optional feature, the EXACT number of tests the filter must match
/// (AC5.4), and the measurement marker its test must print (AC5.1).
struct Invocation {
    filter: &'static str,
    features: Option<&'static str>,
    expected_tests: u32,
    marker: Option<&'static str>,
}

/// One oracle leg's parsed result (aggregated across its sub-invocations).
struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
    /// The measurement markers this leg's invocations REQUIRED (AC5.1) —
    /// serialized beside the verdict so the gate summary carries the derived
    /// host counts, not just a boolean.
    markers: Vec<&'static str>,
    /// The `detection_latency_*` percentile lines the leg's tests printed —
    /// carried verbatim to the gate summary (AC3.4.a third publication
    /// surface). Derived only: this gate computes and defaults nothing; an
    /// empty vec means no percentile was measured.
    percentiles: Vec<String>,
}

/// Invoke ONE oracle invocation and parse its summary. GREEN requires:
/// exit success, a `test result:` line, `passed == expected_tests` (AC5.4 —
/// prefix filters can silently shrink; exact counts make that visible), zero
/// failures, AND the required marker as an exact trimmed line once per
/// expected test (AC5.1).
fn invoke_cargo_test(
    invocation: &Invocation,
) -> Result<(u32, u32, bool, bool, Vec<String>), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", TEST_PACKAGE, "--test", TEST_FILE]);
    if let Some(f) = invocation.features {
        cmd.args(["--features", f]);
    }
    cmd.args(["--", invocation.filter, "--ignored", "--nocapture"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({TEST_PACKAGE}/{TEST_FILE}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    // AC3.4.a — the tests' OWN percentile disclosure lines (e.g.
    // `detection_latency_p99_secs = 0 (n=3)`), carried verbatim to the gate
    // summary. Derived only: the gate computes nothing and defaults nothing;
    // a leg whose tests print no percentile line publishes none.
    let percentiles: Vec<String> = combined
        .lines()
        .filter(|l| l.trim().starts_with("detection_latency_"))
        .map(|l| l.trim().to_string())
        .collect();
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    // AC5.4 — libtest prints `running N test(s)` before the suite: the
    // invocation's exact count must be ATTEMPTED, not merely
    // passed-by-a-smaller-match. AC5.1 — the marker must appear as an EXACT
    // trimmed line (substring containment let an output carrying
    // `SCALE_CHURN_HOSTS_RECONCILED=1000` satisfy a required `=100`) and at
    // least `expected_tests` times, so a 3-test blind filter cannot green on
    // one marker while its other two falsifiers early-return unmeasured.
    let running_exact = combined.contains(&format!("running {} test", invocation.expected_tests));
    let measured = invocation.marker.map_or(true, |m| {
        combined.lines().filter(|l| l.trim() == m).count() as u32 >= invocation.expected_tests
    });
    let green = output.status.success()
        && ran
        && passed == invocation.expected_tests
        && failed == 0
        && running_exact
        && measured;
    if !green {
        eprintln!(
            "{GATE_NAME}: {TEST_FILE} (filter={:?}, features={:?}) NOT green (passed={passed}, expected={}, failed={failed}, ran={ran}, running-marker={running_exact}, measured={measured}, exit={}) — see the cargo output above",
            invocation.filter, invocation.features, invocation.expected_tests, output.status
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

/// The churn test legs as DATA (AC5.2): the same table drives both the runs
/// and the derived-enrollment reconciliation, so a filter added here is
/// checked against the directory and a test added to the directory is
/// checked against this table. One entry per leg — per-leg independence.
fn leg_invocations() -> Vec<(&'static str, Vec<Invocation>)> {
    // 1-char local keeps every row under rustfmt's 100-column line width.
    let f = Some(FAULT_INJECT_FEATURE);
    let legs: &[(
        &'static str,
        &[(
            &'static str,
            Option<&'static str>,
            u32,
            Option<&'static str>,
        )],
    )] = &[
        (
            "mesh-identity-reconcile",
            &[
                ("t_11_3_mesh_identity_reconcile_30_host", None, 1, None),
                (
                    "t_11_3_duplicate_identity_negative_control_hard_fails",
                    None,
                    1,
                    None,
                ),
                ("t_11_3_scale_churn_30_host_drill", None, 1, None),
                // Story 14.1 — the N=100 envelope (AC6.2: Binding, like every
                // other leg; the substrate exists and CI can host it).
                ("t_14_1_mesh_identity_reconcile_n100", None, 1, M100),
                (
                    "t_14_1_duplicate_identity_negative_control_n100",
                    None,
                    1,
                    M99,
                ),
                ("t_14_1_scale_churn_n100_drill", None, 1, M100),
            ],
        ),
        (
            "detection-latency",
            &[
                ("t_11_3_detection_latency_drill", None, 1, None),
                ("t_11_3_fault_inject_blind", f, 3, None),
                ("t_14_1_detection_latency_n100", None, 1, M100),
                ("t_14_1_fault_inject_blind", f, 3, M100),
            ],
        ),
        (
            "blast-recovery-rto",
            &[
                ("t_11_3_blast_recovery_rto_drill", None, 1, None),
                ("t_11_3_fault_inject_blast_overreach_reds_floor", f, 1, None),
                (
                    "t_11_3_fault_inject_isolation_blind_reds_rto_only",
                    f,
                    1,
                    None,
                ),
                (
                    "t_11_3_fault_inject_repin_blind_reds_recovery_only",
                    f,
                    1,
                    None,
                ),
                ("t_14_1_blast_recovery_rto_n100", None, 1, M100),
                (
                    "t_14_1_fault_inject_blast_overreach_reds_floor_n100",
                    f,
                    1,
                    M100,
                ),
                (
                    "t_14_1_fault_inject_isolation_blind_reds_rto_only_n100",
                    f,
                    1,
                    M100,
                ),
                (
                    "t_14_1_fault_inject_repin_blind_reds_recovery_only_n100",
                    f,
                    1,
                    M100,
                ),
                (
                    "t_14_1_fault_inject_full_sweep_no_detection_reds_offered_reached_n100",
                    f,
                    1,
                    M100,
                ),
            ],
        ),
    ];
    legs.iter()
        .map(|(label, rows)| {
            (
                *label,
                rows.iter()
                    .map(|(filter, features, expected_tests, marker)| Invocation {
                        filter,
                        features: *features,
                        expected_tests: *expected_tests,
                        marker: *marker,
                    })
                    .collect(),
            )
        })
        .collect()
}

/// AC5.2 — DERIVE the `#[ignore]`-gated churn test names from the test
/// directory at run time and require every one to be covered by at least one
/// leg filter. A const list is a suggestion, not a control
/// (`check_j1_two_host_signed_run.rs` enrollment leg): a new churn test no
/// leg names must FAIL this gate, not silently never run. The file scope is
/// a RULE (the 11.3 gate file + every `t_14_1_*` churn file), not an
/// enumerated list.

/// Each entry is `(home file stem, test fn)` (AC5.2 binary binding): the
/// legs invoke `--test t_11_3_scale_churn` ONLY, so a derived test whose
/// stem differs can never be executed by any leg — `run()` fails that pair
/// by name before any cargo run.
fn derive_ignored_churn_tests() -> Result<Vec<(String, String, bool)>, String> {
    let mut derived: Vec<(String, String, bool)> = Vec::new();
    let entries = std::fs::read_dir(TEST_DIR)
        .map_err(|e| format!("{GATE_NAME}: cannot walk {TEST_DIR} to derive enrollment: {e}"))?;
    // Fail closed on per-entry errors (AC5.2): `flatten()` would silently
    // drop an unreadable dirent from the derived set while the non-empty
    // guard below stays satisfied.
    for entry in entries {
        let entry = entry.map_err(|e| format!("{GATE_NAME}: {TEST_DIR}: {e}"))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let scoped = (file_name.starts_with("t_11_3_scale_churn")
            || file_name.starts_with("t_14_1_"))
            && file_name.ends_with(".rs");
        if !scoped {
            continue;
        }
        let stem = file_name.trim_end_matches(".rs").to_string();
        let text = std::fs::read_to_string(entry.path())
            .map_err(|e| format!("{GATE_NAME}: cannot read {TEST_DIR}/{file_name}: {e}"))?;
        let (mut pending_ignore, mut pending_cfg, mut in_block) = (false, false, false);
        for line in text.lines() {
            let mut t = line.trim_start();
            // Round-2 P7: strip same-line-closed `/* … */` prefixes so an
            // inline comment cannot swallow a following `#[ignore]`.
            while let Some(c) = t.strip_prefix("/*").and_then(|r| r.find("*/")) {
                t = t[c + 4..].trim_start();
            }
            if in_block || t.starts_with("/*") {
                in_block = !t.contains("*/");
                continue;
            }
            // `#[cfg(feature)]` adjacency is tracked like `#[ignore]`
            // (round-2 P4): coverage must bind a feature-gated test to an
            // invocation that actually compiles it.
            if t.starts_with("#[") {
                pending_ignore |= t.starts_with("#[ignore");
                pending_cfg |= t.contains("churn-fault-inject");
            } else if t.starts_with("fn ") || t.starts_with("async fn ") {
                let name = t.trim_start_matches("async ").trim_start_matches("fn ");
                if pending_ignore && name.starts_with("t_") {
                    let name: String = name
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    derived.push((stem.clone(), name, pending_cfg));
                }
                (pending_ignore, pending_cfg) = (false, false);
            } else if !t.is_empty() && !t.starts_with("//") {
                (pending_ignore, pending_cfg) = (false, false);
            }
        }
    }
    derived.sort();
    if derived.is_empty() {
        return Err(format!(
            "{GATE_NAME}: FAIL — the {TEST_DIR} walk derived ZERO ignored churn tests (AC5.2: an empty derivation is how a filesystem-derived enrollment goes decorative)"
        ));
    }
    Ok(derived)
}

/// Leg 4: kernel-ABI baseline (no PG) — ZERO kernel-Δ at 24472.
fn run_kernel_abi_leg() -> LegResult {
    let green = crate::check_kernel_baseline::run(false).is_ok();
    LegResult {
        label: "kernel-abi-diff",
        passed: if green { 1 } else { 0 },
        failed: if green { 0 } else { 1 },
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

/// Parse the total of `"<n> <key>"` occurrences in `s` (e.g. `19 passed`).
fn parse_count(s: &str, key: &str) -> u32 {
    // Sum every `<digits><space>key` occurrence (e.g. `19 passed`); an
    // occurrence with no immediately-preceding number contributes nothing.
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
    // 1. Read + validate the phase disposition from the registry.
    let disposition = read_disposition(GATE_NAME)?;
    if disposition.get("v2_0").map(|s| s.as_str()) != Some("blocking") {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
            disposition.get("v2_0")
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);
    // Option C (Epic 12 retro B1): hermetic gate — the Blocking binding class
    // hard-fails a RED oracle at HEAD regardless of CURRENT_PHASE. Dev-time
    // enforcement is decoupled from the GA ship-phase ladder (`blocking_now` is
    // retained for JSON reporting). See gate_common::BindingClass.
    let dev_blocks = blocking_now || dev_enforced_red_blocks(BindingClass::Blocking, true);

    // 2. AC5.2 — reconcile the derived test directory against the leg table
    // BEFORE any cargo run: a churn test no leg names fails the gate here.
    // Coverage is bound to the BINARY too (TEST_FILE is the only harness any
    // leg invokes): a derived test whose home file differs can never be
    // executed even when its name prefix-matches a filter.
    let derived = derive_ignored_churn_tests()?;
    let legs_data = leg_invocations();
    for (stem, name, gated) in &derived {
        // Round-2 P4: coverage must bind the FEATURE axis too — a gated test
        // prefix-covered only by a no-feature invocation is compiled out of
        // that invocation and can never run while the gate stays green.
        let covered = stem == TEST_FILE
            && legs_data.iter().any(|(_, invs)| {
                invs.iter()
                    .any(|i| name.starts_with(i.filter) && (!gated || i.features.is_some()))
            });
        if !covered {
            return Err(format!(
                "{GATE_NAME}: FAIL — `{name}` (feature-gated: {gated}) will never run: either this gate never invokes its binary `{stem}`, or no leg both names it AND compiles it (AC5.2)"
            ));
        }
    }
    // 3. Per-leg oracles (each its OWN invocation(s) — per-leg independence).
    let mut legs: Vec<LegResult> = legs_data
        .iter()
        .map(|(label, invocations)| run_leg(label, invocations))
        .collect();
    // 4. Vacuous-green guard (J4/L6 anti-canned): a leg that was ATTEMPTED but
    //    compiled to ZERO tests / never reported results is a re-stubbed
    //    harness — hard-fail at EVERY phase. kernel-abi-diff is exempt
    //    (baseline, not a test count).
    legs.push(run_kernel_abi_leg());
    for leg in &legs {
        if leg.label != "kernel-abi-diff"
            && leg.attempted
            && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}) — a re-stubbed harness cannot pass this gate (J4 anti-canned guard)",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = legs.iter().all(|l| l.green);

    // 5. Verdict. `dev_blocks` is serialized (AC6.4): the JSON verdict and
    // the stderr banner can no longer contradict each other.
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
                    "ship_phase": CURRENT_PHASE,
                    "disposition": disposition,
                    "enrolled_churn_tests": derived.len(),
                    "legs": legs_json(&legs),
                })
            );
        } else {
            eprintln!(
                "{GATE_NAME}: PASSED — oracle green ({} legs, {} enrolled churn tests); {} at {}",
                legs.len(),
                derived.len(),
                if dev_blocks { "BLOCKING" } else { "advisory" },
                CURRENT_PHASE,
            );
            // AC3.4.a — publish the collected percentile lines beside the
            // banner (non-JSON surface): verbatim, no values of the gate's
            // own invention; identity/churn legs print none.
            for line in legs.iter().flat_map(|l| &l.percentiles) {
                eprintln!("{GATE_NAME}:   {line}");
            }
        }
        return Ok(());
    }

    // Oracle RED — dev-time enforcement: `BindingClass::Blocking` hard-fails a
    // RED oracle at HEAD regardless of CURRENT_PHASE
    // (`gate_common::dev_enforced_red_blocks(BindingClass::Blocking, _)` is
    // unconditionally true). The whole-gate advisory tail that used to sit
    // below this point was therefore STRUCTURALLY UNREACHABLE dead code —
    // deleted, never revived (14-1 AC6.1): a whole-gate advisory flag with no
    // per-leg discrimination is D20's exact shape, and AC6.3 forbids
    // re-arming it here (per-leg `held_advisory_reason` is the only lawful
    // hold, and only on a measured contradiction).
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
    if !json {
        eprintln!("{msg}");
    }
    Err(format!(
        "{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE}"
    ))
}
