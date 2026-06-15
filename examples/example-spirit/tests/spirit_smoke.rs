//! Smoke test for ExampleSpirit — fires `on_idle` through the SpiritTest
//! harness and asserts the hook fired exactly once using the v0.5 binding macros.

use example_spirit::__maos_spirit_vtable_ExampleSpirit;
use example_spirit::ExampleSpirit;
use maos_spirit_sdk::spirit_test::{assert, assert_no_deprecations, SpiritTest};

#[test]
fn on_idle_fires_once() {
    let spirit = ExampleSpirit;
    let vtable = __maos_spirit_vtable_ExampleSpirit();
    let mut harness = SpiritTest::new(&spirit, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert!(
        report.base.hooks_fired.get("on_idle").copied().unwrap_or(0) == 1,
        "on_idle should fire exactly once during a default fixture run"
    );
    assert_no_deprecations!(report);
}
