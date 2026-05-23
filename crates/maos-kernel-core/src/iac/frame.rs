#![forbid(unsafe_code)]

//! Re-exports of canonical IAC frame types from `maos-domain::frame`.
//!
//! The single source of truth for `IacFrame`, `FrameAddress`, `FramePayload`,
//! `TaskAssignPayload`, `PosturePreferences`, `PostureHint`, and stub payload
//! types lives at `maos-domain::frame`. This module re-exports them so
//! kernel-internal code can use a single import path.
//!
//! # Why re-export instead of redefine
//!
//! These types are referenced by the `IacBusPort` trait in `maos-domain`.
//! Redefining them here would create a duplicate type that the trait cannot
//! accept — Rust considers identically-shaped structs from different crates
//! as distinct types. The re-export ensures kernel-core code uses the
//! same `IacFrame` type as the port trait.

pub use maos_domain::frame::{
    ConsentEnvelope, ConsentRequestPayload, DecisionDispatchPayload, EpistemicHaltPayload,
    FrameAddress, FramePayload, IacFrame, PostureHint, PosturePreferences, RetractPayload,
    TaskAssignPayload, TaskCompletePayload, TelemetryEventPayload,
};
