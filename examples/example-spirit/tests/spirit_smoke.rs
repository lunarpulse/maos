//! Smoke test for ExampleSpirit — fires `on_idle` through the local_runner
//! and asserts the hook fired exactly once.

use example_spirit::ExampleSpirit;
use example_spirit::__maos_spirit_vtable_ExampleSpirit;
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};

#[test]
fn on_idle_fires_once() {
    let spirit = ExampleSpirit;
    let vtable = __maos_spirit_vtable_ExampleSpirit();
    let fixture = LocalRunnerFixture {
        invoke_on_idle: true,
        ..Default::default()
    };
    let report = LocalRunner::run(&spirit, vtable, &fixture);
    assert_eq!(report.hooks_fired.get("on_idle").copied().unwrap_or(0), 1);
}
