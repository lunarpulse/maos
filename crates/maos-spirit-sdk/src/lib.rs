#![forbid(unsafe_code)]

//! `maos-spirit-sdk` — reference SDK for Spirit authors (ADR-002).
//!
//! Re-exports the proc-macro attribute `#[spirit]` from `maos-spirit-derive`
//! (the `serde`/`serde_derive` precedent: the user-facing crate re-exports
//! the macro from a sibling proc-macro-only crate).
//!
//! Facade pattern: `use maos_spirit_sdk::*;` gives the Spirit author the
//! full ABI surface, the proc-macro, and (with `std` feature) the Tokio
//! cancellation adapter.

pub use maos_spirit_derive::spirit;

// Re-export the full ABI surface so Spirit authors get everything from one import.
pub use maos_spirit_abi::{
    cancellation::{CancellationSignal, NeverCancel},
    compliance,
    ctx::{CapabilityHandle, Ctx, MailboxHandle},
    lifecycle::{
        ConsolidatePayload, FramePayload, HookBudgetKey, SchedulePayload, Spirit,
        SpiritVtable, SwapInPayload, TelemetryEventPayload,
    },
    ABI_VERSION,
};

#[cfg(feature = "std")]
pub mod cancellation;
