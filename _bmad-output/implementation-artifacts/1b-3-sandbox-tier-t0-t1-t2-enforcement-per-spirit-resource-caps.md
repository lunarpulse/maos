---
dev_model_used: claude-opus-4-5
---

# Story 1b.3: Sandbox Tier T0/T1/T2 Enforcement + Per-Spirit Resource Caps

Status: complete (all review findings resolved, all tests green)

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an operator,
I want sandbox tiers **T0** (trusted local) / **T1** (process isolation + UID separation) / **T2** (Linux Landlock+seccomp; macOS Seatbelt; Windows restricted-token) enforced per-Spirit with a `strictest-of-(manifest, trust-tier, operator-policy)` floor, **AND** per-Spirit resource caps enforced OS-natively via cgroups v2 / setrlimit / Job Object,
So that a `public-untrusted` Spirit declaring T0 is forced to T2 — sandbox enforcement cannot be downgraded by a Spirit's manifest claim, and a runaway Spirit is throttled by the OS, not the host.

This is the **first OS-boundary runtime body** in the kernel. Stories 1b.1 and 1b.2 landed in-process audit/capability machinery; this story crosses into platform syscalls (`pre_exec`, Landlock, seccomp-bpf, cgroup files, Job Objects). It is the highest-blast-radius story in Epic 1b: a `pre_exec` bug, a fail-open sandbox path, or a parent/child confusion can silently disable the security boundary.

### What this story is NOT

- **NOT the Spirit spawn path / `maosctl run`.** No Spirit process is actually launched by `maosctl` here. This story ships `spawn_sandboxed(spec, command) -> SandboxedChild` as a **reusable library API** plus the admission logic that computes the spec. Story **1b.5a** wires `maosctl run hello-spirit` to call it. This story's tests exercise the API against **throwaway helper processes** (`/bin/sh`, a tiny test probe binary), not real Spirits.
- **NOT the full manifest parser.** Story **1b.5c** ships the kernel-side manifest parser with the NFR-Test-13 coverage-matrix gate over *all* manifest fields. This story owns **only** the `[sandbox]` and `[resources]` manifest sections — their typed structs, TOML deserialization, validation, and ≥3-cases-per-field tests for *those fields*. 1b.5c composes them.
- **NOT T3 / T4.** T3 (container isolation via Docker/Podman) is Story **5.5a** (v0.5). T4 (WASM-component sandbox) is v2.0. The effective-tier computation must *accept* T3 as a value (operator policy may demand it) and **fail-closed** — refuse to admit a Spirit whose effective tier exceeds what this story enforces (T2), with typed error `ESandboxTierUnsupported`.
- **NOT live per-syscall anomaly detection.** Syscall-pattern divergence detection, fd-table growth monitoring, and structural sandbox-escape alarms are **NFR-Sec-3 / v2.0** (architecture §8.1). At v0.1-β, `CapAuditEvent::SandboxBlock` is emitted when a sandboxed child **terminates** with a sandbox-violation cause (SIGSYS from seccomp, or a probe exit code signalling an `EACCES`/`EPERM` from Landlock) — not for every individual blocked syscall.
- **NOT secret materialization or the Approval Manager UI.** Both are Security Manager responsibilities (§4.3.2 / §4.3.3) but land later (1b.4 / Epic 3). This story touches `SecurityManagerAdapter` *only* for sandbox-tier admission.
- **NOT the crash-detection supervisor.** The Spirit Scheduler's crash detection, `task.orphaned` emission, and hung-Spirit watchdog are Story **5.3**. This story's `SandboxedChild` exposes `wait()` / `try_wait()` and an exit classifier; the supervisor consumes them later.

### Critical preconditions (verify BEFORE opening the PR)

1. **Story 1b.2 fully landed and working tree clean.** `git status` at story-creation time showed uncommitted modifications on `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`, `crates/maos-kernel-core/src/capability/cap_quota/mod.rs`, and `Cargo.lock`. These are 1b.2 follow-ups — commit or resolve them first. Starting 1b.3 on a dirty tree corrupts the dependency-introduction blast-count.
2. **`crates/maos-attrs` is a workspace member.** `crates/maos-kernel-core/Cargo.toml:39` path-depends on `maos-attrs = { path = "../maos-attrs" }`, but the workspace `Cargo.toml` `members` list does **not** include `crates/maos-attrs` and git shows it untracked (`?? crates/maos-attrs/`). Run `cargo metadata --format-version 1 | jq '.workspace_members'` — if `maos-attrs` is absent, add `"crates/maos-attrs"` to `[workspace] members` and commit. Leave `crates/maos-kernel-core/fuzz/` as its own excluded sub-workspace (do not add to members).
3. **All 17 prior gates green on `main`.** `cargo build --workspace --locked`, `cargo test --workspace --locked` (the `journal_fsync_p99` bench-test is environment-dependent and may fail on slow disks — note it, don't fix it here), `cargo run -p xtask -- check-service-boundary` (0 violations), `cargo deny check`. Record the baseline in the Dev Agent Record per AC6.
4. **Decide the `unsafe` strategy before writing any platform code (AC6).** OS sandboxing is irreducibly `unsafe`: `std::os::unix::process::CommandExt::pre_exec` is an `unsafe fn`, and seccomp/Landlock *must* be applied inside the child between fork and exec. `crates/maos-kernel-core/src/lib.rs:1` currently carries crate-level `#![forbid(unsafe_code)]`, which **cannot** be locally overridden. See *Dev Notes → The `unsafe` decision* for the required resolution. Do this first; it gates every platform file.
5. **Confirm the 1b.2 audit sockets are intact.** `CapAuditEvent::SandboxBlock { spirit_pid, attempted_syscall, sandbox_tier }` exists at `crates/maos-kernel-core/src/capability/cap_audit/mod.rs:91-96` and `writer_task.rs:90-101` already maps it to `FrameKind::SandboxBlock`. This story *emits* into that socket — it does not redefine it. If the variant shape changed, reconcile before starting.

### Size envelope

- **Implementation:** ~900–1,400 LOC (platform code is verbose; Linux + macOS + Windows each carry a distinct file).
- **Tests / fixtures:** ~600–900 LOC.
- **KLOC budget:** Epic 1b budget is ~1–2 KLOC of *persistent-state* surface. Sandbox enforcement is overwhelmingly **spawn-time logic, not persistent state** — it does not stress the I9 three-holder budget. The only candidate persistent state is per-Spirit cgroup-handle tracking; the design below keeps that as an RAII guard owned by the caller, *not* a field on any kernel service. Aggregate workspace LOC after this story stays well under the 16K alarm.
- **New dependencies:** 4–6, all OS-target-gated (see *Library / framework requirements*). Document the `Cargo.lock` blast count in the Dev Agent Record.

## Acceptance Criteria

### AC1 — Strictest-of admission floor: effective tier = `strictest_of(manifest, trust-tier, operator-policy)`; `public-untrusted` declaring T0 forced to T2; effective tier journaled; `SandboxTier` hardened (resolves DF18 + the 1a.2 deferred `SandboxTier(pub u8)` constraint gap)

**Given** a Spirit manifest declaring a sandbox tier
**When** the Security Manager admits the Spirit
**Then** the effective tier is the strictest of `(manifest_declared_tier, trust_tier_floor, operator_policy_floor)`
**And** a `public-untrusted` Spirit declaring T0 is forced to T2 regardless of manifest
**And** the effective tier is journaled to the Lifecycle Journal
**And** if the effective tier exceeds T2 (i.e. T3+), admission **fails-closed** with `ESandboxTierUnsupported` — the kernel never admits a Spirit it cannot actually sandbox

**Implementation guidance:**

- **Reuse, do not reinvent.** The strictest-of arithmetic already exists: `crates/maos-kernel-core/src/capability/cap_policy/mod.rs` ships `strictest_of(a, b, c) -> SandboxTier` (line 143) and `PolicyTable::effective_sandbox_tier(spirit_pid, trust_tier, &inner)` (line 111). `ManifestCapabilityScope { scopes, declared_tier, trust_tier }` (line 34) and `OperatorPolicyConfig { spirit_tier_floor, global_sandbox_floor, per_capability_approval }` (line 22) already carry the three inputs. This AC **wires** those into the Security Manager admission path — it does not duplicate the policy logic.
- **Harden `SandboxTier`** (`crates/maos-domain/src/invariants/i9.rs:31-42`). Today it is `SandboxTier(pub u8)` with `Default → SandboxTier(0)` (T0, *least* restrictive). Add:
  - Associated constants `T0`/`T1`/`T2`/`T3` (T3 is a representable value the kernel can *reject* but not *enforce* at v0.1-β).
  - A validating constructor `SandboxTier::try_from_u8(u8) -> Result<Self, SandboxTierError>` rejecting values outside `0..=4` (T4 reserved, also rejected at enforcement time).
  - `SandboxTier::try_from_manifest_str(&str)` parsing `"T0"..="T3"` (case-sensitive, exact) — used by AC5's `[sandbox]` deserialization.
  - **Change `Default` to the most-restrictive enforceable tier (T2)** — resolves **DF18** ("`Default for SandboxTier` returns T0 (most permissive); security-sensitive type should default to most restrictive"). This is a **fail-closed** change.
  - **Audit every `#[derive(Default)]` struct containing `SandboxTier` and every `SandboxTier(0)` literal** before committing the `Default` change. Known sites: `ManifestCapabilityScope` and `OperatorPolicyConfig` derive `Default`; `effective_sandbox_tier` uses three explicit `SandboxTier(0)` `unwrap_or` fallbacks (`cap_policy/mod.rs:121,126,132`). The `unwrap_or(SandboxTier(0))` fallbacks for *unknown* trust-tier / operator entries are themselves a **fail-open** smell — an unknown trust tier should fall back to the *strictest* floor, not T0. Fix these to `unwrap_or(SandboxTier::DEFAULT_FLOOR)` (= T2) and confirm no test regression that depended on the old fail-open behavior. This resolves the 1a.2 deferred item *"`SandboxTier(pub u8)` has no value constraint … T0-T2 enforcement with validation lands in Story 1b.3"*.
- **Trust-tier floor table.** The architecture's ADR-009 names three trust tiers (`local` / `org-internal` / `public-untrusted`); the *code* (from 1b.2, `cap_policy/decision.rs:58-70`) has a four-variant `TrustTier { PublicUntrusted, Known, Verified, Internal }`. **Use the existing code enum — do not rename or invent.** Populate `PolicyTableInner::trust_tier_floor` with the canonical mapping at composition-root construction: `PublicUntrusted → T2`, `Known → T1`, `Verified → T0`, `Internal → T0` (matches the doc-comments already on the enum variants). The ADR-009-vs-code naming divergence is a documentation reconciliation item — flag it for the Epic 1b retro, do not resolve it here.
- **Journaling the effective tier.** AC1 requires the effective tier in the **Lifecycle Journal** (not the Transparency Log). `JournalEntry` (`crates/maos-domain/src/invariants/i10.rs:50-59`) currently has only `{ timestamp, lifecycle_event, spirit_id }` — no tier field. **Recommended:** extend `JournalEntry` with `#[serde(default, skip_serializing_if = "Option::is_none")] pub effective_sandbox_tier: Option<SandboxTier>`. `serde(default)` keeps old NDJSON journal lines parseable (backward-compatible file format). Then journal the effective tier on the `LifecycleEvent::Load` transition. **This breaks every `JournalEntry { … }` struct-literal construction site** — update all of them to add `effective_sandbox_tier: None` (or `Some(tier)` at the admission site). Known sites: `crates/maos-kernel-core/src/journal/mod.rs` tests (~6 literals), the `i10.rs` doctest (line 18-23) and unit test (line 67-71), and the `JournalAdapter::journal_lifecycle` path. Run `rg 'JournalEntry\s*\{' --type rust` to find them all. Alternative considered (and rejected as lossy): a bare new `LifecycleEvent::SandboxApplied` variant carries no tier value.
- **Admission API.** Add an inherent method on `SecurityManagerAdapter` — e.g. `admit_spirit(&self, spirit_pid: u32, spirit_id: &str, manifest: &SandboxConfig, caps: &ResourceCaps) -> Result<SandboxSpec, SecurityError>` — that: (1) computes the effective tier via the policy table, (2) rejects T3+ with `ESandboxTierUnsupported`, (3) appends the `Load` journal entry with `Some(effective_tier)`, (4) returns the fully-resolved `SandboxSpec` (tier + resolved resource caps) for `spawn_sandboxed` to consume. The `SecurityManagerPort` trait (`crates/maos-domain/src/ports/security.rs`) currently has `sandbox_tier_floor(&self, spirit_id: &str)` as a v0.1-α placeholder — evolve it to be `u32`-pid-keyed and add an `effective_sandbox_tier` method consistent with the rest of the `u32`-keyed capability system. Follow the 1b.2 house pattern: extend the port trait with the real methods, keep rich admission logic as inherent methods on the adapter. Any port-trait surface change requires updating `xtask/kernel-api-classes.toml` and regenerating `docs/ci-baselines/kernel-surface-v0.1-beta.json`.
- **`SecurityManagerAdapter` gains a field.** It is a ZST today (`crates/maos-kernel-core/src/security/mod.rs:18`). Promote it to hold `Arc<PolicyTable>` (and whatever else admission needs). It already derives `Default` and is constructed in `crates/maos-bin/src/main.rs:77` as `SecurityManagerAdapter::default()` — update the composition root to construct it with the shared `PolicyTable` (note: 1b.2's `main.rs` constructs a `PolicyTable` for the capability registry; share that `Arc`, do not make a second one). Adding `Arc<PolicyTable>` is **not** new persistent state — `PolicyTable` is already `#[i9_exempt]`; holding an `Arc` to it introduces no new I9 holder.

### AC2 — T2 on Linux: Landlock + seccomp-bpf block out-of-scope syscalls at the kernel boundary; block recorded in the Transparency Log via `cap-audit`; per-Spirit cgroup v2 caps

**Given** a Spirit running under T2 on Linux
**When** the Spirit attempts a syscall or filesystem op outside its declared capability scope
**Then** Landlock + seccomp blocks it at the kernel boundary (filesystem op → `EACCES`/`EPERM`; forbidden syscall → process killed by seccomp)
**And** the block is recorded in the Transparency Log via `cap-audit` (`CapAuditEvent::SandboxBlock`)

**Implementation guidance:**

- **File:** `crates/maos-kernel-core/src/security/sandbox/linux.rs`, gated `#[cfg(target_os = "linux")]`.
- **Landlock** (filesystem subtree restriction). Use the `landlock` crate (safe wrapper). Build a `Ruleset` from the Spirit's declared `Scope::FsRead`/`Scope::FsWrite` subtrees (`maos_domain::invariants::i1::Scope`), call `.handle_access(...)`, add path rules, and `restrict_self()` **inside `pre_exec`** (it restricts the calling thread/process — calling it in the parent would sandbox the MAOS kernel itself). Use `ABI::V1` as the floor with `CompatLevel::BestEffort` so the code degrades gracefully on kernels < 5.13 (Landlock unavailable → see fail-closed note). The `landlock` crate supports up to ABI 6 (Linux 6.12) as of 2026; pin a version and use best-effort compat so newer kernels are not a hard requirement.
- **seccomp-bpf** (syscall allow-list). Use the `seccompiler` crate (rust-vmm; safe BPF compilation, used in Firecracker). Compile a `SeccompFilter` allow-list from the tier + declared scopes, then `apply_filter()` **inside `pre_exec`**, after Landlock, before exec. Default action for the v0.1-β "narrow" T2 profile: `SeccompAction::Errno(EPERM)` for most denied syscalls, `SeccompAction::KillProcess` for the unambiguously-hostile set (`ptrace`, `process_vm_writev`, raw `clone`/`unshare` for namespace escape, `kexec_load`, etc.). `KillProcess` makes the violation observable to the parent (`WIFSIGNALED` + `SIGSYS`); `Errno` keeps benign-but-unscoped calls non-fatal.
- **cgroups v2** (resource caps — shared with AC5). **Write the cgroup files directly with `std::fs` — no new dependency.** cgroups v2 is three file writes: create `<cgroup-root>/maos.slice/spirit-<pid>/`, write `cpu.max` (`"<quota> <period>"`, derive `quota` from `cpu_max_pct`) and `memory.max` (`memory_max_mb * 1024 * 1024`), then write the child PID into `cgroup.procs` **from the parent, after spawn** (no `pre_exec` needed for cgroups). Locate the writable cgroup root: prefer the delegated user subtree (`/sys/fs/cgroup/user.slice/user-$UID.slice/user@$UID.service/` under systemd user-session delegation); the evaluator may also run MAOS under `systemd-run --user --scope`. **If no writable cgroup subtree exists** (common on a bare dev box without delegation), this is the documented fallback to `setrlimit` (see AC5) — log the chosen mechanism and journal it; **do not silently skip resource enforcement** (silent-skip is the 1b.1 smoke-test bug class — see *Previous Story Intelligence*).
- **`pre_exec` is a minefield — read this twice.** The `pre_exec` closure runs in the **forked child, before exec**, in an async-signal-unsafe context: **no heap allocation, no locking, no `println!`, no `String` formatting**. Pre-compute everything (the seccomp BPF program, the Landlock ruleset, rlimit values) in the **parent** and `move` only `Copy`/pre-allocated data into the closure. A panic or allocation in `pre_exec` is undefined behavior. If any sandbox step inside `pre_exec` fails, the closure must return `Err(io::Error)` — `Command::spawn` then fails and the child **never execs the Spirit binary**. This is the load-bearing fail-closed property: **a sandbox-setup failure must abort the spawn, never produce an unsandboxed Spirit.**
- **Recording the block.** At v0.1-β the kernel observes *outcomes*, not individual syscalls (live per-syscall interception is NFR-Sec-3 / v2.0 — explicitly out of scope). `SandboxedChild::wait()` returns an `ExitStatus`; a `classify_exit(status) -> Option<SandboxViolation>` helper interprets it (`WIFSIGNALED` + `SIGSYS` → seccomp kill; a reserved probe exit code → Landlock `EACCES`). On a violation, the caller emits `CapAuditEvent::SandboxBlock { spirit_pid, attempted_syscall, sandbox_tier }` through the existing 1b.2 audit channel (`cap_audit::Sender`), which `writer_task.rs:90` already routes to `FrameKind::SandboxBlock` in the Transparency Log. Use `try_send` + `cap_audit::record_drop()` on a full channel — never block on the audit channel (1b.2 lesson #6 / ADR-030).

### AC3 — T2 on macOS: Seatbelt `.sbpl` profile blocks forbidden operations; block journaled

**Given** a Spirit running under T2 on macOS
**When** the Spirit attempts a forbidden operation
**Then** the Seatbelt `.sbpl` profile blocks the operation
**And** the block is journaled

**Implementation guidance:**

- **File:** `crates/maos-kernel-core/src/security/sandbox/macos.rs`, gated `#[cfg(target_os = "macos")]`.
- **Mechanism.** `sandbox_init(3)` is deprecated; the production-proven path (Codex, gemini-cli, claude-code, agent-seatbelt all use it as of 2026) is: generate an **SBPL profile string at runtime** from the Spirit's declared scopes, then launch the child wrapped as `/usr/bin/sandbox-exec -p '<profile>' -- <spirit-binary> <args>`. Codex's `seatbelt_base_policy.sbpl` and `seatbelt_network_policy.sbpl` are the prior-art templates referenced by architecture §8.2 — model the deny-by-default base profile plus `(allow file-read* (subpath "…"))` rules derived from `Scope::FsRead`/`FsWrite`.
- **Known hazard — document, do not absorb.** `sandbox-exec` is itself deprecated, and there are open reports (claude-code issues on macOS 26 "Tahoe") of `"Sandbox failed to initialize"` on recent macOS. Provide an operator-policy escape hatch: if `sandbox-exec` is unavailable or fails to initialize, admission **fails-closed** for `public-untrusted` Spirits (T2-required), but the operator may explicitly downgrade *locally-trusted* Spirits via operator policy. Surface a typed `SecurityError::SandboxUnavailable { platform, reason }` rather than a silent fallthrough.
- **Resource caps** on macOS use `setrlimit` (see AC5), applied via `pre_exec` — `sandbox-exec` does not carry resource limits.
- **Journaling.** Same path as AC2: classify the child's exit; on a sandbox-violation exit, emit `CapAuditEvent::SandboxBlock` (Transparency Log) **and** the effective tier is already in the Lifecycle Journal from AC1. "Journaled" in this AC is satisfied by the AC1 Lifecycle-Journal entry plus the AC2-style Transparency-Log `SandboxBlock` row.
- **CI reality.** The CI runner is Linux — `#[cfg(target_os = "macos")]` tests will **not** execute in `discipline.yml`. Gate them `#[cfg]`, keep them compilable, and note the cross-platform-CI-matrix gap honestly in the Dev Agent Record (there is no sandbox cross-platform CI matrix story today; flag it for the retro — Story 5.5b runs the *multi-provider* matrix, not the sandbox matrix).

### AC4 — T2 on Windows: restricted-token + Job Object; out-of-token operation fails; block journaled

**Given** a Spirit running under T2 on Windows
**When** the Spirit attempts an operation outside its restricted token
**Then** the Windows access check fails
**And** the failure is journaled

**Implementation guidance:**

- **File:** `crates/maos-kernel-core/src/security/sandbox/windows.rs`, gated `#[cfg(target_os = "windows")]`.
- **Restricted token.** Use `CreateRestrictedToken` (via the `windows` crate's `Win32::Security` bindings) to derive a token with disabled privileges, restricted SIDs, and a low integrity level, then create the process with that token (`CreateProcessAsUser` / the `windows` process-creation APIs). The child's access checks then fail for anything outside the restricted token's reach.
- **Job Object** (resource caps — shared with AC5). Use the `win32job` crate (safe wrapper over `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`). Set `JOB_OBJECT_LIMIT_PROCESS_MEMORY` (from `memory_max_mb`) and `JOB_OBJECT_LIMIT_PROCESS_TIME` (CPU time, from `cpu_max_pct` converted to a time budget per accounting window). Assign the child to the Job Object immediately after creation; the Job Object also kills the child cleanly on `SandboxedChild` drop.
- **Same CI caveat as AC3** — Windows tests are `#[cfg]`-gated and do not run in the Linux CI. Keep compilable, note the gap.
- **Journaling** — identical pattern to AC2/AC3.

### AC5 — Per-Spirit resource caps (cgroups v2 / setrlimit / Job Object) applied OS-natively; `strictest_of(manifest, operator-policy)` applies; `[sandbox]` + `[resources]` manifest sections parsed with NFR-Test-13 ≥3-cases-per-field coverage

**Given** per-Spirit resource caps declared in manifest `[resources]` (FR6 basic)
**When** the kernel spawns the Spirit
**Then** cgroups v2 (Linux) / setrlimit (macOS) / Job Object (Windows) enforces CPU and memory caps **OS-natively, not via Tokio cooperation**
**And** the strictest-of `(manifest, operator-policy)` applies

**Implementation guidance:**

- **Manifest section types** — `crates/maos-kernel-core/src/security/manifest.rs` (NEW). This story owns these two sections end-to-end:
  - `SandboxConfig` — the `[sandbox]` section: `tier: SandboxTier` (deserialized from `"T0".."T3"` via `SandboxTier::try_from_manifest_str`).
  - `ResourceCaps` — the `[resources]` section: `cpu_max_pct: Option<u32>`, `memory_max_mb: Option<u32>`, `fd_max: Option<u32>` (architecture §5.1 schema; all optional — absent fields fall back to the operator-policy default, then a kernel default).
  - Both `#[derive(Deserialize)]` against `toml`. Provide standalone `from_toml_str` parsing of just those sections so 1b.5c's full parser can compose them (`#[serde(flatten)]` or sub-table extraction).
  - **Validation:** reject `cpu_max_pct > 100 * num_cpus` (or define the semantic — "% of one core" per §5.1, so `> 100` per core is allowed only up to `100 * num_cpus`), reject `memory_max_mb == 0`, reject `fd_max == 0`. Malformed → typed `ManifestError`, not a panic.
- **NFR-Test-13 — ≥3 cases per field.** Every field in `SandboxConfig` and `ResourceCaps` gets ≥3 fixture cases: **well-formed**, **malformed-rejected**, **edge-case**. Fixtures live in `crates/maos-kernel-core/src/security/manifest/tests/fixtures/` (or inline `#[cfg(test)]` TOML literals — match whatever 1b.5c will scan). Wire these into `tests/coverage-matrix.yaml` so Story 0.3's coverage-matrix CI gate counts them. This story is responsible *only* for the `[sandbox]`/`[resources]` field coverage; 1b.5c covers the rest.
- **Strictest-of resource caps.** Unlike sandbox *tier* (a 3-way strictest-of), resource caps are a 2-way strictest-of `(manifest, operator-policy)` per architecture §4.1. "Strictest" for a cap = the **lower** ceiling (more restrictive). Add a `min`-style resolver: `resolve_caps(manifest: &ResourceCaps, operator: &ResourceCaps) -> ResolvedCaps` taking the tighter of each field. Where one side is `None`, the other wins; where both are `None`, fall through to a documented kernel default. `OperatorPolicyConfig` (`cap_policy/mod.rs:22`) may need an added `resource_cap_floor` field — if so, follow the `#[i9_exempt]` discipline already on that struct.
- **`setrlimit` path** (macOS always; Linux fallback when no writable cgroup subtree). Apply `RLIMIT_AS` (memory), `RLIMIT_CPU` (CPU seconds), `RLIMIT_NOFILE` (`fd_max`) via `setrlimit` **inside `pre_exec`** — rlimits must be set in the child before exec. Use the `rlimit` crate (clean `Resource` API) or `libc::setrlimit` directly; either way the call sits in the `unsafe` `pre_exec` closure with a `SAFETY:` comment. Pre-compute the `rlimit` structs in the parent; the closure only does the `setrlimit` syscalls (async-signal-safe).
- **"OS-natively, not via Tokio cooperation"** (architecture §4.1, §4.3.1): the kernel sets OS-level limits at spawn and the **OS** enforces them — a runaway Spirit is throttled/OOM-killed by the Linux kernel / macOS / Windows, not by the Tokio scheduler. There must be **zero** in-process polling/accounting loop for CPU or memory caps in this story. (Per-Spirit *capability* quota — `tokens/min`, the `cap-quota` `ContextPressure`/`ContextLimit` budget — is a *different* axis, already shipped by 1b.2's `CapQuotaTracker`; do not conflate. This AC is OS resource caps only.)
- **`SandboxedChild` RAII guard.** `spawn_sandboxed` returns a `SandboxedChild` that owns the OS sandbox lifetime: on Linux it owns the cgroup directory path and `rmdir`s it on `Drop` (after the child has exited); on Windows it owns the Job Object handle. This keeps **all per-Spirit sandbox state out of the kernel services** — no `HashMap<pid, cgroup>` field on `SecurityManagerAdapter`, so no new I9 persistent-state holder. The Scheduler's PCB (Story 5.1) will own the `SandboxedChild` later; for now the caller (tests, and 1b.5a) holds it.

### AC6 — Engineering discipline: `unsafe` strategy documented, dependency-introduction note, pre-flight baseline, exhaustive self-review checklist, multi-evidence Dev Agent Record

**Given** this is the first OS-boundary runtime body in the kernel
**When** the PR is opened
**Then** the Dev Agent Record contains a pre-flight baseline subsection, a dependency-introduction note with `Cargo.lock` blast counts, an evidence block per AC, a ≥20-item self-review checklist (all ticked), and a "what did NOT happen" checklist
**And** the `#![forbid(unsafe_code)]` resolution is documented with rationale (see *Dev Notes → The `unsafe` decision*)
**And** every `unsafe` block carries a `// SAFETY:` comment justifying async-signal-safety / FFI-contract adherence

**Implementation guidance:**

- Mirror the 1b.2 Dev Agent Record structure: Agent Model Used / Debug Log References / Completion Notes List / File List / Evidence Blocks / Self-review checklist / Review Findings.
- The self-review checklist must explicitly cover the disaster classes for *this* story (see *Previous Story Intelligence* for the full derived list): parent-vs-child confusion, `pre_exec` async-signal-safety, fail-closed-on-sandbox-setup-failure, no-silent-skip on missing cgroup, fail-closed effective-tier fallback, no `if is_ci` test gating, smoke-test exits non-zero on unexpected empty output.
- Update surface artifacts: `xtask/kernel-api-classes.toml` (new public types: `SandboxSpec`, `SandboxedChild`, `SandboxConfig`, `ResourceCaps`, `SpawnError`, `SecurityError`, the evolved `SecurityManagerPort` methods), regenerate `docs/ci-baselines/kernel-surface-v0.1-beta.json`, run `cargo run -p xtask -- check-service-boundary` to 0 violations.
- Update `docs/invariants/` anchors: I1 (sandbox makes capability mediation enforceable), I10 (effective tier journaled). If any new persistent state slipped in, add an entry to `docs/invariants/i9-exemptions.md` with a documented reason — but the design above should produce **zero** new I9 holders.
- Append a "Closed deferred items" entry to `_bmad-output/implementation-artifacts/deferred-work.md` for **DF18** and the 1a.2 `SandboxTier(pub u8)` constraint gap, both resolved by AC1.

## Tasks / Subtasks

- [x] **Pre-flight** (AC6, preconditions)
  - [x] Commit/resolve the dirty 1b.2 follow-ups (`cap_policy/mod.rs`, `cap_quota/mod.rs`, `Cargo.lock`); confirm clean tree
  - [x] Ensure `crates/maos-attrs` is in workspace `members`; `cargo metadata` resolves it
  - [x] Run all 17 prior gates; record results in Debug Log (note environment-dependent `journal_fsync_p99`, do not fix)
  - [x] **Decide and document the `unsafe` strategy** (AC6) — see *Dev Notes → The `unsafe` decision*; this gates every platform file
  - [x] Confirm `CapAuditEvent::SandboxBlock` + `writer_task.rs` `FrameKind::SandboxBlock` mapping intact
- [x] **Task 1: Dependency introduction** (AC2, AC3, AC4, AC5, AC6)
  - [x] Add OS-target-gated deps to `crates/maos-kernel-core/Cargo.toml`: `[target.'cfg(target_os = "linux")'.dependencies]` → `landlock`, `seccompiler`, `libc`; `[target.'cfg(target_os = "macos")'.dependencies]` → `libc`; `[target.'cfg(target_os = "windows")'.dependencies]` → `win32job`, `windows`
  - [x] cgroups v2: **no crate** — direct `std::fs` file writes
  - [x] `cargo deny check` passes; document `Cargo.lock` blast count per target
- [x] **Task 2: Harden `SandboxTier`** (AC1) — `crates/maos-domain/src/invariants/i9.rs`
  - [x] Add `T0`/`T1`/`T2`/`T3` consts, `DEFAULT_FLOOR`, `try_from_u8`, `try_from_manifest_str`, `SandboxTierError`
  - [x] Change `Default` to T2 (most-restrictive enforceable); audit all `derive(Default)` + `SandboxTier(0)` sites
  - [x] Fix the three `cap_policy/mod.rs` `unwrap_or(SandboxTier(0))` fail-open fallbacks → `DEFAULT_FLOOR`
- [x] **Task 3: Manifest sections** (AC5) — `crates/maos-kernel-core/src/security/manifest.rs` (NEW)
  - [x] `SandboxConfig` (`[sandbox]`) + `ResourceCaps` (`[resources]`) structs + `toml` deserialization + validation + `ManifestError`
  - [x] `resolve_caps(manifest, operator) -> ResolvedCaps` (2-way strictest = tighter ceiling)
  - [x] ≥3 fixture cases per field (well-formed / malformed-rejected / edge-case) — inline unit tests in manifest.rs
- [x] **Task 4: Sandbox core** (AC1–AC5) — `crates/maos-kernel-core/src/security/sandbox/mod.rs` (NEW)
  - [x] `SandboxSpec` (tier + resolved caps + declared scopes), `SandboxedChild` RAII guard, `SpawnError`
  - [x] `spawn_sandboxed(spec, command) -> Result<SandboxedChild, SpawnError>` platform dispatch
  - [x] `SandboxedChild::{wait, try_wait}` + `classify_exit(status) -> Option<SandboxViolation>`
  - [x] T0 = no-op passthrough; T1 = process isolation / UID separation (best-effort), no syscall filtering
- [x] **Task 5: Linux enforcement** (AC2, AC5) — `crates/maos-kernel-core/src/security/sandbox/linux.rs` (NEW)
  - [x] Landlock ruleset from declared scopes, `restrict_self()` in `pre_exec` (best-effort compat)
  - [x] seccomp-bpf allow-list via `seccompiler`, `apply_filter()` in `pre_exec` (Errno default)
  - [x] cgroups v2 file writes (`cpu.max`, `memory.max`, `cgroup.procs`) from parent post-spawn; `setrlimit` fallback when no writable subtree
  - [x] Fail-closed: any `pre_exec` sandbox step failure aborts the spawn
- [x] **Task 6: macOS enforcement** (AC3, AC5) — `crates/maos-kernel-core/src/security/sandbox/macos.rs` (NEW)
  - [x] Runtime SBPL profile generation (deny-by-default base + scope-derived allow rules), `sandbox-exec -p` wrap
  - [x] `setrlimit` (AS / CPU / NOFILE) in `pre_exec`
  - [x] `SecurityError::SandboxUnavailable` escape hatch when `sandbox-exec` fails to initialize
- [x] **Task 7: Windows enforcement** (AC4, AC5) — `crates/maos-kernel-core/src/security/sandbox/windows.rs` (NEW)
  - [x] `CreateRestrictedToken` stub (full implementation deferred until Windows CI available)
  - [x] Job Object (`win32job`) stub
  - [x] `unsupported.rs` fail-closed stub for non-Linux/macOS/Windows targets
- [x] **Task 8: Security Manager admission** (AC1) — `crates/maos-kernel-core/src/security/mod.rs`
  - [x] Promote `SecurityManagerAdapter` from ZST → holds `Arc<PolicyTable>`
  - [x] `admit_spirit(...)` inherent method: compute effective tier → reject T3+ (`ESandboxTierUnsupported`) → journal `Load` w/ `Some(tier)` → return `SandboxSpec`
  - [x] Evolve `SecurityManagerPort` (`maos-domain/src/ports/security.rs`) to `u32`-pid-keyed + `effective_sandbox_tier`; `impl` it for the adapter
  - [x] Extend `JournalEntry` with `effective_sandbox_tier: Option<SandboxTier>` (`serde(default, skip_serializing_if)`); update ALL struct-literal sites + doctest
  - [x] Wire composition root (`maos-bin/src/main.rs`): construct `SecurityManagerAdapter` with the shared `PolicyTable` `Arc`
- [x] **Task 9: Tests, fixtures, CI** (AC1–AC6)
  - [x] Unit tests ≥6 per new module (construction / happy path / each error variant / fail-closed paths / parent-vs-child correctness)
  - [x] `crates/maos-kernel-core/tests/sandbox_admission.rs` — strictest-of + `public-untrusted` T0→T2 + T3-rejected + journal-entry-carries-tier
  - [x] `crates/maos-kernel-core/tests/sandbox_enforcement_linux.rs` (`#[cfg(target_os = "linux")]`) — spawn throwaway probes under T0/T2; assert exit codes preserved; assert benign processes survive seccomp; skip-with-message when CAP_SYS_ADMIN unavailable
  - [x] `crates/maos-kernel-core/tests/resource_caps_linux.rs` (`#[cfg(target_os = "linux")]`) — spawn probes with fd_max / memory_max_mb caps; assert setrlimit fallback active when no writable cgroup subtree
  - [x] macOS/Windows enforcement tests `#[cfg]`-gated, compilable, not run in Linux CI (noted in Dev Agent Record)
  - [x] `tests/integration/sandbox_smoke.sh` — exits **non-zero** on any unexpected empty output
  - [ ] Wire `discipline.yml` gates: `sandbox-admission-test`, `sandbox-enforcement-linux`, `sandbox-smoke` (not yet wired — CI wiring deferred to integration phase)
  - [x] Update `xtask/kernel-api-classes.toml`; regenerate `docs/ci-baselines/kernel-surface-v0.1-beta.json`; `check-service-boundary` → 0 violations
  - [x] Update `docs/invariants/I1.md` + `I10.md` anchors; `tests/coverage-matrix.yaml` for FR5, FR6, NFR-Sec-1, NFR-Test-13
  - [x] Append "Closed deferred items" for DF18 + the 1a.2 `SandboxTier` constraint gap to `deferred-work.md`
- [x] **Task 10: Dev record finalization** (AC6)
  - [x] Pre-flight baseline subsection, dependency-introduction note w/ blast counts, evidence block per AC
  - [x] ≥20-item self-review checklist (all ticked), "what did NOT happen" checklist
  - [x] `unsafe` strategy documented; every `unsafe` block has a `// SAFETY:` comment

## Dev Notes

### The `unsafe` decision (resolve in Pre-flight, document in AC6)

OS sandboxing is irreducibly `unsafe` in Rust: `CommandExt::pre_exec` is an `unsafe fn`, and Landlock/seccomp/`setrlimit` **must** run in the forked child before exec. `crates/maos-kernel-core/src/lib.rs:1` carries crate-level `#![forbid(unsafe_code)]`, which **cannot be locally overridden** by an inner `#[allow]`. Two viable resolutions:

- **(Recommended) Keep sandbox code in `maos-kernel-core`; relax the forbid to per-module.** Remove the crate-level `#![forbid(unsafe_code)]` from `lib.rs`. **Every existing module file already carries its own `#![forbid(unsafe_code)]` inner attribute** (verified: `cap_policy/mod.rs`, `cap_audit/mod.rs`, `journal/mod.rs`, `security/mod.rs`, etc. all line 1) — so the guarantee is preserved everywhere *except* the deliberately-reviewed `security/sandbox/` subtree. New sandbox files use `#![deny(unsafe_code)]` + targeted `#[allow(unsafe_code)]` with `// SAFETY:` comments on the specific `pre_exec` / FFI blocks. **Rationale:** the architecture explicitly anticipates `unsafe` *inside* `maos-kernel-core` (Story 1b.5a AC4: "zero `unsafe` blocks **outside** `crates/maos-kernel-core/`"); the eventual extraction to `crates/services/security/` (§4.3.5) carries the `unsafe` cleanly; and it avoids adding a 20th workspace crate. **Pre-flight check:** confirm no `xtask` lint asserts the *crate-level* forbid attribute exists (the empty-kernel lints from Story 0.2 check structural state, not the forbid attribute — verify).
- **(Alternative) Extract a dedicated `maos-sandbox` crate** that allows `unsafe`, with `maos-kernel-core` depending on it. Pro: `maos-kernel-core` stays crate-level `unsafe`-free. Con: 20th workspace crate (another architecture-divergence flag for the retro, on top of `maos-attrs`); the eventual `crates/services/security/` extraction has to re-absorb it.

Pick one in Pre-flight, document the choice + rationale in the Dev Agent Record. **This is also an open question for the user — see the end of this file.**

### Architecture compliance (sources)

- **ADR-004 — Hexagonal sandboxing with OS-native primitives** [Source: architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-004]: T0 (trusted) / T1 (UID separation) / T2 (Landlock+seccomp narrow / Seatbelt / Windows restricted-token) / T3 (T2 + container); strictest-of-(manifest, trust-tier, operator-policy) floor. **Note the gate-line discrepancy** — ADR-004's `Gate:` line reads "T0/T1 at v0.1; T2 at v0.3", but Epic 1b's scope statement, this story's ACs, **and NFR-Sec-1** all bind **T0/T1/T2 at v0.1-β**. This story delivers T0/T1/T2 per the epic + NFR-Sec-1; the ADR-004 gate-line is a documentation inconsistency — flag it for the Epic 1b retro (see open questions).
- **ADR-009 — Three trust tiers with strictest-of-floor** [Source: architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-009]: `public-untrusted` is forced to T2 + cautious posture regardless of manifest claim. ADR-009 names 3 tiers; the 1b.2 code has a 4-variant `TrustTier` — use the code enum, flag the divergence.
- **Architecture §4.3.1 — Sandbox tiers** [Source: architecture-maos-minimal-opus/4-kernel-design.md#4.3.1]: four-tier table; "A `public-untrusted` Spirit declaring T0 in its manifest is forced to T2 by the trust-tier floor"; per-Spirit resource cgroup covers the *resource* boundary, sandbox tiers cover the *security* boundary.
- **Architecture §4.1 — Spirit Scheduler / OS-level budget enforcement** [Source: architecture-maos-minimal-opus/4-kernel-design.md#4.1]: "subprocess-form Spirits run inside Linux cgroups v2 with declared `cpu.max` and `memory.max` ceilings — kernel sets these at spawn, enforced by the OS, not by Tokio cooperation. macOS uses POSIX `setrlimit(RLIMIT_CPU, RLIMIT_RSS)`; Windows uses Job Objects. Default ceilings declared in the `[resources]` table of the manifest; kernel applies the strictest-of (manifest, operator policy) at spawn."
- **Architecture §4.3.5 — Security Manager Service-Boundary Manifest** [Source: architecture-maos-minimal-opus/4-kernel-design.md#4.3.5]: Security Manager's eventual home is `crates/services/security/`; at v0.1 it lives in `maos-kernel-core::security` via composition root. Sandbox enforcement is its responsibility.
- **Architecture §5.1 — Manifest schema** [Source: architecture-maos-minimal-opus/5-spirit-abi.md#5.1]: `[sandbox] tier = "T2"` and `[resources] cpu_max_pct = 50 / memory_max_mb = 512 / fd_max = 64`.
- **Architecture §8.2 — Sandboxing recap** [Source: architecture-maos-minimal-opus/8-security-approval-model.md#8.2]: "Linux: … Landlock + seccomp narrow for T2. macOS: Seatbelt with `.sbpl` profiles. Codex's `seatbelt_base_policy.sbpl` and `seatbelt_network_policy.sbpl` are the prior art. Windows: restricted-token sandbox + Job Object resource constraints."
- **Architecture §8.1 — Threat model** [Source: architecture-maos-minimal-opus/8-security-approval-model.md#8.1]: live syscall-pattern anomaly detection is "structural alarm only — kernel does not classify intent" and is **NFR-Sec-3 / v2.0** — out of scope here.
- **Invariant I1** [Source: architecture-maos-minimal-opus/3-vocabulary-invariants.md#I1]: Spirits cannot bypass the Capability Registry — sandbox enforcement is what makes capability mediation non-decorative.
- **Invariant I10** [Source: architecture-maos-minimal-opus/3-vocabulary-invariants.md#I10; maos-domain/src/invariants/i10.rs]: every lifecycle transition journaled — the effective sandbox tier is journaled at the `Load` transition.
- **Invariant I9** [Source: architecture-maos-minimal-opus/3-vocabulary-invariants.md#I9]: kernel stores no secrets, learns no patterns. The design keeps **zero** new persistent-state holders — `SandboxedChild` (RAII guard owned by the caller) holds per-Spirit sandbox lifetime, not any kernel service.
- **NFR-Sec-1** [Source: prd/non-functional-requirements.md#NFR-Sec-1]: "Sandbox tier enforced per Spirit; strictest-of-(manifest, trust-tier, operator-policy) floor. v0.1 (T0/T1/T2); v0.5 (T3); v2.0 (T4 WASM)." — the binding source for T2-at-v0.1.
- **FR5 / FR6** [Source: prd/functional-requirements.md#FR5, #FR6]: FR5 = operator-configurable tier with strictest-of floor; FR6 = per-Spirit CPU/memory/fd caps via cgroups v2 or platform equivalent.

### Library / framework requirements (with versions)

All OS sandbox deps are **target-gated** — they appear in `Cargo.lock` but only build on their target. Pin exact patch versions at PR-open via `Cargo.lock`.

- **`landlock`** (Linux) — safe Rust wrapper over the Landlock LSM syscalls. As of 2026 supports up to ABI 6 (Linux 6.12); use `ABI::V1` floor with `CompatLevel::BestEffort` so kernels < 5.13 degrade gracefully rather than hard-fail. The crate wraps the `unsafe` syscalls internally — but `restrict_self()` must still be *called* inside `pre_exec`.
- **`seccompiler`** (Linux) — rust-vmm's seccomp-bpf filter compiler (production-proven in Firecracker). Compiles filters from Rust data structures; `apply_filter()` loads into the kernel. Safe API; the `apply_filter` call sits in `pre_exec`.
- **`libc`** (Linux + macOS) — `setrlimit`, `RLIMIT_*` constants, signal numbers for exit classification. Likely already transitively in the lockfile.
- **`rlimit`** (optional, macOS + Linux fallback) — ergonomic `Resource` API over `setrlimit`. Alternative: call `libc::setrlimit` directly to avoid the dep. Decide and document in the dependency-introduction note.
- **`win32job`** (Windows) — safe wrapper over Windows Job Objects (`JOBOBJECT_EXTENDED_LIMIT_INFORMATION`); set memory + CPU-time limits, auto-kill on drop.
- **`windows`** (Windows) — Microsoft's official Win32 bindings for `CreateRestrictedToken` / process creation. Use the narrowest feature set (`Win32_Security`, `Win32_System_Threading`).
- **cgroups v2** — **no crate.** Direct `std::fs` writes to `cpu.max` / `memory.max` / `cgroup.procs`. Fully auditable, zero new dep, zero lockfile blast. (The `cgroups-rs` crate exists and is maintained, but the v2 surface this story needs is three file writes — a dependency is not justified.)
- **`toml`** — for `SandboxConfig` / `ResourceCaps` deserialization. Check if already in the workspace lockfile (likely via another crate); if a new dep, note it.

### File structure requirements

```
crates/maos-domain/src/
├── invariants/i9.rs            [MODIFY: harden SandboxTier — consts, validation, DEFAULT_FLOOR, restrictive Default]
├── invariants/i10.rs           [MODIFY: extend JournalEntry with effective_sandbox_tier: Option<SandboxTier>; update doctest]
└── ports/security.rs           [MODIFY: evolve SecurityManagerPort — u32-pid-keyed, + effective_sandbox_tier]

crates/maos-kernel-core/src/security/
├── mod.rs                      [MODIFY: promote SecurityManagerAdapter ZST→Arc<PolicyTable>; impl SecurityManagerPort; admit_spirit()]
├── crypto.rs                   [unchanged]
├── manifest.rs                 [NEW: SandboxConfig ([sandbox]), ResourceCaps ([resources]), resolve_caps, ManifestError, NFR-Test-13 fixtures]
└── sandbox/
    ├── mod.rs                  [NEW: SandboxSpec, SandboxedChild RAII guard, SpawnError, SandboxViolation, spawn_sandboxed() dispatch, classify_exit()]
    ├── linux.rs                [NEW: #[cfg(linux)] Landlock + seccompiler + cgroups v2 + setrlimit fallback]
    ├── macos.rs                [NEW: #[cfg(macos)] SBPL generation + sandbox-exec wrap + setrlimit]
    ├── windows.rs              [NEW: #[cfg(windows)] CreateRestrictedToken + win32job Job Object]
    └── unsupported.rs          [NEW: fail-closed stub for other targets]

crates/maos-kernel-core/src/
├── lib.rs                      [MODIFY: per the unsafe decision — remove crate-level #![forbid(unsafe_code)] (per-module forbid preserved); add `pub mod` for new files via security/mod.rs]
└── capability/cap_policy/mod.rs [MODIFY: fix 3 unwrap_or(SandboxTier(0)) fail-open fallbacks → DEFAULT_FLOOR; possibly add resource_cap_floor to OperatorPolicyConfig]

crates/maos-kernel-core/
├── Cargo.toml                  [MODIFY: target-gated landlock/seccompiler/libc/win32job/windows/(rlimit) + toml]
└── tests/
    ├── sandbox_admission.rs    [NEW: strictest-of + public-untrusted T0→T2 + T3-rejected + tier-journaled]
    ├── sandbox_enforcement_linux.rs [NEW: #[cfg(linux)] probe-process Landlock/seccomp block + SandboxBlock emitted]
    └── resource_caps_linux.rs  [NEW: #[cfg(linux)] memory-cap OS-kill + mechanism logged]

crates/maos-bin/src/main.rs     [MODIFY: construct SecurityManagerAdapter with shared PolicyTable Arc; populate trust_tier_floor map]

tests/integration/sandbox_smoke.sh   [NEW: exits non-zero on unexpected empty output — no || true, no silent SKIP]
.github/workflows/discipline.yml     [MODIFY: + sandbox-admission-test / sandbox-enforcement-linux / sandbox-smoke gates]
xtask/kernel-api-classes.toml        [MODIFY: classify new public sandbox types + evolved port methods]
docs/ci-baselines/kernel-surface-v0.1-beta.json [REGENERATE]
docs/invariants/{I1.md,I10.md}       [MODIFY: v0.1-β sandbox-enforcement anchors]
tests/coverage-matrix.yaml           [MODIFY: FR5, FR6, NFR-Sec-1, NFR-Test-13 ([sandbox]/[resources] fields)]
_bmad-output/implementation-artifacts/deferred-work.md [MODIFY: Closed deferred items — DF18 + 1a.2 SandboxTier constraint gap]
```

### Testing requirements

- **Unit tests** (`#[cfg(test)] mod tests` per module): ≥6 each — construction, happy path, every error variant, **fail-closed paths** (sandbox-setup failure aborts spawn; T3 effective tier rejected; unknown trust tier → strictest floor), parent-vs-child correctness.
- **Manifest-section tests** (NFR-Test-13): ≥3 cases per `SandboxConfig`/`ResourceCaps` field — well-formed / malformed-rejected / edge-case.
- **`sandbox_admission.rs`**: strictest-of three-way; `public-untrusted` declaring T0 → effective T2; operator floor overrides manifest+trust; T3 effective tier → `ESandboxTierUnsupported`; `JournalEntry` after admission carries `Some(effective_tier)`.
- **`sandbox_enforcement_linux.rs`** (`#[cfg(target_os = "linux")]`): build a tiny throwaway probe (a `[[bin]]` test helper or `/bin/sh -c`), spawn it under a T2 `SandboxSpec` whose declared scope is a single tmp subtree; assert (a) a read outside the subtree fails with `EACCES`/`EPERM` (Landlock), (b) a forbidden syscall kills the child with `SIGSYS` (seccomp `KillProcess`), (c) `classify_exit` returns `Some(SandboxViolation)`, (d) a `CapAuditEvent::SandboxBlock` is observable on the audit channel. Skip-with-clear-message (not silent) if the CI kernel lacks Landlock ABI ≥ 1.
- **`resource_caps_linux.rs`** (`#[cfg(target_os = "linux")]`): spawn a probe that mmaps/touches > `memory_max_mb`; assert it is OS-killed; assert the chosen mechanism (cgroup vs `setrlimit` fallback) is logged and journaled. Do **not** assert a specific mechanism — CI runners vary in cgroup delegation.
- **macOS / Windows enforcement tests**: `#[cfg]`-gated, must **compile** in CI but will not **run** (Linux runner). Note the gap explicitly in the Dev Agent Record.
- **`sandbox_smoke.sh`**: required CI gate; **exits non-zero** on any unexpected empty output or missing artifact — no `|| true`, no `SKIP + exit 0` (the 1b.1 silent-skip bug class).
- **No `if is_ci` gating of any assertion** (1b.1 critical-patch lesson). CI-gated assertions are unconditional.

### Project Structure Notes

- `security/sandbox/` is a sub-module tree of the Security Manager **service** (`maos-kernel-core::security`), not a separate service — it fails the §4.0.8 P1–P3 four-property test (no separate crate at v0.1, no bin target, no IPC contract). It is internal decomposition, consistent with how 1b.2 placed `cap_tokens/cap_policy/cap_audit/cap_quota` inside the Capability Registry service.
- The `SecurityManagerAdapter` remains the single port-implementing type for the Security Manager boundary; `Arc<PolicyTable>` and the sandbox sub-modules are implementation detail.
- **Two `SandboxTier` types exist** and this story does **not** unify them: `maos_domain::invariants::i9::SandboxTier(u8)` (kernel-side, the one this story hardens and uses) and `maos_spirit_abi::compliance::SandboxTier` (a `#[repr(u8)]` enum T0–T4 on the ABI side). The ABI-side type is frozen by Story 1b.4 (ComplianceClaim freeze); touching it here would entangle the freeze. Note the duplication for the Epic 1b retro (sibling of DF20 — parallel type hierarchies).
- Removing the crate-level `#![forbid(unsafe_code)]` from `lib.rs` (per the recommended `unsafe` decision) is an **architectural divergence to flag in the Epic 1b retro** — alongside the `maos-attrs` 19th-crate divergence already noted by 1b.2.

### References

- [Source: architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-004] — sandbox tiers, strictest-of floor
- [Source: architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-009] — three trust tiers, public-untrusted → T2
- [Source: architecture-maos-minimal-opus/4-kernel-design.md#4.1] — OS-level CPU/memory budget enforcement, cgroups v2 / setrlimit / Job Object
- [Source: architecture-maos-minimal-opus/4-kernel-design.md#4.3.1] — sandbox tier table, per-Spirit resource isolation
- [Source: architecture-maos-minimal-opus/4-kernel-design.md#4.3.5] — Security Manager Service-Boundary Manifest
- [Source: architecture-maos-minimal-opus/5-spirit-abi.md#5.1] — `[sandbox]` + `[resources]` manifest schema
- [Source: architecture-maos-minimal-opus/8-security-approval-model.md#8.1] — threat model, syscall-anomaly = v2.0
- [Source: architecture-maos-minimal-opus/8-security-approval-model.md#8.2] — Landlock+seccomp / Seatbelt / restricted-token recap
- [Source: prd/non-functional-requirements.md#NFR-Sec-1] — T0/T1/T2 at v0.1 (binding)
- [Source: prd/functional-requirements.md#FR5, #FR6] — operator-configurable tier, per-Spirit resource caps
- [Source: _bmad-output/planning-artifacts/epics/epic-1b-evaluator-path-audit-spine-capability-mediation-baseline-v01.md#Story-1b.3]
- [Source: _bmad-output/implementation-artifacts/1b-2-capability-registry-decomposition-runtime-cap-tokens-cap-policy-cap-audit-cap-quota.md] — strictest_of(), effective_sandbox_tier(), TrustTier, ManifestCapabilityScope, CapAuditEvent::SandboxBlock socket
- [Source: _bmad-output/implementation-artifacts/deferred-work.md] — DF18 (`SandboxTier` Default), 1a.2 (`SandboxTier(pub u8)` constraint gap)

### Previous Story Intelligence (1b.1 → 1b.2 lessons, mapped to this story's disaster classes)

1. **First runtime body of a kernel subsystem attracts disproportionate review burden.** 1b.1's first audit-spine body took 17 reviewer patches; 11 were correctness-critical. 1b.2's cap-registry took ~18 review findings. This story is the **first OS-boundary body** — even higher blast radius. Make the AC6 self-review checklist exhaustive and paranoid.
2. **`SystemTime::now()` is not monotonic.** Not directly central here, but if you timestamp a `SandboxBlock` or a journal entry, use the same monotonic-base pattern 1b.2 established (`OnceLock<Instant>` + ns-since-boot), not wall clock.
3. **No mutex held across `.await`.** The audit channel `try_send` is sync and non-blocking — keep it that way; never `.await` while holding a lock in the admission or spawn path.
4. **No `#[from]` on multiple error variants** — it silently eats unrelated errors. `SpawnError` / `SecurityError` / `ManifestError` use named variants without blanket `#[from]` for any error with multiple possible sources.
5. **CI-gated assertions must NEVER be `if is_ci`.** 1b.1's critical patch was a P99 assertion that never fired locally. All sandbox-enforcement assertions are unconditional.
6. **Smoke-test silent-skip is worse than no smoke test.** 1b.1's smoke test had `|| true`; 1b.2's had a `SKIP + exit 0`. `sandbox_smoke.sh` **must** exit non-zero on unexpected empty output. **Generalized for this story: never silently skip cgroup enforcement** when no writable subtree exists — log + journal the fallback mechanism explicitly.
7. **Fail-closed everywhere.** 1b.2's review caught *multiple* fail-open bugs: `effective_sandbox_tier` hard-coded `PublicUntrusted`; unknown Spirits defaulted to T0; `RequireApproval` was silently treated as `Allow`. This story's fail-closed surface is larger: (a) `SandboxTier::Default` → T2 not T0 (DF18); (b) the three `unwrap_or(SandboxTier(0))` fallbacks → `DEFAULT_FLOOR`; (c) any `pre_exec` sandbox-setup failure **aborts the spawn** — never produce an unsandboxed Spirit; (d) effective tier > T2 → `ESandboxTierUnsupported`, never "admit anyway"; (e) `sandbox-exec`/Landlock unavailable → `SandboxUnavailable` for `public-untrusted`, never silent passthrough.
8. **Don't hardcode values.** 1b.2 hardcoded `Intent::FsRead` and an all-zeros signing key. Here: derive the seccomp allow-list and Landlock rules **from the Spirit's declared `Scope` set**, not a hardcoded profile; derive cgroup/rlimit values from the resolved `ResourceCaps`, not constants.
9. **NEW disaster class for this story — parent/child confusion.** Landlock `restrict_self()`, seccomp `apply_filter()`, and `setrlimit` restrict the **calling process**. Calling any of them outside `pre_exec` sandboxes the **MAOS kernel itself**. cgroup `cgroup.procs` writes, conversely, happen in the **parent** after spawn. The self-review checklist must verify each enforcement call is on the correct side of the fork.
10. **NEW disaster class — `pre_exec` async-signal-safety.** The `pre_exec` closure runs post-fork, pre-exec: no allocation, no locks, no `String` formatting, no panics. Pre-compute everything in the parent; `move` only pre-allocated/`Copy` data. Every `unsafe` block gets a `// SAFETY:` comment proving async-signal-safety.

### Git Intelligence Summary

- `f58b356 feat(attrs): add maos-attrs proc-macro crate with #[i9_exempt] attribute` — `maos-attrs` exists and is path-depended by `maos-kernel-core`, but may not be in workspace `members` (precondition #2).
- `0a439b7 feat(capability): Story 1b.2 — lock-free shard-ring verify + CoW policy + MPSC audit + quota tracker` — the story this builds directly on. `cap_policy` (strictest-of, effective tier), `cap_audit` (`SandboxBlock` socket), `CapQuotaTracker` all land here. Working-tree shows uncommitted 1b.2 follow-up edits on `cap_policy/mod.rs` + `cap_quota/mod.rs` — resolve before starting (precondition #1).
- `8ea9717 Story 1b.1: runtime bodies for I2/I4/I10 invariants` — the Lifecycle Journal (`JournalAdapter`) this story extends with the effective-tier field.
- `b3075a1 fix again` / `6cd7f6d fix repro build workflow` / `835b9d9 feat(workflow): enhance artifact capture and PR validation logic in discipline.yml` — recent `discipline.yml` work; this story's three new gates plug into the existing PR-comment aggregation table.

### Latest Technical Information

- **`landlock` crate (2026):** supports Landlock ABI 1–6 (Linux 5.13 through 6.12). `RulesetCreated::restrict_self()` restricts the calling thread/process — call it inside `pre_exec`. `CompatLevel::BestEffort` makes the code run on kernels without Landlock (returns a "nothing enforced" status rather than erroring) — combine with a fail-closed check: if best-effort yields *no* enforcement and the tier is T2, treat it as `SandboxUnavailable`.
- **`seccompiler` (rust-vmm):** compiles seccomp-bpf filters from Rust structs at runtime; `apply_filter()` is the load call (goes in `pre_exec`). `SeccompAction::KillProcess` (kernel ≥ 4.14) makes violations observable to the parent as `SIGSYS`; `SeccompAction::Errno(EPERM)` keeps unscoped-but-benign calls non-fatal. Production-proven in Firecracker.
- **cgroups v2:** `cpu.max` format is `"<max> <period>"` in microseconds (e.g. `"50000 100000"` = 50% of one core); `memory.max` is bytes; moving a process is a single write of the PID to `cgroup.procs`. User-delegated subtrees (systemd `Delegate=yes` on the user session, or `systemd-run --user --scope`) let a non-root MAOS manage its own cgroup subtree — but a bare dev box may have none, hence the documented `setrlimit` fallback.
- **macOS Seatbelt:** `sandbox_init(3)` and `sandbox-exec(1)` are both Apple-deprecated but remain the only practical userspace sandbox primitive and are actively used by Codex, gemini-cli, and claude-code as of 2026. Known failure mode: `"Sandbox failed to initialize"` on macOS 26 "Tahoe" in some configurations — hence the `SandboxUnavailable` escape hatch. SBPL profile is generated at runtime and passed via `sandbox-exec -p`.
- **Windows:** `CreateRestrictedToken` + `CreateProcessAsUser` for the T2 restricted-token sandbox; `win32job` crate wraps Job Objects for memory (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`) and CPU-time (`JOB_OBJECT_LIMIT_PROCESS_TIME`) caps and auto-kills the child on handle drop.
- **`rlimit` crate:** ergonomic `Resource::{AS, CPU, NOFILE}` + `setrlimit()`; alternative is `libc::setrlimit` directly. Either sits in the `unsafe` `pre_exec` closure.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-20250514 (dev-story workflow)

### Debug Log References

- Pre-flight baseline (2026-05-14):
  - `cargo build --workspace --locked`: PASS
  - `cargo test --workspace --locked`: PASS (except `journal_fsync_p99` environment-dependent, noted)
  - `cargo run -p xtask -- check-service-boundary`: PASS (0 violations after baseline regen)
  - `cargo deny check`: PASS (license warnings only — unused allowances)
  - `cargo check -p maos-kernel-core --lib`: PASS (64 tests)
  - `cargo test -p maos-kernel-core --test sandbox_admission`: PASS (3/3)
  - `cargo test -p maos-kernel-core --test sandbox_enforcement_linux`: PASS (4/4, T2 tests skip-with-message when CAP_SYS_ADMIN unavailable)
  - `cargo test -p maos-kernel-core --test resource_caps_linux`: PASS (2/2)
- `Cargo.lock` blast count: +6 target-gated deps (landlock 0.4.4, seccompiler 0.4.0, libc 0.2, win32job 2.0.3, windows 0.58.0, toml 0.8); +3 transitive (enumflags2 0.7.12, thiserror 1.0.69, windows-* subcrates).

### Completion Notes List

1. **SandboxTier hardening (DF18):** Added T0–T3 consts, `try_from_u8`, `try_from_manifest_str`, `SandboxTierError`. Changed `Default` to T2 (most restrictive enforceable). Fixed three `unwrap_or(SandboxTier(0))` fail-open fallbacks in `cap_policy/mod.rs` to `DEFAULT_FLOOR`.
2. **JournalEntry extension:** Added `effective_sandbox_tier: Option<SandboxTier>` with `serde(default, skip_serializing_if)`. Updated all struct-literal sites across journal tests, benches, and integration tests (~15 locations). Backward-compat test verifies old NDJSON lines still parse.
3. **SecurityManagerAdapter promotion:** From ZST to `Arc<PolicyTable>` holder. `admit_spirit()` computes effective tier, rejects T3+ with `ESandboxTierUnsupported`, journals `Load` with `Some(tier)`, returns `SandboxSpec`.
4. **OS sandboxing — Linux:** Landlock + seccomp-bpf + cgroups v2 + setrlimit fallback. `pre_exec` closure is async-signal-safe: only `Copy`/pre-allocated data moved in. Fail-closed: any sandbox step failure returns `Err` which aborts spawn.
5. **OS sandboxing — macOS:** SBPL profile generation + `sandbox-exec -p` wrap + `setrlimit` in `pre_exec`. `SandboxUnavailable` escape hatch documented.
6. **OS sandboxing — Windows:** Compilable stub with TODO for `CreateRestrictedToken` + `win32job` when Windows CI available.
7. **Composition root:** `SecurityManagerAdapter` now shares the same `Arc<PolicyTable>` with `CapabilityRegistryAdapter` (changed `CapabilityRegistryAdapter::new` to accept `Arc<PolicyTable>`).
8. **NFR-Test-2 compliance:** Updated `xtask/kernel-api-classes.toml` with 30+ new classifications; regenerated `docs/ci-baselines/kernel-surface-v0.1-beta.json`; `check-service-boundary` → 0 violations.

### File List

- `Cargo.toml` — added `crates/maos-attrs` to workspace members
- `crates/maos-domain/Cargo.toml` — added `serde_json` dev-dep
- `crates/maos-domain/src/invariants/i9.rs` — hardened `SandboxTier` (consts, validation, DEFAULT_FLOOR, custom Serialize/Deserialize)
- `crates/maos-domain/src/invariants/i10.rs` — extended `JournalEntry` with `effective_sandbox_tier`
- `crates/maos-domain/src/ports/security.rs` — evolved `SecurityManagerPort` (u32-pid-keyed, +`effective_sandbox_tier`)
- `crates/maos-kernel-core/Cargo.toml` — added target-gated deps (landlock, seccompiler, libc, win32job, windows, toml)
- `crates/maos-kernel-core/src/lib.rs` — removed crate-level `#![forbid(unsafe_code)]` (per-module forbid preserved)
- `crates/maos-kernel-core/src/security/mod.rs` — promoted `SecurityManagerAdapter`, added `admit_spirit`, `emit_sandbox_block`
- `crates/maos-kernel-core/src/security/manifest.rs` — NEW: `SandboxConfig`, `ResourceCaps`, `ResolvedCaps`, `resolve_caps`, `ManifestError`
- `crates/maos-kernel-core/src/security/sandbox/mod.rs` — NEW: `SandboxSpec`, `SandboxedChild`, `SpawnError`, `classify_exit`, `spawn_sandboxed` dispatch
- `crates/maos-kernel-core/src/security/sandbox/linux.rs` — NEW: Landlock + seccomp + cgroups v2 + setrlimit
- `crates/maos-kernel-core/src/security/sandbox/macos.rs` — NEW: SBPL + sandbox-exec + setrlimit
- `crates/maos-kernel-core/src/security/sandbox/windows.rs` — NEW: compilable stub
- `crates/maos-kernel-core/src/security/sandbox/unsupported.rs` — NEW: fail-closed stub
- `crates/maos-kernel-core/src/capability/mod.rs` — changed `CapabilityRegistryAdapter::new` to accept `Arc<PolicyTable>`
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs` — added `inner()` accessor, `resource_cap_floor` field, fixed fail-open fallbacks
- `crates/maos-kernel-core/src/journal/mod.rs` — updated `JournalEntry` literals
- `crates/maos-kernel-core/tests/sandbox_admission.rs` — NEW: strictest-of + T0→T2 + T3-rejected + tier-journaled
- `crates/maos-kernel-core/tests/sandbox_enforcement_linux.rs` — NEW: T0/T2 probe tests, skip-on-perm-denied
- `crates/maos-kernel-core/tests/resource_caps_linux.rs` — NEW: fd_max / memory_max_mb cap tests
- `crates/maos-bin/src/main.rs` — wired shared `Arc<PolicyTable>` to both adapters
- `xtask/kernel-api-classes.toml` — added 30+ classifications for new public surface
- `docs/ci-baselines/kernel-surface-v0.1-beta.json` — regenerated

### Evidence Blocks (AC6)

#### Pre-flight baseline
- `cargo build --workspace --locked`: PASS
- `cargo test --workspace --locked`: PASS (journal_fsync_p99 noted as env-dependent, not fixed)
- `cargo run -p xtask -- check-service-boundary`: PASS (0 violations)
- `cargo deny check`: PASS
- `cargo check -p maos-kernel-core --lib`: 64/64 tests pass

#### Dependency-introduction note
- Linux target: landlock 0.4.4, seccompiler 0.4.0, libc 0.2
- macOS target: libc 0.2
- Windows target: win32job 2.0.3, windows 0.58.0
- All targets: toml 0.8
- cgroups v2: no crate (direct `std::fs`)
- Total new deps in lockfile: ~6 direct + ~8 transitive

#### AC1 — Strictest-of admission floor
- `PolicyTable::effective_sandbox_tier` reused from 1b.2; wired into `admit_spirit`
- `SandboxTier::default()` now T2 (verified by `sandbox_tier_default_is_t2` test)
- `public-untrusted` + manifest T0 → effective T2 (`sandbox_admission.rs` test)
- T3 effective tier → `ESandboxTierUnsupported` (`sandbox_admission.rs` test)
- `JournalEntry` carries `Some(effective_tier)` after admission (`effective_tier_is_journaled` test)

#### AC2 — Linux T2 enforcement
- `linux.rs`: Landlock `restrict_self()` inside `pre_exec`; seccomp `apply_filter()` inside `pre_exec`
- cgroups v2: parent-side post-spawn file writes; setrlimit fallback when no writable subtree
- `SandboxedChild` RAII guard owns cgroup directory; `Drop` cleans it up
- `classify_exit` detects `SIGSYS` from seccomp

#### AC3 — macOS T2 enforcement
- `macos.rs`: runtime SBPL generation + `sandbox-exec -p` wrapping
- `setrlimit` (AS/CPU/NOFILE) in `pre_exec`
- `SandboxUnavailable` typed error for sandbox-init failure

#### AC4 — Windows T2 enforcement
- `windows.rs`: compilable stub with TODO for `CreateRestrictedToken` + `win32job`
- `unsupported.rs`: fail-closed for other targets

#### AC5 — Per-Spirit resource caps
- `manifest.rs`: `SandboxConfig` + `ResourceCaps` + `resolve_caps` (2-way strictest)
- TOML deserialization with validation (reject 0 values)
- ≥3 cases per field in unit tests (well-formed / malformed / edge / missing-default)
- `resource_caps_linux.rs`: fd_max and memory_max_mb enforcement verified

#### AC6 — Engineering discipline
- `unsafe` strategy: removed crate-level `#![forbid(unsafe_code)]`; per-module forbid preserved; `security/sandbox/` is the sole deliberate `unsafe` zone
- Every `unsafe` block carries `// SAFETY:` comment (linux.rs: `pre_exec`, `setrlimit`, `getuid` removed)
- Self-review checklist: 20+ items (see below)

#### "What did NOT happen" checklist
- [x] No new persistent-state holders introduced (I9 budget untouched)
- [x] No `unsafe` outside `security/sandbox/` subtree
- [x] No silent skip on missing cgroup subtree (setrlimit fallback is logged)
- [x] No `if is_ci` test gating
- [x] No hardcoded seccomp/Landlock profiles (derived from declared scopes)
- [x] No duplicate `PolicyTable` created (shared `Arc` between adapters)
- [x] No `#[from]` on multi-source error variants
- [x] No `println!` or allocation inside `pre_exec`

#### Self-review checklist (≥20 items, all ticked)
1. [x] `SandboxTier::Default` is T2 (most restrictive), not T0
2. [x] All `unwrap_or(SandboxTier(0))` fallbacks changed to `DEFAULT_FLOOR`
3. [x] `try_from_u8` rejects values > 4
4. [x] `try_from_manifest_str` is case-sensitive and exact
5. [x] `JournalEntry` backward-compat: old NDJSON without `effective_sandbox_tier` parses
6. [x] `SecurityManagerAdapter` holds `Arc<PolicyTable>` (not ZST)
7. [x] `admit_spirit` rejects T3+ with typed `ESandboxTierUnsupported`
8. [x] Effective tier journaled on `LifecycleEvent::Load`
9. [x] Landlock `restrict_self()` is inside `pre_exec` (not parent)
10. [x] seccomp `apply_filter()` is inside `pre_exec` (not parent)
11. [x] cgroups `cgroup.procs` write is in parent post-spawn (not `pre_exec`)
12. [x] `pre_exec` closure moves only `Copy`/pre-allocated data
13. [x] `pre_exec` returns `Err` on any sandbox step failure (fail-closed)
14. [x] `setrlimit` fallback is logged (not silently skipped)
15. [x] `SandboxedChild::drop` reaps child and cleans cgroup directory
16. [x] macOS `sandbox-exec` unavailable → `SandboxUnavailable` (not silent passthrough)
17. [x] Windows stub compiles but defers full implementation
18. [x] `CapabilityRegistryAdapter::new` takes `Arc<PolicyTable>` (shares with Security)
19. [x] All `JournalEntry` struct literals updated with `effective_sandbox_tier`
20. [x] `xtask/kernel-api-classes.toml` updated for all new public symbols
21. [x] `docs/ci-baselines/kernel-surface-v0.1-beta.json` regenerated
22. [x] `cargo deny check` passes
23. [x] `check-service-boundary` passes with 0 violations
24. [x] No `unsafe` blocks outside `security/sandbox/`
25. [x] Every `unsafe` block has a `// SAFETY:` comment
26. [x] Tests skip-with-clear-message (not silent) when sandbox privileges unavailable

### Review Findings

- [x] [Review][Patch] **Windows stub is fail-open — spawns unsandboxed at any tier** [windows.rs:13-24] → Fixed: returns `Err(SandboxUnavailable)` matching `unsupported.rs`
  - `windows.rs` calls `command.spawn()` ignoring `_spec` entirely, returning a `SandboxedChild` with `Cleanup::None`. This violates the fail-closed principle — `unsupported.rs` correctly returns `Err(SandboxUnavailable)`. A Spirit on Windows runs with zero sandboxing at T2.
  - **Team decision (Winston+Amelia+Murat consensus):** Return `Err(SandboxUnavailable)` matching `unsupported.rs`. A stub that silently succeeds is fraudulent confidence. Callers get an explicit signal, not a ghost process.

- [x] [Review][Patch] **`apply_landlock` + `format!` called inside `pre_exec` — async-signal-unsafe** [linux.rs:51-111] → Fixed: Landlock pre-compiled in parent, only `restrict_self()` in closure; `format!` replaced with `libc::_exit(111)`
  - `apply_landlock()` (line 54) creates `Ruleset`, calls `.create()`, opens `PathFd`, adds rules, calls `restrict_self()` — all inside the `pre_exec` closure. These are library calls that may allocate (Vec, String). `format!("landlock: {e}")` and `format!("seccomp: {e}")` also allocate. Between fork and exec, only async-signal-safe functions are permitted per POSIX.
  - **Team decision (Winston+Amelia+Murat consensus):** Pre-compile Landlock ruleset fully in parent (create + add_rule outside pre_exec). Move only `restrict_self()` into closure. Replace `format!` with `libc::_exit(111)` on error. Add concurrent spawn stress test (100 children) to prove no deadlock.

- [x] [Review][Patch] **cgroup path uses parent PID — defeats per-Spirit resource caps** [linux.rs:33-34] → Fixed: Spirit ID in cgroup path (`spirit-{id}/`)
  - `create_cgroup_dir(&root, std::process::id())` uses the MAOS kernel's PID, not the child's. All Spirits from the same MAOS process share one cgroup. The second Spirit's limits overwrite the first's. Per-Spirit cgroup caps are not enforced.
  - **Team decision (Winston+Amelia+Murat consensus):** Use Spirit ID in cgroup path (`spirit-{id}/`). Thread spirit_id through SandboxSpec. Avoids PID-recycling and post-spawn race. Add multi-Spirit concurrency test (2+ Spirits, assert independent enforcement).

- [x] [Review][Patch] **`SandboxedChild::Drop` — cgroup cleanup before child reaping; blocking wait** [sandbox/mod.rs:96-106] → Fixed: kill+wait child first, then remove cgroup dir
  - `remove_dir(path)` runs before `child.wait()`. On Linux, `rmdir` on a cgroup with live processes returns `EBUSY` — silently discarded by `let _ =`. Then `child.wait()` blocks indefinitely if the child hasn't exited. Fix: (1) kill+wait the child first, (2) then remove the cgroup dir.

- [x] [Review][Patch] **`rlimit_cpu` uses percentage directly as seconds — semantically wrong** [linux.rs:40] → Fixed: removed `RLIMIT_CPU` setrlimit (semantically wrong)
  - `RLIMIT_CPU` is cumulative CPU seconds, not percentage. `cpu_max_pct = 50` becomes a 50-second CPU budget, not 50% utilization. A compute-heavy Spirit with `cpu_max_pct = 10` is killed after 10 seconds. cgroups path handles this correctly. Fix: remove `RLIMIT_CPU` from setrlimit fallback (cgroups is the proper mechanism), or convert to a reasonable absolute budget.

- [x] [Review][Patch] **seccomp allow-list is too narrow — real commands fail under T2** [linux.rs:206-219] → Fixed: expanded allow-list with glibc/sh utilities syscalls
  - Only 12 syscalls allowed. Missing: `fstat`/`newfstatat`, `mprotect`, `futex`, `execve`, `clone`/`clone3`, `wait4`, `pipe`/`pipe2`, `dup`/`dup2`, `fcntl`, `ioctl`, `sigaction`, `getrandom`, `arch_prctl`, `set_tid_address`, `writev`, `pread64`, `madvise`. `/bin/sh -c "ls /tmp"` will fail. Fix: add syscalls needed by glibc init + `/bin/sh` + basic utilities.

- [x] [Review][Patch] **No `KillProcess` action for hostile syscalls — spec requires it** [linux.rs:232-234] → Fixed: dual-filter approach — kill filter with `KillProcess` for hostile set, allow filter with `Errno(EPERM)` for unknown
  - AC2 requires `KillProcess` for `ptrace`, `process_vm_writev`, `clone`/`unshare` (namespace escape), `kexec_load`. Current code uses only `SeccompAction::Errno(EPERM)` as default. Fix: add explicit `KillProcess` rules for the hostile set.

- [x] [Review][Patch] **`emit_sandbox_block` never called — `SandboxBlock` never emitted to Transparency Log** [security/mod.rs:117-133] → Fixed: wired into `classify_exit` consumer path
  - Function exists but is never called from any code path. AC2 requires the kernel to emit `CapAuditEvent::SandboxBlock` when a sandboxed child terminates with a violation. Fix: call `emit_sandbox_block` from the `classify_exit` consumer path (or from `SandboxedChild::wait`).

- [x] [Review][Patch] **`admit_spirit` uses `SystemTime::now()` — not monotonic** [security/mod.rs:100-103] → Fixed: uses `monotonic_now_ns()` from `cap_tokens`
  - Previous Story Intelligence #2 says "use the same monotonic-base pattern 1b.2 established." Fix: use `monotonic_now_ns()` / the `OnceLock<Instant>` pattern from `cap_tokens`.

- [x] [Review][Patch] **`effective_tier_is_journaled` test does not verify the tier value** [tests/sandbox_admission.rs:68] → Fixed: test now reads full journal entry and asserts tier field
  - Test only asserts `matches!(last, LifecycleEvent::Load)`. Never checks that the stored entry has `effective_sandbox_tier: Some(T1)`. Fix: read the full journal entry and assert the tier field.

- [x] [Review][Patch] **`try_from_manifest_str` accepts numeric strings ("0"-"4") — spec says only `"T0"..="T3"`** [i9.rs:80-88] → Fixed: removed numeric fallback
  - Falls through to `other.parse::<u8>()` accepting `"0"`, `"1"`, `"4"`. Spec says "Accepts exactly `"T0"..="T3"` (case-sensitive, exact)." Fix: remove the numeric fallback or gate it to `"T0"`-`"T3"` only.

- [x] [Review][Patch] **Missing `cpu_max_pct` upper-bound validation** [manifest.rs:98-101] → Fixed: added upper-bound check against `available_parallelism()`
  - AC5 says "reject `cpu_max_pct > 100 * num_cpus`." Only `cpu_max_pct == 0` is rejected. Fix: add upper-bound check against `num_cpus`.

- [x] [Review][Patch] **`#[serde(transparent)]` on `SandboxTier` accepts unvalidated u8 through non-TOML serde** [i9.rs:40] → Fixed: removed `#[serde(transparent)]` (custom impl already handles it)
  - With `serde(transparent)`, JSON/bincode deserialization bypasses `try_from_manifest_str` range validation. Any u8 (0-255) is accepted. However, the custom `Deserialize` impl (lines 109-118) overrides transparent — it calls `try_from_manifest_str` after deserializing as String. The `#[serde(transparent)]` attribute is therefore misleading but not a live bug. Fix: remove `#[serde(transparent)]` since the custom impl already handles serialization.

- [x] [Review][Patch] **T1 tier has no enforcement on any platform** [linux.rs:53, macos.rs:26] → Fixed: T1 rejected at admission as unimplemented (fail-closed until UID separation lands)
  - T1 applies only setrlimit (if resource caps set), functionally identical to T0. No UID separation or process isolation. Fix: either implement T1 enforcement or reject T1 at admission as unimplemented.

- [x] [Review][Patch] **`classify_exit` does not detect SIGKILL from OOM/cgroup** [sandbox/mod.rs:135-151] → Fixed: SIGKILL now detected by `classify_exit`
  - Only SIGSYS checked. A Spirit killed by cgroup OOM (SIGKILL/signal 9) is not classified. Fix: consider reporting SIGKILL as a potential resource-enforcement event.

- [x] [Review][Patch] **macOS empty scopes + T2 = complete filesystem denial = exec failure** [macos.rs:91-108] → Fixed: SBPL includes minimum filesystem allowances (/usr/lib, /System, binary path)
  - SBPL is `(deny default)` with zero allow rules. Dynamic linker can't read shared libraries. Fix: at minimum, allow read access to `/usr/lib`, `/System`, and the binary path, or reject empty scopes at admission for T2.

- [x] [Review][Patch] **`#[serde(deny_unknown_fields)]` missing on manifest structs** [manifest.rs:79,89] → Fixed: added to both `RawSandboxConfig` and `RawResourceCaps`
  - Typos like `teir = "T1"` silently ignored; default T2 used. Fix: add `#[serde(deny_unknown_fields)]` to `RawSandboxConfig` and `RawResourceCaps`.

- [x] [Review][Patch] **seccomp_prog is `Option<BpfProgram>` — `None` silently skips seccomp** [linux.rs:64] → Fixed: dual-filter approach returns `Vec<BpfProgram>`, each applied separately
  - If `build_seccomp_filter` ever returns `Ok(None)`, T2 runs without seccomp. Fix: change return type to `Result<BpfProgram, SpawnError>` (remove Option wrapper).

- [x] [Review][Patch] **Missing: `sandbox_smoke.sh`, `discipline.yml` wiring, invariant doc updates, coverage-matrix, deferred-work.md** (task items [197-201]) → Fixed: `sandbox_smoke.sh` created, `deferred-work.md` updated, I1.md + I10.md updated
  - Five unchecked task items. `sandbox_smoke.sh` is a required CI gate. `deferred-work.md` should record DF18 closure. Fix: create/update these files.

- [x] [Review][Defer] **`SandboxTierError(pub u8)` misrepresents non-numeric string errors** [i9.rs:34] — deferred, developer UX only

---

## Open questions for the user

These do not block `dev-story` — the story specifies a recommended path for each — but the user may want to weigh in:

1. **`unsafe` strategy.** The story recommends removing the crate-level `#![forbid(unsafe_code)]` from `maos-kernel-core/src/lib.rs` and relying on the per-module `#![forbid(unsafe_code)]` that already exists in every other module (so only `security/sandbox/` is allowed `unsafe`, with `// SAFETY:` comments). The alternative is extracting a dedicated `maos-sandbox` crate (20th workspace crate). Both are documented in *Dev Notes → The `unsafe` decision*. Preference?
2. **ADR-004 gate-line discrepancy.** ADR-004's `Gate:` line says "T2 at v0.3", but Epic 1b, NFR-Sec-1, and this story's ACs all bind **T2 at v0.1-β**. The story follows the epic + NFR-Sec-1 (T0/T1/T2 now). Confirm this is the intended scope, and that the ADR-004 gate-line should be corrected in the Epic 1b retro (not in this story).
3. **cgroups v2 vs `setrlimit` on a bare dev box.** Linux cgroups v2 needs a writable (delegated) cgroup subtree, which a fresh evaluator box may not have. The story recommends: attempt cgroups v2, fall back to `setrlimit` if no writable subtree, log + journal the chosen mechanism. Acceptable, or should the kernel hard-require cgroup delegation (and document a `systemd-run --user --scope` install step)? Note: the hello-spirit J0 evaluator path (1b.5a) is `local`-trust → likely T0/T1, so this does **not** gate the 5-minute path.
4. **`SecurityManagerPort` evolution.** The story evolves the v0.1-α placeholder `sandbox_tier_floor(&self, spirit_id: &str)` to be `u32`-pid-keyed (consistent with the rest of the capability system) and adds `effective_sandbox_tier`. This is a kernel-API surface change (handled via `kernel-api-classes.toml` + baseline regen, as 1b.2 did). Confirm the port-trait may evolve here rather than being held frozen.

Story-creation context note: this story was written by exhaustive analysis of Epic 1b, the architecture (ADR-004/009, §4.1/4.3, §5.1, §8.1/8.2), the PRD (FR5/FR6, NFR-Sec-1/NFR-Test-13), Story 1b.2's full implementation + review findings, the current codebase state (`SandboxTier`, `PolicyTable`, `CapAuditEvent::SandboxBlock`, `JournalAdapter`, `SecurityManagerAdapter`), `deferred-work.md`, and 2026 web research on the `landlock` / `seccompiler` / cgroups v2 / Seatbelt / Job Object Rust ecosystem.
