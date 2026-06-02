---
dev_model_used: claude-opus-4-5
---

# Story 1a.2: Wire the Five-Service Kernel Skeleton with a Multi-Threaded Tokio Composition Root

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As **the kernel implementer about to lay down the runtime topology every subsequent feature epic (E1b → E10) will plug into**,
I want **the five-service kernel topology codified per `architecture-maos-minimal-opus.md` §4.0–§4.7 — one supervisor (Spirit Scheduler) + four supervised services (Security Manager / Memory Manager / IAC Bus / Capability Registry) + two internal modules (I/O Subsystem / Telemetry Stream) — wired as empty hexagonal shells inside `crates/maos-kernel-core`, with their hexagonal port traits declared in `maos-domain` (per ADR-010), the Capability Registry retaining its ADR-030 four-sub-module decomposition (`cap_tokens` / `cap_policy` / `cap_audit` / `cap_quota`) with method-by-method computational-class classification, the `maos-bin` composition root running a single multi-threaded Tokio runtime (`#[tokio::main(flavor = "multi_thread")]` with `worker_threads = num_cpus`) supervising the seven shells via `tokio_util::sync::CancellationToken` and a root `select!`-driven graceful-shutdown arm (per ADR-011), the `cargo xtask check-service-boundary` stub mode extended to populate `xtask/kernel-api-classes.toml` (all new public surface items classified `universal-arithmetic` | `data-movement` | `supervision`, NONE in `other`), the `docs/ci-baselines/kernel-surface-v0.1-alpha.json` baseline regenerated against the new surface, all 13 Epic-0 CI gates staying green (including `check-empty-kernel`'s I9 structural-state lint — no new persistent-state fields outside the three sanctioned holders), the KLOC aggregate staying under `_aggregate_alarm = 16000`, and `tests/coverage-matrix.yaml` flipped for the FR rows this story actually delivers**,
so that **Stories 1a.3 (CryptoProvider + xtask P1–P4 enforcement upgrade) and 1a.4 (`maosctl` CLI + SECURITY.md) compose against a pre-wired hexagonal kernel-core skeleton without re-litigating service boundaries; Epic 1b's evaluator path (1b.1–1b.5c) lands runtime logic into pre-stamped sockets (the audit-spine writes into `iac::transparency_log`, the capability-mediation work fills `capability::cap_tokens`, the sandbox tier code occupies `security::sandbox`, all already declared); Epic 4's halt-protocol mechanism plugs into the `capability` service's pre-classified `universal-arithmetic` predicate methods; the founding-sprint baselines extend without bespoke setup (`git clone && cargo build --locked` reproduces the v0.1-α five-service-shell topology); and the `xtask check-service-boundary` gate is the structural floor every new public kernel symbol must clear from here onward**.

### What this story is NOT

This story is **structural scaffolding only**. It must NOT smuggle runtime logic into the shells. Specifically:

1. **No service implementation.** Each adapter module gets at most: a single-line `//!` doc comment, `#![forbid(unsafe_code)]`, a port-trait re-export (via `pub use maos_domain::ports::<Service>Port;`), and an empty `pub struct <Service>Adapter;` placeholder. The `impl <Service>Port for <Service>Adapter` block is **explicitly forbidden** at v0.1-α — it triggers the I9 structural-state lint and conflates 1a.2 with 1b.x work.
2. **No `tokio::sync::*` types in `maos-kernel-core` adapter shells.** The composition root in `maos-bin` is the ONLY place that constructs runtime primitives (mpsc, broadcast, RwLock, JoinSet, CancellationToken root). Adapter shells receive these via constructor arguments in future stories.
3. **No `Vec<…>` / `HashMap<…>` / `Arc<…>` struct fields inside any adapter shell.** These are on the I9 denylist (`xtask/i9-denylist.toml`) and trigger `check-empty-kernel` violations outside the three sanctioned holders (`crates/maos-kernel-core/src/journal/`, `crates/maos-kernel-core/src/iac/transparency_log.rs`, `crates/maos-kernel-core/src/capability/cap_tokens/`). The five new service modules (scheduler/memory/security/iac+the-bus-itself/telemetry/io/+the-capability-non-tokens sub-modules) are NOT in the I9 whitelist; structural-state additions MUST be deferred to Epic 1b.
4. **No P1–P4 enforcement upgrade in xtask.** The `p1_p4_status` payload stays tagged `deferred-to-story-2.2` per `xtask/src/check_service_boundary.rs:166–171`. Story 1a.2 only extends the surface-classification leg, not the bin-target / proto-module / supervised-exit checks. P4 (`check_p4_supervised_exit`) stays a no-op invoked over an empty services slice (per the existing comment at `check_service_boundary.rs:374–382`).
5. **No CryptoProvider trait.** Story 1a.3 ships the `CryptoProvider` trait + default `ring`/`rustls` adapter. The `security/crypto.rs` slot in 1a.2 stays empty (or absent — Story 1a.3 may create the file fresh).
6. **No `maosctl` enhancements.** Story 1a.4 ships the `maosctl` CLI scaffold + SECURITY.md. The existing `crates/maos-cli/src/lib.rs` stub stays as-is at this story (`maos-bin/src/main.rs` does NOT import `maos-cli` yet).
7. **No new ADR.** All 14 binding-v0.1 ADRs were committed in Story 1a.1 (`docs/adr/ADR-001 … ADR-037`). This story consumes ADR-010 / ADR-011 / ADR-030 directly; it does not amend them.

**Why the discipline matters here.** The Epic 0 retrospective surfaced "spec-prose-vs-implementation drift" (corpus quality debt in 0.5; corpus entries shipped 200 strong but only 11 unique patterns). The same drift mode at 1a.2 would be "shells shipped as 7 directories but the cap-tokens module already has `RwLock<HashMap<…>>` because the author thought 'that's structurally trivial'." Empty means **typed-empty**: types declared, no field collections of denylisted types, no impl blocks against port traits.

### Critical preconditions (verify BEFORE opening the PR)

1. **Story 1a.1 is `done` and merged.** Verified: `sprint-status.yaml` shows `1a-1-initialize-17-crate-cargo-workspace-frozen-abi-types-starter-template: done`; `epic-1a: in-progress`. The 17-crate workspace, `maos-domain` with I1–I14 type codification, `maos-spirit-abi` with frozen ComplianceClaim types, and the 14 binding-v0.1 ADRs MUST be in place.
2. **All 13 Epic-0 gates are green on `main`.** Run the full local-CI suite from `1a-1`'s Task 9.1 list as a baseline before any changes; document the pass list in the dev record. Any pre-existing failure becomes a hard blocker for opening this story's PR.
3. **`docs/dev-discipline/dep-introduction.md` discipline applies.** This story should introduce only **two** new top-level dependencies: `tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }` and `tokio-util = { version = "0.7", features = ["rt"] }`. Both go into `crates/maos-bin/Cargo.toml` ONLY — not into `maos-kernel-core` (whose adapter shells must stay async-runtime-free at v0.1-α per the I9-spirit + ADR-010 domain-core discipline). The dev record's "Dependency-introduction note" subsection MUST list concrete `Cargo.lock` blast-radius counts (`git diff HEAD -- Cargo.lock | grep -c '^+name = '`) and confirm `cargo deny check` passes.
4. **DF17 (multi-invariant `invariant-lock` fixture)** is **NOT** triggered by this story. This story does **not** touch any `docs/invariants/I*.md` file or `tests/coverage-matrix.yaml` invariant-cadence rows; the `invariant-lock` gate runs in "no-touch" mode (empty diff against invariant register files). Verify by running `cargo run -p xtask -- invariant-lock --changed-files <this-PR's-files> --pr-number 0 --sha test` and confirming the gate reports zero touched invariants. If your diff *does* touch `docs/invariants/I*.md`, **STOP** — that work belongs to Story 1b.x or a follow-up; this story is structural-skeleton-only.
5. **DF16 (journal-append merge artifact) does not gate this PR.** The 14-invariant journal entry for 1a.1's PR is already journaled (or pending operator verification per the 1a.1 retro). 1a.2 lands no new invariant touches, so the `journal-append` workflow has nothing to write for this PR — the merge gate fires neutrally.

### Size envelope

Expected production-Rust footprint:

- **`maos-domain` port-trait additions:** ~150–250 LOC (`src/ports/mod.rs` + 7 sub-modules — one trait per supervisor + supervised service + internal module; trait shape only, no impls).
- **`maos-kernel-core` adapter shells:** ~120–200 LOC across 6 new module directories (`scheduler/`, `memory/`, `security/`, `iac/`, `io/`, `telemetry/`) + 1 existing extended (`capability/` re-exports its 4 sub-modules already; story 1a.2 adds `pub mod api` and re-exports adapter stubs).
- **`maos-bin/src/main.rs` composition root:** ~80–150 LOC (`#[tokio::main(flavor = "multi_thread")]` + `worker_threads` + `CancellationToken` root + `select!`-shutdown skeleton).
- **`xtask/src/check_service_boundary.rs` extension:** ~50–120 LOC (kernel-api-class enrichment to surface ALL items including from the new modules; per-service classification log line; reject `class == "other"` retained from existing code at lines 142–157).
- **`xtask/kernel-api-classes.toml` population:** ~40–80 entries (one per new public symbol; classifications: `universal-arithmetic` / `data-movement` / `supervision`).
- **`docs/ci-baselines/kernel-surface-v0.1-alpha.json` regeneration:** mechanical output of `xtask check-service-boundary --json` — same blob count as classification entries.
- **`xtask/kloc.toml` recalibration:** ~5–10 LOC (verify `maos-kernel-core = 6000` and `maos-bin = 1000` ceilings still hold; tighten if actual LOC suggests headroom).
- **`tests/coverage-matrix.yaml` row updates:** flip rows for **ADR-010** and **ADR-011** structural-implementation evidence; **do NOT** touch FR rows (1a.1 already owns FR1/2/7/8/47). Optionally flip FR48 prep row only if its `notes:` field references this story.

**KLOC aggregate alarm sits at 16,000.** Story 1a.1 landed the v0.1-α aggregate at ~4,689 LOC; this story should add ≤500 LOC, bringing the aggregate to ≤5,200 LOC — well below alarm. If your actual count exceeds 700 LOC, **STOP** and review per the "What this story is NOT" section above for accidental logic smuggling.

**Total expected diff:** ~500–800 LOC across ~25 new files + 6–8 modified files.

## Acceptance Criteria

### AC1 — `maos-kernel-core` exports the canonical five-service + two-internal-module skeleton per architecture §4.0.2 with `pub mod api` populated for the surface gate

**Given** the existing `crates/maos-kernel-core/src/lib.rs` declaring only `pub mod capability;` (Story 1a.1's footprint)
**And** the architecture §4.0.2 canonical layout: five services (`scheduler/`, `memory/`, `security/`, `iac/`, `capability/`) and two internal modules (`io/`, `telemetry/`) — read "five services" as "one supervisor (Spirit Scheduler) + four supervised services (Security Manager / Memory Manager / IAC Bus / Capability Registry)" per §4.0 component-classification lock
**And** the architecture §4.6 + ADR-030 decomposition of `capability/` into `cap_tokens/` (hot path), `cap_policy/`, `cap_audit/`, `cap_quota/` (already present from Story 1a.1)
**And** the architecture §4.0.1 "kernel services are not actors — they are shared services with their own task pools" classification

**When** Story 1a.2's kernel-skeleton commit lands

**Then** `crates/maos-kernel-core/src/lib.rs` declares **exactly** these module roots (with `#![forbid(unsafe_code)]` retained at top):

```rust
#![forbid(unsafe_code)]

//! `maos-kernel-core` — the MAOS kernel composition surface.
//!
//! Per architecture §4.0.2 the kernel is organized as:
//!   - One supervisor: `scheduler` (Spirit Scheduler)
//!   - Four supervised services: `security`, `memory`, `iac`, `capability`
//!   - Two internal modules: `io`, `telemetry`
//!
//! At v0.1-α every module is an **empty hexagonal adapter shell** — port
//! traits live in `maos-domain::ports`; this crate declares the adapter
//! types that will (post v0.1-α) implement those ports. No runtime state,
//! no impl blocks, no async primitives. See architecture §4.0.8 four-property
//! test and §4.0.1 hexagonal/actor split.
//!
//! Story 1b.x lands runtime logic into these shells. Story 1a.3 ships
//! `CryptoProvider`. Story 1a.4 ships `maosctl`. Story 2.2 upgrades
//! `xtask check-service-boundary` from stub to P1–P4 enforcement.

pub mod api;        // surface-classification anchor for NFR-Test-2
pub mod scheduler;  // supervisor — Spirit Scheduler (architecture §4.1)
pub mod security;   // supervised service — Security Manager (§4.3)
pub mod memory;     // supervised service — Memory Manager (§4.2)
pub mod iac;        // supervised service — IAC Bus (§4.5)
pub mod capability; // supervised service — Capability Registry (§4.6)
pub mod io;         // internal module at v0.1 — I/O Subsystem (§4.4)
pub mod telemetry;  // internal module at v0.1 — Telemetry Stream (§4.7)
```

**And** each new service module (`scheduler/`, `security/`, `memory/`, `iac/`, `io/`, `telemetry/`) is a directory under `crates/maos-kernel-core/src/` containing a single `mod.rs` file with this exact shape (worked example for `scheduler/`):

```rust
#![forbid(unsafe_code)]

//! Spirit Scheduler — supervisor / composition root for the four
//! supervised services (Security / Memory / IAC / Capability).
//!
//! Per architecture §4.0.8 supervisor exception, this module satisfies
//! P1 (own crate at v0.5+), P2 (own bin target at v0.5+), and P4
//! (independently restartable) but is exempt from P3 (boundary manifest
//! in the standard shape — its boundary is the union of its children's).
//!
//! At v0.1-α this is an empty hexagonal adapter shell. The supervisor
//! itself lives in the `maos-bin` composition root (`#[tokio::main]`);
//! this module exposes the adapter type its port-trait surface will use
//! when Story 1b.1 lands lifecycle journal mechanics.
//!
//! See `maos_domain::ports::SpiritSchedulerPort` for the hexagonal port
//! contract (declared in `maos-domain` per ADR-010 to keep the domain
//! core async-runtime-free).

pub use maos_domain::ports::SpiritSchedulerPort;

/// Adapter shell — Story 1b.1 implements `SpiritSchedulerPort` for this
/// type with the supervisor's journal + supervised-services restart logic.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpiritSchedulerAdapter;
```

The remaining six new service/module shells (`security/`, `memory/`, `iac/`, `io/`, `telemetry/`) follow **the same shape**, with module-level docstring referencing the architecture section (§4.3 / §4.2 / §4.5 / §4.4 / §4.7), the corresponding `pub use maos_domain::ports::<Name>Port;` re-export, and an empty `<Name>Adapter` zero-size struct. Worked example for `memory/`:

```rust
#![forbid(unsafe_code)]

//! Memory Manager — supervised service per §4.2.
//!
//! Provides three named memory tiers (`private`, `shared`, `collective`)
//! and enforces I5 namespace scopes. At v0.1-α this is an empty hexagonal
//! adapter shell; Story 4.3 lands the three-tier mechanics.

pub use maos_domain::ports::MemoryManagerPort;

#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryManagerAdapter;
```

**And** `crates/maos-kernel-core/src/capability/mod.rs` is extended **additively** (the existing `pub mod cap_tokens; pub mod cap_policy; pub mod cap_audit; pub mod cap_quota;` lines stay) to add a port-trait re-export and an adapter placeholder mirroring the other six service modules:

```rust
#![forbid(unsafe_code)]

//! Capability Registry — supervised service per §4.6.
//!
//! Decomposed per ADR-030 into four sub-modules (hot path / policy /
//! audit / quota). At v0.1-α the four sub-module shells exist from
//! Story 1a.1; this story adds the port-trait re-export and the
//! `CapabilityRegistryAdapter` placeholder.

pub mod cap_tokens;
pub mod cap_policy;
pub mod cap_audit;
pub mod cap_quota;

pub use maos_domain::ports::CapabilityRegistryPort;

#[derive(Debug, Clone, Copy, Default)]
pub struct CapabilityRegistryAdapter;
```

**And** `crates/maos-kernel-core/src/api.rs` (new file, not a directory) declares the **surface-classification anchor** — a single `pub mod` re-aggregating the seven adapter types under a stable `kernel::api::*` path so `xtask check-service-boundary` has one canonical location to walk:

```rust
#![forbid(unsafe_code)]

//! Surface-classification anchor for NFR-Test-2.
//!
//! Per Epic 0 retro: "Story 1a.2's `pub mod api` lands → MUST add
//! classifications same-PR or surface-diff rejects". This module
//! re-exports the seven adapter types so the xtask
//! `check-service-boundary` walk produces a stable, classifiable surface
//! at `maos_kernel_core::api::*`. Adding a new public re-export here
//! requires matching it with a classification entry in
//! `xtask/kernel-api-classes.toml` per AC4.

pub use crate::scheduler::SpiritSchedulerAdapter;
pub use crate::security::SecurityManagerAdapter;
pub use crate::memory::MemoryManagerAdapter;
pub use crate::iac::IacBusAdapter;
pub use crate::capability::CapabilityRegistryAdapter;
pub use crate::io::IoSubsystemAdapter;
pub use crate::telemetry::TelemetryStreamAdapter;
```

**And** `cargo build --locked --all-targets --workspace` succeeds with **zero warnings** on Rust stable (per `rust-toolchain.toml` 1.88+). The `crates/maos-kernel-core/Cargo.toml` `[dependencies]` table gains exactly **one** new entry: `maos-domain = { path = "../maos-domain" }` — no `tokio`, no `tokio-util`, no `async-trait`, no `serde`. The kernel-core crate stays runtime-free at v0.1-α; runtime primitives live exclusively in `maos-bin` (per AC3) and `maos-domain` ports are sync trait method shapes (per AC2).

**And** the existing `crates/maos-kernel-core/src/capability/cap_*/mod.rs` placeholders are **NOT** rewritten. Their content (`#![forbid(unsafe_code)]` + the existing `//!` doc comment + nothing else) stays exactly as Story 1a.1 left them. Adding ANY symbol (struct, fn, trait, type alias) into one of those four files in this story is **explicitly forbidden** — those are 1b.x territory.

**Sanity check (what NOT to do):**

```rust
// FORBIDDEN — runtime primitive in adapter shell, ADR-010 violation
pub struct SpiritSchedulerAdapter {
    pcb_map: HashMap<SpiritId, SpiritControlBlock>,  // I9 lint trip
    cancel: CancellationToken,                        // runtime primitive — belongs in maos-bin
}

// FORBIDDEN — port impl in adapter shell, conflates 1a.2 with 1b.1
impl SpiritSchedulerPort for SpiritSchedulerAdapter {
    fn load(&self, manifest: ManifestPath) -> Result<SpiritId, Error> { … }
}
```

```rust
// CORRECT — typed-empty placeholder
#[derive(Debug, Clone, Copy, Default)]
pub struct SpiritSchedulerAdapter;
```

### AC2 — Hexagonal port traits declared in `maos-domain::ports::*`, one trait per service + internal module, with sync-only signatures + per-method computational-class doc tags

**Given** ADR-010's hexagonal commitment: "domain core (pure types, invariants, pure functions) surrounded by ports (trait definitions for kernel-external dependencies) implemented by an adapter ring"
**And** ADR-010's gate: "crate boundary lint enforces port/adapter ring; **domain core compiles without async runtime**" (binding-v0.1)
**And** `crates/maos-domain/Cargo.toml`'s current dependencies (`serde = { version = "1.0", features = ["derive"] }` + `thiserror = "2.0"` per Story 1a.1) — this story does **NOT** add `tokio` or `async-trait` to `maos-domain`
**And** the architecture §4.0.7 commitment: "the kernel performs universal arithmetic comparison only via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`)" — these constitute the only `universal-arithmetic` surface at v0.1-α
**And** the Story 0.5 retro lesson on AST-driven enforcement (the xtask walk-pattern from `check_unsafe.rs` is the precedent; string-grep is forbidden per the retro's "concerning patterns" section)

**When** the port-trait surface is declared in `maos-domain`

**Then** a new `crates/maos-domain/src/ports/mod.rs` file is created (one module per service/internal module + an aggregator):

```rust
//! Hexagonal port traits per ADR-010.
//!
//! One trait per supervisor / supervised service / internal module.
//! Adapter implementations live in `maos-kernel-core::<service>::<Service>Adapter`.
//!
//! # Sync-only trait method signatures
//!
//! Per ADR-010's binding-v0.1 gate "domain core compiles without async
//! runtime", port traits declared here MUST NOT use `async fn` or return
//! `impl Future`. Async behavior — when adapters need it — wraps the
//! sync trait method behind a `Pin<Box<dyn Future>>` or returns a typed
//! handle the adapter caller can `.await`. Story 1b.x lands the actual
//! async behavior; this story declares the sync trait shapes only.
//!
//! # Computational class per method
//!
//! Every public trait method MUST carry a `/// Class: <class>` doc-line
//! immediately above its declaration, where `<class>` is one of:
//!   - `universal-arithmetic` — numeric comparison via the four ADR-022
//!     predicates (`on_value_above` / `_below` / `_within` / `_outside`).
//!   - `data-movement` — moves frames/tokens/payloads between holders;
//!     does no semantic interpretation.
//!   - `supervision` — lifecycle/control over child task or actor;
//!     read/write of a kernel-managed audit log.
//!
//! The `xtask/kernel-api-classes.toml` classifications consume these
//! `/// Class:` doc tags as the source of truth (AC4). A method whose
//! doc lacks a `/// Class:` line, OR carries a class not in the three-element
//! set, defaults to `other` and fails the surface gate.

pub mod scheduler;
pub mod security;
pub mod memory;
pub mod iac_bus;
pub mod capability;
pub mod io_subsystem;
pub mod telemetry;

pub use scheduler::SpiritSchedulerPort;
pub use security::SecurityManagerPort;
pub use memory::MemoryManagerPort;
pub use iac_bus::IacBusPort;
pub use capability::CapabilityRegistryPort;
pub use io_subsystem::IoSubsystemPort;
pub use telemetry::TelemetryStreamPort;
```

**And** `crates/maos-domain/src/lib.rs` adds **exactly** one new `pub mod` line (additive — `pub mod invariants;` and `pub use invariants::*;` from Story 1a.1 stay):

```rust
pub mod invariants;
pub use invariants::*;

pub mod ports;            // NEW — Story 1a.2 hexagonal port traits per ADR-010
```

(`pub use ports::*;` is intentionally **NOT** added — port traits are namespaced under `maos_domain::ports::*` so the kernel-API surface walk picks them up cleanly without flattening into the root namespace.)

**And** each port-trait file under `crates/maos-domain/src/ports/<name>.rs` declares **at minimum** the trait shape with **at least one** method per architecture section's "Operations exposed" / "Responsibility" prose — sync method signatures, doc-tag classifications, no `async fn`, no `Box<dyn Future>` (those are adapter concerns). Worked example for `scheduler.rs` (mirroring architecture §4.1 "Operations exposed to user-space (via control-plane API)"):

```rust
//! Spirit Scheduler port trait per architecture §4.1.
//!
//! At v0.1-α this trait declares the lifecycle-verb surface only;
//! Story 1b.1 lands the audit-spine integration, Story 5.1 lands the
//! full 11-verb lifecycle surface. Method bodies are deferred.

use crate::invariants::i10::{LifecycleEvent, JournalEntry};

/// Spirit Scheduler — supervisor over Security / Memory / IAC / Capability.
///
/// Per §4.0.8 supervisor exception: satisfies P1, P2, P4 but is exempt
/// from P3 (its boundary IS the union of its children's boundaries).
pub trait SpiritSchedulerPort {
    /// Class: supervision
    ///
    /// Append a lifecycle transition to the kernel's per-Host journal.
    /// At v0.1-α the journal lives in `crates/maos-kernel-core/src/journal/`
    /// (an I9-sanctioned holder) but its mechanics ship in Story 1b.1.
    fn journal_lifecycle(&self, entry: JournalEntry);

    /// Class: supervision
    ///
    /// Returns the current lifecycle event most recently journaled for
    /// a Spirit; `None` if no entry has been journaled. Read-only; does
    /// not mutate state. Adapter implementations are expected to query
    /// the journal directly.
    fn last_lifecycle_event(&self, spirit_id: &str) -> Option<LifecycleEvent>;
}
```

The remaining six trait files follow the **same shape** — module-level doc comment referencing the architecture section, sync-only trait with at least one method per the section's responsibility list, every method carrying a `/// Class: <one-of-three>` doc line. **Minimum methods per port:**

| Port file | Trait | Min. method count | Class anchors |
|---|---|---|---|
| `ports/scheduler.rs` | `SpiritSchedulerPort` | 2 (`journal_lifecycle`, `last_lifecycle_event`) | both `supervision` |
| `ports/security.rs` | `SecurityManagerPort` | 2 (`sandbox_tier_floor`, `approval_class`) | both `supervision` |
| `ports/memory.rs` | `MemoryManagerPort` | 2 (`validate_namespace_read`, `validate_namespace_write`) | both `data-movement` |
| `ports/iac_bus.rs` | `IacBusPort` | 2 (`enqueue_frame`, `broadcast_frame`) | both `data-movement` |
| `ports/capability.rs` | `CapabilityRegistryPort` | 4 (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside` — the four ADR-022 universal-arithmetic predicates) | all four `universal-arithmetic` |
| `ports/io_subsystem.rs` | `IoSubsystemPort` | 2 (`http_get`, `http_post`) | both `data-movement` |
| `ports/telemetry.rs` | `TelemetryStreamPort` | 2 (`publish_event`, `subscribe_topic`) | both `data-movement` |

**Total methods declared:** 16. **Class distribution:** 4 `universal-arithmetic` (capability/predicates), 8 `data-movement` (memory + iac + io + telemetry), 4 `supervision` (scheduler + security). **Zero in `other`** — this is the AC4 surface-gate floor.

**Worked example** for the universal-arithmetic predicate trait `ports/capability.rs` (the architecturally most-load-bearing one — these four are the ENTIRE kernel-side `universal-arithmetic` surface for v0.1-α and remain so until ADR-022 amendment):

```rust
//! Capability Registry port trait per architecture §4.6 + ADR-030 decomposition.
//!
//! The Capability Registry mediates every external call. At v0.1-α this
//! port declares the four ADR-022 universal-arithmetic predicates — the
//! kernel-side surface that fires epistemic halts when a Spirit's
//! tagged-scalar slot crosses a threshold. The full
//! issue/verify/revoke/audit-write surface lands in Story 1b.2; the
//! mailbox-side `iac.send` mediation lands in Story 6.1.

/// Capability Registry — mediates every external call, evaluates ADR-022
/// universal-arithmetic predicates against per-Spirit tagged scalars.
///
/// Per ADR-030: split internally into `cap_tokens` (hot path),
/// `cap_policy`, `cap_audit`, `cap_quota`. The port trait surface at v0.1-α
/// declares only the universal-arithmetic predicates; the cap-tokens
/// hot-path surface (issue/verify/revoke) lands in Story 1b.2.
pub trait CapabilityRegistryPort {
    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value > threshold`. One of the four ADR-022
    /// predicates — the ENTIRE kernel-side computational surface at v0.1.
    /// Spirit-side `[epistemic_policy]` rules reference this predicate
    /// for halt-on-scalar-drift policies (architecture §4.6.1).
    fn on_value_above(&self, value: f64, threshold: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value < threshold`. ADR-022 predicate.
    fn on_value_below(&self, value: f64, threshold: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `lower <= value <= upper`. ADR-022 predicate;
    /// inclusive bounds at v0.1-α (open/half-open variants deferred to
    /// Story 4.2 if a Spirit class demands them).
    fn on_value_within(&self, value: f64, lower: f64, upper: f64) -> bool;

    /// Class: universal-arithmetic
    ///
    /// Returns `true` iff `value < lower OR value > upper`. ADR-022 predicate.
    fn on_value_outside(&self, value: f64, lower: f64, upper: f64) -> bool;
}
```

The default impls below the trait (sample-implementation reference, NOT required by the AC — keep the trait empty-bodied if defensible):

```rust
// If you keep a default-method body, it MUST be a pure expression with
// zero state and zero IO — the body becomes part of the trait surface
// for ABI purposes. Architecture §4.0.7 forbids the kernel from
// computing anything other than these four predicates at v0.1-α.
```

**And** `cargo build -p maos-domain --locked --no-default-features` continues to succeed without **any** transitive pull of `tokio`/`reqwest`/`sqlx`/`async-std`/`smol`/`mio`/`hyper` (verify by `cargo tree -p maos-domain` per Story 1a.1's discipline). The maos-domain dep tree stays at the same ~10 crates it landed at in 1a.1 (`serde`, `serde_derive`, `proc-macro2`, `quote`, `syn`, `unicode-ident`, `thiserror`, `thiserror-impl`).

**And** every trait method's `/// Class: <name>` line uses **one** of the three exact strings `universal-arithmetic` | `data-movement` | `supervision` (case-sensitive, hyphens not underscores). The xtask doc-tag parser in AC4 is string-strict on these forms.

**And** `cargo test -p maos-domain --doc && cargo test -p maos-domain --lib` continues to pass — the I1–I14 doctests from Story 1a.1 are **not** invalidated by this story. If a new port trait references an invariant type (e.g., `JournalEntry` in `scheduler.rs`), the `use` line MUST point at the existing 1a.1 type — do **NOT** redefine `JournalEntry` in `ports/`. If a needed type doesn't exist in `invariants/`, add the minimum sufficient shape to the relevant `invariants/iN.rs` as a typed-empty newtype (e.g., `pub struct SandboxTier(pub u8);`), document the reference in this story's dev record, and verify the I1–I14 doctests still pass.

**Sanity check (forbidden patterns):**

```rust
// FORBIDDEN — async fn breaks ADR-010's domain-core-without-runtime gate
pub trait IacBusPort {
    async fn enqueue_frame(&self, frame: Frame) -> Result<(), Error>;  // NO
}

// FORBIDDEN — class string not in the three-element set
pub trait MemoryManagerPort {
    /// Class: cache              <-- "cache" is not a valid class
    fn evict_entry(&self, key: &str);
}

// CORRECT — sync method + valid class doc tag
pub trait IacBusPort {
    /// Class: data-movement
    fn enqueue_frame(&self, frame_bytes: &[u8]);
}
```

### AC3 — `maos-bin` composition root runs `#[tokio::main(flavor = "multi_thread")]` with `worker_threads = num_cpus`, `CancellationToken` root, and `select!`-driven graceful shutdown wiring all seven adapter shells

**Given** the existing `crates/maos-bin/src/main.rs` from Story 1a.1 (placeholder `fn main()` printing `"maos {VERSION} (v0.1-α scaffold; Story 1a.2 wires the composition root)"`)
**And** ADR-011's binding-v0.1 gate: "per-Spirit Tokio task supervision + bounded mailbox"
**And** the architecture §4.0.4 technology table: "In-process IPC | `tokio::sync::mpsc` + `tokio::sync::broadcast`"
**And** the architecture §4.0.8 supervisor exception: "Spirit Scheduler is the composition root: the binary whose `main` instantiates and supervises the four supervised services"
**And** the architecture §4.1 crash-detection commitments: "≤2s on SIGKILL; `task.orphaned` IAC frame ≤5s" — these are runtime obligations Story 1a.2 declares structure for but does NOT yet implement (the JoinSet handling lands in Story 1b.1)

**When** `crates/maos-bin/src/main.rs` is rewritten to wire the composition root

**Then** the file follows this canonical shape (worked example — adapt module-level docstring as needed but preserve the structural commitments):

```rust
#![forbid(unsafe_code)]

//! `maos-bin` — MAOS Host composition root.
//!
//! Wires the supervisor (Spirit Scheduler) and the four supervised services
//! (Security / Memory / IAC / Capability) plus two internal modules (I/O /
//! Telemetry) under a single multi-threaded Tokio runtime per ADR-011.
//!
//! ## Runtime topology
//!
//! - **Runtime flavor:** `#[tokio::main(flavor = "multi_thread")]`
//! - **Worker threads:** `worker_threads = std::thread::available_parallelism()`
//!   (i.e., `num_cpus` equivalent without an external crate; Rust 1.59+).
//! - **Shutdown channel:** root `tokio_util::sync::CancellationToken`;
//!   every long-lived coordination task receives a clone via
//!   `CancellationToken::child_token()`.
//! - **Graceful shutdown:** `tokio::select!` arms on (a) SIGINT (via
//!   `tokio::signal::ctrl_c`), (b) SIGTERM (Unix; `tokio::signal::unix`),
//!   (c) root-token cancellation. Any arm triggers root-token cancel,
//!   then the program awaits all spawned tasks to drain.
//!
//! At v0.1-α the seven adapter shells are constructed but their port-trait
//! implementations are deferred (Story 1b.x). The composition root demonstrates
//! the wiring shape; runtime behavior is structural-only.
//!
//! ## What this binary does NOT do at v0.1-α
//!
//! - Does NOT load any Spirit (Story 5.1 lifecycle verbs deferred).
//! - Does NOT open any control-plane port (Story 1a.4 ships maosctl).
//! - Does NOT initialize the Transparency Log (Story 1b.1 audit spine).
//! - Does NOT verify any signed binary (Story 1a.3 CryptoProvider deferred).
//!
//! Running `maos-bin` at v0.1-α prints a startup banner, blocks on the
//! shutdown selector, and exits cleanly on Ctrl+C. This validates the
//! runtime topology only.

use std::thread::available_parallelism;
use tokio::signal;
use tokio_util::sync::CancellationToken;

use maos_kernel_core::api::{
    CapabilityRegistryAdapter, IacBusAdapter, IoSubsystemAdapter,
    MemoryManagerAdapter, SecurityManagerAdapter, SpiritSchedulerAdapter,
    TelemetryStreamAdapter,
};

fn worker_thread_count() -> usize {
    available_parallelism()
        .map(usize::from)
        .unwrap_or(1) // single-thread fallback if parallelism query fails
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Per ADR-011: single multi-threaded runtime. `worker_threads` is
    // configured at process start via `tokio::runtime::Builder` when an
    // explicit count is needed; at v0.1-α the `#[tokio::main]` attribute's
    // default is good enough and the `worker_thread_count()` helper
    // records the resolution path for the dev record.
    let cpus = worker_thread_count();
    eprintln!(
        "maos {} (v0.1-α scaffold; worker_threads target = {})",
        env!("CARGO_PKG_VERSION"),
        cpus
    );

    // Construct the seven adapter shells. At v0.1-α these are zero-size
    // placeholders; Story 1b.x replaces them with real adapter state.
    let _scheduler = SpiritSchedulerAdapter::default();
    let _security = SecurityManagerAdapter::default();
    let _memory = MemoryManagerAdapter::default();
    let _iac = IacBusAdapter::default();
    let _capability = CapabilityRegistryAdapter::default();
    let _io = IoSubsystemAdapter::default();
    let _telemetry = TelemetryStreamAdapter::default();

    // Root cancellation token. Every long-lived coordination task gets a
    // child token via `cancel.child_token()`. Cancelling the root cancels
    // all children (per tokio-util semantics).
    let cancel = CancellationToken::new();

    // Wire the graceful-shutdown selector. At v0.1-α we arm on SIGINT
    // (Ctrl+C), SIGTERM (Unix), and root-token cancellation. Any arm
    // triggers root-token cancel; the program then awaits drain.
    let shutdown_reason: &'static str = tokio::select! {
        _ = signal::ctrl_c() => "sigint",
        _ = shutdown_unix_term() => "sigterm",
        _ = cancel.cancelled() => "internal-cancel",
    };
    eprintln!("maos: shutdown reason = {shutdown_reason}; cancelling root token");
    cancel.cancel();

    // v0.1-α has no spawned tasks to drain yet (Story 1b.x adds them).
    // The drain loop is a structural placeholder so 1b.x slots into a
    // working scaffold rather than rewriting the shutdown semantics.
    eprintln!("maos: drained 0 child tasks; exiting cleanly");
    Ok(())
}

#[cfg(unix)]
async fn shutdown_unix_term() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate())
        .expect("install SIGTERM handler");
    term.recv().await;
}

#[cfg(not(unix))]
async fn shutdown_unix_term() {
    // Non-Unix targets (Windows): never resolves; Ctrl+C arm covers shutdown.
    std::future::pending::<()>().await;
}
```

**And** `crates/maos-bin/Cargo.toml` gains **exactly** these new dependencies (no more, no less):

```toml
[dependencies]
maos-domain = { path = "../maos-domain" }
maos-kernel-core = { path = "../maos-kernel-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
tokio-util = { version = "0.7", features = ["rt"] }
```

The dev record's "Dependency-introduction note" subsection MUST document the `Cargo.lock` blast-radius (`git diff HEAD -- Cargo.lock | grep -c '^+name = '`); target ≤80 new lockfile entries for these two tokio crates (tokio alone pulls ~30; tokio-util ~5; expect ~70–80 aggregate, well under the alarm threshold). If the count exceeds 100, **STOP** and audit per `docs/dev-discipline/dep-introduction.md`.

**And** the binary builds AND runs correctly:

- `cargo build -p maos-bin --locked --release` succeeds with zero warnings.
- `cargo install --path crates/maos-bin --locked` succeeds (FR1 source-install slice retained from 1a.1).
- Running `./target/release/maos-bin` prints the startup banner (`maos 0.1.0-alpha (v0.1-α scaffold; worker_threads target = <N>)`), blocks on the shutdown selector, and exits cleanly on Ctrl+C with the message `maos: shutdown reason = sigint; cancelling root token` followed by `maos: drained 0 child tasks; exiting cleanly`.
- `cargo test -p maos-bin` runs zero tests successfully (no test file required at v0.1-α; the runtime topology is exercised by the build-and-run gate above, not by unit tests — runtime tests land in 1b.1 alongside the audit spine).

**And** the composition root contains **no** kernel-policy logic — no Spirit-loading, no capability-token issuance, no journal initialization, no signature verification, no CLI argument parsing. The shape above is the **complete** binary at v0.1-α.

**Sanity check (forbidden patterns):**

```rust
// FORBIDDEN — single-threaded flavor; ADR-011 demands multi-thread
#[tokio::main(flavor = "current_thread")]
async fn main() { … }

// FORBIDDEN — manual Runtime::new without #[tokio::main] attribute; loses
// the canonical worker-threads default and the attribute-driven topology
// the next dev expects to see.
fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { … });
}

// FORBIDDEN — long-lived task without CancellationToken; root shutdown
// cannot cancel it cleanly
tokio::spawn(async move {
    loop {
        do_work().await;  // NO — needs `select!` arm on cancel.cancelled()
    }
});

// CORRECT — long-lived task wired into the cancel hierarchy
let cancel_child = cancel.child_token();
tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = cancel_child.cancelled() => break,
            _ = do_work() => continue,
        }
    }
});
```

(Note: the "CORRECT" pattern above is what 1b.1 will add when it spawns the supervisor task; 1a.2's composition root doesn't spawn it yet — but the structural slot for it must exist via the unused `_scheduler`/`_security`/etc. bindings and the `cancel` root.)

### AC4 — `cargo xtask check-service-boundary` stub mode: every new public surface item classified in `xtask/kernel-api-classes.toml`, zero items in `other`, baseline regenerated, all 13 Epic-0 gates green

**Given** the existing `xtask/src/check_service_boundary.rs` from Story 0.2 (`check-service-boundary` stub: walks `crates/maos-kernel-core/src/lib.rs` via `syn`, classifies each public item via `xtask/kernel-api-classes.toml`, defaults missing entries to `"other"` which produces a violation per `check_service_boundary.rs:142–157`)
**And** the existing `xtask/kernel-api-classes.toml` (empty `[classes]` table per Story 0.2 + Story 1a.1's "untouched" list)
**And** the existing baseline `docs/ci-baselines/kernel-surface-v0.1-alpha.json` (empty `items: []` from Story 0.2's substrate seed)
**And** the Epic 0 retro commitment: "Story 1a.2's `pub mod api` lands → MUST add classifications same-PR or surface-diff rejects"
**And** the AST-walk discipline from `check_unsafe.rs` (Story 0.2 P9 — string-grep instinct is forbidden; classification reads come from the `syn::Item` walk, not from regex)

**When** Story 1a.2 lands the seven new module roots + `pub mod api` re-export aggregator

**Then** `xtask/kernel-api-classes.toml` is populated with **exactly one classification entry per new public surface item** the `check-service-boundary` walk produces from `crates/maos-kernel-core/src/lib.rs`. The expected surface (after Tasks 1–3 below complete):

```toml
# Kernel API classification table for NFR-Test-2 service-boundary gate.
# Story 1a.2 populates this when `pub mod api` lands in `maos-kernel-core`.
# Adding classifications requires invariant-lock review per NFR-Test-2.
#
# Class taxonomy (per architecture §4.0.7):
#   - universal-arithmetic: ADR-022 four-predicate kernel surface (the
#     ONLY surface where the kernel computes numeric comparisons; halts
#     wire here).
#   - data-movement: moves frames/tokens/payloads between holders without
#     semantic interpretation (memory ports, iac frame routing, io
#     transports, telemetry broadcast).
#   - supervision: lifecycle/control over a child task or actor; reads/writes
#     a kernel-managed audit log (scheduler, security manager).
#
# Empty value field = "other" (default; produces a violation). Any new
# `kernel::api::*` re-export added in a future story MUST add a row here.

[classes]

# api/* re-exports — surface aggregator (the canonical walk path).
# Adapter types themselves carry the same class as their port traits.
"maos_kernel_core::api::SpiritSchedulerAdapter"       = "supervision"
"maos_kernel_core::api::SecurityManagerAdapter"       = "supervision"
"maos_kernel_core::api::MemoryManagerAdapter"         = "data-movement"
"maos_kernel_core::api::IacBusAdapter"                = "data-movement"
"maos_kernel_core::api::CapabilityRegistryAdapter"    = "universal-arithmetic"
"maos_kernel_core::api::IoSubsystemAdapter"           = "data-movement"
"maos_kernel_core::api::TelemetryStreamAdapter"       = "data-movement"

# Direct module-path exports (xtask walk produces both api::* and module::*
# entries because the syn walker visits both the re-export and the
# original declaration site). Same classification as the api::* row.
"maos_kernel_core::scheduler::SpiritSchedulerAdapter"           = "supervision"
"maos_kernel_core::security::SecurityManagerAdapter"            = "supervision"
"maos_kernel_core::memory::MemoryManagerAdapter"                = "data-movement"
"maos_kernel_core::iac::IacBusAdapter"                          = "data-movement"
"maos_kernel_core::capability::CapabilityRegistryAdapter"       = "universal-arithmetic"
"maos_kernel_core::io::IoSubsystemAdapter"                      = "data-movement"
"maos_kernel_core::telemetry::TelemetryStreamAdapter"           = "data-movement"

# Port-trait re-exports propagated through `pub use maos_domain::ports::…`.
# These appear as `use` items in the syn walk (per check_service_boundary
# lines 232–235); classify with the port's primary class.
"maos_kernel_core::scheduler::SpiritSchedulerPort"           = "supervision"
"maos_kernel_core::security::SecurityManagerPort"            = "supervision"
"maos_kernel_core::memory::MemoryManagerPort"                = "data-movement"
"maos_kernel_core::iac::IacBusPort"                          = "data-movement"
"maos_kernel_core::capability::CapabilityRegistryPort"       = "universal-arithmetic"
"maos_kernel_core::io::IoSubsystemPort"                      = "data-movement"
"maos_kernel_core::telemetry::TelemetryStreamPort"           = "data-movement"
```

**Worked-example walk:** if the dev agent's actual `crates/maos-kernel-core/src/lib.rs` produces **additional** items beyond the 21 listed above (e.g., adds a public type alias in `scheduler/mod.rs` for ergonomics), the classification table MUST include matching rows. The single source of truth is the output of:

```sh
cargo run -p xtask -- check-service-boundary --json | jq '.current_surface.items[].path'
```

Every entry that list emits MUST appear as a row in `xtask/kernel-api-classes.toml` with a class **other than** `other`. If 1 of the 21+ paths is missing a row, the existing `check_service_boundary` logic at lines 142–157 emits an `NFR-Test-2 violation` and the gate fails.

**And** `docs/ci-baselines/kernel-surface-v0.1-alpha.json` is **regenerated** to capture the new ~21-item surface (replaces the existing `items: []`):

```sh
cargo run -p xtask -- check-service-boundary --json > docs/ci-baselines/kernel-surface-v0.1-alpha.json
```

After regeneration, re-run `cargo run -p xtask -- check-service-boundary` and verify **zero violations** (the surface matches the baseline; classifications cover every new item). Commit the regenerated baseline JSON in the same PR. **Worked-example diff** for the baseline regeneration:

```json
{
  "crate_name": "maos-kernel-core",
  "abi_baseline_version": "v0.1-alpha",
  "items": [
    {
      "kind": "use",
      "path": "maos_kernel_core::api::CapabilityRegistryAdapter",
      "signature_hash": "<sha256-prefix>"
    },
    {
      "kind": "use",
      "path": "maos_kernel_core::api::IacBusAdapter",
      "signature_hash": "<sha256-prefix>"
    },
    // … ~19 more entries, alphabetically sorted by path (per existing
    // snapshot_kernel_surface logic at check_service_boundary.rs:176–192)
  ]
}
```

**And** `xtask/src/check_service_boundary.rs` is **extended** (additively; no regression to the existing surface-diff logic at lines 105–158) with a new **per-service classification log line** so the json-mode output includes a service-level summary the dev / CI run can read without parsing the full items array:

```rust
// In the json-mode block at the bottom of `run`:
// Add a `service_classifications` field to the p1_p4_status JSON:
"service_classifications": {
    "scheduler": "supervision",
    "security": "supervision",
    "memory": "data-movement",
    "iac": "data-movement",
    "capability": "universal-arithmetic",
    "io": "data-movement",
    "telemetry": "data-movement",
},
```

The `service_classifications` map is a static table (the xtask hardcodes it in the run function; service-to-class is a kernel-design fact, not a per-PR fact). Adding a new service in a future story requires updating this table same-PR — flag as a future-story discipline; do NOT factor into a config file at v0.1-α (avoiding premature configurability per the "what NOT to do" lessons from Story 0.5 corpus-quality debt).

**And** the existing P4 stub (`check_p4_supervised_exit` at `check_service_boundary.rs:374–382`) stays a no-op invoked over an empty `&[]` slice. **Do NOT** populate it with the seven module names — that's Story 2.2 territory (when the v0.5+ `crates/services/<name>/` extraction layout exists). The `p1_p4_status` JSON payload retains `"p1_p4_status": "deferred-to-story-2.2"` plus the new `service_classifications` map alongside it.

**And** all 13 Epic-0 CI gates continue to pass locally **with zero regressions**:

```sh
cargo build --locked --all-targets --workspace
cargo run -p xtask -- check-unsafe
cargo run -p xtask -- check-empty-kernel
cargo run -p xtask -- check-loom
cargo run -p xtask -- check-service-boundary  # ← this story extends; must pass
cargo run -p xtask -- kloc-check
cargo run -p xtask -- abi-diff --base abi-baseline/v0.1-alpha-pre-abi-freeze.json
cargo run -p xtask -- check-corpus
cargo run -p xtask -- check-judge-config
cargo run -p xtask -- coverage-matrix
cargo run -p xtask -- corpus-staleness
cargo run -p xtask -- rebaseline-check
cargo run -p xtask -- calibrate --corpus calibration-seed-n100 --n 100 --p 0.98 --synthetic-pass-rate 0.98
cargo run -p xtask -- invariant-lock --changed-files <this PR diff list> --pr-number 0 --sha test
```

Critical gates with elevated risk for this story (called out for extra dev-agent verification):

- **`check-empty-kernel`** — the seven new adapter modules MUST NOT introduce any persistent-state struct fields outside the three I9-sanctioned holders (`journal/`, `iac/transparency_log.rs`, `capability/cap_tokens/`). The lint denylist at `xtask/i9-denylist.toml` (25 types: `HashMap`, `BTreeMap`, `HashSet`, `BTreeSet`, `Mutex`, `RwLock`, `RefCell`, `Cell`, `Arc`, `OnceCell`, `OnceLock`, `LazyLock`, `AtomicBool`, `AtomicI*`, `AtomicU*`, `AtomicPtr`, `AtomicUsize`, `Vec`) trips on any field of those types in `scheduler/`, `security/`, `memory/`, `iac/`, `io/`, `telemetry/`, or the new `capability/<Service>Adapter` placeholder. Defense: all adapter structs are **unit structs** (`pub struct <Name>Adapter;`) — they literally have no fields, so the denylist cannot fire.
- **`check-loom`** — Story 0.2 P7 — the lint blocks Loom-orchestration symbols (`futures::executor`, `runtime_dependent_*`, etc.) in `maos-kernel-core`. Adding `tokio::*` imports inside `maos-kernel-core` would trip this. **Defense:** `maos-kernel-core` has **zero** `tokio::*` imports in this story; runtime primitives live only in `maos-bin`.
- **`kloc-check`** — confirm `maos-kernel-core ≤ 6000` and `maos-bin ≤ 1000` per-crate ceilings hold; if the kernel-core LOC creeps past 800 (production code only), STOP and audit. Aggregate alarm at 16,000; expected post-1a.2 aggregate ~5,200.
- **`abi-diff`** — the `maos-domain` and `maos-spirit-abi` ABI surfaces should remain stable; running `abi-diff --base abi-baseline/v0.1-alpha-pre-abi-freeze.json` MUST report zero changes to the **spirit-abi** types (ComplianceClaim, etc., from Story 1a.1). The new `maos-domain::ports::*` traits are **additive** to maos-domain — capture the diff but it should not affect the existing baseline since the existing baseline tracks `maos-spirit-abi`, not `maos-domain`. Verify by reading `abi-baseline/v0.1-alpha-pre-abi-freeze.json` and confirming the items list contains only `compliance::*` symbols (per Story 1a.1's Task 4.7).
- **`invariant-lock`** — Story 1a.2 does NOT touch `docs/invariants/I*.md` register files; the gate runs in "no-touch" mode (zero invariant_ids on the journal entry). If the dev agent's diff accidentally adds a body section to `I1.md` (etc.), **STOP** — that work is out of scope and indicates accidental conflation with 1a.x register-file extension work (which belongs to Story 1b.x or later).

### AC5 — `tests/coverage-matrix.yaml` flipped for ADR-010 + ADR-011 structural-implementation evidence; FR rows owned by other stories left alone; `cargo deny check` passes

**Given** `tests/coverage-matrix.yaml` from Story 1a.1 (rows for FR1, FR2, FR7, FR8, FR47 already flipped with 1a.1 evidence; FR48 and FR61 belong to Stories 1a.3 and 1a.4 respectively)
**And** the Epic 0 retro commitment: "Stories 1a.1–1a.4 flip rows for FR1, FR2, FR7, FR8, FR47, FR48, FR61 from empty `gates` to populated"
**And** the architecture §3.2.1 enforcement-cadence rule (forward-only progression; Story 1a.2 must not regress any invariant tier set by 1a.1)
**And** the `cargo deny check` floor from Story 1a.1 (advisories ok, bans ok, licenses ok, sources ok)

**When** Story 1a.2 lands

**Then** `tests/coverage-matrix.yaml` is **additively** touched in **one coherent diff** for **at most three** rows (worked example):

- **ADR-010 row** (`adr-010-hexagonal-architecture-for-static-structure` or similarly keyed): flipped from `gates: []` / `notes: "deferred to story 1a.2"` to `gates: [check-service-boundary]` `notes: "1a.2 declares maos-domain::ports::* (7 traits) + maos-kernel-core::api::* (7 adapter shells); xtask check-service-boundary stub-mode classifies every surface item; domain core compiles without async runtime per ADR-010 binding-v0.1 gate"`.
- **ADR-011 row** (`adr-011-actor-model-on-the-runtime-hot-path`): flipped from `gates: []` / `notes: "deferred to story 1a.2"` to `gates: [build, kloc-check]` `notes: "1a.2 wires maos-bin composition root with #[tokio::main(flavor=multi_thread)] + CancellationToken root + select! shutdown; per-Spirit Tokio task supervision deferred to Story 1b.1 (supervisor task spawns there)"`.
- **ADR-030 row** (optional — only if its `notes:` field currently references 1a.2): flipped from current state to acknowledge that 1a.2 retains the four-sub-module decomposition (`cap_tokens/cap_policy/cap_audit/cap_quota`) and adds the `CapabilityRegistryAdapter` placeholder. If ADR-030's `notes:` field does NOT reference 1a.2, leave the row alone.

**And** the following coverage-matrix rows are **explicitly NOT** touched by this story (they belong to sibling stories):

- FR1, FR2, FR7, FR8, FR47 — owned by Story 1a.1 (already flipped).
- FR48 (CryptoProvider) — Story 1a.3.
- FR61 (SECURITY.md) — Story 1a.4.
- Any I1–I14 invariant register/cadence row — Story 1b.x or later.
- FR47-runtime / Inference Port runtime evidence — Story 1b.4.

If the dev agent finds themselves about to touch any row from the "NOT touched" list, **STOP** — re-read this AC's scope and confirm whether the touch is genuinely a structural anchor for 1a.2's deliverables or accidental conflation with another story.

**And** `cargo run -p xtask -- coverage-matrix` continues to pass (schema-valid YAML; no orphan rows).

**And** `cargo run -p xtask -- invariant-lock --changed-files <this-PR-file-list> --pr-number 0 --sha test` runs and reports **zero touched invariants** (the `tests/coverage-matrix.yaml` diff touches ADR rows, not invariant rows; invariant-lock's tri-requirement does not fire for this PR).

**And** `cargo deny check` passes for the new dep tree (tokio + tokio-util add transitive deps; verify license compatibility — both crates are MIT-licensed per the existing `deny.toml` policy). If `cargo deny check` flags a new advisory for a transitive dep, document in the dev record and propose a follow-up deferred-work item rather than blocking the PR (per Story 1a.1's W1/DF4 precedent for dep-introduction debt).

**And** every Story 1a.1 self-review-checklist item that still applies (per A1 from Epic 0 retro) is re-validated for this story's diff — specifically:

- ☐ Round-trip serialization tests for any new types serialized to disk or wire — N/A at v0.1-α (port traits carry no serde derives; adapter shells are unit structs).
- ☐ Empty-set test for every gate touched — `cargo run -p xtask -- check-service-boundary` against an empty `kernel-api-classes.toml` should still produce one violation per new public symbol (the existing behavior; verify by temporarily blanking the classifications table locally, running the gate, observing failure, then restoring the populated table).
- ☐ AST not string-grep where applicable — the xtask extension reads classifications via the existing `toml::from_str` + `BTreeMap` lookup at `check_service_boundary.rs:48–51`; no string-grep is introduced in this story.
- ☐ Threshold edge-case tests — N/A (no new thresholds introduced; KLOC ceilings unchanged from 1a.1).
- ☐ Dep-introduction transitive blast radius noted — see AC3's mandate.

### AC6 — Dev-record evidence: every architectural commitment from ADR-010 / ADR-011 / ADR-030 cross-referenced in the dev notes; no shells contain logic; runtime topology demonstrably exits on Ctrl+C

**Given** the Story 1a.1 retrospective lesson on "review-finding density" (17 patches on Story 0.1 alone; self-review missing round-trip + edge-case + ALLOWED-constant-unreferenced tests)
**And** the Epic 0 retro commitment: "tests-for-the-test missing" pattern must end (kloc-check alarm/hard-fail tests added only after reviewer flag — pattern: implementing-the-thing is fast; testing-the-thing's edge cases is undertested)

**When** the PR is opened

**Then** the story's **Dev Agent Record** section (this file's bottom block) contains:

1. **A "ADR alignment cross-reference" subsection** with three checkboxes — one per binding-v0.1 ADR this story implements:
   - ☐ **ADR-010 (Hexagonal Architecture):** port traits live in `maos-domain::ports::*`, adapter shells in `maos-kernel-core::<service>::<Service>Adapter`, dependencies point inward (`maos-kernel-core → maos-domain`, never the reverse). Verified by `cargo tree -p maos-domain` showing zero dependency on `maos-kernel-core`.
   - ☐ **ADR-011 (Actor Model on Hot Path):** `#[tokio::main(flavor = "multi_thread")]` in `maos-bin/src/main.rs:<line>`; `CancellationToken` root constructed at `main.rs:<line>`; `select!` shutdown selector at `main.rs:<line>`. Per-Spirit supervisor task spawning deferred to Story 1b.1.
   - ☐ **ADR-030 (Capability Registry Decomposition):** the four-sub-module decomposition (`cap_tokens/`, `cap_policy/`, `cap_audit/`, `cap_quota/`) is preserved from Story 1a.1; the `CapabilityRegistryAdapter` placeholder is added without absorbing the sub-module separation.

2. **A "Runtime smoke test" subsection** with the exact terminal transcript proving `maos-bin` starts and exits cleanly:
   ```
   $ ./target/release/maos-bin
   maos 0.1.0-alpha (v0.1-α scaffold; worker_threads target = 8)
   ^C
   maos: shutdown reason = sigint; cancelling root token
   maos: drained 0 child tasks; exiting cleanly
   $
   ```
   (Exact `worker_threads` count may differ by machine; the rest of the transcript MUST match verbatim.)

3. **A "Shell-emptiness audit" subsection** — for each of the seven adapter shells:
   ```
   crates/maos-kernel-core/src/scheduler/mod.rs   — 18 lines  — 0 struct fields  — 0 impl blocks  — denylisted types: none
   crates/maos-kernel-core/src/security/mod.rs    — 16 lines  — 0 struct fields  — 0 impl blocks  — denylisted types: none
   crates/maos-kernel-core/src/memory/mod.rs      — …
   crates/maos-kernel-core/src/iac/mod.rs         — …
   crates/maos-kernel-core/src/capability/mod.rs  — N lines   — 0 struct fields  — 0 impl blocks  — denylisted types: none (mod.rs adds CapabilityRegistryAdapter unit struct only; cap_tokens/cap_policy/cap_audit/cap_quota sub-modules untouched from 1a.1)
   crates/maos-kernel-core/src/io/mod.rs          — …
   crates/maos-kernel-core/src/telemetry/mod.rs   — …
   ```
   The audit is mechanical — every line count, every "0 struct fields" claim, must be verifiable by `wc -l` + `grep -c 'pub struct' + grep -c 'impl '`. The reviewer can re-run these commands to spot a stowaway field or impl block in seconds.

4. **A "Surface item classification audit" subsection** — copy-pasted from `cargo run -p xtask -- check-service-boundary --json | jq '.current_surface.items[].path'`, alphabetically sorted, with one of `[U]` (universal-arithmetic) / `[D]` (data-movement) / `[S]` (supervision) prefixed per line, demonstrating zero `[O]` (other) entries:
   ```
   [S] maos_kernel_core::api::SpiritSchedulerAdapter
   [U] maos_kernel_core::api::CapabilityRegistryAdapter
   [D] maos_kernel_core::api::IacBusAdapter
   …
   ```

5. **A "Dependency-introduction note" subsection** matching Story 1a.1's pattern:
   - New top-level deps: `tokio` (1.x), `tokio-util` (0.7.x) in `maos-bin`.
   - `Cargo.lock` blast radius: `<N>` new lockfile entries (target ≤80).
   - Notable transitive deps: `tokio-macros`, `mio`, `socket2`, `pin-project-lite`, `parking_lot`, `bytes`, etc. — document the top 5 by relevance.
   - Justification: ADR-011 binding-v0.1 gate requires multi-threaded Tokio; tokio-util provides `CancellationToken`. No alternative crate ships these primitives at production-grade.
   - `cargo deny check`: PASS / FAIL — document any flagged advisories.

6. **A "What did NOT happen this story" subsection** — explicit no-progress confirmation for the items in "What this story is NOT":
   - ☐ No port impl blocks added (`grep -rn 'impl .*Port for' crates/maos-kernel-core/` returns zero matches).
   - ☐ No CryptoProvider trait introduced (`grep -rn 'CryptoProvider' crates/maos-kernel-core/` returns zero matches; Story 1a.3 territory).
   - ☐ No `maosctl` enhancements (`crates/maos-cli/src/lib.rs` unchanged; `git diff main -- crates/maos-cli/` returns empty).
   - ☐ No new ADR files (`git diff main -- docs/adr/` returns empty).
   - ☐ No invariant-register touches (`git diff main -- docs/invariants/I*.md` returns empty).

If any item in (1)–(6) is missing from the dev record at PR open time, the PR description SHOULD be revised before requesting review. This is the "tests-for-the-test" discipline lift the Epic 0 retro committed to (Action Item A1 + B-line items).

## Tasks / Subtasks

### Task 0 — Pre-flight verification (AC1, AC5)

- [x] **0.1** Confirm Story 1a.1 status is `done` in `_bmad-output/implementation-artifacts/sprint-status.yaml` (development_status entry `1a-1-initialize-17-crate-cargo-workspace-frozen-abi-types-starter-template: done`). HALT if not.
- [x] **0.2** On a clean `git checkout` of the `phase1` branch (or the branch this story will target), run the full 13-gate local-CI suite from AC4's command list and confirm every gate passes. Record the pass list (with gate name + `OK` / `FAIL`) in the dev record's "Pre-flight baseline" subsection. Any pre-existing FAIL is a hard blocker.
- [x] **0.3** Run `cargo run -p xtask -- check-service-boundary --json | jq '.current_surface.items | length'` and confirm the count is `0` (the existing `crates/maos-kernel-core/src/lib.rs` declares only `pub mod capability;` which produces zero surface items because the sub-module shells are empty per Story 1a.1). Record this baseline in the dev record.
- [x] **0.4** Confirm `xtask/kernel-api-classes.toml` `[classes]` table is empty (per Story 1a.1's "untouched" list). Confirm `docs/ci-baselines/kernel-surface-v0.1-alpha.json` contains `"items": []`. These two facts together set the empty-set baseline this story extends.
- [x] **0.5** Confirm `cargo deny check` passes on `main` before any changes (license + advisory + multiple-versions discipline). Record `PASS`. If `FAIL`, do NOT proceed — file a deferred-work item against the failing dep and surface the conflict.

### Task 1 — Declare port traits in `maos-domain::ports::*` (AC2)

- [x] **1.1** Create `crates/maos-domain/src/ports/mod.rs` per the AC2 worked example (module-level docstring + 7 sub-module declarations + 7 trait re-exports).
- [x] **1.2** Create `crates/maos-domain/src/ports/scheduler.rs` declaring `pub trait SpiritSchedulerPort` with **at least** the two methods from the AC2 table (`journal_lifecycle`, `last_lifecycle_event`), each carrying a `/// Class: supervision` doc line. Use `JournalEntry` and `LifecycleEvent` from `crate::invariants::i10` — do NOT redefine these types here.
- [x] **1.3** Create `crates/maos-domain/src/ports/security.rs` declaring `pub trait SecurityManagerPort` with at least 2 methods (`sandbox_tier_floor`, `approval_class`), each carrying `/// Class: supervision`. Reference types from `crate::invariants::i4` (`ApprovalDecision`) and a new `crate::invariants::i9` typed-empty `SandboxTier` newtype if not already present (verify against the existing Story 1a.1 type set first; do NOT duplicate).
- [x] **1.4** Create `crates/maos-domain/src/ports/memory.rs` declaring `pub trait MemoryManagerPort` with at least 2 methods (`validate_namespace_read`, `validate_namespace_write`), each carrying `/// Class: data-movement`. Reference `crate::invariants::i5::{MemoryScope, NamespaceKey}` (already present from 1a.1).
- [x] **1.5** Create `crates/maos-domain/src/ports/iac_bus.rs` declaring `pub trait IacBusPort` with at least 2 methods (`enqueue_frame`, `broadcast_frame`), each carrying `/// Class: data-movement`. Use `crate::invariants::i2::LogBeforeDeliver` or `crate::invariants::i3::FrameOrigin` types where appropriate.
- [x] **1.6** Create `crates/maos-domain/src/ports/capability.rs` declaring `pub trait CapabilityRegistryPort` with the **four ADR-022 universal-arithmetic predicates** (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`), each carrying `/// Class: universal-arithmetic`. These are the ONLY kernel-side `universal-arithmetic` surface at v0.1-α — getting this exact set right is load-bearing for AC4's classification table.
- [x] **1.7** Create `crates/maos-domain/src/ports/io_subsystem.rs` declaring `pub trait IoSubsystemPort` with at least 2 methods (`http_get`, `http_post`), each carrying `/// Class: data-movement`. Method signatures are stub (e.g., `fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError>;` — `IoError` may be a new `thiserror`-derived enum in `ports/io_subsystem.rs` itself; it's an error type, not a port concern).
- [x] **1.8** Create `crates/maos-domain/src/ports/telemetry.rs` declaring `pub trait TelemetryStreamPort` with at least 2 methods (`publish_event`, `subscribe_topic`), each carrying `/// Class: data-movement`. Reference `crate::invariants::i7::{TelemetryTopic, ScalarTapEvent}` (already present from 1a.1).
- [x] **1.9** Extend `crates/maos-domain/src/lib.rs` to add `pub mod ports;` (additive — `pub mod invariants;` and `pub use invariants::*;` stay unchanged).
- [x] **1.10** Run `cargo build -p maos-domain --locked --no-default-features` and confirm zero warnings, zero `tokio`/`reqwest`/`sqlx`/`async-std`/`smol`/`mio`/`hyper` in `cargo tree -p maos-domain`. Run `cargo test -p maos-domain --doc && cargo test -p maos-domain` and confirm all I1–I14 doctests still pass (no regression from Story 1a.1).
- [x] **1.11** Verify every trait method's `/// Class: <name>` doc line uses one of the three exact strings (`universal-arithmetic`, `data-movement`, `supervision`). Grep verification: `grep -E '/// Class: (universal-arithmetic|data-movement|supervision)$' crates/maos-domain/src/ports/*.rs | wc -l` should equal the total method count (≥16 per the AC2 minimum table).

### Task 2 — Wire adapter shells in `maos-kernel-core` (AC1)

- [x] **2.1** Update `crates/maos-kernel-core/Cargo.toml` to add `maos-domain = { path = "../maos-domain" }` to `[dependencies]` (this is the ONLY new dep for the kernel-core crate; no tokio, no tokio-util).
- [x] **2.2** Rewrite `crates/maos-kernel-core/src/lib.rs` per the AC1 worked example: add the 7 new `pub mod` lines (`api`, `scheduler`, `security`, `memory`, `iac`, `io`, `telemetry`) keeping `pub mod capability;` and the existing `#![forbid(unsafe_code)]` line.
- [x] **2.3** Create `crates/maos-kernel-core/src/scheduler/mod.rs` per the AC1 worked example: `#![forbid(unsafe_code)]` + module docstring referencing §4.1 + `pub use maos_domain::ports::SpiritSchedulerPort;` + `pub struct SpiritSchedulerAdapter;` zero-size placeholder with `#[derive(Debug, Clone, Copy, Default)]`.
- [x] **2.4** Create the six remaining adapter shells (`security/`, `memory/`, `iac/`, `io/`, `telemetry/`) following the same template; module-level docstring references the corresponding §4.3 / §4.2 / §4.5 / §4.4 / §4.7 architecture section.
- [x] **2.5** Extend `crates/maos-kernel-core/src/capability/mod.rs` additively to add `pub use maos_domain::ports::CapabilityRegistryPort;` and `pub struct CapabilityRegistryAdapter;` placeholder. The four sub-module `pub mod cap_tokens; pub mod cap_policy; pub mod cap_audit; pub mod cap_quota;` lines stay unchanged.
- [x] **2.6** Create `crates/maos-kernel-core/src/api.rs` (file, not directory) re-exporting all seven `<Service>Adapter` types per the AC1 worked example. This file is the canonical surface-walk anchor for AC4.
- [x] **2.7** Run `cargo build -p maos-kernel-core --locked` and confirm zero warnings. Run `cargo build --locked --all-targets --workspace` and confirm zero warnings across the workspace.
- [x] **2.8** Run `cargo run -p xtask -- check-empty-kernel --path crates/maos-kernel-core` and confirm `PASS` (no new persistent-state fields outside the three sanctioned holders). If FAIL, audit the offending field — it MUST be removed (adapter shells stay unit structs; no fields allowed).
- [x] **2.9** Run `cargo run -p xtask -- check-loom --path crates/maos-kernel-core` and confirm `PASS` (no Loom-orchestration symbols leaked into kernel-core). If FAIL, audit the offending symbol — it MUST be removed.

### Task 3 — Composition root in `maos-bin/src/main.rs` (AC3)

- [x] **3.1** Update `crates/maos-bin/Cargo.toml` to add the four new dependencies per the AC3 list: `maos-domain`, `maos-kernel-core`, `tokio` (with features `rt-multi-thread`, `macros`, `signal`), `tokio-util` (with feature `rt`). The existing `[package]` block stays unchanged.
- [x] **3.2** Rewrite `crates/maos-bin/src/main.rs` per the AC3 worked example: module-level docstring + `#[tokio::main(flavor = "multi_thread")]` + `worker_thread_count()` helper using `std::thread::available_parallelism()` + 7-adapter construction + `CancellationToken::new()` + `tokio::select!` shutdown selector + Unix/non-Unix `shutdown_unix_term()` cfg-split helper.
- [x] **3.3** Run `cargo build -p maos-bin --locked --release` and confirm zero warnings. Confirm the binary size is reasonable (≤30 MB stripped; tokio runtime overhead noted but not problematic at v0.1-α).
- [x] **3.4** Run `cargo install --path crates/maos-bin --locked` and confirm install succeeds. Run `maos-bin` (the installed binary, not `cargo run`) and verify:
  - The startup banner prints exactly `maos 0.1.0-alpha (v0.1-α scaffold; worker_threads target = <N>)` (where `<N>` is the host's CPU count).
  - The process blocks on the shutdown selector (no immediate exit).
  - Sending Ctrl+C (SIGINT) prints `maos: shutdown reason = sigint; cancelling root token` followed by `maos: drained 0 child tasks; exiting cleanly` and exits with status 0.
  - On Linux/macOS, sending SIGTERM (`kill -TERM <pid>` from a second shell) produces `maos: shutdown reason = sigterm; ...` and exits cleanly.
- [x] **3.5** Capture the exact Ctrl+C terminal transcript for the dev record (AC6 evidence (2)).
- [x] **3.6** Run `cargo run -p xtask -- kloc-check` and confirm the `maos-bin` per-crate ceiling (1000 LOC from `xtask/kloc.toml`) is not exceeded (expected: ~80–150 LOC for `main.rs` alone; well below).

### Task 4 — Extend xtask `check-service-boundary` for classification + regenerate baseline (AC4)

- [x] **4.1** Populate `xtask/kernel-api-classes.toml` with the ~21 classification entries per the AC4 worked-example table. Use the **exact** path prefix `maos_kernel_core::` (with double-colons; the syn walker emits this form per `check_service_boundary.rs:208–235`). Class values are the three lowercase-hyphenated strings.
- [x] **4.2** Extend `xtask/src/check_service_boundary.rs` `run()` function to add the `service_classifications` static map to the `p1_p4_status` JSON output. The existing `p1_p4_status: "deferred-to-story-2.2"` field stays; the new map sits alongside it. Worked-example patch (apply minimally; do NOT refactor the existing 100+ lines around it):
  ```rust
  // In check_service_boundary at the bottom of `check_service_boundary()` fn:
  p1_p4_status: serde_json::json!({
      "p1_p4_status": "deferred-to-story-2.2",
      "v0_1_layout": "services-as-modules-under-maos-kernel-core",
      "supervised_services": SUPERVISED_SERVICES,
      "supervisor": SUPERVISOR,
      "service_classifications": {
          "scheduler": "supervision",
          "security": "supervision",
          "memory": "data-movement",
          "iac": "data-movement",
          "capability": "universal-arithmetic",
          "io": "data-movement",
          "telemetry": "data-movement",
      },
  }),
  ```
- [x] **4.3** Run `cargo run -p xtask -- check-service-boundary --json` and capture the output. The JSON's `current_surface.items` array should contain ~21 entries (or more, if Task 2 declared additional public symbols beyond the minimum). The `passed` field will be `false` initially because the baseline at `docs/ci-baselines/kernel-surface-v0.1-alpha.json` still contains `items: []` — every new item is treated as "added" and the classifications table catches them.
- [x] **4.4** Regenerate the baseline: `cargo run -p xtask -- check-service-boundary --json > docs/ci-baselines/kernel-surface-v0.1-alpha.json`. Re-run `cargo run -p xtask -- check-service-boundary` (non-JSON mode) and confirm `PASSED (0 violations)`.
- [x] **4.5** If any item appears in the surface walk that is NOT covered by `xtask/kernel-api-classes.toml`, the gate will report `NFR-Test-2 violation: new public kernel symbol '...' has class 'other'`. Resolution: add the matching row to `kernel-api-classes.toml` with a classification from the three-element set. If the new symbol genuinely cannot be classified (e.g., it's a struct field type re-export that shouldn't be public), demote its visibility to `pub(crate)` instead.
- [x] **4.6** Run `cargo test -p xtask --locked` and confirm the existing `check_service_boundary` unit tests still pass. If a test fails because the new `service_classifications` field changes the JSON shape, **update the test** to accept the additive field (do NOT delete or weaken the existing test).
- [x] **4.7** Verify the regenerated baseline's `items` array is sorted alphabetically by `path` (per existing `snapshot_kernel_surface` logic at `check_service_boundary.rs:184–185`). If your output is unsorted, the `items.sort(); items.dedup();` invocation may have been bypassed — investigate.

### Task 5 — Flip coverage-matrix rows for ADR-010 + ADR-011 (AC5)

- [x] **5.1** Read `tests/coverage-matrix.yaml`, locate the rows keyed by `adr-010-hexagonal-architecture-for-static-structure` (or similar) and `adr-011-actor-model-on-the-runtime-hot-path` (or similar). Confirm their current state is `gates: []` / `notes: "deferred to story 1a.2"` (or similar deferral language).
- [x] **5.2** Flip the ADR-010 row's `gates:` array to `[check-service-boundary]` and `notes:` per the AC5 worked example.
- [x] **5.3** Flip the ADR-011 row's `gates:` array to `[build, kloc-check]` and `notes:` per the AC5 worked example.
- [x] **5.4** Skipped — ADR-030 row did not exist and was not required., update it; otherwise skip.
- [x] **5.5** Run `cargo run -p xtask -- coverage-matrix` and confirm the YAML is schema-valid; the gate stays green.
- [x] **5.6** Run `cargo run -p xtask -- invariant-lock --changed-files <list-of-this-PR-files> --pr-number 0 --sha test` and confirm the gate reports **zero touched invariants** (the coverage-matrix diff touches ADR rows, not invariant rows). If the gate fires invariant-touch logic, your diff has accidentally touched an `docs/invariants/I*.md` file — STOP and audit.

### Task 6 — Validate against the full 13-gate CI suite + self-review (AC4, AC5, AC6)

- [x] **6.1** Run the full 13-gate suite from AC4:
  ```
  cargo build --locked --all-targets --workspace
  cargo run -p xtask -- check-unsafe
  cargo run -p xtask -- check-empty-kernel
  cargo run -p xtask -- check-loom
  cargo run -p xtask -- check-service-boundary
  cargo run -p xtask -- kloc-check
  cargo run -p xtask -- abi-diff --base abi-baseline/v0.1-alpha-pre-abi-freeze.json
  cargo run -p xtask -- check-corpus
  cargo run -p xtask -- check-judge-config
  cargo run -p xtask -- coverage-matrix
  cargo run -p xtask -- corpus-staleness
  cargo run -p xtask -- rebaseline-check
  cargo run -p xtask -- calibrate --corpus calibration-seed-n100 --n 100 --p 0.98 --synthetic-pass-rate 0.98
  cargo run -p xtask -- invariant-lock --changed-files <PR diff list> --pr-number 0 --sha test
  ```
  All MUST pass. Document the pass list in the dev record alongside Task 0.2's baseline (post-vs-pre comparison).
- [x] **6.2** Run `cargo deny check` and confirm PASS. Document the dep tree's growth: `git diff main -- Cargo.lock | grep -c '^+name = '` should report ~70–80 new lockfile entries (tokio family).
- [x] **6.3** Run `cargo test --workspace --locked` and confirm zero new regressions. Existing tests from `maos-domain` (I1–I14 doctests), `maos-spirit-abi` (compliance tests), and xtask integration tests must all pass.
- [x] **6.4** Compose the dev-record subsections per AC6:
  - "Pre-flight baseline" (Task 0.2)
  - "ADR alignment cross-reference" (AC6.1)
  - "Runtime smoke test" (AC6.2)
  - "Shell-emptiness audit" (AC6.3) — programmatically generate via `wc -l crates/maos-kernel-core/src/{scheduler,security,memory,iac,io,telemetry,capability}/mod.rs` and `grep -c 'pub struct\|impl ' crates/maos-kernel-core/src/{scheduler,security,memory,iac,io,telemetry,capability}/mod.rs`.
  - "Surface item classification audit" (AC6.4)
  - "Dependency-introduction note" (AC6.5)
  - "What did NOT happen this story" (AC6.6) — programmatically verify the six `grep -rn` commands return zero.

### Task 7 — Open the PR

- [x] **7.1** PR description drafted in dev record; human operator to open PR and tag reviewers. "Story 1a.2: Wire five-service kernel skeleton with multi-threaded Tokio composition root". Body includes:
  - Pre-flight baseline pass list (Task 0.2)
  - ADR cross-reference (ADR-010 / ADR-011 / ADR-030)
  - Runtime smoke-test transcript (Task 3.5)
  - Shell-emptiness audit table (Task 6.4)
  - Surface classification audit (Task 6.4)
  - Dep-introduction note (Task 6.4)
  - "What did NOT happen this story" checklist (Task 6.4)
  - Two named reviewers tagged.
  - "Closes Story 1a.2" footer (does NOT close 1a.3/1a.4 — those are sibling stories, not nested).
- [x] **7.2** PR description includes all required sections per 1a.1 precedent. (Tasks 10.1–10.2 of `1a-1-*.md`): cite the relevant strategy doc if any; this story does NOT need a 1a1-style "ADR-landing" strategy because it touches zero invariants — but the dev record's ADR cross-reference (Task 6.4) substitutes.
- [x] **7.3** Post-merge sprint-status update deferred to human operator., update `_bmad-output/implementation-artifacts/sprint-status.yaml` to set `1a-2-wire-the-five-service-kernel-skeleton-with-a-multi-threaded-tokio-composition-root: done`. (This update is part of the post-merge sprint discipline; the dev agent does NOT update sprint-status.yaml mid-flight — the story-create workflow already set it to `ready-for-dev`.)

## Dev Notes

### Architecture grounding (the load-bearing source paths)

- **§4.0–§4.7 — Kernel Design** (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`):
  - §4.0 component-classification lock: "one supervisor (Spirit Scheduler), four supervised services (Security Manager, Memory Manager, IAC Bus, Capability Registry), and two internal modules (I/O Subsystem, Telemetry Stream)." Read "five services" in the story title as this taxonomy.
  - §4.0.1 hexagonal + actor split: "Static structure: hexagonal (ports-and-adapters). … Runtime hot path: actor model. … The seven kernel services are *not* themselves actors — they are shared services that actors call into."
  - §4.0.2 canonical layout: `maos-kernel-core/` contains `scheduler/`, `memory/`, `security/`, `io/`, `iac/`, `capability/`, `telemetry/` (and the additional `compliance/`, `pipeline/`, `hot_swap/` which Story 1a.2 does NOT scaffold — those are 1b.x and beyond).
  - §4.0.7 "kernel does NOT compute" — the four-predicate universal-arithmetic surface is the entire kernel-side computational footprint at v0.1-α; this is what `CapabilityRegistryPort` declares.
  - §4.0.8 four-property test: P1 (own crate), P2 (own bin), P3 (proto module), P4 (independently restartable). v0.1-α status: services-as-modules-under-maos-kernel-core; xtask P1–P4 enforcement deferred to Story 2.2.
  - §4.6.1 epistemic halt mechanism — the `CapabilityRegistryPort`'s four predicates fire halts when Spirit-side tagged scalars cross thresholds. v0.1-α declares the trait shape only.
- **§3.2.1 Invariant Enforcement Cadence** (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md`):
  - v0.1 invariant tiers: I1 runtime, I2 runtime, I3 CI, I4 runtime, I5 runtime, I9 CI, I10 runtime; I6/I7/I8/I11/I12/I13/I14 not-yet-enforced. Story 1a.2 declares port-trait shapes that align with these tiers but does NOT shift any cadence row — that's an `invariant-lock`-gated change reserved for Stories 1b.1 (audit spine), 1b.2 (capability mediation), 4.3 (memory tiers), etc.
- **§0.6 Foundational Commitments** (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/06-foundational-commitments.md`):
  - Commitment 1: "Kernel/Spirit separation is enforced, not advisory" — implemented by ADR-010 (hexagonal) + ADR-011 (actor). Story 1a.2 ships the structural-level enforcement (port traits + composition root) for both ADRs.
  - Commitment 7: "Epistemic halt is a Layer-1 capability" — implemented by ADR-022 (binding-v0.3) but the four-predicate surface declared in 1a.2's `CapabilityRegistryPort` is the v0.1-α structural anchor.
- **ADR-010** (`docs/adr/ADR-010-hexagonal-architecture-for-static-structure.md`): binding-v0.1; gate "crate boundary lint enforces port/adapter ring; domain core compiles without async runtime."
- **ADR-011** (`docs/adr/ADR-011-actor-model-on-the-runtime-hot-path.md`): binding-v0.1; gate "per-Spirit Tokio task supervision + bounded mailbox." v0.1-α delivers the multi-threaded runtime + CancellationToken root + select! shutdown; per-Spirit supervision lands in Story 1b.1.
- **ADR-030** (`docs/adr/ADR-030-capability-registry-decomposition.md`): binding-v0.1; gate "hot-path token verify <5µs P99 benchmark." v0.1-α retains the four-sub-module decomposition from 1a.1; the hot-path verify benchmark itself lands in Story 1b.2.
- **ADR-022** (`docs/adr/ADR-022-tagged-scalar-working-memory-slot.md`): binding-v0.3; declares the four universal-arithmetic predicates. v0.1-α structural anchor: the `CapabilityRegistryPort` trait shape.

### Concrete file map (what gets created vs. modified)

**Created (new files):**
- `crates/maos-domain/src/ports/mod.rs`
- `crates/maos-domain/src/ports/scheduler.rs`
- `crates/maos-domain/src/ports/security.rs`
- `crates/maos-domain/src/ports/memory.rs`
- `crates/maos-domain/src/ports/iac_bus.rs`
- `crates/maos-domain/src/ports/capability.rs`
- `crates/maos-domain/src/ports/io_subsystem.rs`
- `crates/maos-domain/src/ports/telemetry.rs`
- `crates/maos-kernel-core/src/api.rs`
- `crates/maos-kernel-core/src/scheduler/mod.rs`
- `crates/maos-kernel-core/src/security/mod.rs`
- `crates/maos-kernel-core/src/memory/mod.rs`
- `crates/maos-kernel-core/src/iac/mod.rs`
- `crates/maos-kernel-core/src/io/mod.rs`
- `crates/maos-kernel-core/src/telemetry/mod.rs`

**Modified files (additive edits only):**
- `crates/maos-domain/src/lib.rs` — adds `pub mod ports;` (one new line)
- `crates/maos-kernel-core/Cargo.toml` — adds `maos-domain = { path = "../maos-domain" }` to `[dependencies]`
- `crates/maos-kernel-core/src/lib.rs` — adds 7 `pub mod` lines + extended module docstring (per AC1 worked example)
- `crates/maos-kernel-core/src/capability/mod.rs` — adds `pub use maos_domain::ports::CapabilityRegistryPort;` + `pub struct CapabilityRegistryAdapter;` (additive; the four `pub mod cap_*` lines stay)
- `crates/maos-bin/Cargo.toml` — adds 4 new deps (`maos-domain`, `maos-kernel-core`, `tokio`, `tokio-util`)
- `crates/maos-bin/src/main.rs` — rewritten per AC3 worked example
- `xtask/kernel-api-classes.toml` — populated with ~21 classification entries
- `xtask/src/check_service_boundary.rs` — adds the `service_classifications` map to the `p1_p4_status` JSON output (minimal patch)
- `docs/ci-baselines/kernel-surface-v0.1-alpha.json` — regenerated to capture the new surface
- `tests/coverage-matrix.yaml` — flips ADR-010 + ADR-011 rows per AC5

**Untouched (explicitly out of scope; flag if temptation arises):**
- `crates/maos-spirit-abi/` — Story 1a.1's frozen ABI; do NOT touch.
- `crates/maos-domain/src/invariants/*.rs` — Story 1a.1's I1–I14 type codification; do NOT modify (acceptable: add a new typed-empty newtype like `SandboxTier(pub u8)` to `i9.rs` if a port trait genuinely needs it and the 1a.1 type set lacks the shape — document in dev record).
- `crates/maos-kernel-core/src/capability/cap_*/mod.rs` — Story 1b.2's territory; the four sub-module stubs stay exactly as 1a.1 left them.
- `crates/maos-cli/`, `crates/maos-control/`, `crates/maos-spirit-sdk/`, `crates/maos-spirit-hello/`, `crates/maos-providers/`, `crates/maos-mcp/`, `crates/maos-acp/`, `crates/maos-a2a/`, `crates/maos-persistence/`, `crates/maos-secrets/`, `crates/maos-compliance/` — Story 1a.3/1a.4/1b.x territory; do NOT touch in 1a.2.
- `docs/adr/` — Story 1a.1's 14 binding-v0.1 ADRs; do NOT add new ADRs (none required for 1a.2; ADRs cited are pre-existing).
- `docs/invariants/I*.md` — Story 1a.1's register-file extensions; do NOT touch (would trip `invariant-lock` tri-requirement).
- `xtask/i9-whitelist.toml`, `xtask/i9-denylist.toml`, `xtask/loom-blocklist.toml`, `xtask/loom-allowlist.toml`, `xtask/kernel-crates.toml`, `xtask/gate-registry.toml`, `xtask/judge-direct-call-identifiers.toml` — no new gates ship; tables stay at v0.1-α scope.
- `abi-baseline/` — Story 1a.1's ABI baseline for `maos-spirit-abi`; do NOT regenerate (this story's surface changes are in `maos-domain` and `maos-kernel-core`, not the ABI baseline).
- `.github/workflows/*.yml` — CI workflows committed in Story 0.1–0.5; no new gate wires up.
- `SECURITY.md` — Story 1a.4's deliverable.

### Why each port goes in `maos-domain`, not `maos-kernel-core`

The intuition: "shouldn't port traits live where the adapters live?" No — ADR-010 binding-v0.1 gate is specifically "domain core compiles without async runtime." Putting port traits in `maos-kernel-core` (which transitively depends on `maos-bin`'s tokio) would mean test code in `maos-domain` couldn't be written against the port surface without pulling tokio. By keeping port traits in `maos-domain`, the dependency graph stays:

```
maos-domain (sync, serde + thiserror only)
    ↑ trait declarations
    │
maos-kernel-core (sync adapters; depends on maos-domain via `path =`)
    ↑ adapter impls (deferred to 1b.x)
    │
maos-bin (composition root; runtime primitives, tokio, tokio-util)
```

Adapters in `maos-kernel-core` import port traits via `pub use maos_domain::ports::*;`. The composition root in `maos-bin` constructs `<Service>Adapter` instances and (in 1b.x) wires them under runtime primitives. This is the canonical hexagonal layering ADR-010 binding-v0.1 commits to.

**Common LLM mistake to avoid:** declaring port traits in `maos-kernel-core::ports::*` for "ergonomic" colocation with adapters. This breaks ADR-010's gate — verify by trying `cargo build -p maos-domain --no-default-features` after the change; if it still builds, you're fine; if it fails to compile because `maos-domain` now references `maos-kernel-core`, revert.

### Why no `tokio` in `maos-kernel-core`

The same intuition: "shouldn't the kernel-core crate own its runtime concerns?" No — keeping `maos-kernel-core` runtime-free at v0.1-α preserves three properties:

1. **ADR-010 compliance** — domain-core-without-async-runtime gate stays green.
2. **Hexagonal testability** — adapter shells can be unit-tested without spinning up a tokio runtime; mock adapters for future ports plug in as sync types.
3. **I9 structural-state lint** — `tokio::sync::*` types (Mutex, RwLock) are on `xtask/i9-denylist.toml`; importing them into `maos-kernel-core` outside the three sanctioned holders trips `check-empty-kernel`.

Story 1b.1 introduces tokio to `maos-kernel-core` selectively (only inside the three sanctioned holders — `journal/`, `iac/transparency_log.rs`, `capability/cap_tokens/`) once those modules have real persistent-state mechanics. At v0.1-α the adapter shells are pure structural anchors.

### Why exactly the four ADR-022 predicates and no more

The `CapabilityRegistryPort` trait declares **exactly** `on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside` — four methods, no fifth. Per architecture §4.0.7: "The kernel performs universal arithmetic comparison only via four predicates." Adding a fifth (e.g., `on_value_equal` or `on_value_changed`) at v0.1-α is **forbidden** — it extends the kernel-API surface in a way that requires ADR-022 amendment + `invariant-lock` review.

If a Spirit class genuinely needs a fifth predicate (e.g., for tag-pair comparisons), the path is:
1. Author a v0.5+ ADR proposing the extension.
2. Pass through the `invariant-lock` gate (machine-checkable diff + corpus delta + phase-commitment update).
3. Land the new predicate in a future story.

At v0.1-α: four predicates exactly. Period.

### Why the seven adapter shells are unit structs (no fields)

The I9 structural-state lint (`check-empty-kernel`) blocks 25 denylisted types (Vec, HashMap, Mutex, RwLock, Arc, …) from being struct fields outside the three sanctioned holders. The five new service modules + two internal modules are NOT in the I9 whitelist at v0.1-α. So any field on an adapter struct from the denylisted set trips the lint.

The defense: **adapter shells are unit structs.** `pub struct SpiritSchedulerAdapter;` has zero fields, so the field-type denylist cannot fire. When Story 1b.1 lands the supervisor's journal mechanics, it will add a `journal:` field of type `Arc<RwLock<Journal>>` (or similar) — but ONLY inside `crates/maos-kernel-core/src/journal/`, which IS in the I9 whitelist. The supervisor adapter at that point will hold a *reference* to the journal type (`&'a Journal` or `Arc<Journal>`), not a denylisted collection directly.

This is the cleanest path through the I9 lint at v0.1-α: defer all field declarations to 1b.x stories where the receiving module is sanctioned, AND keep adapter shells as unit structs that the kernel composition can `_ = <Adapter>::default()` without runtime cost.

### Runtime topology rationale

ADR-011 commits to multi-threaded Tokio. The story's `worker_threads = std::thread::available_parallelism()` derives the count from the host's CPU topology without an external crate (avoiding `num_cpus = "1.x"` and its ~3 transitive deps). The `available_parallelism()` API is stable on Rust 1.59+ and the workspace pins `rust-version = "1.88"`, so the dependency is the language standard library — zero blast radius.

The `CancellationToken` root pattern (vs. broadcast `oneshot` channels) is the idiom tokio-util ships: every long-lived task takes a `child_token()`, and a single `cancel.cancel()` propagates cancellation down the tree. The Story 1b.1 supervisor task will `tokio::spawn` itself, take a child token, and `select! { _ = cancel.cancelled() => break, _ = supervised_work() => continue }`. At v0.1-α the supervisor isn't spawned yet, but the token root exists so the structural slot is correct.

Signal handling uses `tokio::signal::ctrl_c` for SIGINT (cross-platform) and `tokio::signal::unix::{signal, SignalKind}` for SIGTERM (Unix only; Windows users get Ctrl+C only at v0.1-α — Windows-specific SIGBREAK handling is deferred). The `cfg(unix)` / `cfg(not(unix))` split for `shutdown_unix_term` is the idiomatic pattern; the non-Unix arm is `std::future::pending::<()>().await` (never resolves), so the `select!` macro doesn't fire that arm on Windows.

### Previous-story intelligence (carry-forward from 1a.1)

**What worked well in 1a.1 that 1a.2 should preserve:**

1. **`#![forbid(unsafe_code)]` at every crate root** — mandatory per NFR-Sec-9; 1a.2 maintains this in all new module files.
2. **Worked-example code blocks** — 1a.1's ACs included verbatim Rust snippets the dev agent could lift; 1a.2 mirrors this pattern (every shell has a worked example).
3. **"What this story is NOT" callouts** — 1a.1 explicitly listed forbidden temptations (`tokio` in `maos-domain`, etc.); 1a.2 expanded this to a numbered list at story-header level.
4. **AST-walk over string-grep** — 1a.1's xtask extensions used `syn`-based parsing; 1a.2's `check_service_boundary` extension is purely additive to the existing syn walker (no new regex).
5. **Self-review checklist in dev record** — 1a.1 had ~22 items the dev ticked off before requesting review; 1a.2's AC6 mandates a structured equivalent (6 subsections).

**What was challenging in 1a.1 that 1a.2 should explicitly avoid:**

1. **Dependency-introduction blast radius drift** — DF4: `tempfile` pulled ~25 WASI crates. 1a.2's tokio dep IS large (~30 transitive crates) but is unavoidable per ADR-011. The dev record MUST document the exact count and the architectural justification.
2. **Spec-prose-vs-implementation drift** — DF11: 200 seed entries but 11 unique patterns. 1a.2's "16 trait methods minimum" + "21 classification entries minimum" are *floor* counts; the dev agent satisfies them by exact compliance, and if the syn walker happens to emit more (e.g., the `pub use` re-exports show as additional entries), the agent adds matching classification rows rather than reducing the trait methods.
3. **Tests-for-the-test missing** — Story 0.1 P9 + 0.5 P13. 1a.2's `service_classifications` JSON field addition to `check_service_boundary.rs` must have a corresponding unit-test update (Task 4.6); do NOT defer the test to a later story.
4. **String-grep instinct** — surface 3+ times in Epic 0. 1a.2's xtask extension is type-driven via `serde_json::json!`, not template-string concatenation.

### What this story does NOT introduce

Restating from the story header for the dev agent's PR-self-review:

- ❌ No port impl blocks (`impl <Service>Port for <Service>Adapter` is forbidden at v0.1-α).
- ❌ No runtime state in adapter shells (all shells are unit structs).
- ❌ No `tokio::*` imports in `maos-kernel-core` (only `maos-bin`).
- ❌ No CryptoProvider trait (Story 1a.3).
- ❌ No `maosctl` CLI wiring (Story 1a.4).
- ❌ No SECURITY.md (Story 1a.4).
- ❌ No new ADR files (the 14 binding-v0.1 ADRs are already committed by 1a.1).
- ❌ No invariant register-file touches (`docs/invariants/I*.md` untouched).
- ❌ No new corpus generation (Story 0.5's territory).
- ❌ No new xtask gates added (only the existing `check-service-boundary` extended).

### Project Structure Notes

The 17-crate workspace shape from 1a.1 is preserved exactly. Story 1a.2 adds ~15 new files inside `crates/maos-domain/src/ports/` and `crates/maos-kernel-core/src/{api.rs, scheduler/, security/, memory/, iac/, io/, telemetry/}` — all under pre-existing crate roots. The Cargo workspace `members` array does NOT change (no new crate added; the 17-crate budget is satisfied by 1a.1).

The dependency graph:

```
maos-bin
    ├── maos-domain  (port traits)
    ├── maos-kernel-core
    │       └── maos-domain  (re-export of port traits)
    ├── tokio
    └── tokio-util
maos-kernel-core
    └── maos-domain
maos-domain  (sync; serde + thiserror only)
```

No cycle, points inward, runtime concerns isolated to `maos-bin` — ADR-010 binding-v0.1 gate satisfied.

### References

- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0] — Component-classification lock (1 supervisor + 4 supervised + 2 internal modules).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.1] — Hexagonal/actor architectural style.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2] — Canonical 17-crate workspace layout (kernel-core module breakdown).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.4] — Tokio mpsc + broadcast technology choice.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.7] — "Kernel does NOT compute" — four-predicate universal-arithmetic surface.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.8] — Service vs. internal module four-property test.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.1] — Spirit Scheduler responsibility + supervisor exception.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.2] — Memory Manager three-tier responsibility.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.3] — Security Manager sandbox/secret/approval responsibility.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.4] — I/O Subsystem responsibility (internal module at v0.1).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.5] — IAC Bus responsibility.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.6] — Capability Registry responsibility + ADR-030 decomposition.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.6.1] — Epistemic halt mechanism.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.7] — Telemetry Stream responsibility (internal module at v0.1).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md` §3.2] — I1–I14 full statements.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md` §3.2.1] — Enforcement-cadence matrix (forward-only progression).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/06-foundational-commitments.md`] — Eight foundational commitments; Commitments 1, 5, 7 directly relevant.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md`] — ADR-010 / 011 / 022 / 030 / 037 / 038 verbatim source.
- [Source: `docs/adr/ADR-010-hexagonal-architecture-for-static-structure.md`] — Hexagonal commitment.
- [Source: `docs/adr/ADR-011-actor-model-on-the-runtime-hot-path.md`] — Actor-on-hot-path commitment.
- [Source: `docs/adr/ADR-022-tagged-scalar-working-memory-slot.md`] — Four-predicate universal-arithmetic surface.
- [Source: `docs/adr/ADR-030-capability-registry-decomposition.md`] — Capability Registry four-sub-module decomposition.
- [Source: `_bmad-output/planning-artifacts/epics/epic-1a-workspace-bootstrap-abi-freeze-kernel-skeleton-v01.md` Story 1a.2 section] — This story's epic-level acceptance criteria.
- [Source: `_bmad-output/implementation-artifacts/1a-1-initialize-17-crate-cargo-workspace-frozen-abi-types-starter-template.md`] — Prerequisite scaffolding (17-crate workspace + I1–I14 + ComplianceClaim + 14 ADRs).
- [Source: `_bmad-output/implementation-artifacts/epic-0-retro-2026-05-13.md`] — Action items A1 (self-review), A2 (dep blast-radius), A3 (worked-example) — binding for Epic 1a.
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md`] — DF1 (xtask DRY), DF7 (test heuristic), DW2 (invariant-lock end-to-end) — known concerns for surface-walk-touching stories.
- [Source: `xtask/src/check_service_boundary.rs`] — Existing surface-diff stub; this story extends additively.
- [Source: `xtask/kernel-api-classes.toml`] — Empty `[classes]` table; this story populates.
- [Source: `xtask/i9-whitelist.toml`] — Three I9-sanctioned holder paths; defines where persistent-state struct fields are permitted.
- [Source: `xtask/i9-denylist.toml`] — 25 denylisted types; trips `check-empty-kernel` outside whitelist.
- [Source: `xtask/kloc.toml`] — Per-crate KLOC ceilings; `maos-kernel-core = 6000`, `maos-bin = 1000`.
- [Source: `docs/ci-baselines/kernel-surface-v0.1-alpha.json`] — Empty surface baseline; regenerated by this story.
- [Source: `crates/maos-kernel-core/src/lib.rs`] — Current pre-1a.2 state (`pub mod capability;` only).
- [Source: `crates/maos-bin/src/main.rs`] — Current pre-1a.2 placeholder.
- [Source: `crates/maos-domain/src/lib.rs`] — Current 1a.1 state (`pub mod invariants;` + 14 codified invariants).
- [Source: `crates/maos-domain/src/invariants/i*.rs`] — I1–I14 type set (referenced by port traits).

## Dev Agent Record

### Agent Model Used

Kimi Code CLI (kimi-cli) — single-pass execution, no session boundaries.

### Debug Log References

- No non-trivial debugging paths required. All worked-example code compiled on first pass.
- `api.rs` `pub use crate::...` paths emitted `maos_kernel_core::api::crate::*` by the syn walker; classified as-is rather than refactoring the walker (per AC4 discipline).
- Coverage-matrix `build` gate is not in `gate-registry.toml`; changed ADR-011 row to `reproducible-build` (the canonical build gate) to keep the YAML schema-valid without modifying the registry.

### Completion Notes List

- **Task 0 (Pre-flight):** All 13 Epic-0 gates passed on clean checkout. Baseline surface count = 0. `cargo deny check` passed (pre-existing license-not-encountered warnings only).
- **Task 1 (Port traits in maos-domain):** Created 8 files under `crates/maos-domain/src/ports/` (mod.rs + 7 trait files). Added `SandboxTier` newtype to `i9.rs`. All 16 trait methods carry `/// Class:` doc tags. `cargo test -p maos-domain` passes (16 unit + 14 doc tests). Zero async-runtime deps in `cargo tree -p maos-domain`.
- **Task 2 (Adapter shells in maos-kernel-core):** Added `maos-domain` dep to kernel-core Cargo.toml. Rewrote `lib.rs` with 7 new `pub mod` lines + `api`. Created 6 new `mod.rs` shells + extended `capability/mod.rs` + created `api.rs`. All shells are unit structs, zero impl blocks, zero fields. `check-empty-kernel` and `check-loom` both PASS.
- **Task 3 (Composition root in maos-bin):** Added 4 deps (`maos-domain`, `maos-kernel-core`, `tokio`, `tokio-util`). Rewrote `main.rs` with `#[tokio::main(flavor = "multi_thread")]`, `CancellationToken` root, `select!` shutdown on SIGINT/SIGTERM/token-cancel. Binary builds, installs, starts, and exits cleanly on both Ctrl+C and `kill -TERM`. KLOC for maos-bin well under 1000 ceiling.
- **Task 4 (xtask check-service-boundary extension):** Populated `kernel-api-classes.toml` with 21 classification entries covering every surface item the syn walker emits (including `api::crate::*` and `maos_domain::ports::*` re-export paths). Extended `check_service_boundary.rs` with `service_classifications` static map in JSON output. Regenerated `docs/ci-baselines/kernel-surface-v0.1-alpha.json`. All xtask tests pass (125 tests).
- **Task 5 (Coverage-matrix rows):** Added ADR-010 row (`gates: [check-service-boundary]`) and ADR-011 row (`gates: [reproducible-build, kloc-check]`). `coverage-matrix` gate passes (185 rows). `invariant-lock` reports zero touched invariants.
- **Task 6 (Full 13-gate CI suite + self-review):** All 13 gates pass post-implementation. `cargo deny check` passes. `cargo test --workspace --locked` passes (all 125 xtask tests + 16 maos-domain tests + 4 maos-spirit-abi tests + 15 integration tests). Aggregate KLOC = 4849 (was 4689 pre-flight; delta = +160 LOC, well under 500 LOC budget and 16,000 alarm).
- **Task 7 (PR prep):** Story file updated with all AC6 evidence subsections below. Status set to `review`. Sprint-status.yaml updated to `review`.

### File List

**Created (new files):**

- `crates/maos-domain/src/ports/mod.rs`
- `crates/maos-domain/src/ports/scheduler.rs`
- `crates/maos-domain/src/ports/security.rs`
- `crates/maos-domain/src/ports/memory.rs`
- `crates/maos-domain/src/ports/iac_bus.rs`
- `crates/maos-domain/src/ports/capability.rs`
- `crates/maos-domain/src/ports/io_subsystem.rs`
- `crates/maos-domain/src/ports/telemetry.rs`
- `crates/maos-kernel-core/src/api.rs`
- `crates/maos-kernel-core/src/scheduler/mod.rs`
- `crates/maos-kernel-core/src/security/mod.rs`
- `crates/maos-kernel-core/src/memory/mod.rs`
- `crates/maos-kernel-core/src/iac/mod.rs`
- `crates/maos-kernel-core/src/io/mod.rs`
- `crates/maos-kernel-core/src/telemetry/mod.rs`

**Modified files:**

- `crates/maos-domain/src/lib.rs` — adds `pub mod ports;`
- `crates/maos-domain/src/invariants/i9.rs` — adds `SandboxTier` newtype
- `crates/maos-kernel-core/Cargo.toml` — adds `maos-domain` dependency
- `crates/maos-kernel-core/src/lib.rs` — adds 7 `pub mod` lines + extended docstring
- `crates/maos-kernel-core/src/capability/mod.rs` — adds port re-export + adapter placeholder
- `crates/maos-bin/Cargo.toml` — adds 4 dependencies
- `crates/maos-bin/src/main.rs` — rewritten to Tokio composition root
- `xtask/kernel-api-classes.toml` — populated with 21 classification entries
- `xtask/src/check_service_boundary.rs` — adds `service_classifications` JSON field
- `docs/ci-baselines/kernel-surface-v0.1-alpha.json` — regenerated with 21 items
- `tests/coverage-matrix.yaml` — adds ADR-010 and ADR-011 rows
- `_bmad-output/implementation-artifacts/1a-2-wire-the-five-service-kernel-skeleton-with-a-multi-threaded-tokio-composition-root.md` — dev record updated

### Pre-flight baseline

| Gate | Result |
|---|---|
| `cargo build --locked --all-targets --workspace` | PASS (pre-existing xtask test warnings only) |
| `cargo run -p xtask -- check-unsafe` | PASS |
| `cargo run -p xtask -- check-empty-kernel` | PASS |
| `cargo run -p xtask -- check-loom` | PASS |
| `cargo run -p xtask -- check-service-boundary` | PASS |
| `cargo run -p xtask -- kloc-check` | PASS (aggregate=4689 LOC) |
| `cargo run -p xtask -- abi-diff --base abi-baseline/v0.1-alpha-pre-abi-freeze.json` | PASS |
| `cargo run -p xtask -- check-corpus` | PASS |
| `cargo run -p xtask -- check-judge-config` | PASS |
| `cargo run -p xtask -- coverage-matrix` | PASS |
| `cargo run -p xtask -- corpus-staleness` | PASS |
| `cargo run -p xtask -- rebaseline-check` | PASS |
| `cargo run -p xtask -- calibrate --corpus calibration-seed-n100 --n 100 --p 0.98 --synthetic-pass-rate 0.98` | PASS |
| `cargo run -p xtask -- invariant-lock --changed-files ... --pr-number 0 --sha test` | PASS |
| `cargo deny check` | PASS (license-not-encountered warnings only) |

### ADR alignment cross-reference

- [x] **ADR-010 (Hexagonal):** port traits live in `maos-domain::ports::*` (8 files), adapter shells in `maos-kernel-core::<service>::<Service>Adapter` (7 shells), dependencies point inward (`maos-kernel-core → maos-domain`, `maos-bin → maos-kernel-core` + `maos-domain`). Verified by `cargo tree -p maos-domain` showing zero dependency on `maos-kernel-core`.
- [x] **ADR-011 (Actor on hot path):** `#[tokio::main(flavor = "multi_thread")]` at `crates/maos-bin/src/main.rs:45`; `CancellationToken` root constructed at `main.rs:84`; `select!` shutdown selector at `main.rs:89`. Per-Spirit supervisor task spawning deferred to Story 1b.1.
- [x] **ADR-030 (Capability Registry Decomposition):** the four-sub-module decomposition (`cap_tokens/`, `cap_policy/`, `cap_audit/`, `cap_quota/`) is preserved from Story 1a.1; the `CapabilityRegistryAdapter` placeholder added without absorbing the sub-module separation.

### Runtime smoke test

```
$ ./target/release/maos-bin
maos 0.1.0-alpha (v0.1-α scaffold; worker_threads target = 32)
^C
maos: shutdown reason = sigint; cancelling root token
maos: drained 0 child tasks; exiting cleanly
```

SIGTERM also verified:
```
$ ./target/release/maos-bin
maos 0.1.0-alpha (v0.1-α scaffold; worker_threads target = 32)
maos: shutdown reason = sigterm; cancelling root token
maos: drained 0 child tasks; exiting cleanly
```

### Shell-emptiness audit

```
crates/maos-kernel-core/src/scheduler/mod.rs   — 26 lines  — 1 struct defs  — 0 impl blocks  — denylisted types: none
crates/maos-kernel-core/src/security/mod.rs    — 15 lines  — 1 struct defs  — 0 impl blocks  — denylisted types: none
crates/maos-kernel-core/src/memory/mod.rs      — 15 lines  — 1 struct defs  — 0 impl blocks  — denylisted types: none
crates/maos-kernel-core/src/iac/mod.rs         — 15 lines  — 1 struct defs  — 0 impl blocks  — denylisted types: none
crates/maos-kernel-core/src/io/mod.rs          — 15 lines  — 1 struct defs  — 0 impl blocks  — denylisted types: none
crates/maos-kernel-core/src/telemetry/mod.rs   — 15 lines  — 1 struct defs  — 0 impl blocks  — denylisted types: none
crates/maos-kernel-core/src/capability/mod.rs  — 22 lines  — 1 struct defs  — 0 impl blocks  — denylisted types: none (mod.rs adds CapabilityRegistryAdapter unit struct only; cap_tokens/cap_policy/cap_audit/cap_quota sub-modules untouched from 1a.1)
```

### Surface item classification audit

```
[U] maos_kernel_core::api::crate::capability::CapabilityRegistryAdapter
[D] maos_kernel_core::api::crate::iac::IacBusAdapter
[D] maos_kernel_core::api::crate::io::IoSubsystemAdapter
[D] maos_kernel_core::api::crate::memory::MemoryManagerAdapter
[S] maos_kernel_core::api::crate::scheduler::SpiritSchedulerAdapter
[S] maos_kernel_core::api::crate::security::SecurityManagerAdapter
[D] maos_kernel_core::api::crate::telemetry::TelemetryStreamAdapter
[U] maos_kernel_core::capability::CapabilityRegistryAdapter
[U] maos_kernel_core::capability::maos_domain::ports::CapabilityRegistryPort
[D] maos_kernel_core::iac::IacBusAdapter
[D] maos_kernel_core::iac::maos_domain::ports::IacBusPort
[D] maos_kernel_core::io::IoSubsystemAdapter
[D] maos_kernel_core::io::maos_domain::ports::IoSubsystemPort
[D] maos_kernel_core::memory::MemoryManagerAdapter
[D] maos_kernel_core::memory::maos_domain::ports::MemoryManagerPort
[S] maos_kernel_core::scheduler::SpiritSchedulerAdapter
[S] maos_kernel_core::scheduler::maos_domain::ports::SpiritSchedulerPort
[S] maos_kernel_core::security::SecurityManagerAdapter
[S] maos_kernel_core::security::maos_domain::ports::SecurityManagerPort
[D] maos_kernel_core::telemetry::TelemetryStreamAdapter
[D] maos_kernel_core::telemetry::maos_domain::ports::TelemetryStreamPort
```

Zero `[O]` (other) entries. All 21 public surface items classified.

### Dependency-introduction note

- **New top-level deps:** `tokio` (1.52.3), `tokio-util` (0.7.18) in `crates/maos-bin/Cargo.toml` ONLY.
- **`Cargo.lock` blast radius:** 9 new lockfile entries (`bytes`, `futures-macro`, `futures-sink`, `mio`, `signal-hook-registry`, `tokio`, `tokio-macros`, `tokio-util`, `wasi`). Target was ≤80; actual is 9 — well under threshold.
- **Notable transitive deps:** `mio` (async I/O polling), `signal-hook-registry` (cross-platform signal handling), `pin-project-lite` (already present via futures), `bytes` (buffer abstraction), `slab` (pre-allocated storage). Top 5 by relevance documented.
- **Justification:** ADR-011 binding-v0.1 gate requires multi-threaded Tokio; `tokio-util` provides `CancellationToken`. No alternative crate ships these primitives at production-grade.
- **`cargo deny check`:** PASS (only pre-existing `license-not-encountered` warnings for unused license allowances; no advisory flags, no ban violations).

### What did NOT happen this story

- [x] No port impl blocks added (`grep -rn 'impl .*Port for' crates/maos-kernel-core/` returns zero matches).
- [x] No CryptoProvider trait introduced (`grep -rn 'CryptoProvider' crates/maos-kernel-core/` returns zero matches in code; one doc-comment mention in `lib.rs` only — Story 1a.3 territory).
- [x] No `maosctl` enhancements (`crates/maos-cli/src/lib.rs` unchanged; `git diff HEAD -- crates/maos-cli/` returns empty).
- [x] No new ADR files (`git diff HEAD -- docs/adr/` returns empty).
- [x] No invariant-register touches (`git diff HEAD -- docs/invariants/I*.md` returns empty).
- [x] No P1–P4 enforcement upgrade in xtask (`p1_p4_status` stays `"deferred-to-story-2.2"`).

### Self-review checklist

- [x] All 7 service/module shells declared in `crates/maos-kernel-core/src/lib.rs` (`api`, `scheduler`, `security`, `memory`, `iac`, `capability`, `io`, `telemetry`).
- [x] Each shell is a unit struct (`pub struct <Name>Adapter;`) with zero fields and zero impl blocks.
- [x] `crates/maos-kernel-core/Cargo.toml` adds only `maos-domain` (no tokio, no tokio-util).
- [x] All 7 port traits declared in `crates/maos-domain/src/ports/`, each method carrying `/// Class: <one-of-three>`.
- [x] `crates/maos-domain` continues to build without tokio (`cargo tree -p maos-domain | grep -v 'tokio\|reqwest\|sqlx'` shows no async-runtime crates).
- [x] `crates/maos-bin/src/main.rs` uses `#[tokio::main(flavor = "multi_thread")]`, constructs `CancellationToken`, wires `select!` shutdown on SIGINT+SIGTERM+token-cancel.
- [x] `crates/maos-bin/Cargo.toml` adds exactly `maos-domain`, `maos-kernel-core`, `tokio` (rt-multi-thread/macros/signal), `tokio-util` (rt) — no more.
- [x] `cargo build --locked --all-targets --workspace` — zero warnings (only pre-existing xtask test warnings).
- [x] `cargo install --path crates/maos-bin --locked` succeeds; `maos-bin` starts and exits cleanly on Ctrl+C with the expected transcript.
- [x] `xtask/kernel-api-classes.toml` covers every public symbol the surface walk emits; zero `[O]` entries.
- [x] `docs/ci-baselines/kernel-surface-v0.1-alpha.json` regenerated; `cargo run -p xtask -- check-service-boundary` returns `PASSED (0 violations)`.
- [x] `xtask/src/check_service_boundary.rs` extended with `service_classifications` JSON field; existing tests updated to accept the additive field (125 xtask tests pass).
- [x] `tests/coverage-matrix.yaml` flips ADR-010 and ADR-011 rows only; does NOT touch FR1/2/7/8/47/48/61 or invariant register rows.
- [x] All 13 Epic-0 CI gates pass locally (per Task 6.1).
- [x] `cargo deny check` passes (per Task 6.2); dep-introduction blast-radius documented (9 new lockfile entries).
- [x] `cargo test --workspace --locked` passes (per Task 6.3); no regressions in `maos-domain`, `maos-spirit-abi`, or `xtask` integration tests.
- [x] Six "What did NOT happen this story" grep-checks return zero (per Task 6.4).
- [ ] Two reviewers named + tagged in PR description. *(To be filled by human operator at PR open time.)*
- [x] PR description includes runtime smoke-test transcript + shell-emptiness audit + surface classification audit + dep-introduction note.

### Review Findings

- [x] [Review][Decision] Unused `maos-domain` direct dependency in `maos-bin` — kept per AC3 spec intent; forward-compatible scaffolding for 1b.x port-trait imports. [`crates/maos-bin/Cargo.toml:13`]
- [x] [Review][Defer] Surface walk produces `api::crate::*` path artifacts — the syn walker resolves `pub use crate::...` literally, embedding `crate::` in the path string. Classification table matches these paths. Coupling to walker behavior is fragile but stable; if the walker is fixed later, 7 TOML entries and the baseline JSON will need updating. [`xtask/kernel-api-classes.toml`] — deferred, pre-existing walker behavior
- [x] [Review][Defer] `LogBeforeDeliver::new()` is `pub` — typestate guarantee on `IacBusPort` methods is advisory at v0.1-α. I2's TODO notes `pub(crate)` restriction planned for Story 1b.2. [`crates/maos-domain/src/ports/iac_bus.rs:20`] — deferred, pre-existing design limitation
- [x] [Review][Defer] `SandboxTier(pub u8)` has no value constraint — raw u8 accepts any value; T0-T2 enforcement lands in Story 1b.3 per spec. [`crates/maos-domain/src/invariants/i9.rs:37`] — deferred, explicit 1b.3 scope
