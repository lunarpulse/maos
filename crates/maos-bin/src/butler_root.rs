//! Story 18-1 (D-18-1-04 A; review BH-2/EC-2) — the `--once` verdict on a
//! Butler pass. In the library, not `main.rs`, so `crates/maos-bin/tests/`
//! drives the production function (an in-`src` test module is budget-charged).

use maos_kernel_core::scheduler::HookOutcome;
use maos_kernel_core::security::OutputShapePredicate;

/// A refused pass, a bound pass the dispatcher did not run to completion
/// (budget exceeded, panicked — its blocking thread may still be running) and
/// an `output_shape` violation are all refusals the root exits non-zero on.
pub fn pass_verdict(
    butler: Option<&butler::Butler>,
    outcome: &HookOutcome,
    output: Option<&serde_json::Value>,
    predicate: &OutputShapePredicate,
) -> Result<(), String> {
    if let Some(error) = butler.and_then(butler::Butler::take_pass_error) {
        return Err(format!("butler on_idle pass refused: {error}"));
    }
    let completed = matches!(
        outcome,
        HookOutcome::Fired { .. } | HookOutcome::BudgetWarning80 { fired: true, .. }
    );
    if butler.is_some_and(butler::Butler::is_inference_bound) && !completed {
        return Err(format!("butler on_idle pass did not complete: {outcome:?}"));
    }
    match output.map(|output| predicate.check(output)) {
        Some(Err(violation)) => Err(format!("output_shape violation: {violation}")),
        _ => Ok(()),
    }
}
