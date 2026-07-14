//! Test-only probe for Story 11.1a AC4's T2 forbidden-syscall cell.
//!
//! Issues a raw `ptrace(PTRACE_TRACEME, ...)` syscall — present on the
//! kernel T2 seccomp `hostile_syscalls` KillProcess list
//! (`crates/maos-kernel-core/src/security/sandbox/linux.rs`). Under a T2
//! sandbox this process must be killed with `SIGSYS` (the seccomp
//! KillProcess action always reports as SIGSYS to the parent's wait status,
//! per Linux semantics) BEFORE this binary can print or exit normally.
//!
//! This binary is NOT shipped: it lives outside the main workspace
//! (own `[workspace]` table) and is built ad hoc by the AC4 T2 integration
//! test, never by `cargo build`/`cargo test` at the repo root.

fn main() {
    // SAFETY: ptrace(PTRACE_TRACEME, 0, null, null) is the canonical
    // self-trace request — no pointers are dereferenced by the kernel for
    // this request, and we pass null for both the unused addr/data params.
    // This call is the proof payload: it is on the seccomp hostile list and
    // must never execute to completion under a T2 sandbox.
    let rc = unsafe { libc::ptrace(libc::PTRACE_TRACEME, 0, std::ptr::null_mut::<libc::c_void>(), std::ptr::null_mut::<libc::c_void>()) };
    // If we get here, the syscall was NOT blocked — T2 failed to confine us.
    println!("forbidden-syscall-probe: ptrace returned {rc} — T2 did NOT block this syscall");
    std::process::exit(0);
}
