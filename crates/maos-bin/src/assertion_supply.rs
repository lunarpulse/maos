//! Story 23-2 — the `SupplyAssertion` seam (D-23-2-01 A, D-23-2-04 A, D-23-2-05 A).
//!
//! `issue_enterprise_governed_capability` has ten production callers and no caller-identity
//! parameter, so the assertion a mint verifies is resolved here, by the pid the mint is for:
//!
//! 1. the assertion of the directed task whose turn is running on that pid (a [`TurnScope`]), else
//! 2. the process environment's `MAOS_SSO_ASSERTION` (the static, air-gap source; also what a
//!    task that carried no assertion uses).
//!
//! A scope exists for exactly one turn EXPORT, and the session owns it, not the caller waiting
//! on the answer (Story 23-2 F2):
//!
//! * an installed Spirit's task travels with its request (`ShellPort::arm_labelled`); the session
//!   [`enter`]s the scope when it CLAIMS that exact frame and drops it when the export completes,
//!   when the session loop aborts or, at the latest, after the child is stopped and reaped. A
//!   mailbox turn running or queued ahead of the frame resolves the environment, and a caller
//!   that gave up at its deadline cannot strip the scope from a guest still working: that
//!   guest's late mints stay on the principal that asked, never on the next task's and never on
//!   the environment's;
//! * Mira (in-process) enters it around her own handler, under a per-runner serial lock.
//!
//! The guard is released by `Drop`, including a panic unwinding through the turn. It is keyed by
//! pid, so a mint for any other Spirit, and any mint outside the turn, still resolves the
//! environment. Every mint verifies the assertion it was given.
//!
//! Hook boundary (RELEASE-HOLDS 23-2.c): a lifecycle hook that mints for the same pid while the
//! turn runs uses the task's assertion, because the seam sees a pid, not a caller. The same holds
//! for Mira's own background work that mints for her pid while her task runs: the seam cannot
//! tell callers apart.
//!
//! The registry is bounded by the number of running turns (one per Spirit pid): a second scope for
//! a pid that already has one is refused, never replaced.

use std::collections::{hash_map::Entry, HashMap};
use std::sync::{LazyLock, Mutex, MutexGuard};

use maos_control::Assertion;

static SCOPES: LazyLock<Mutex<HashMap<u32, Assertion>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn scopes() -> MutexGuard<'static, HashMap<u32, Assertion>> {
    SCOPES.lock().unwrap_or_else(|error| error.into_inner())
}

/// A turn already holds an assertion scope for this pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeBusy;

/// The scope of one turn; dropping it ends the scope.
#[derive(Debug)]
pub struct TurnScope {
    pid: u32,
}

impl Drop for TurnScope {
    fn drop(&mut self) {
        scopes().remove(&self.pid);
    }
}

/// Enter the scope of a turn running on `pid`. `Ok(None)` when the task carries no assertion and
/// no other scope holds `pid`: its mints resolve the environment (D-23-2-05 A). A frame that
/// carries no assertion is refused as well while another scope holds the pid, so it can never
/// run (and mint) under that scope's principal.
///
/// # Errors
/// [`ScopeBusy`] when `pid` already has a scope (two turns on one pid would mix principals).
pub fn enter(pid: u32, assertion: Option<&Assertion>) -> Result<Option<TurnScope>, ScopeBusy> {
    let mut held = scopes();
    let Some(assertion) = assertion else {
        return if held.contains_key(&pid) {
            Err(ScopeBusy)
        } else {
            Ok(None)
        };
    };
    match held.entry(pid) {
        Entry::Occupied(_) => Err(ScopeBusy),
        Entry::Vacant(slot) => {
            slot.insert(assertion.clone());
            Ok(Some(TurnScope { pid }))
        }
    }
}

/// The assertion a mint for `spirit_pid` verifies: the turn scope's, else the environment's.
/// `None` when neither exists. The text is a credential: hand it to a verifier only.
#[must_use]
pub fn supply(spirit_pid: u32) -> Option<String> {
    if let Some(scoped) = scopes().get(&spirit_pid) {
        return Some(scoped.expose().to_owned());
    }
    std::env::var("MAOS_SSO_ASSERTION").ok()
}

/// Whether `pid` currently has a turn scope (tests and the doctor).
#[must_use]
pub fn is_scoped(pid: u32) -> bool {
    scopes().contains_key(&pid)
}
