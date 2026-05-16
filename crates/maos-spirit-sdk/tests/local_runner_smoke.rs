#![cfg(feature = "local_runner")]

//! Smoke test for local_runner — fires on_idle + on_frame through
//! a #[spirit]-derived TestSpirit and asserts the report shape.

use maos_spirit_sdk::spirit;
use maos_spirit_sdk::{Ctx, Spirit};
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};

pub struct TestSpirit;

#[spirit]
impl TestSpirit {
    fn on_idle(&self, _ctx: &mut Ctx) {}
}

#[test]
fn on_idle_fires_once() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let f = LocalRunnerFixture { invoke_on_idle: true, ..Default::default() };
    let r = LocalRunner::run(&s, v, &f);
    assert_eq!(r.hooks_fired.get("on_idle").copied().unwrap_or(0), 1);
    assert_eq!(r.hooks_fired.get("on_frame").copied().unwrap_or(0), 0);
    assert!(r.mock_bus_frames.is_empty(), "v0.3 prerequisite: no frames expected");
    assert!(r.elapsed_per_hook.contains_key("on_idle"));
}

#[test]
fn frames_fire_per_entry() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let f = LocalRunnerFixture {
        frames: vec![b"f0".to_vec(), b"f1".to_vec(), b"f2".to_vec()],
        ..Default::default()
    };
    let r = LocalRunner::run(&s, v, &f);
    assert_eq!(r.hooks_fired.get("on_frame").copied().unwrap_or(0), 3);
}

#[test]
fn report_default_is_empty() {
    let r = maos_spirit_sdk::local_runner::RunReport::default();
    assert!(r.hooks_fired.is_empty());
    assert!(r.mock_bus_frames.is_empty());
    assert!(r.elapsed_per_hook.is_empty());
}
