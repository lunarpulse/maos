//! Story 12.3 — pin the single halt-receipt OBSERVER call site and the absence
//! of any arbitration sink in the router (AC3, observability NOT arbitration).
//!
//! The load-bearing "no arbitration" guarantee is the DEPENDENCY GRAPH
//! (`maos-cohort` ↛ `maos-kernel-core`, verified by `check-dependency-closure`),
//! so `HaltRegistry::resolve` / `KernelHaltResolver` are graph-unreachable and
//! any receipt the observer holds is inert (P6). This chokepoint REINFORCES that
//! by pinning the routing: exactly ONE observer call site, and the router names
//! no arbitration sink.

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn halt_receipt_observed_at_one_site_with_no_arbitration_sink() {
    let router = strip_line_comments(include_str!("../src/router.rs"));

    // Exactly one halt-receipt observer call site — a bug that adds a second
    // (e.g. also observing on the unverified `handle_intake` path) reds here.
    assert_eq!(
        router.matches(".observe_receipt(").count(),
        1,
        "exactly one halt-receipt observer call site"
    );

    // The single observation is bound to the injected port (P5r verified path).
    assert!(
        router.contains("halt_receipt_observer"),
        "the observer is the injected halt_receipt_observer port"
    );

    // The router names NO arbitration sink on the halt-receipt path (AC3). These
    // live in `maos-kernel-core` and are graph-unreachable from the observer's
    // crate; the router must never reference them either.
    assert_eq!(
        router.matches("HaltRegistry").count(),
        0,
        "router names no halt arbitration registry"
    );
    assert_eq!(
        router.matches("KernelHaltResolver").count(),
        0,
        "router names no kernel halt resolver"
    );
}
