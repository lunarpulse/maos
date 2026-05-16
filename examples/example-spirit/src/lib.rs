#![forbid(unsafe_code)]

//! example-spirit — a MAOS Spirit scaffolded from templates/spirit-rust.
//!
//! Edit `on_idle` to implement your Spirit's idle-time behavior. See
//! README.md for the 30-minute first-Spirit path.

use maos_spirit_sdk::{spirit, Ctx, Spirit};

pub struct ExampleSpirit;

#[spirit]
impl ExampleSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        // Bail early if the kernel has signaled cancellation.
        if ctx.cancellation().is_cancelled() {
            return;
        }
        // TODO: implement your Spirit's idle behavior here.
    }
}
