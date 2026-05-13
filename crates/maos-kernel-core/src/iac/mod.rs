#![forbid(unsafe_code)]

//! IAC Bus — supervised service per §4.5.
//!
//! Routes frames between Spirits and the kernel. At v0.1-α this is an
//! empty hexagonal adapter shell; Story 6.1 lands the full IAC Bus
//! with retract primitive and DRR fairness scheduler.
//!
//! Story 1b.1 lands the Transparency Log + Approval Decision Log
//! adapter (`TransparencyLogAdapter`) in the I9-sanctioned single-file
//! holder `transparency_log.rs`, plus the redaction filter and mailbox
//! stub.

pub mod transparency_log;
pub mod redaction;
pub mod mailbox_stub;

pub use maos_domain::ports::IacBusPort;

pub use transparency_log::{
    TransparencyLogAdapter, FrameKind, FrameFilter, TransparencyLogEntry, AuditError,
};
pub use redaction::{RedactionPolicy, CorpusBackedRedactionPolicy};
pub use mailbox_stub::MailboxStub;

/// Adapter shell — Story 6.1 implements `IacBusPort` for this type
/// with frame routing, transparency logging, and fairness scheduling.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
/// Story 1b.1 ships `TransparencyLogAdapter` as the runtime I2 audit-spine.
#[derive(Debug, Clone, Copy, Default)]
pub struct IacBusAdapter;
