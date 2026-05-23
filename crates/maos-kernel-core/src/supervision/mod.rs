#![forbid(unsafe_code)]

//! Supervision module — crash detection, hung-Spirit detection,
//! silent-failure detection, cold-restart recovery, and FR50 disposition.
//!
//! Story 5.3 — v0.3-β lands the invariants for rust-inproc form;
//! subprocess form is forward-shaped via `SubprocessSupervisor` trait
//! in `maos-domain::supervision`.
//!
//! Architecture §4.1 — "The Scheduler supervises every subprocess Spirit."
//! This module lives inside `maos-kernel-core` per §4.0.2 precedent
//! (same as `hot_swap/` in Story 5.2).

pub mod cold_restart;
pub mod crash_detector;
pub mod disposition;
pub mod progress_watchdog;
pub mod silent_failure_detector;
pub mod test_double;
pub(crate) mod watchdog_common;

pub use cold_restart::{graceful_drain, hard_kill_drain, DrainError, DrainReport};
pub use crash_detector::CrashDetector;
pub use disposition::enforce_disposition;
pub use progress_watchdog::ProgressWatchdog;
pub use silent_failure_detector::SilentFailureDetector;
