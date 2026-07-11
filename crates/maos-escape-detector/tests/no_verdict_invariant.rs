//! Story 11.4b AC2 — the no-verdict invariant (structural-not-semantic).
//!
//! ADR-024 §2/§5/Gate: the kernel emits ONLY a raw structural fact. The
//! `CapAuditEvent::SandboxBlock` / `FrameKind::SandboxBlock` types carry
//! `{ spirit_pid, attempted_syscall, sandbox_tier }` and NOTHING more — no
//! `malice` / `verdict` / `severity` / `intent` field. Adding one reds this
//! test (and the `no-verdict-invariant` gate leg). The interpretation ("is this
//! malice?") lives in the user-space detector, never the kernel frame.

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::cap_audit::CapAuditEvent;
use maos_kernel_core::iac::transparency_log::FrameKind;

/// Compile-time + runtime structural assertion: the kernel sandbox-violation
/// emission type carries EXACTLY the three structural fields and none of the
/// forbidden verdict axes.
#[test]
fn sandbox_block_carries_no_verdict_field() {
    // COMPILE-TIME GUARD — this construction names EXACTLY the three fields the
    // structural-not-semantic boundary permits. If a `malice`/`verdict`/
    // `severity`/`intent` field is added to the variant, this construction FAILS
    // TO COMPILE (missing field) → the test binary does not build → the
    // `no-verdict-invariant` gate leg REDs. (ADR-024 §5; AC2.)
    let event = CapAuditEvent::SandboxBlock {
        spirit_pid: 4242,
        attempted_syscall: "unknown".to_string(),
        sandbox_tier: SandboxTier::T2,
    };

    // RUNTIME BELT-AND-SUSPENDERS — the Debug rendering carries the three
    // structural field names and NONE of the forbidden verdict axes.
    let dbg = format!("{event:?}");
    assert!(
        dbg.contains("spirit_pid"),
        "structural field present: {dbg}"
    );
    assert!(
        dbg.contains("attempted_syscall"),
        "structural field present: {dbg}"
    );
    assert!(
        dbg.contains("sandbox_tier"),
        "structural field present: {dbg}"
    );
    for forbidden in ["malice", "verdict", "severity", "intent"] {
        assert!(
            !dbg.contains(forbidden),
            "AC2 violation: CapAuditEvent::SandboxBlock carries forbidden verdict \
             axis '{forbidden}': {dbg}"
        );
    }
}

/// The frame discriminator the detector reads (kind=8) is pinned — a silent
/// renumbering of `FrameKind::SandboxBlock` would decouple the producer from the
/// consumer.
#[test]
fn frame_kind_sandbox_block_discriminator_is_8() {
    assert_eq!(FrameKind::SandboxBlock as i64, 8);
}
