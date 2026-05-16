//! no_std smoke test — verifies the ABI surface compiles without std.
//!
//! The test runner uses std (required by Cargo's test harness), but the
//! Spirit implementation only touches `maos-spirit-abi` no_std types.
//! The actual `#![no_std]` compilation gate is `cargo build -p maos-spirit-sdk --no-default-features`.

use maos_spirit_sdk::spirit;
use maos_spirit_sdk::{Ctx, Spirit};

pub struct NoStdSpirit;

#[spirit]
impl NoStdSpirit {
    fn on_load(&self, ctx: &mut Ctx) {
        let _ = ctx.cancellation().is_cancelled();
    }
}

#[test]
fn no_std_spirit_trait_impl_exists() {
    let s = NoStdSpirit;
    let mut ctx = Ctx::mock();
    s.on_load(&mut ctx);
    s.on_idle(&mut ctx);
    s.on_unload(&mut ctx);
}

#[test]
fn no_std_spirit_vtable_accessible() {
    let s = NoStdSpirit;
    let vtable = __maos_spirit_vtable_NoStdSpirit();
    let mut ctx = Ctx::mock();
    (vtable.on_load)(&s, &mut ctx);
    (vtable.on_idle)(&s, &mut ctx);
    (vtable.on_unload)(&s, &mut ctx);
}

#[test]
fn no_std_spirit_abi_version_accessible() {
    assert_eq!(maos_spirit_sdk::ABI_VERSION, 1);
}
