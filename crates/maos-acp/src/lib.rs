#![forbid(unsafe_code)]

//! `maos-acp` — Agent Communication Protocol server (NDJSON over stdio).
//!
//! Per architecture §7.5 + appendix-a-cohort-prior-art-map.md (convergent
//! across openclaw / opencode / hermes; ratified by Zed).
//!
//! Consumer-facing surface: `AcpServer::new(lifecycle, halts).run(stdin, stdout)`.

pub mod frame;
#[cfg(any(test, feature = "fixture_replay"))]
pub mod fixture_replay;
pub mod notification_channel;
pub mod server;

pub use frame::{AcpFrameIn, AcpFrameOut, DecisionId, SessionId};
pub use notification_channel::AcpEditorChannelImpl;
pub use server::{AcpError, AcpServer};
