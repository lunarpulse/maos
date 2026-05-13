# Epic 1a: Workspace Bootstrap + ABI Freeze + Kernel Skeleton (v0.1-α)

**Goal:** `cargo new` the canonical 17-crate Cargo workspace per `architecture-maos-minimal-opus.md` §4.0.2. Land 14 binding-v0.1 ADRs simultaneously. Wire kernel-core skeleton as empty shells with frozen ABI types. Story 1.1 carries the **starter-template flag**.

**Owns:**
- 17-crate Cargo workspace scaffold (`maos-domain`, `maos-spirit-abi`, `maos-kernel-core/*`, `maos-spirit-sdk`, `maos-spirit-hello`, `maos-providers`, `maos-mcp`, `maos-acp`, `maos-a2a`, `maos-persistence`, `maos-secrets`, `maos-compliance`, `maos-control`, `maos-cli`, `maos-bin`, `spirits/`, `schemas/`, `fuzz/`, `wit/spirit.wit`).
- `maos-domain` codifies invariants I1–I14 (zero deps; no tokio/reqwest/sqlx; `serde + thiserror`).
- `maos-spirit-abi` frozen with `src/compliance.rs` (ComplianceClaim schema types) — `#![no_std]`, wire-stable.
- `maos-kernel-core` skeleton: five services (scheduler / memory / security / iac / capability) + two internal modules (io / telemetry) as empty shells with hexagonal port boundaries declared (ADR-010).
- `maos-bin` composition root with `#[tokio::main(flavor = "multi_thread")]` (ADR-011: single multi-threaded Tokio runtime).
- `maosctl` skeleton — six v0.1 subcommand stubs (`install`, `start`, `stop`, `unload`, `run`, `audit`). `audit` is forward scaffolding for Story 1b.5b's `audit query`; `run` is forward scaffolding for Stories 1b.5a + 1b.5c. Every stub at v0.1-α emits a deterministic "not yet implemented" diagnostic and exits with code 2. See Story 1a.4 AC1 for the binding declaration.
- SECURITY.md (`security@maos.dev` GPG key, 90-day embargo, advisory-publication channel, supported-versions matrix — NFR-Ops-4).
- `cargo xtask check-service-boundary` STUB (boundary types defined; full P1–P4 enforcement in E2 once Spirit ABI exists).
- `CryptoProvider` trait definition + default `ring`/`rustls` implementation (FR48 architectural commitment).

**ADRs binding simultaneously (14 binding-v0.1):** ADR-001 (Rust+Tokio), ADR-002 (subprocess form at v0.1; rust-inproc gated on §13.1), ADR-004 (sandbox tier ladder declared), ADR-006 (empty-kernel I9 — enforced by E0), ADR-010 (hexagonal architecture), ADR-011 (actor model on hot path), ADR-012 (typed-intent A2A consent — types only; runtime in E6), ADR-014 (storage/journal foundation), ADR-022 (epistemic halt skeleton — types only; mechanism in E4), ADR-023 (capability-token TTL ≤60s + PID-binding — types only; runtime in E1b), ADR-026 (principal namespace types — runtime in E4), ADR-030 (capability registry decomposition — types only), ADR-032 (subprocess wire protocol LSP-style + CBOR — types only), ADR-037 (invariant-lock CI gate — enforced by E0).

**FRs covered:** FR1 (basic source install path `cargo install --path crates/maos-bin`), FR2 (basic uninstall stub), FR7 (telemetry opt-in declared default), FR8 (manifest schema frozen; signed + journaled at runtime), FR47 (Inference Port type skeleton), FR48 (CryptoProvider trait + default), FR61 (SECURITY.md).

**Key NFRs:** NFR-Sec-9 (zero-`unsafe` in capability path), NFR-Maint-2 v0.1 floor (capability-registry fuzz ≥60% line), NFR-Tenancy-1 (single-tenant declared).

**KLOC budget:** ~2–3 KLOC. Alarm if this exceeds 4 — means logic smuggled in.

**Acceptance demo:** `cargo build --locked` produces signed `maos-bin` binary; `cargo xtask check-service-boundary` passes (stub mode); SECURITY.md renders; `maosctl --version` runs.

### Stories

## Story 1a.1: Initialize 17-Crate Cargo Workspace + Frozen ABI Types (Starter Template)

As a founding MAOS contributor,
I want the canonical 17-crate Cargo workspace per `architecture-maos-minimal-opus.md` §4.0.2 scaffolded with `maos-domain` invariants I1–I14 codified and `maos-spirit-abi` frozen with the ComplianceClaim schema types,
So that all subsequent epics build against a stable, ADR-bound workspace shape from day one. **This story carries the starter-template flag.**

**Acceptance Criteria:**

**Given** an empty repository
**When** the workspace bootstrap story is executed
**Then** the repository contains the exact crate layout from §4.0.2 (17 crates under `crates/`, plus `spirits/`, `schemas/`, `docs/`, `fuzz/`, `wit/spirit.wit`)
**And** `cargo build --locked` succeeds on Rust stable for the empty workspace

**Given** the `maos-domain` crate
**When** the crate is compiled
**Then** the crate has zero async dependencies (no tokio/reqwest/sqlx; only `serde + thiserror`)
**And** invariants I1 through I14 are codified as types with doctested invariant statements
**And** the crate compiles without a Tokio runtime present

**Given** the `maos-spirit-abi` crate
**When** the crate is compiled
**Then** the crate is `#![no_std]`
**And** the crate contains `src/compliance.rs` with the frozen ComplianceClaim schema types
**And** the crate contains the wire-stable Spirit ABI types

**Given** the 14 binding-v0.1 ADRs (ADR-001, 002, 004, 006, 010, 011, 012, 014, 022, 023, 026, 030, 032, 037)
**When** the workspace bootstrap completes
**Then** each ADR is committed to `docs/adr/` with status `accepted`
**And** the ADR identifiers are journaled in `docs/adr/index.md`

**Given** the workspace
**When** an external author runs `git clone` and `cargo build --locked`
**Then** the starter-template flag is satisfied: the build reproduces the v0.1-α baseline without bespoke setup

## Story 1a.2: Wire the Five-Service Kernel Skeleton with a Multi-Threaded Tokio Composition Root

As a kernel implementer,
I want the five supervised kernel services (Spirit Scheduler / Security Manager / Memory Manager / IAC Bus / Capability Registry) and two internal modules (I/O / Telemetry) wired as empty hexagonal shells with their port/adapter boundaries declared, AND the `maos-bin` composition root driving a single multi-threaded Tokio runtime,
So that all subsequent feature epics have a ready socket to plug runtime logic into without re-litigating service boundaries.

**Acceptance Criteria:**

**Given** the `maos-kernel-core` crate
**When** the crate is compiled
**Then** the crate exports five service modules (`scheduler/`, `memory/`, `security/`, `iac/`, `capability/`) and two internal modules (`io/`, `telemetry/`)
**And** the `capability/` module is decomposed per ADR-030 into `cap-tokens/`, `cap-policy/`, `cap-audit/`, `cap-quota/` subdirectories with empty type shells
**And** each service has its hexagonal port trait declared in `maos-domain` and adapter implementations stubbed in `maos-kernel-core/<service>/`

**Given** the `maos-bin` composition root
**When** the binary is compiled
**Then** `main.rs` uses `#[tokio::main(flavor = "multi_thread")]` per ADR-011
**And** the worker count is configured to the number of CPU cores
**And** every long-lived coordination task takes a `CancellationToken` (from `tokio-util`)
**And** root-level shutdown cancels all child tasks via `select!` with cancellation arm

**Given** the kernel-core skeleton
**When** `cargo xtask check-service-boundary` runs in stub mode
**Then** the xtask passes with all five services classified by computational class (universal-arithmetic / data-movement / supervision)
**And** no service exposes methods in the `other` class

**Given** the hexagonal architecture (ADR-010)
**When** the crate-boundary lint runs
**Then** `maos-domain` does not import any I/O adapter
**And** services depend only on their port traits, never on adapter implementations
**And** the lint hard-fails on any port→adapter direct reference

## Story 1a.3: CryptoProvider Trait + xtask Service-Boundary Stub Implementation

As a kernel security architect,
I want the `CryptoProvider` trait plumbed end-to-end as the indirection point for signature verification, sealed-export encryption, and capability-token signing (with a default `ring`/`rustls` implementation) AND the `cargo xtask check-service-boundary` P1–P4 four-property test stub committed,
So that FIPS-validated, hardware-backed, and post-quantum crypto can be substituted in later phases without recompiling Spirits, and the kernel-API surface invariant has a stub enforcer from day one.

**Acceptance Criteria:**

**Given** the `CryptoProvider` trait in `maos-kernel-core/security/crypto.rs`
**When** the trait is compiled
**Then** the trait declares operations for signature verification, sealed-export encryption, and capability-token signing
**And** the trait is implemented by the default `ring`/`rustls` adapter
**And** all kernel call sites for cryptographic operations route through the trait, never the default adapter directly

**Given** the FR48 architectural commitment
**When** a v1.0+ alternate provider (FIPS-validated / HSM-backed / post-quantum) is plugged in
**Then** the swap is a composition-root-level change in `maos-bin/main.rs`
**And** no Spirit binary requires recompilation (verified by ABI-diff lint)

**Given** the `cargo xtask check-service-boundary` P1–P4 stub
**When** the xtask runs against the empty kernel-core skeleton
**Then** P1 (service has a single supervising owner) passes for all five services
**And** P2 (service exposes ports, not adapters) passes for all five services
**And** P3 (service is stateless or owns its state behind `Arc<DashMap>`/`RwLock`) passes for all five services
**And** P4 (audit-chain integrity at service boundary) is stubbed pending E2's full ABI types
**And** the stub clearly reports which properties are stubbed vs enforced

## Story 1a.4: Ship the maosctl CLI Scaffold with SECURITY.md and Accessibility Defaults

As an evaluator,
I want a `maosctl` CLI scaffold with v0.1 subcommands stubbed (`install`, `start`, `stop`, `unload`, `run`, `audit`) plus accessibility flags (`--plain`, honors `NO_COLOR` and `TERM=dumb`) AND a complete `SECURITY.md` shipped before any external Spirit can run,
So that the operator surface and security disclosure pipeline exist on day one — not after the first vulnerability report.

**Acceptance Criteria:**

**Given** the `maos-cli` crate compiled to `maosctl`
**When** `maosctl --help` runs
**Then** the help output lists the v0.1 subcommands (`install`, `start`, `stop`, `unload`, `run`, `audit`)
**And** the help respects `NO_COLOR` and `TERM=dumb` environment variables (NFR-Ops-5)
**And** the `--plain` flag suppresses all ANSI color sequences

**Given** the SECURITY.md file at the repo root
**When** the file is read
**Then** the file documents `security@maos.dev` as the disclosure contact with a published GPG key
**And** the file documents the 90-day coordinated-disclosure embargo window (NFR-Ops-4)
**And** the file documents the supported-versions matrix for security backports
**And** the file documents the advisory-publication channel

**Given** the v0.1 ship gate
**When** the SECURITY.md presence check runs
**Then** the gate passes only if `SECURITY.md` exists, parses, and includes all four required sections (disclosure address, embargo, supported-versions, advisory channel)
**And** the gate is part of E0's continuous CI

**Given** a fresh OS install (Linux or macOS)
**When** the user runs `cargo install --path crates/maos-bin`
**Then** the install succeeds without nightly features (FR1 v0.1 source-build slice)
**And** `maosctl --version` reports the workspace version from `Cargo.toml`

---
