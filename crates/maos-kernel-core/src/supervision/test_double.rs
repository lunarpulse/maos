#![forbid(unsafe_code)]

//! Test double for `SubprocessSupervisor` — drives the SIGKILL crash corpus
//! without spawning real subprocesses.
//!
//! Same pattern as `MockLifecycleResolver` (Story 5.1) — lives under
//! `pub mod test_double` (NOT `#[cfg(test)]`) so integration tests can use it.
//! `xtask check-mock-not-in-release` excludes the symbol from release builds.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use maos_domain::supervision::{ChildExitStatus, ChildHandle, SubprocessSupervisor, SupervisionError};

/// Test double: `wait_for_exit` resolves with a pre-configured exit status.
#[maos_attrs::i9_exempt(reason = "test double — transient per-test state; production wiring uses real SubprocessSupervisor impl")]
pub struct SimulatedChildSupervisor {
    next_handle: Mutex<u64>,
    exit_status: Mutex<Option<ChildExitStatus>>,
}

impl SimulatedChildSupervisor {
    pub fn new() -> Self {
        Self {
            next_handle: Mutex::new(1),
            exit_status: Mutex::new(None),
        }
    }

    /// Configure the exit status for the next `wait_for_exit` call.
    pub fn set_next_exit(&self, status: ChildExitStatus) {
        *self.exit_status.lock().expect("lock poisoned") = Some(status);
    }
}

impl SubprocessSupervisor for SimulatedChildSupervisor {
    fn spawn_child(&self, _spirit_id: &str) -> Result<ChildHandle, SupervisionError> {
        let mut h = self.next_handle.lock().expect("lock poisoned");
        let handle = *h;
        *h += 1;
        Ok(handle)
    }

    fn wait_for_exit(
        &self,
        _child: ChildHandle,
    ) -> Pin<Box<dyn Future<Output = ChildExitStatus> + Send>> {
        let status = self.exit_status.lock().expect("lock poisoned").take().unwrap_or(ChildExitStatus::CleanEof);
        Box::pin(async move { status })
    }
}
