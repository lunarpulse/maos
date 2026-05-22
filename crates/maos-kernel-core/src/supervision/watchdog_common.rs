#![forbid(unsafe_code)]

//! Shared watchdog utilities — poll cadence selection.
//!
//! Story 5.3.

pub(crate) fn pick_poll_cadence() -> std::time::Duration {
    let fast = std::env::var_os("MAOS_SUPERVISION_FAST").is_some();
    if fast {
        std::time::Duration::from_millis(100)
    } else {
        std::time::Duration::from_secs(1)
    }
}
