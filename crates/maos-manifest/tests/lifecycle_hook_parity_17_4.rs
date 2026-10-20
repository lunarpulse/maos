#![forbid(unsafe_code)]

//! Behavioral parity with the ABI's complete, compile-checked lifecycle surface.

use maos_manifest::LifecycleSection;
use maos_spirit_abi::lifecycle::Spirit;

struct AbiSpirit;
impl Spirit for AbiSpirit {}

#[test]
fn manifest_accepts_every_abi_hook_and_rejects_a_non_hook() {
    // Each identifier must resolve to a real ABI method, not just a string.
    macro_rules! abi_hooks {
        ($($hook:ident),+ $(,)?) => {{
            $(let _ = <AbiSpirit as Spirit>::$hook;)+
            [$(stringify!($hook)),+]
        }};
    }
    let hooks = abi_hooks![
        on_load,
        on_start,
        on_frame,
        on_idle,
        on_telemetry_event,
        on_schedule,
        on_swap_in,
        on_pause,
        on_resume,
        on_unload,
        on_consolidate,
        on_swap_out,
        snapshot,
        migrate,
    ];
    assert_eq!(hooks.len(), maos_spirit_abi::count_hooks!());
    for hook in hooks {
        let parsed = LifecycleSection::from_toml_str(&format!("enabled_hooks = [{hook:?}]"));
        assert!(
            parsed.is_ok(),
            "ABI hook {hook:?} must validate: {parsed:?}"
        );
        assert_eq!(parsed.unwrap().enabled_hooks, [hook]);
    }
    assert!(
        LifecycleSection::from_toml_str("enabled_hooks = [\"not_an_abi_hook\"]").is_err(),
        "a fifteenth, non-ABI hook must not validate"
    );
}
