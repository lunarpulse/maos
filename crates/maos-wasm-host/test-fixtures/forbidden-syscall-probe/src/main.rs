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
//!
/// `--benign` (Story 17-6 D-17-6-H): the Rust negative control — spawn and join
/// one thread (glibc >= 2.34 `pthread_create` issues `clone3`), print
/// `forbidden-syscall-probe: benign ok`, exit 0. Proves a Rust binary reaches
/// `main` under the identical T2 spec that kills the `ptrace` mode.
///
/// `--auxv` (Story 17-6 AC3): prove that T2's argument-filtered `prctl(2)`
/// exception permits only `PR_GET_AUXV`. It must retrieve the process auxv, then
/// two unrelated query options must be refused with seccomp's `EPERM`.
fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--benign") => {
            std::thread::spawn(|| {}).join().expect("thread join");
            println!("forbidden-syscall-probe: benign ok");
            std::process::exit(0);
        }
        Some("--auxv") => auxv_probe(),
        _ => {}
    }
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

fn auxv_probe() {
    const PR_GET_AUXV: libc::c_int = 0x4155_5856;
    const PR_GET_DUMPABLE: libc::c_int = 3;
    const PR_GET_NAME: libc::c_int = 16;

    let mut auxv = [0 as libc::c_ulong; 128];
    // SAFETY: `auxv` is writable for precisely the byte count passed. Linux
    // copies this process's auxiliary vector into that buffer.
    let auxv_bytes = unsafe {
        libc::prctl(
            PR_GET_AUXV,
            auxv.as_mut_ptr(),
            std::mem::size_of_val(&auxv),
            0,
            0,
        )
    };
    if auxv_bytes <= 0 {
        eprintln!(
            "forbidden-syscall-probe: PR_GET_AUXV failed: {}",
            std::io::Error::last_os_error()
        );
        std::process::exit(1);
    }

    for (option, name) in [
        (PR_GET_DUMPABLE, "PR_GET_DUMPABLE"),
        (PR_GET_NAME, "PR_GET_NAME"),
    ] {
        // SAFETY: seccomp rejects every non-PR_GET_AUXV `prctl` before the
        // kernel can inspect these unused arguments. If the filter broadens,
        // the non-EPERM result below makes this probe fail closed.
        let rc = unsafe { libc::prctl(option, 0, 0, 0, 0) };
        let error = std::io::Error::last_os_error();
        if rc != -1 || error.raw_os_error() != Some(libc::EPERM) {
            eprintln!(
                "forbidden-syscall-probe: {name} unexpectedly returned {rc}: {error}"
            );
            std::process::exit(1);
        }
    }

    println!(
        "forbidden-syscall-probe: PR_GET_AUXV bytes={auxv_bytes}; \
         PR_GET_DUMPABLE,PR_GET_NAME denied=EPERM"
    );
    std::process::exit(0);
}
