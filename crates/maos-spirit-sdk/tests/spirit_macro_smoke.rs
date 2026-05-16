//! End-to-end `#[spirit]` macro test.
//!
//! Verifies: trait impl exists, vtable accessible via __maos_spirit_vtable_TestSpirit(),
//! undeclared hooks are no-ops, on_idle is invokable through the vtable.

use maos_spirit_sdk::spirit;
use maos_spirit_sdk::{Ctx, Spirit, SpiritVtable};

pub struct TestSpirit;

#[spirit]
impl TestSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        let _ = ctx.cancellation().is_cancelled();
    }
}

#[test]
fn test_spirit_implements_trait() {
    let s = TestSpirit;
    let mut ctx = Ctx::mock();
    s.on_load(&mut ctx);
    s.on_start(&mut ctx);
    s.on_idle(&mut ctx);
    s.on_pause(&mut ctx);
    s.on_resume(&mut ctx);
    s.on_unload(&mut ctx);
}

#[test]
fn test_spirit_vtable_exists_and_dispatch_works() {
    let s = TestSpirit;
    let vtable = __maos_spirit_vtable_TestSpirit();
    let mut ctx = Ctx::mock();

    (vtable.on_idle)(&s, &mut ctx);
    (vtable.on_load)(&s, &mut ctx);
    (vtable.on_start)(&s, &mut ctx);
}

#[test]
fn test_all_undeclared_hooks_are_noops() {
    let s = TestSpirit;
    let vtable = __maos_spirit_vtable_TestSpirit();
    let mut ctx = Ctx::mock();

    let data = b"test";
    let fp = maos_spirit_sdk::FramePayload { frame_data: data, frame_len: 4 };
    (vtable.on_frame)(&s, &mut ctx, &fp);

    let tp = maos_spirit_sdk::TelemetryEventPayload { event_data: data, event_len: 4 };
    (vtable.on_telemetry_event)(&s, &mut ctx, &tp);

    let sp = maos_spirit_sdk::SchedulePayload { schedule_data: data, schedule_len: 4 };
    (vtable.on_schedule)(&s, &mut ctx, &sp);

    let sip = maos_spirit_sdk::SwapInPayload { predecessor_state: data, state_len: 4 };
    (vtable.on_swap_in)(&s, &mut ctx, &sip);

    let cp = maos_spirit_sdk::ConsolidatePayload { batch_data: data, batch_len: 4 };
    (vtable.on_consolidate)(&s, &mut ctx, &cp);

    (vtable.on_pause)(&s, &mut ctx);
    (vtable.on_resume)(&s, &mut ctx);
    (vtable.on_unload)(&s, &mut ctx);
}

#[test]
fn test_static_vtable_returns_same_ref() {
    let v1 = __maos_spirit_vtable_TestSpirit();
    let v2 = __maos_spirit_vtable_TestSpirit();
    let p1: *const SpiritVtable<TestSpirit> = v1;
    let p2: *const SpiritVtable<TestSpirit> = v2;
    assert_eq!(p1, p2, "vtable should return the same static reference");
}

#[test]
fn test_spirit_name_default_is_type_name() {
    assert!(
        __MAOS_SPIR_NAME.contains("TestSpirit"),
        "__MAOS_SPIR_NAME should contain the type name, got: {}",
        __MAOS_SPIR_NAME
    );
}
