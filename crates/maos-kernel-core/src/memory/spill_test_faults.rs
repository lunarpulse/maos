//! Test-only fault injection for the private spill transaction (TI-1/2/3).
//!
//! Excluded from the kernel-core kloc count by `xtask/src/kloc_check.rs` (the
//! gate measures *production* code only; this is cfg(test/debug_assertions)
//! scaffolding). Ratified 2026-08-03 (Winston): a measurement-scope
//! refinement consistent with the gate's stated intent — NOT a ceiling
//! increase. The whole module is included only behind
//! `#[cfg(any(test, debug_assertions))]` at the `mod` declaration in
//! `memory/mod.rs`, so it is absent from release artifacts.
//!
//! State is **thread-local** so the parallel test suite never crosses wires:
//! each fault test arms the fault on the very thread that runs the spill
//! transaction (TI-1/TI-3 arm on the test thread; TI-2 arms inside the worker
//! thread that holds the io_lock). `disarm()` clears the arming thread's
//! state between cases.

use std::cell::RefCell;
use std::sync::mpsc::{Receiver, Sender};

use maos_domain::memory::MemoryError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePoint {
    TempFileSync,
    Rename,
    CommitDirectorySync,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PausePoint {
    BeforeRename,
    BeforeCandidateOpen,
    BeforeDirectoryOpen(String),
}

#[derive(Debug)]
enum Fault {
    Failure(FailurePoint),
    Pause(PausePoint, Sender<()>, Receiver<()>),
}

thread_local! {
    static FAULT: RefCell<Option<Fault>> = const { RefCell::new(None) };
}

/// Variant key so a `BeforeDirectoryOpen(name)` pause matches regardless of
/// the namespace name the production path passes.
fn pause_kind(p: &PausePoint) -> u8 {
    match p {
        PausePoint::BeforeRename => 0,
        PausePoint::BeforeCandidateOpen => 1,
        PausePoint::BeforeDirectoryOpen(_) => 2,
    }
}

pub fn arm_failure(point: FailurePoint) {
    FAULT.with(|fault| *fault.borrow_mut() = Some(Fault::Failure(point)));
}

pub fn arm_pause(point: PausePoint, arrived: Sender<()>, release: Receiver<()>) {
    FAULT.with(|fault| *fault.borrow_mut() = Some(Fault::Pause(point, arrived, release)));
}

/// Disarm any armed fault on the calling thread (call between cases).
pub fn disarm() {
    FAULT.with(|fault| *fault.borrow_mut() = None);
}

/// If a failure is armed for `point`, consume it and return an error;
/// otherwise `Ok(())`. A fault fires at most once per arming.
pub(crate) fn fail_if_armed(point: FailurePoint) -> Result<(), MemoryError> {
    let fire = FAULT.with(|fault| {
        let mut guard = fault.borrow_mut();
        if matches!(guard.as_ref(), Some(Fault::Failure(armed)) if *armed == point) {
            *guard = None;
            true
        } else {
            false
        }
    });
    if fire {
        Err(MemoryError::Io(std::io::Error::other(format!(
            "injected private spill failure at {point:?}"
        ))))
    } else {
        Ok(())
    }
}

/// If a pause is armed for `point` (by variant), signal arrival and block
/// until the test releases. No-op when nothing is armed. A non-matching armed
/// pause is left in place for a later consultation.
pub(crate) fn pause(point: PausePoint) {
    let kind = pause_kind(&point);
    let armed = FAULT.with(|fault| {
        let mut guard = fault.borrow_mut();
        if matches!(guard.as_ref(), Some(Fault::Pause(p, _, _)) if pause_kind(p) == kind) {
            guard.take()
        } else {
            None
        }
    });
    if let Some(Fault::Pause(_, arrived, release)) = armed {
        let _ = arrived.send(());
        let _ = release.recv();
    }
}
