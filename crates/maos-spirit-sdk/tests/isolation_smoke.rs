#![cfg(feature = "spirit_test")]

//! Smoke test for the NFR-Sec-14 isolation framework — constructs a
//! 2-Spirit fixture, runs a trivial attack case through DefaultIsolationHook,
//! asserts the 4 hook points fire in order + the returned outcome is
//! well-formed.

use maos_spirit_sdk::spirit_test::{
    CrossSpiritIsolationFixture, DefaultIsolationHook, IsolationAttackCase, IsolationAttackCategory,
};

mod spirit_a {
    use maos_spirit_sdk::{spirit, Ctx, FramePayload, Spirit};

    pub struct SpiritA;
    #[spirit]
    impl SpiritA {
        fn on_frame(&self, _ctx: &mut Ctx, _payload: &FramePayload) {}
    }
}

mod spirit_b {
    use maos_spirit_sdk::{spirit, Ctx, Spirit};

    pub struct SpiritB;
    #[spirit]
    impl SpiritB {
        fn on_idle(&self, _ctx: &mut Ctx) {}
    }
}

use spirit_a::__maos_spirit_vtable_SpiritA;
use spirit_a::SpiritA;
use spirit_b::__maos_spirit_vtable_SpiritB;
use spirit_b::SpiritB;

#[test]
fn isolation_framework_fires_4_hook_points_in_order() {
    let a = SpiritA;
    let va = __maos_spirit_vtable_SpiritA();
    let b = SpiritB;
    let vb = __maos_spirit_vtable_SpiritB();
    let fixture = CrossSpiritIsolationFixture::new(&a, va, &b, vb);
    let mut hook = DefaultIsolationHook::default();
    let case = IsolationAttackCase {
        id: "iso-smoke-001".to_string(),
        category: IsolationAttackCategory::NamespaceEnumeration,
        attack_payload: b"smoke-attack".to_vec(),
        expected_isolation_maintained: true,
    };
    let outcome = fixture.run_attack_case(&case, &mut hook);
    assert_eq!(hook.records.len(), 4);
    assert_eq!(hook.records[0].hook_name, "before_spirit_a_attempt");
    assert_eq!(hook.records[1].hook_name, "after_spirit_a_attempt");
    assert_eq!(hook.records[2].hook_name, "before_spirit_b_observe");
    assert_eq!(hook.records[3].hook_name, "after_spirit_b_observe");
    assert_eq!(outcome.case_id, "iso-smoke-001");
    assert!(outcome.isolation_maintained);
    assert!(outcome
        .attempt_result
        .hooks_fired_during_attempt
        .iter()
        .any(|h| h == "on_frame"));
    assert!(outcome
        .observation_result
        .hooks_fired_during_observation
        .iter()
        .any(|h| h == "on_idle"));
}

#[test]
fn all_8_categories_constructible() {
    let _all = [
        IsolationAttackCategory::NamespaceEnumeration,
        IsolationAttackCategory::WorkingMemoryReadAcross,
        IsolationAttackCategory::DecisionFrameObservation,
        IsolationAttackCategory::HaltSignalObservation,
        IsolationAttackCategory::TransparencyLogCrossRead,
        IsolationAttackCategory::WorkingMemoryDigestCrossRead,
        IsolationAttackCategory::CapabilityTokenForgeryCrossSpirit,
        IsolationAttackCategory::SandboxEscapeLateral,
    ];
    assert_eq!(
        _all.len(),
        8,
        "architecture §8.1 + epic-4 line 17 enumerate exactly 8 categories"
    );
}
