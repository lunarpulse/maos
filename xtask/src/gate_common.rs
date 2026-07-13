#![forbid(unsafe_code)]

//! Shared utilities for conditional ship-gate modules (Stories 10.1b, 10.2).
//! Extracted during 10.2 review-patch deferred-item #32/#33 to DRY up date
//! validation and workflow-command emission across all gate modules.

use chrono::NaiveDate;
use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Option C — leg-level gate binding (Epic 12 retro B1, 2026-07-13).
//
// Two independent axes were previously conflated in one string (`CURRENT_PHASE`):
//   (1) the GA / ship-phase ladder — how far the product has SHIPPED, correctly
//       held at v1.5 by the two external holds (pen-test, export counsel); and
//   (2) DEV-TIME enforcement — whether a RED oracle hard-fails CI at HEAD.
//
// Keying (2) off (1) made ~7 hermetic v2.0 gates advisory at HEAD (a real red
// returned Ok() + a WOULD-HAVE-BLOCKED banner). Story 12.1 had to hand-build a
// phase-independent hard-fail carve-out to make `check-cohort-mesh` bind. This
// module promotes that carve-out into a shared `BindingClass` so dev-enforcement
// is governed by the binding class and the phase ladder governs ONLY GA
// disposition — the two axes separated. See `project_gate_binding_decay`.
// ---------------------------------------------------------------------------

/// Ship-phase ladder, oldest → newest. Centralized here so a phase advance is a
/// one-line edit, not a hunt across ~10 duplicated per-gate copies.
pub const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0", "v2_2"];

/// The GA / ship phase. Held at v1_5 by v1.5's two external holds (pen-test,
/// export counsel). This governs ONLY the GA ship-gate ladder (`is_blocking_at`)
/// — NEVER dev-time enforcement, which is governed by [`BindingClass`].
pub const CURRENT_PHASE: &str = "v1_5";

/// The GA disposition in effect at `phase`: the nearest explicitly-set
/// disposition at or before `phase` in [`PHASE_ORDER`] (a gate that is
/// `blocking` at v2_0 is also blocking at v2_2 unless it overrides).
pub fn phase_disposition<'a>(
    disposition: &'a HashMap<String, String>,
    phase: &str,
) -> Option<&'a str> {
    let idx = PHASE_ORDER.iter().position(|p| *p == phase)?;
    (0..=idx)
        .rev()
        .find_map(|i| disposition.get(PHASE_ORDER[i]).map(String::as_str))
}

/// GA-ladder blocking test: is this gate's disposition `blocking` (or
/// `blocking-when-present`) at `phase`? Used for the GA ship-gate aggregate,
/// NOT for dev-time enforcement.
pub fn is_blocking_at(disposition: &HashMap<String, String>, phase: &str) -> bool {
    matches!(
        phase_disposition(disposition, phase),
        Some("blocking") | Some("blocking-when-present")
    )
}

/// Read a gate's phase disposition map from `xtask/gate-registry.toml`.
/// Errors if the gate is absent or its disposition is empty (a registry defect).
pub fn read_disposition(gate_name: &str) -> Result<HashMap<String, String>, String> {
    let registry: crate::corpus_types::ShipGateRegistry =
        crate::corpus_types::load_toml(Path::new("xtask/gate-registry.toml"))
            .map_err(|e| format!("cannot read gate-registry.toml: {e}"))?;
    for entry in &registry.ship_gates {
        if entry.name == gate_name {
            if entry.disposition.is_empty() {
                return Err(format!("{gate_name} has an empty disposition"));
            }
            return Ok(entry.disposition.clone());
        }
    }
    Err(format!("{gate_name} not found in gate-registry.toml"))
}

/// Dev-time enforcement class for a gate's oracle, decoupled from [`CURRENT_PHASE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingClass {
    /// Hermetic — CI can run every leg with no external substrate. A RED oracle
    /// hard-fails at HEAD regardless of `CURRENT_PHASE` (Story 12.1's carve-out,
    /// promoted). This is the class every v2.0/v2.2 hermetic gate should carry so
    /// a planted red reds CI now, not "at v2.0".
    Blocking,
    /// Requires a substrate CI cannot always provision (Postgres, multi-region
    /// geo, a measurement engagement, a seccomp-capable kernel). Hard-fails when
    /// the substrate IS present and the oracle is RED; when the substrate is
    /// ABSENT, the caller emits a WOULD-HAVE-BLOCKED banner and passes advisory
    /// — never silent-green (E11 retro A2, advisory-substrate-gated).
    AdvisorySubstrate,
}

/// Whether a RED oracle must hard-fail CI under dev-time enforcement.
///
/// `substrate_present` is ignored for [`BindingClass::Blocking`] (always blocks)
/// and is the deciding factor for [`BindingClass::AdvisorySubstrate`]: block only
/// when the live substrate was actually available this run.
pub fn dev_enforced_red_blocks(class: BindingClass, substrate_present: bool) -> bool {
    match class {
        BindingClass::Blocking => true,
        BindingClass::AdvisorySubstrate => substrate_present,
    }
}

#[cfg(test)]
mod option_c_tests {
    use super::*;

    #[test]
    fn blocking_class_hard_fails_a_red_oracle_regardless_of_substrate() {
        // The decoupling invariant: a Blocking (hermetic) gate blocks a RED
        // oracle whether or not any substrate is present — i.e. regardless of
        // CURRENT_PHASE. This is the property that makes the ~6 hermetic v2.0
        // gates bind at HEAD (Epic 12 retro B1).
        assert!(dev_enforced_red_blocks(BindingClass::Blocking, true));
        assert!(dev_enforced_red_blocks(BindingClass::Blocking, false));
    }

    #[test]
    fn advisory_substrate_blocks_only_when_the_substrate_is_present() {
        // The substrate axis (E11 retro A2): block a RED oracle when the live
        // substrate was available; when it is absent, the caller emits a
        // WOULD-HAVE-BLOCKED banner and passes advisory — never silent-green.
        assert!(dev_enforced_red_blocks(
            BindingClass::AdvisorySubstrate,
            true
        ));
        assert!(!dev_enforced_red_blocks(
            BindingClass::AdvisorySubstrate,
            false
        ));
    }

    #[test]
    fn phase_disposition_inherits_the_nearest_earlier_phase() {
        let mut d = HashMap::new();
        d.insert("v1_0".to_string(), "advisory".to_string());
        d.insert("v2_0".to_string(), "blocking".to_string());
        // v1_5 has no explicit entry → inherits v1_0 = advisory.
        assert_eq!(phase_disposition(&d, "v1_5"), Some("advisory"));
        // v2_2 has no explicit entry → inherits v2_0 = blocking.
        assert_eq!(phase_disposition(&d, "v2_2"), Some("blocking"));
        assert!(!is_blocking_at(&d, "v1_5"));
        assert!(is_blocking_at(&d, "v2_0"));
    }
}

/// Validate that date strings are non-empty, parseable as ISO-8601 (YYYY-MM-DD),
/// and that `start <= end` (chronological ordering).
///
/// #32 (was deferred): the prior copy only checked `contains('-') && len >= 10`,
/// accepting impossible dates like `'2026-99-99'` and ignoring start<=end ordering.
/// Now uses `chrono::NaiveDate::parse_from_str` for real ISO-8601 validation.
pub fn validate_dates(
    start_label: &str,
    start: &str,
    end_label: &str,
    end: &str,
) -> Result<(), String> {
    let parse = |label: &str, s: &str| -> Result<NaiveDate, String> {
        if s.is_empty() {
            return Err(format!("{label} is empty"));
        }
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| format!("{label} is not a valid ISO-8601 date (YYYY-MM-DD): {s}"))
    };
    let start_date = parse(start_label, start)?;
    let end_date = parse(end_label, end)?;
    if end_date < start_date {
        return Err(format!(
            "{end_label} ({end}) is before {start_label} ({start}) — dates must be ordered"
        ));
    }
    Ok(())
}

/// #33: in JSON mode, commands go to stderr (stdout stays clean for JSON parsing);
/// the structured warning/error is also carried in the JSON payload fields so
/// programmatic consumers assert on the JSON, not stderr. In non-JSON mode
/// (production CI), commands go to stdout where Actions parses them.
pub fn emit_command(json: bool, level: &str, msg: &str) {
    if json {
        // #33: in JSON mode, workflow commands go to stderr so stdout stays clean
        // for JSON parsing. The structured warning/error is ALSO in the JSON payload
        // (callers add `advisory: true` / `failures: [...]` fields), so programmatic
        // consumers don't need to parse stderr. Actions only parses stdout commands
        // in non-JSON mode (production CI), which uses the else branch below.
        eprintln!("::{level}::{msg}");
    } else {
        // Production (non-JSON): stdout, where GitHub Actions parses workflow commands.
        println!("::{level}::{msg}");
    }
}
