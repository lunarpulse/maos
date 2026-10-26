#![forbid(unsafe_code)]

//! Story 18-5 AC5.5 — the ADR-016 context-token ledger (CR-1, D-18-5-08 A).

use maos_capability::context_budget::{is_exit, ContextBudget, ContextWindow, Crossing};
use maos_domain::invariants::i1::Scope;
use maos_domain::ports::context_budget::ContextLevel::{self, *};

const BUTLER: ContextWindow = ContextWindow {
    size: 8192,
    pressure_permille: 800,
    limit_permille: 950,
};
const RESEARCHER: ContextWindow = ContextWindow {
    size: 16384,
    pressure_permille: 800,
    limit_permille: 950,
};

fn levels(crossings: &[Crossing]) -> Vec<ContextLevel> {
    crossings.iter().map(|c| c.level).collect()
}

#[test]
fn thresholds_are_ceilings_at_8192_and_16384() {
    for (window, pressure_at, limit_at) in [(BUTLER, 6554, 7783), (RESEARCHER, 13108, 15565)] {
        let ledger = ContextBudget::default();
        assert_eq!(ledger.report(1, window, pressure_at - 1).0.level, Within);
        assert_eq!(ledger.report(1, window, pressure_at).0.level, Pressure);
        assert_eq!(ledger.report(1, window, limit_at - 1).0.level, Pressure);
        assert_eq!(ledger.report(1, window, limit_at).0.level, Limit);
        assert_eq!(ledger.report(1, window, window.size).0.level, Limit);
        assert_eq!(ledger.report(1, window, window.size + 1).0.level, Exhausted);
    }
}

#[test]
fn each_upward_crossing_signals_once_and_rearms_below_its_threshold() {
    let ledger = ContextBudget::default();
    let (status, crossings) = ledger.report(7, BUTLER, 4000);
    assert_eq!((status.level, status.crossed), (Within, None));
    assert!(crossings.is_empty());

    let (status, crossings) = ledger.report(7, BUTLER, 6600);
    assert_eq!((status.level, status.crossed), (Pressure, Some(Pressure)));
    assert_eq!(levels(&crossings), [Pressure]);
    assert_eq!(crossings[0].threshold_permille, 800);

    let (status, crossings) = ledger.report(7, BUTLER, 6700);
    assert_eq!((status.level, status.crossed), (Pressure, None));
    assert!(crossings.is_empty(), "the latch holds while above pressure");

    let (_, crossings) = ledger.report(7, BUTLER, 7800);
    assert_eq!(levels(&crossings), [Limit]);
    assert_eq!(crossings[0].threshold_permille, 950);

    let (status, crossings) = ledger.report(7, BUTLER, 8300);
    assert_eq!((status.level, status.crossed), (Exhausted, Some(Exhausted)));
    assert!(crossings.is_empty(), "exhaustion's signal is the refusal");
    assert!(ledger.exhausted(7));

    // Distil back below pressure: re-armed, exhaustion cleared.
    let (status, _) = ledger.report(7, BUTLER, 3000);
    assert_eq!(status.level, Within);
    assert!(!ledger.exhausted(7));
    assert_eq!(levels(&ledger.report(7, BUTLER, 6600).1), [Pressure]);
}

#[test]
fn a_jump_from_within_to_exhausted_emits_pressure_then_limit() {
    let ledger = ContextBudget::default();
    let (status, crossings) = ledger.report(3, RESEARCHER, 17204);
    assert_eq!(status.level, Exhausted);
    assert_eq!(levels(&crossings), [Pressure, Limit]);
}

#[test]
fn window_zero_is_accounting_off() {
    let off = ContextWindow { size: 0, ..BUTLER };
    let ledger = ContextBudget::default();
    let (status, crossings) = ledger.report(9, off, u32::MAX);
    assert_eq!((status.level, status.crossed), (Within, None));
    assert!(crossings.is_empty());
    assert!(!ledger.exhausted(9));
}

#[test]
fn the_gate_admits_only_the_exits_and_remove_clears_the_slot() {
    let ledger = ContextBudget::default();
    ledger.report(5, BUTLER, 9000);
    let exits = [
        Scope::ProviderInfer {
            provider: "anthropic".into(),
        },
        Scope::MemWrite {
            scope: "self".into(),
        },
        Scope::IacSend {
            peer_class: "broadcast".into(),
        },
    ];
    for scope in &exits {
        assert!(is_exit(scope) && ledger.admits(5, scope), "{scope:?}");
    }
    for scope in [
        Scope::McpCall {
            server: "s".into(),
            tool: "t".into(),
        },
        Scope::FsRead {
            subtree: "/tmp".into(),
        },
        Scope::LogRecall,
        Scope::DistillateWrite,
    ] {
        assert!(!ledger.admits(5, &scope), "{scope:?}");
        assert!(ledger.admits(6, &scope), "another pid is unaffected");
    }
    ledger.remove(5);
    assert!(!ledger.exhausted(5));
}
