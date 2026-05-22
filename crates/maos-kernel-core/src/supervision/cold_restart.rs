#![forbid(unsafe_code)]

//! Cold-restart recovery — graceful drain + hard-kill drain + in-flight recovery.
//!
//! Story 5.3 — AC6.
//!
//! v0.3-β: The smoke-supervision-5 arm exercises journal recovery
//! (`JournalAdapter::append_in_flight` + `recover_in_flight_with_tasks`).
//! The per-spirit drain functions are forward-shaped for Story 5.4's
//! `--policy cold-swap` graceful-restart path.

/// Drain report from a graceful or hard-kill drain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainReport {
    pub spirit_pid: u32,
    pub halts_drained: usize,
    pub in_flight_tasks_recovered: usize,
    pub tokens_revoked: usize,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum DrainError {
    #[error("spirit not loaded: pid={0}")]
    NotLoaded(u32),
    #[error("drain timed out after {0}s")]
    Timeout(u64),
}

/// Graceful drain for a single Spirit.
///
/// v0.3-β: returns a synthetic report; the real scheduler-driven drain
/// lands at Story 5.4 via `scheduler.unload(pid)`.
pub async fn graceful_drain(
    spirit_pid: u32,
    _timeout_seconds: u64,
) -> Result<DrainReport, DrainError> {
    Ok(DrainReport {
        spirit_pid,
        halts_drained: 0,
        in_flight_tasks_recovered: 0,
        tokens_revoked: 0,
    })
}

/// Hard-kill drain — fsync the journal so in-flight records survive.
///
/// v0.3-β: returns a synthetic report; production hard-kill integration
/// (SIGKILL + journal fsync) lands at Story 5.5x.
pub async fn hard_kill_drain(
    spirit_pid: u32,
    _timeout_seconds: u64,
) -> Result<DrainReport, DrainError> {
    Ok(DrainReport {
        spirit_pid,
        halts_drained: 0,
        in_flight_tasks_recovered: 0,
        tokens_revoked: 0,
    })
}
