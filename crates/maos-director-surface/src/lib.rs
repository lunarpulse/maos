#![forbid(unsafe_code)]

//! `maos-director-surface` — kernel-adjacent notification dispatcher.
//!
//! Per architecture §7.4, notification events are kernel-rendered, not
//! Spirit-rendered. This crate provides the `NotificationDispatcher` that
//! fans out kernel events (task assignment, approval prompts, halts) to
//! registered notification channels (terminal, ACP editor, mobile push).
//!
//! At v0.3-β only `TerminalChannel` (stderr) ships real; ACP-editor and
//! mobile-push channels are scaffolded as stubs naming their owning stories
//! (Story 5.5c / Story 6.5).
//!
//! # Service classification per §4.0.8
//!
//! This crate is a **kernel-adjacent service**, not a supervised kernel
//! service:
//! - P1 ✅ own crate
//! - P2 ❌ no own bin
//! - P3 ❌ no IPC proto
//! - P4 ❌ no independent restart at v0.3
//!
//! Eligible for promotion at v0.5+ if surface contention justifies extraction.

pub mod notification;
pub mod halt_ui;
