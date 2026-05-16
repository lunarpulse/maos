---
Status: binding-v0.1
Gate: `xtask check-unsafe` walks the kernel-core src tree; every `#![allow(unsafe_code)]` site must be enumerated in `xtask/unsafe-allowlist.toml` with rationale
Decided: 2026-05-16
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.0.4 (kernel design), §8 (security model)
Supersedes: implicit Epic 1a discipline (`#![forbid(unsafe_code)]` at the crate root of `maos-kernel-core`)
---

# ADR-039 — Per-module `#![forbid(unsafe_code)]` (kernel-core unsafe policy)

**Decision.** `maos-kernel-core` switches from **crate-level** `#![forbid(unsafe_code)]` to **per-module** `#![forbid(unsafe_code)]` in every module of the crate **except** the explicitly-enumerated set listed in `xtask/unsafe-allowlist.toml`. At v0.1-β the allowlist contains exactly one entry: `crates/maos-kernel-core/src/security/sandbox/` (and its OS-specific submodules `linux/`, `macos/`, `unsupported/`, `windows/`).

The `xtask check-unsafe` discipline gate (existing) walks the kernel-core source tree, confirms the per-module `#![forbid(unsafe_code)]` is present in every non-allowlisted module file, and rejects any new `unsafe` block whose enclosing module path is not enumerated in `xtask/unsafe-allowlist.toml`. The allowlist file requires a one-paragraph rationale per entry signed by ≥2 maintainers (mirrors `docs/invariants/i9-exemptions.md` discipline).

**Rationale.**

1. **Story 1b.3 found the floor of "crate-level forbid".** The Linux sandbox enforcement story needed `unsafe { libc::seccomp_load(...) }`, `unsafe { libc::syscall(...) }`, and `unsafe { ... }` inside a `pre_exec` closure (which runs after `fork()` but before `exec()` in a single-threaded, async-signal-safe context — no allocator, no locks, no panics). These calls are unavoidable: there is no safe Rust equivalent for the kernel-level syscalls Landlock + seccomp-bpf require. Crate-level `forbid(unsafe_code)` made the story un-shippable; the only way forward was to relax.

2. **Per-module > crate-level for surface containment.** Relaxing the entire kernel-core crate to `allow(unsafe_code)` would have been the easiest path but would silently permit `unsafe` to appear anywhere — including the IAC bus, capability registry hot path, journal, and inference port. Per-module `#![forbid(unsafe_code)]` keeps the surface containment property: every module file declares its discipline, every `unsafe` block lives only where the allowlist sanctions it.

3. **Epic 2 needs the same precedent.** Story 2.1's `#[spirit]` proc-macro will derive code that may touch `unsafe` (e.g., for trait-object vtable generation or `no_std` arithmetic intrinsics). Story 2.4's spirit-test SDK harness may need `unsafe` for cross-Spirit memory isolation probes (NFR-Sec-14). Both stories need a documented governance precedent for "where unsafe is permitted, what review process applies, what audit cadence."

4. **Mechanical enforcement beats memo discipline.** The `xtask check-unsafe` walker is non-negotiable: it runs per-commit in `discipline.yml` (existing gate as of Epic 0). Per-module annotation + allowlist toml + CI gate combine to make "no new unsafe surface lands without explicit approval" a build-time invariant, not a code-review hope.

**Enforcement specifics.**

- Every `.rs` file inside `crates/maos-kernel-core/src/` MUST start with `#![forbid(unsafe_code)]` UNLESS its directory path matches an entry in `xtask/unsafe-allowlist.toml`.
- `xtask/unsafe-allowlist.toml` schema:
  ```toml
  # Story 1b.3 — sandbox enforcement requires libc syscalls
  paths = [
      "crates/maos-kernel-core/src/security/sandbox/",
  ]
  ```
- Adding a new entry to `xtask/unsafe-allowlist.toml` requires invariant-lock review per ADR-037 (constitutional amendment process). The rationale must include: (a) which syscall/intrinsic is needed, (b) why no safe alternative exists, (c) which audit cadence applies (e.g., quarterly review, paired with red-team corpus run).
- `xtask check-unsafe` failure modes:
  - Module file lacks `#![forbid(unsafe_code)]` AND path is not allowlisted → **build break**.
  - New `unsafe` block appears in an allowlisted path → **flagged for invariant-lock review** (not auto-rejected; the path is sanctioned for unsafe).
  - `xtask/unsafe-allowlist.toml` modified → invariant-lock review per ADR-037.

**Alternatives considered.**

- **Stay crate-level `forbid`, refuse Story 1b.3.** Rejected: Landlock + seccomp + cgroups enforcement is the v0.1-β NFR-Sec-1 floor; cannot ship without OS-level sandbox.
- **Relax to crate-level `allow(unsafe_code)`.** Rejected: silently permits `unsafe` anywhere in kernel-core; loses the surface-containment property that motivated the original `forbid` discipline.
- **Use `unsafe_op_in_unsafe_fn` lint instead of `forbid`.** Rejected: the lint addresses unsafe-block placement within unsafe-fn bodies, not whether unsafe is permitted at all. Orthogonal concern.
- **Separate `maos-kernel-sandbox` crate (no_std-compatible) hosting all unsafe.** Considered for v0.5+ (a service-extraction along the lines of §4.0.8's four-property test). At v0.1-β, the per-module + allowlist approach is the minimum-viable governance.

**What would force a revisit.**

- Story 2.1's `#[spirit]` proc-macro work requires `unsafe` in a non-sandbox path → amendment via ADR-037; allowlist gains a second entry.
- A new kernel service (e.g., the v0.5 measurement gate at §13.1, or v1.0 IAC bus retract primitive) requires `unsafe` outside the sandbox path → amendment via ADR-037.
- Aggregate `unsafe`-block count in allowlisted paths exceeds a soft threshold (suggested: 20 blocks across all allowlisted paths) → trigger an audit-cadence review and consider service extraction.
- The per-module discipline shows >5% of non-allowlisted files missing the `#![forbid(unsafe_code)]` annotation in spot checks → tooling problem; tighten the walker or pre-commit hook.

**Implementation status (post Story 1b.6).**

- `xtask check-unsafe` gate is GREEN on `main`.
- `xtask/unsafe-allowlist.toml` contains exactly one entry: `crates/maos-kernel-core/src/security/sandbox/`.
- Allowlisted unsafe sites (Story 1b.3):
  - `crates/maos-kernel-core/src/security/sandbox/linux.rs` — Landlock + seccomp-bpf rule installation + `pre_exec` closure.
  - `crates/maos-kernel-core/src/security/sandbox/macos.rs` — `sandbox-exec` profile load (planned; v0.1-β stub).
  - `crates/maos-kernel-core/src/security/sandbox/unsupported.rs` — no unsafe (fail-closed stub).
  - `crates/maos-kernel-core/src/security/sandbox/windows.rs` — `CreateRestrictedToken` / Job Object (planned; v0.1-β stub).
- All other modules retain `#![forbid(unsafe_code)]`.

**Cross-references.**

- ADR-010 (hexagonal architecture) — the sandbox module is at the adapter ring (OS-native primitives); unsafe lives at the boundary, not in the domain core.
- ADR-004 (hexagonal sandboxing with OS-native primitives) — establishes that T0/T1/T2 enforcement uses OS primitives, which inherently require unsafe in Rust.
- ADR-030 (capability registry decomposition) — the hot-path crates (`cap_tokens` lock-free) explicitly forbid unsafe; the per-module discipline preserves this.
- ADR-037 (constitutional amendment process) — governs allowlist additions.
- `docs/invariants/i9-exemptions.md` — parallel discipline pattern for I9 state-exempt registrations.
- Story 1b.3 dev record — original motivation; documents the parent/child `pre_exec` async-signal-safety constraints.
- Story 1b.6 dev record — this ADR's accepting story (Epic 2 prep bundle).
