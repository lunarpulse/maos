# Epic 2: Spirit ABI + Developer SDK + Boundary Contracts (v0.1 → v0.3)

**Goal:** A Spirit author at 9pm Tuesday clones a template, implements `on_idle`, runs `spirit-test` harness locally, and ships a binary Spirit without ever touching kernel internals. NFR-Onb-1 v0.3 gate prerequisites land here.

**Owns:**
- Full Spirit ABI contract crate (`maos-spirit-abi` extended with full vtable + lifecycle hook signatures).
- `maos-spirit-sdk` with `#[spirit]` proc-macro and Spirit-author helpers.
- **`cargo xtask check-service-boundary` P1–P4 FULL implementation** (boundary enforcer against real Spirit ABI types — resolves circular dependency from E1a stub).
- Thin `cargo generate maos-spirit` template (Rust only at v0.5; per-language at E7) — **enough for NFR-Onb-1 v0.3 gate**.
- `spirit-test` SDK seed: local runner without kernel + manifest self-check + class-specific regression corpus skeleton.
- Spirit ABI lifecycle hook signatures: `on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate`.
- `output_shape` declaration skeleton (full fail-loud `output_shape_version` mismatch in E7).
- Spirit boundary contract test cases (~20 cases asserting FR17 + FR58 Spirit-side boundary).
- NFR-Sec-14 framework hooks (cross-Spirit memory isolation test scaffolding; corpus 200 authored in E4).
- NFR-Test-6 LCAS framework + **clearly-decidable bucket** (70 of 210 items authored at v0.3).

**FRs covered:** FR33 (thin cargo-generate slice — full per-language in E7), FR34 (spirit-test SDK seed — full SDK with assertion macros in E7), FR40 (output_shape_version skeleton — full fail-loud in E7), FR55 (lifecycle hook ABI signatures — runtime firing in E5), Spirit-side of FR17 (Spirit's manifest capability + halt declaration).

**Key NFRs:** **NFR-Onb-1 prerequisites** (cargo-generate template + local runner + ≥1 example Spirit with passing CI — full gate execution at E7 against Butler in E8), NFR-Test-3 SDK coverage ≥80% (validated by external-author trial in 5+ third-party Spirits — full at E7).

**Corpora authored in E2:**
- Spirit boundary contract cases ~20 (FR17 + FR58 boundary assertions).
- LCAS clearly-decidable bucket 70 items.

**Acceptance demo:** External developer clones `spirit-template`, implements `on_idle`, runs `cargo test` (which invokes spirit-test SDK harness), gets passing report — **without** reading kernel internals.

### Stories

## Story 2.1: Ship the Full Spirit ABI with `#[spirit]` Proc-Macro and 11 Lifecycle Hooks

As a Spirit author,
I want the full Spirit ABI contract crate with a `#[spirit]` proc-macro that derives the Spirit boilerplate plus all 11 lifecycle hook signatures (`on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate`),
So that I can implement a Spirit by writing only the hooks I need without re-deriving the trait machinery for every Spirit.

**Acceptance Criteria:**

**Given** the `maos-spirit-sdk` crate
**When** a Spirit author writes `#[spirit] impl MySpirit { fn on_idle(&self, ctx: &mut Ctx) {...} }`
**Then** the proc-macro derives the Spirit trait implementation, registers manifest entries, and wires the ABI vtable
**And** the resulting binary is `#[no_std]`-compatible at the ABI boundary

**Given** the 11 lifecycle hook signatures
**When** the Spirit ABI is exported
**Then** every hook is declared in `maos-spirit-abi` with a stable signature carrying a `CancellationToken` for cancellation discipline
**And** each hook declares the resource budget envelope per manifest `[budget]`
**And** the kernel calls only hooks the Spirit has declared in its manifest

**Given** the `output_shape` declaration skeleton
**When** a Spirit declares `[output_shape]` in its manifest
**Then** the kernel parses the declaration into a shape predicate (full fail-loud enforcement in E7)
**And** the parser rejects malformed shape declarations at admission

**Given** Spirit-side capability declarations (FR17 Spirit half)
**When** a Spirit declares `[capabilities.required]` in its manifest
**Then** the kernel enforces these as the issued capability scope at admission
**And** mismatches between declared and observed capabilities surface as drift events (full drift detection in E9)

## Story 2.2: `xtask check-service-boundary` P1–P4 Full Implementation + Spirit-Boundary Invariant Cases

As an architectural-discipline maintainer,
I want the full P1–P4 four-property test enforced against real Spirit ABI types (resolving E1a's stub) plus ~20 spirit-boundary invariant test cases,
So that the kernel-API surface invariant (NFR-Test-2) is mechanically enforced from v0.3 onward and any new kernel function landing outside the permitted computational classes is a build-break.

**Acceptance Criteria:**

**Given** the full `cargo xtask check-service-boundary` against real Spirit ABI types
**When** the xtask runs on every PR
**Then** P1 (single supervising owner per service) is enforced via supervision-tree static analysis
**And** P2 (ports not adapters at service boundary) is enforced via trait-direction lint
**And** P3 (state ownership behind `Arc<DashMap>`/`RwLock`/atomic) is enforced via type analysis
**And** P4 (audit-chain integrity at service boundary) is enforced via call-graph reachability — every external call reaches Capability Registry before exit

**Given** build-time reflection over `kernel::api::*` (Rust `syn` static analyzer)
**When** the analyzer classifies a function
**Then** the classification is decidable for the permitted subset (allowlist-based predicate definitions; no theorem prover)
**And** functions falling outside `{universal-arithmetic, data-movement, supervision}` are build-break

**Given** the spirit-boundary invariant test cases
**When** the test suite runs
**Then** ≥20 cases exercise the FR17/FR58 boundary (Spirit-side capability declaration, ComplianceClaim emit, output_shape conformance)
**And** the cases are registered in `coverage-matrix.yaml` per Story 0.3

## Story 2.3: Thin `cargo-generate` Template + Local Runner (NFR-Onb-1 v0.3 Prerequisite)

As a Spirit author working in a 9pm-Tuesday window,
I want a thin `cargo generate maos-spirit` Rust template that produces a compilable Spirit + a local runner that invokes lifecycle hooks without a kernel instance,
So that I can build and test a Spirit on my laptop within 30 minutes without learning kernel internals — meeting the v0.3 NFR-Onb-1 gate prerequisites.

**Acceptance Criteria:**

**Given** an installed `cargo-generate` tool
**When** the author runs `cargo generate maos-spirit --name my-spirit`
**Then** the template scaffolds a `my-spirit` crate with a working `on_idle` hook, a TOML manifest, and a passing `cargo test`
**And** the scaffold uses the `#[spirit]` proc-macro from Story 2.1
**And** the README documents the 30-minute first-Spirit path

**Given** the local runner shipped in `maos-spirit-sdk`
**When** the author runs `cargo test` against their Spirit
**Then** the runner invokes lifecycle hooks via the ABI without spinning up a real kernel
**And** the runner emits IAC frames into a mock bus that the test asserts against

**Given** the v0.3 NFR-Onb-1 gate prerequisites
**When** the gate runs (E7 Story 7.5 owns execution)
**Then** cargo-generate template + local runner + ≥1 example Spirit with passing CI are all present
**And** the Butler reference Spirit (E8 Story 8.1) uses this exact template

**Given** the `30-Min First Spirit` recruitment criteria
**When** a participant clones the template and follows the README
**Then** they reach a passing `cargo test` within median ≤45 min, p95 ≤90 min (per NFR-Onb-1)

## Story 2.4: Seed the spirit-test SDK with LCAS Framework and Cross-Spirit Isolation Hooks

As a test architect,
I want the spirit-test SDK seed (lifecycle hooks + IAC frame I/O + halt resolution + manifest self-check + class-specific regression-corpus skeleton) AND the LCAS (Long-context Ambiguity Stress) framework + clearly-decidable bucket (70 of 210 items) AND the cross-Spirit memory isolation framework hooks,
So that Story 4.5's 200-corpus authoring (NFR-Sec-14) and Story 8.x reference-Spirit acceptance tests have a working harness from v0.3 — not retrofitted at v1.0.

**Acceptance Criteria:**

**Given** the spirit-test SDK seed
**When** a Spirit author calls `spirit_test::run(&my_spirit, &fixture)`
**Then** the harness invokes every declared lifecycle hook with the fixture
**And** the harness verifies IAC frame I/O against the fixture's expected frames
**And** the harness exercises halt resolution under all three resolution kinds
**And** the harness runs the manifest self-check (well-formed/malformed/edge-case per NFR-Test-13)

**Given** the LCAS framework
**When** corpus authoring begins
**Then** the 70-item clearly-decidable bucket is committed to `tests/corpora/lcas-v0.3.jsonl`
**And** the remaining 140 items (genuinely-ambiguous + adversarially-misleading) are explicitly deferred to E2 + E7/E8 (require A2A scenarios from E6 to be valid)
**And** each item carries gold labels for halt-recall/precision measurement

**Given** the NFR-Sec-14 cross-Spirit memory isolation framework hooks
**When** a future test (E4 Story 4.5) attempts an adversarial cross-Spirit read
**Then** the framework provides hook points to inject Spirit-A's attempt and observe Spirit-B's state
**And** the framework is registered in `coverage-matrix.yaml` with a `valid_until` date

**Given** all of the above
**When** the SDK seed is published
**Then** external authors can extend the harness for their own Spirit classes
**And** the SDK seed counts toward NFR-Test-3's ≥80% coverage floor (full validation at E7)

---
