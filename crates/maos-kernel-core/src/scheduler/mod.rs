#![forbid(unsafe_code)]

//! Spirit Scheduler — supervisor / composition root for the four
//! supervised services (Security / Memory / IAC / Capability).
//!
//! Per architecture §4.0.8 supervisor exception, this module satisfies
//! P1 (own crate at v0.5+), P2 (own bin target at v0.5+), and P4
//! (independently restartable) but is exempt from P3 (boundary manifest
//! in the standard shape — its boundary is the union of its children's).
//!
//! At v0.1-α this is an empty hexagonal adapter shell. The supervisor
//! itself lives in the `maos-bin` composition root (`#[tokio::main]`);
//! this module exposes the adapter type its port-trait surface will use
//! when Story 1b.1 lands lifecycle journal mechanics.
//!
//! See `maos_domain::ports::SpiritSchedulerPort` for the hexagonal port
//! contract (declared in `maos-domain` per ADR-010 to keep the domain
//! core async-runtime-free).

pub use maos_domain::ports::SpiritSchedulerPort;

/// Adapter shell — Story 1b.1 implements `SpiritSchedulerPort` for this
/// type with the supervisor's journal + supervised-services restart logic.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpiritSchedulerAdapter;
