//! T3 container isolation — wraps T2 protections inside a Docker / Podman
//! container for defense-in-depth sandboxing of broad-capability-surface
//! Spirits (architecture §4.3.1, §8.2).
//!
//! # Platform-availability matrix (v0.5-α)
//!
//! | Platform | Status |
//! |---|---|
//! | Linux (Podman/Docker) | ✅ v0.5-α baseline |
//! | macOS | ❌ `SpawnError::SandboxUnavailable` — pending macOS CI runners and container-runtime equivalents |
//! | Windows | ❌ `SpawnError::SandboxUnavailable` — pending Windows CI runners and container-runtime equivalents |
//!
//! # T2-inside-T3 layering
//!
//! The container boundary is the outer security ring. The in-container
//! T2 stack (Landlock+seccomp) is invoked by the Spirit binary's ABI-side
//! `t2_apply()` hook at startup. v0.5-α's reference smoke binary (busybox)
//! does NOT call `t2_apply()` — the in-container T2 layer is **deferred to
//! Epic 6** when the subprocess Spirit wire protocol lands. See
//! Architecture DR-5.5a-8.
//!
//! # Decision Register
//!
//! **DR-5.5a-1:** `crates/maos-sandbox` crate extraction deferred;
//! T3 lands in-place at `crates/maos-kernel-core/src/security/sandbox/t3/`.
//! Trigger to revisit: Story 5.5e KLOC review or Epic 6 subprocess-form work.
//!
//! **DR-5.5a-2:** Linux-only T3 at v0.5-α. Trigger: Epic 6+.
//!
//! **DR-5.5a-3:** Container backend = shell out to `/usr/bin/podman` / `/usr/bin/docker`;
//! NO new Rust deps. Trigger: performance regression.
//!
//! **DR-5.5a-4:** Podman-first, Docker fallback; operator override via
//! `MAOS_T3_RUNTIME=podman|docker|auto|none`.
//!
//! **DR-5.5a-5:** Base image = distroless-based; SHA pinned in `t3-image.lock`.
//! Trigger: Story 5.5d multi-image registry support.
//!
//! **DR-5.5a-6:** `--network=none` default; MCP outbound routed through parent.
//! Trigger: Story 5.5c.
//!
//! **DR-5.5a-7:** PID identity = host-namespace PID (NOT in-container PID 1).
//!
//! **DR-5.5a-8:** T2-inside-T3 layering = ABI-side `t2_apply()` at Epic 6;
//! v0.5-α relies on container boundary alone.
//!
//! # `pre_exec` discipline at T3 = NO-OP
//!
//! T3 does NOT call `pre_exec` because the container itself is the
//! boundary. The runtime parent's `pre_exec` closure is empty
//! (the in-container T2 stack from the Spirit binary's `t2_apply()` is
//! the inner ring). Do NOT reintroduce the T2 closure pattern here.

pub mod argv;
pub mod cap_audit_bridge;
pub mod child;
pub mod image_lock;
pub mod image_verify;
pub mod quarantine;
pub mod runtime_detect;
pub mod spawn;
