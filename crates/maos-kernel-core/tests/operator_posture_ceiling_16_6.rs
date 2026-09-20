//! Story 16-6 (§16a R3, §17 R10) — the operator's posture ceiling.
//!
//! `maosctl load` is the first runtime path a manifest has into a live root:
//! 16-1's `EndpointInUse` fast-fail killed the "they could just exec
//! `maos run`" equivalence, so the manifest's own `[posture] allowed_max`
//! became the attacker's own number. Posture was the fifth axis of the leash
//! and the only one of five with no operator floor — `global_sandbox_floor`,
//! `spirit_tier_floor`, `resource_cap_floor` and the two subtract-only
//! capability deny sets all had one.
//!
//! The escalation these tests close, all shipped verbs and no new code:
//! read `<home>/control.json` as the operator uid, write a manifest naming a
//! compiled-in class with `allowed_max = autonomous-with-halt`, POST it, and
//! `maosctl posture --shift autonomous-with-halt` then passes
//! `shift_posture`'s ceiling check because the ceiling is the value the
//! manifest supplied.

use std::sync::Arc;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::manifest::Posture;
use maos_kernel_core::security::posture::PostureError;
use maos_kernel_core::security::{
    CapabilitiesRequired, ClassSection, EpistemicPolicySection, PostureSection, ResourceCaps,
    SandboxConfig, SecurityManagerAdapter,
};

struct Harness {
    security: SecurityManagerAdapter,
    policy: Arc<PolicyTable>,
    journal: JournalAdapter,
    _scratch: tempfile::TempDir,
}

fn harness(ceiling: Option<Posture>) -> Harness {
    cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T0);
    inner.operator_policy.operator_posture_ceiling = ceiling;
    policy.update(inner);
    let scratch = tempfile::TempDir::new().expect("scratch dir");
    let journal =
        JournalAdapter::open(&scratch.path().join("journal.ndjson")).expect("journal opens");
    Harness {
        security: SecurityManagerAdapter::new(Arc::clone(&policy)),
        policy,
        journal,
        _scratch: scratch,
    }
}

fn class() -> ClassSection {
    ClassSection {
        name: "escalator".into(),
        version: "0.1.0".into(),
        abi: "1.0".into(),
        manifest_schema_version: maos_spirit_abi::MANIFEST_SCHEMA_VERSION,
        min_substrate_version: "0.0.1".into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "posture ceiling fixture".into(),
    }
}

/// Admit a Spirit whose manifest declares `default` / `allowed_max`.
fn admit(h: &Harness, pid: u32, default: Posture, allowed_max: Posture) {
    h.security
        .admit_spirit(
            pid,
            "escalator",
            &SandboxConfig {
                tier: SandboxTier::T0,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &CapabilitiesRequired {
                provider: maos_kernel_core::security::ProviderCapabilities { complete: vec![] },
                mcp: maos_kernel_core::security::manifest::McpCapabilities { servers: vec![] },
                loom: maos_kernel_core::security::manifest::LoomCapabilities::default(),
            },
            None,
            &h.journal,
            &PostureSection {
                default,
                allowed_max,
            },
            Some(&EpistemicPolicySection::default_open_fail()),
            None,
            None,
            None,
            None,
            None,
            Some(&class()),
        )
        .expect("admission must succeed — the clamp lowers, it never refuses");
}

fn stored(h: &Harness, pid: u32) -> (Posture, Posture) {
    let inner = h.policy.inner().load_full();
    let state = inner
        .spirit_postures
        .get(&pid)
        .expect("admitted Spirit must have a posture state");
    (state.current, state.allowed_max)
}

/// Obligation (z) — the escalation must be seen WORKING before it is closed.
/// A clamp never seen bypassed is not a control.
#[test]
fn without_an_operator_ceiling_a_manifest_picks_its_own() {
    let h = harness(None);
    admit(&h, 9001, Posture::Assistive, Posture::AutonomousWithHalt);

    assert_eq!(
        stored(&h, 9001),
        (Posture::Assistive, Posture::AutonomousWithHalt),
        "with no ceiling configured the manifest's own numbers are stored verbatim"
    );
    h.policy
        .shift_posture(9001, Posture::AutonomousWithHalt)
        .expect("the manifest-supplied ceiling admits the shift — this is the escalation");
}

/// §16a's falsifier — the operator ceiling is the strictest-of, and the shift
/// the escalation depends on now fails closed.
#[test]
fn an_operator_ceiling_clamps_the_manifest_and_refuses_the_shift() {
    let h = harness(Some(Posture::Assistive));
    admit(&h, 9002, Posture::Assistive, Posture::AutonomousWithHalt);

    assert_eq!(
        stored(&h, 9002),
        (Posture::Assistive, Posture::Assistive),
        "the declared ceiling must be clamped to the operator's"
    );
    let error = h
        .policy
        .shift_posture(9002, Posture::AutonomousWithHalt)
        .expect_err("the shift must now fail closed");
    assert!(
        matches!(
            error,
            PostureError::AboveCeiling {
                requested: Posture::AutonomousWithHalt,
                allowed: Posture::Assistive,
            }
        ),
        "expected AboveCeiling against the clamped ceiling, got {error:?}"
    );
}

/// Obligation (ac) — the R10 inversion. Clamping `allowed_max` alone would
/// store `current > allowed_max` for any class whose declared default
/// outranks the ceiling, and the Spirit would run ABOVE its own ceiling.
/// `orchestrator` declares `default = autonomous-with-halt`, so it is the
/// class that exhibits it.
#[test]
fn a_default_above_the_ceiling_is_clamped_too_never_left_inverted() {
    let h = harness(Some(Posture::Cautious));
    admit(
        &h,
        9003,
        Posture::AutonomousWithHalt,
        Posture::AutonomousWithHalt,
    );

    let (current, allowed_max) = stored(&h, 9003);
    assert_eq!(
        (current, allowed_max),
        (Posture::Cautious, Posture::Cautious),
        "BOTH fields are clamped — clamping only allowed_max is the R10 inversion"
    );
    assert!(
        current <= allowed_max,
        "the ceiling invariant must hold after admission: current {current:?} \
         must not outrank allowed_max {allowed_max:?}"
    );
}

/// The clamp lowers; it never raises. A ceiling looser than the manifest is a
/// no-op, so an operator cannot widen a Spirit's leash by configuring one.
#[test]
fn a_ceiling_looser_than_the_manifest_changes_nothing() {
    let h = harness(Some(Posture::Autonomous));
    admit(&h, 9004, Posture::Cautious, Posture::Assistive);

    assert_eq!(
        stored(&h, 9004),
        (Posture::Cautious, Posture::Assistive),
        "min() is monotone-down: a looser ceiling must not raise either field"
    );
}
/// Story 16-6 review 2026-09-20 — changing the operator ceiling after
/// admission must re-clamp existing Spirits, not merely affect future loads.
#[test]
fn updating_the_operator_ceiling_reclamps_existing_spirit_postures() {
    let h = harness(None);
    admit(
        &h,
        9005,
        Posture::AutonomousWithHalt,
        Posture::AutonomousWithHalt,
    );
    assert_eq!(
        stored(&h, 9005),
        (Posture::AutonomousWithHalt, Posture::AutonomousWithHalt)
    );

    h.policy
        .set_operator_posture_ceiling(Some(Posture::Cautious));

    assert_eq!(
        stored(&h, 9005),
        (Posture::Cautious, Posture::Cautious),
        "the runtime ceiling update must clamp BOTH current and allowed_max"
    );
}

/// Obligation (ab) / §11 row 7 — stated, not assumed: the manifest's
/// `[sandbox] tier` is parsed and DISCARDED on the admission path
/// (`effective_sandbox_tier` seeds the manifest leg from
/// `manifest_scopes[pid].declared_tier`, absent on first admission), so a
/// manifest cannot select its own sandbox tier and AC1 must not promise a
/// T1/T3/T4 refusal vector.
#[test]
fn a_manifest_declared_sandbox_tier_is_ignored_on_first_admission() {
    let h = harness(None);
    h.security
        .admit_spirit(
            9005,
            "escalator",
            &SandboxConfig {
                tier: SandboxTier::T4,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &CapabilitiesRequired {
                provider: maos_kernel_core::security::ProviderCapabilities { complete: vec![] },
                mcp: maos_kernel_core::security::manifest::McpCapabilities { servers: vec![] },
                loom: maos_kernel_core::security::manifest::LoomCapabilities::default(),
            },
            None,
            &h.journal,
            &PostureSection {
                default: Posture::Cautious,
                allowed_max: Posture::Assistive,
            },
            Some(&EpistemicPolicySection::default_open_fail()),
            None,
            None,
            None,
            None,
            None,
            Some(&class()),
        )
        .expect("a T4-declaring manifest is admitted, not refused — the tier is discarded");

    let inner = h.policy.inner().load_full();
    let effective = inner
        .manifest_scopes
        .get(&9005)
        .expect("admitted Spirit has a manifest scope")
        .declared_tier;
    assert_eq!(
        effective,
        SandboxTier::DEFAULT_FLOOR,
        "first admission has no prior manifest scope, so its declared tier \
         determinately starts at the policy default floor"
    );
}
