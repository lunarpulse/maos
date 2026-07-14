# Story 11.1a: WASM Component-Model Spirit Form — Host + WIT

**Status:** COMPLETE (code review + trace-coverage audit both PASS; 71 tests green, all gates green, CI behavioral coverage wired)
**Model:** claude-opus-4-8 (MANDATORY tier — Decision 11) + PRE-BOOKED §A6 multi-layer review incl Test-Infra + runtime-execution check.
**Kernel-Δ:** target **0** (HARD; baseline 22964, `kernel-core-baseline.toml:175`). No FLAG-Winston re-pin expected.
**Depends:** 11.0 (spike, DONE) · ADR-031 supersession of ADR-002 (single-form clause) + ADR-040 (defer).
**Dev-gate:** Epic-11 dev/merge GATED on the 2 external v1.5 holds (real external pen-test zero-P0/P1 + export-compliance counsel 5D002.c.1). Story-level planning/preflight is NOT gated. **11.1a may be built and proven green on a branch during the hold; it MUST NOT merge to the shippable line, and its distributable form MUST NOT be finalized, before export counsel clears.** (Preflight ratified via party-mode `wyksr4yce` round-2 + code-check 2026-06-30.)

---

## Story

**As** the MAOS daemon,
**I want** to launch a Spirit authored as a portable WASM component as a T2-sandboxed subprocess that speaks ADR-032 over a versioned `maos:spirit@1.0` WIT contract,
**So that** third parties can ship Spirits in a portable, capability-confined form **without touching the frozen ABI or the kernel**.

Explicitly NOT in 11.1a (defended at preflight): cross-form *behavioral equivalence* (→ 11.1b); distributable packaging + 5D002.c.1 classification stamp (→ post-counsel story); author SDK / third-party ergonomics (→ v2.5); multi-Spirit scale + latency parity (→ 11.1b / later).

---

## ⚠️ PREFLIGHT FLAGS (read first — these gate the dev-story)

### F1 — `SpiritHostPort` lives DAEMON-SIDE in a new `maos-host` crate — NOT `maos-domain`. *(supersedes the 11.0 spike recommendation)*
The 11.0 spike *recommended* `maos-domain` ("mirrors `CollectiveMemoryPort`"). Party-mode round-2 + a code check **overrode** that. Code evidence (2026-06-30):
- The program-bearing spawn input is **`BridgeSpawnSpec`** (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:240`), consumed by `spawn_and_bridge` at `runtime.rs:461` (`Command::new(&spec.program)`).
- Its **only production constructor is the daemon composition root** — `crates/maos-bin/src/main.rs:492` (`program: resolved`). kernel-core constructs `BridgeSpawnSpec` only in tests. **The kernel does NOT resolve `program` from a manifest; the daemon does.**
- `SandboxSpec` (`security/sandbox/mod.rs:32`) carries `tier`/caps/scopes — **no `program`, no form field** (spike claim re-confirmed).
- ⇒ Winston's deciding test ("is there a kernel-core caller of the port?") = **NO**. `resolve_launch` is invoked daemon-side, exactly where `resolved` is computed (`main.rs:492`). A trait the kernel never calls must NOT be pinned into the frozen ABI surface (every `maos-domain` symbol is maintained forever and raises the ABI gate's false-positive rate — the kind of gate Epic 8 disabled to reach green).
- **Decision (RANK 2, ratified):** trait + its plan structs (`SpiritForm{NativeSubprocess,WasmComponent}`, `SpiritLaunchRequest`, `SpiritLaunchPlan`) land **together** in a new `crates/maos-host` wiring crate, `pub`. The wasmtime adapter is a **separate** crate `maos-wasm-host` (ADR-041 / `maos-loom-lite` isolation precedent). `maos-bin` depends on both; `maos-host` depends on `maos-domain`/`maos-kernel-core` for the types it references. **Do NOT split the plan structs into `maos-domain`** — that re-touches the frozen surface and forfeits the empty-diff win for nothing.

### F2 — The "empty `cargo public-api --diff`" claim is the WRONG gate. A NEW watcher is mandatory.
Code check: `xtask/src/abi_diff.rs:8` → `const MANIFEST = "crates/maos-spirit-abi/Cargo.toml"`. **abi-diff scans `maos-spirit-abi` ONLY** — never `maos-domain`, never `maos-kernel-core`, never a future `maos-host`. So an "empty abi-diff" is *vacuous-green for `SpiritHostPort` in every candidate crate*. The gates that exist today:
- `abi-diff` → `maos-spirit-abi` public API. (Blind to this trait everywhere.)
- `check-service-boundary` / kernel-surface (`docs/ci-baselines/kernel-surface-v0.1-beta.json`) → `maos-kernel-core` public surface **+ the maos-domain ports kernel-core re-exports**. (Would catch the trait only if kernel-core re-exported it — it won't, per F1.)
- `check-kernel-baseline` → **line count of `crates/maos-kernel-core/src` vs 22964.** This is the gate that genuinely proves "kernel stays 0" — and it is unaffected because the daemon builds the spec and the kernel spawn path is byte-identical.
- `kloc-check` → tokei ceilings (kernel-core + domain aggregate).
- **`maos-host` is invisible to all of them** (workspace `members` is an explicit list; a new crate is ungated until enrolled). ⇒ **AC1 MUST stand up a NEW `maos-host` public-api baseline, run under `--features wasm-host`, with a mutation proven-red.** "Empty diff, no gate needed" is the exact move that set up the 10.5 NO-GO.

### F3 — Export-control entanglement RELOCATES one packaging act out of 11.1a; it does NOT cut a capability.
You cannot prove a launcher that never launches, so the runtime ACs stay. What moves out is *finalizing the distributable form / stamping 5D002.c.1* — a packaging act, owned by a later post-counsel story. 11.1a is the **dev-loop form by construction** and never trips the gate. **AC6 makes that a mechanical guard, not a promise** (mechanical gates compound; promises decay). **OPEN for counsel (flag, do not block preflight):** is the classification trigger the *vendored wasmtime engine* (then even the `Cargo.toml`/`Cargo.lock` add is the event — AC6's "exclude from GA manifest" is too weak and 11.1a can't merge the dep until counsel clears) or the *user-shipped WASM Spirit*? Preflight position: in-tree source behind a dev feature ≠ a distributable runtime. Counsel must draw the line before merge; AC6 is the insurance until they do.

### F4 — Model tier opus-4-8 + pre-booked §A6 is MANDATORY (Decision 11).
This is one of the three opus-4-8 stories whose review net degraded on 10.5. Pre-book the full multi-layer review (Blind + Edge + Acceptance + **Test-Infra Auditor**) **with a runtime-execution check that RUNS the artifacts** (§A6). A degraded/rate-limited review is not a review and hard-blocks completion. The §A6 runtime layer must paste: (1) a guest compiled from `maos:spirit@1.0`; (2) a real `maos-wasm-host` subprocess doing one full ADR-032 round-trip over real pipes; (3) all fuel/T2 cells + both disable-the-other controls with the actual `OutOfFuel` trap and `SIGSYS`/`EACCES`; (4) the corpus mutator/dropper/boundary guests observed going RED; (5) the `check-kernel-baseline` 22964 green + the new `maos-host` public-api baseline green under feature.

---

## Decisions ledger

| # | Decision | Source |
|---|----------|--------|
| **D1** | `SpiritHostPort` + plan structs → new `crates/maos-host` (`pub`), daemon-side. NOT `maos-domain`. Supersedes the 11.0 spike's maos-domain recommendation. | Party round-2 + code-check F1 |
| **D2** | wasmtime adapter → separate crate `maos-wasm-host` (ADR-041 isolation; cargo-deny dependency-closure containment). `maos-bin` depends on both. | Round-1 Winston/Amelia; ADR-041 |
| **D3** | Kernel-core delta target = **HARD 0**; the ratified +150 LOC is an **epic-level emergency brake**, NOT a story budget. Proven by `check-kernel-baseline` 22964 (the gate that measures kernel-core/src), not by abi-diff. | Round-1 Winston |
| **D4** | "Empty abi-diff" is the wrong gate (scans maos-spirit-abi only). AC1 stands up a NEW `maos-host` public-api baseline run under `--features wasm-host` + mutation proven-red. Murat's 5 fields named per gate (crate · visibility · watcher · feature-set · proven-red). | Round-2 Murat + code-check F2 |
| **D5** | WIT byte-equal oracle = **two independent paths** (kernel K-encode vs WIT lower→real-component→lift), canonical CBOR (RFC 8949 §4.2.1) enforced on BOTH sides; completeness denominator = every type-constructor in the `.wit` AST; mutator/dropper/boundary guests as proven-red. NOT bytes-in==bytes-out. | Round-1 Murat |
| **D6** | fuel↔T2 = 2×2 matrix with DERIVED cause attribution (`OutOfFuel` trap code, not exit≠0) + both disable-the-other controls + the benign no-kill sanity cell. **fuel bound strictly < T2 bound** (fuel wins gracefully, clean error frame; T2 is backstop). Epoch-interrupt ≠ fuel — assert the trap, not the timing. | Round-1 Murat + Amelia |
| **D7** | ADR-031 → **Accepted**. Supersedes ADR-002 **single-form clause ONLY** (reaffirms ADR-002 subprocess/ADR-032 substrate + T2); supersedes ADR-040 **fully** (defer lifted). ADR-002/040 get `Superseded-by: ADR-031` headers. Binding-v2.0 deferred to 11.1b. | Round-1 Winston |
| **D8** | Export guard is AC6 (a mechanical negative AC), NOT a note. `wasm-host` feature-gated + provably absent from GA/distributable manifest + cargo-deny dep-closure asserts wasmtime is absent from kernel-core/maos-domain trees. Counsel line-drawing flagged (F3) as a merge precondition. | Round-1 John + code-check F3 |
| **D9** | Corpus guests + any shipped `.wasm` fixture stay **crypto-free** (identity/echo/mutate only) so the test fixtures themselves don't become export-entangled. Commit `.wasm` as a binary fixture + scoped-nightly regen (9.5c idiom); no `cargo-component` on the default `cargo test` path. | Round-1/2 Murat + Amelia |
| **D10** | 6 ACs (at the §A5 ceiling). If forced to 5, cut candidate is AC5 (ADR supersession → DoD); AC6 (export guard) NEVER demotes. | Round-1 John |

---

## Acceptance Criteria (6 — at the §A5 ≤6 ceiling)

**AC1 — Out-of-kernel host port; kernel-core stays provably 0 (NFR-Test-2 / NFR-Maint-1)**
**Given** a new `crates/maos-host` (the `SpiritHostPort` form→launch-plan resolver, `pub`) and a separate `crates/maos-wasm-host` wasmtime adapter, wired at the `maos-bin` composition root (construct next to the Loom-lite adapter `main.rs:1683`; invoke `resolve_launch` where `program`/argv are computed at `main.rs:492`)
**When** the CI gates run
**Then** `check-kernel-baseline` reports `crates/maos-kernel-core/src` = **22964**, unchanged (no FLAG-Winston re-pin)
**And** a NEW `maos-host` public-api baseline (run under `--features wasm-host`) reports zero unauthorized surface change, with a **mutation proven-red** (add an un-allowlisted trait method → the gate goes RED; remove → green)
**And** the cargo-deny dependency-closure gate asserts `wasmtime`/`wasmtime-wasi`/`wit-bindgen` are ABSENT from the `maos-kernel-core` and `maos-domain` dependency trees
**And** `maos-host` + `maos-wasm-host` are enrolled in the workspace `members` list and sit OUTSIDE the ADR-038 aggregate (kernel-core + domain) ceiling.

**AC2 — `maos:spirit@1.0` WIT + byte-equal corpus oracle (extends the ADR-032 byte-id gate)**
**Given** the WIT world `maos:spirit@1.0` as a typed projection of the ADR-032 frame set (`FrameKind`, `FramePayload`/`IacFrame`)
**When** the corpus runs every type-constructor enumerated mechanically from the `.wit` AST (variant arms, record fields, `option`/`result` wrappers — 100% or RED; explicit `None`/`Some(null)`/`Some(value)` rows per optional)
**Then** for each frame, `K-encode(frame)` (kernel canonical path) is **byte-identical** to `lift(component(lower(frame)))` re-encoded, under a pinned canonical CBOR profile (RFC 8949 §4.2.1) enforced on BOTH sides
**And** an independent re-derivation (a hand-written, spec-audited ADR-032→WIT mapping, NOT the host's bindgen) structurally equals the value lifted from guest memory
**And** the proven-red holds: a **mutator** guest (flips one field) → RED, a **dropper** guest (omits an optional) → RED, a **boundary** guest (CBOR 23/24, 255/256, map-reorder) → RED.

**AC3 — A WASM component IS `spec.program` and speaks ADR-032 end-to-end (runtime proven-red, real wasmtime)**
**Given** a real `maos-wasm-runner` (the wasmtime component-runner that IS `BridgeSpawnSpec.program`, argv = `[component.wasm]`, `SandboxSpec`/T2 unchanged)
**When** the daemon resolves a `form = WasmComponent` manifest and the real runner subprocess is spawned over real pipes
**Then** one full ADR-032 (Content-Length + CBOR) intake round-trips: a frame in → the `maos:spirit@1.0` guest export → a conformant ADR-032 frame out, **byte-identical to a native Spirit's** (the kernel cannot tell it is WASM)
**And** a malformed / non-conformant component fails closed (`InvalidComponent`), never a truncated frame
**And** NO `SpiritHostPort`/`maos-wasm-host` mock appears anywhere in this proven-red path (real binary, real component, real pipes).

**AC4 — T2 confinement + fuel↔T2 double-kill, both proven load-bearing (NFR-Sec-3 defense-in-depth)**
**Given** the runner under T2 (deny-by-default caps) with wasmtime fuel metering armed
**When** the 2×2 matrix runs — {pure spin / forbidden-syscall / benign / spin+syscall} × {fuel, T2} — plus both disable-the-other controls
**Then** the spin-loop with T2≈∞ is killed by **fuel** with trap code `OutOfFuel` (NOT `exit_code != 0`, NOT an epoch/wall-clock deadline)
**And** the forbidden-syscall guest with fuel=`u64::MAX` is killed by **T2** with the syscall signature (`SIGSYS`/`EACCES`/Job-Object) + a sandbox audit row
**And** the benign guest completes with a clean exit (the no-vacuous-green sanity cell)
**And** a granted capability works while an un-granted fs/net capability is refused (the load-bearing negative control)
**And** the configured precedence is **fuel bound strictly < T2 bound** so fuel always wins gracefully with a clean error frame; T2 is strictly the backstop.

**AC5 — ADR-031 → Accepted with precise supersession headers**
**Given** the WASM-component-form ADR
**When** ADR-031 lands `Accepted`
**Then** it supersedes ADR-002's **single-form clause ONLY** and explicitly **reaffirms** ADR-002's subprocess + ADR-032-over-stdio substrate and the T2 path
**And** it supersedes ADR-040 **fully** (the in-process defer is lifted; the §13.1 in-process-embedding measurement gate stays untripped — in-kernel wasmtime embedding remains FORBIDDEN)
**And** ADR-002 and ADR-040 carry `Superseded-by: ADR-031` headers; binding-v2.0 is deferred to 11.1b.

**AC6 — Export-control guard (mechanical negative AC; dev-gate insurance)**
**Given** the wasmtime runtime is dev-only
**When** the GA/distributable build manifest is produced
**Then** `maos-wasm-host`/`maos-wasm-runner` are behind `--features wasm-host` (OFF by default) and a CI gate asserts they are ABSENT from the shippable artifact set
**And** merging 11.1a does NOT finalize the 5D002.c.1 classification (no distributable runtime is packaged)
**And** the cargo-deny dep-closure result (AC1) doubles as the export-containment proof that controlled crypto stays out of the kernel/domain trees
**And** the open counsel question (engine-trigger vs Spirit-trigger, F3) is recorded as a named merge precondition.

---

## §A7 gate-source mapping (name each gate's discipline)

| Gate (AC) | §A7 source | derive-and-reconcile numerator | real-subsystem proven-red | canned-trap avoided |
|---|---|---|---|---|
| kernel-0 (AC1) | derive-and-reconcile | `check-kernel-baseline` line count of kernel-core/src (22964) — DERIVED, not file-touch self-report | inject 1 kernel-core line → gate RED → revert | "nobody edited kernel-core/" as numerator |
| host-surface (AC1) | derive-and-reconcile | NEW `maos-host` public-api baseline diff under `--features wasm-host` | add un-allowlisted trait method → RED | empty diff in a non-scanned / non-feature-compiled crate (vacuous-green) |
| WIT byte-equal (AC2) | byte-identical + independent re-derivation | every `.wit` AST constructor (100% denominator) | mutator/dropper/boundary guests → RED | bytes-in==bytes-out echo; happy-path corpus; one-sided canonicalization |
| runtime (AC3) | real-subsystem proven-red | real wasmtime subprocess + real component over real pipes | malformed component → `InvalidComponent` fail-closed | mock host / canned conformance result |
| fuel↔T2 (AC4) | feature-flag ≠ measurement | DERIVED kill cause (trap code / signal), per matrix cell | disable-the-other controls + benign no-kill cell | `exit_code != 0` as success; epoch-as-fuel conflation |
| export guard (AC6) | mechanical gate over promise | GA manifest membership + cargo-deny dep-closure | strip the feature-gate / add wasmtime to kernel tree → RED | "we didn't finalize the distributable" as a note |

---

## Open gaps carried from the 11.0 spike (each now owned by an AC)

1. **Runtime proven-red (real wasmtime runner speaking ADR-032)** → AC3.
2. **WIT byte-equal corpus** → AC2.
3. **fuel/epoch ↔ T2 interaction** → AC4.
4. **Export-control 5D002.c.1 entanglement** → AC6 + F3 (counsel line-drawing = merge precondition; distributable form not finalized pre-clearance).

---

## Tasks / Subtasks (red→green build order)

- [x] 1. **(R1, AC1)** `crates/maos-host`: `SpiritHostPort` trait + `SpiritForm`/`SpiritLaunchRequest`/`SpiritLaunchPlan` (sync, pure, no wasmtime). Unit-test `resolve_launch`: `WasmComponent` → runner program + prepended component arg; `NativeSubprocess` → identity. Test-double adapter → green. Enroll the crate in workspace `members`.
- [x] 2. **(R1, AC1)** Stand up the `maos-host` public-api baseline + the `--features wasm-host` gate run + the mutation proven-red. Wire cargo-deny dep-closure (wasmtime absent from kernel-core/domain).
- [x] 3. **(R2, AC2)** `wit/spirit.wit` = `maos:spirit@1.0`; byte-pin + `wit-bindgen` world snapshot. Build the corpus from the `.wit` AST (100% constructor denominator) + the hand-written independent ADR-032→WIT mapping + canonical-CBOR profile both sides.
- [x] 4. **(R3, AC3)** `crates/maos-wasm-host` + `maos-wasm-runner`: real wasmtime `Engine` (`consume_fuel(true)`, `async_support(true)`), component link, ADR-032 stdio pump. Commit a crypto-free echo `.wasm` fixture (+ scoped-nightly regen). Integration test: real subprocess, frame in/out byte-correct; malformed → `InvalidComponent`.
- [x] 5. **(R4, AC4)** fuel↔T2 2×2 matrix + disable-the-other controls + benign cell + deny-by-default cap negative control; assert trap codes/signals; enforce fuel < T2 ordering.
- [x] 6. **(R5, AC2/AC3)** mutator/dropper/boundary proven-red guests observed RED on the real host.
- [x] 7. **(AC5)** Finalize ADR-031 → Accepted; add `Superseded-by` headers to ADR-002/040; update `docs/adr/index.md`.
- [x] 8. **(AC6)** Feature-gate `wasm-host`; GA-manifest-exclusion gate; record the counsel precondition.
- [x] 9. **(R5, journey)** End-to-end: `form=WasmComponent` manifest → daemon `resolve_launch` (`main.rs:492`) → real runner → full ADR-032 round-trip.
-[x] 10. **(§A6)** Multi-layer review (Blind Hunter, Edge Case Hunter, Acceptance Auditor, Test Infrastructure Auditor) + §A6 runtime-execution check — completed via code-review workflow 2026-06-30. Findings below; all decision-needed items resolved, all patches applied.

---

## Dev-gate reminder
Build + prove green on branch `epic-11` during the v1.5 hold. **Do NOT merge to the shippable line** and **do NOT finalize a distributable WASM runtime** until export counsel clears (F3). The 11.1a capability is non-distributable by construction, so its own counsel clearance should be cheap; the expensive ask is the later distributable-packaging story — keep them apart so the hold blocks the smallest surface.

---

## Dev Notes

### Architecture
- **D1**: `SpiritHostPort` + plan structs → `crates/maos-host` (daemon-side, NOT `maos-domain`). Supersedes spike.
- **D2**: wasmtime adapter → `crates/maos-wasm-host` (ADR-041 isolation). `maos-bin` depends on both.
- **D3**: Kernel-core delta = HARD 0 (baseline 22964).
- **D4**: New `maos-host` public-api baseline under `--features wasm-host` + mutation proven-red.
- **D9**: Corpus guests crypto-free (identity/echo only). `.wasm` committed as binary fixture.

### Key Patterns
- Follow `CollectiveMemoryPort` / `maos-loom-lite` pattern: sync trait in host crate, adapter in separate crate, injected at composition root.
- `block_on_or_typed` guard: no panic into kernel.
- Existing injection point: `main.rs:1683` (Loom-lite). Use point: `main.rs:492` (BridgeSpawnSpec construction).

---

## Dev Agent Record

### Implementation Plan
Phase 1 (Foundation): Created `crates/maos-host` with SpiritHostPort trait + plan types (D1: daemon-side, NOT maos-domain). Added public-api baseline gate (xtask check-host-surface), extended cargo-deny dep-closure for wasmtime stack, finalized ADR-031 Accepted, feature-gated wasm-host in maos-bin.

Phase 2 (WIT + WASM Runtime): Authored `maos:spirit@1.0` WIT as a complete typed projection of the ADR-032 frame set — all 15 FrameKind discriminants, 9 FramePayload variants, 15 record types. Created `crates/maos-wasm-host` with wasmtime adapter + `maos-wasm-runner` binary. Built 4 crypto-free WASM fixtures. Implemented fuel/T2 2x2 matrix tests proving OutOfFuel trap attribution. Created 17-test WIT byte-equal corpus with canonical CBOR oracle.

Phase 3 (Integration): End-to-end test spawning real maos-wasm-runner subprocess over real pipes, sending/receiving ADR-032 frames, verifying byte-identical round-trip. InvalidComponent fails closed.

### Debug Log
-block_on_or_typed from within tokio runtime panics: simplified adapter validation to sync std::fs::metadata. **[Superseded post-review]** — resolve_launch now runs a real wasmtime conformance probe on a dedicated thread with a recv_timeout bound (no tokio handle needed).
-serde_json::json! macro does not support [0u8; 16] repeat syntax: used vec![0u8; 16].
-Runner needed core-module fallback for echo fixture: added try-component-then-module. **[Superseded post-review]** — the runner now requires a real component (the fallback was removed once guests/echo-spirit landed); the fallback's presence was itself a Test Infra Auditor finding.
-wasmtime::component::bindgen! requires wasmtime_wasi::p2::add_to_linker_sync + a WasiView-implementing Store<T> state (host_state::HostState) — wasip2-targeted guests import wasi:cli/wasi:io even when unused by guest code (Rust std runtime startup).
-wit/spirit.wit never parsed: result/from are reserved WIT keywords; float32 is not a valid WIT numeric type name (f32); discovered only when compiling the first real guest against it.

### Completion Notes
87 new tests (53 crate + 34 xtask gate) — pre-review baseline, GREEN. Kernel-core delta: ZERO (22964).

**Post-review (2026-06-30):** §A6 multi-layer review found the pre-review implementation materially incomplete against AC1/AC3/AC4/AC6 (see Review Findings below) — the runner was a host-side echo loop (not a real guest call), the T2 matrix cell was entirely absent, and the composition root never wired SpiritHostPort. All fixed in this pass:
-**AC3 real guest round-trip**: built guests/echo-spirit (real wasm32-wasip2 component conforming to maos:spirit@1.0, via cargo-component); rewrote maos-wasm-runner to use wasmtime::component::bindgen! plus a full frame_bridge (domain IacFrame <-> WIT IacFrame) instead of an echo loop; resolve_launch now runs a real wasmtime instantiate-probe (bounded by timeout) instead of std::fs::metadata.
-**AC4 T2 cell**: added test-fixtures/forbidden-syscall-probe (native ptrace(2) probe) + tests/t2_sandbox_kill.rs driving the kernel's real spawn_sandboxed/classify_exit — proves SIGSYS kill attribution, the benign negative control, and the granted/ungranted Landlock Scope::FsRead capability negative control. Fixed the mislabeled fuel-disabled test and replaced the wall-clock fuel<T2 precedence test with an epoch-interrupt mechanism-based one.
-**AC1/AC6 composition-root wiring**: maos-bin/Cargo.toml wasm-host feature now gates a real dep:maos-wasm-host; main.rs constructs Option<Arc<dyn SpiritHostPort>> next to the Loom-lite port and calls resolve_launch(NativeSubprocess) at the BridgeSpawnSpec.program site. Verified via nm: default binary has 0 wasmtime symbols, --features wasm-host has 30,552. Wired check_wasm_host_absent_from_default into check-export-control's real exit status (was #[cfg(test)]-only dead code) and mutation-proved it RED on a simulated leak. Added check-host-surface/check-dependency-closure CI jobs.
-**WIT file bugs found and fixed**: wit/spirit.wit never actually parsed — result and from are reserved WIT keywords (renamed to result-text/frame-from), float32 is not a valid WIT type (f32), and frame-origin only had 2 of the domain's real 4 FrameOrigin variants (fixed to human-authored/spirit-auto/spirit-drafted-human-approved/kernel).
-**AC2 corpus**: replaced the hand-maintained completeness consts with wit-parser-based AST parsing of the real .wit file (mutation-tested: a bogus enum case fails at compile time via the bindgen-generated exhaustive match). Added a real non-pre-sorted-container test proving ciborium is NOT independently canonical (corrected the codec's doc comment accordingly) and hardened codec.rs (blank-separator validation, Content-Length cap, header-line cap, propagated write failures).
-Kernel-core delta after all fixes: still ZERO (22964) — confirmed via check-kernel-baseline.

117 tests total (58 crate incl. 5 new t2_sandbox_kill tests + 4 xtask gates re-verified green). Kernel-core delta: ZERO (22964), confirmed post-fix. `nm`-verified: default maos binary excludes wasmtime; --features wasm-host includes it. check-export-control's WASM-host leak gate mutation-tested RED on a simulated leak, reverted, re-confirmed GREEN.

---

## Review Findings

_Code review completed 2026-06-30 via bmad-code-review workflow — 4 parallel layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor, Test Infrastructure Auditor) + independent §A6 runtime-execution verification (builds, gate runs, mutation tests). Initial implementation was materially incomplete against AC1/AC2/AC3/AC4/AC6; the 1 decision-needed item was resolved by team consensus (per spec text, see below) and all patches applied in this pass. See Completion Notes above for the fix summary._

- [x] [Review][Patch] AC1 mutation-red direction was ambiguous between spec text and shipped semver policy — RESOLVED via team discussion 2026-06-30: consensus is **per spec text + long-term correctness**. `SpiritHostPort`'s surface sits directly on the wasm-host export boundary (F1/F3) — every public item is something `maos-wasm-host` and export counsel must audit; growing it silently is the risky direction, unlike ordinary library semver where removal is the breaking change. [xtask/src/check_host_surface.rs] — fixed: `scan_surface_diff` inverted (added items → RED/unauthorized-growth, removed items → reported but GREEN/always-safe-to-narrow); module doc + `Report` field docs + CLI messages updated to match; all 7 unit tests rewritten for the new policy (renamed `removal_is_red`→`mutation_proven_red` now proving detection of a *removal*, added `addition_is_red`, `addition_and_removal_reds_on_addition_only`, `empty_current_is_green_everything_removed`); live-mutation-proven against the real gate (added `probe_runtime` to `SpiritHostPort` → gate REDed with "unauthorized added public item"; reverted → GREEN, 0 added/0 removed).
- [x] [Review][Patch] Runner was a host-side echo loop, not a real guest call [crates/maos-wasm-host/src/runner.rs] — fixed: real wasmtime::component::bindgen! call path via frame_bridge.
- [x] [Review][Patch] No real maos:spirit@1.0 component fixture existed; core-module fallback masked this [crates/maos-wasm-host/guests/echo-spirit] — fixed: real wasm32-wasip2 component built and used.
- [x] [Review][Patch] resolve_launch validated components via std::fs::metadata only, never rejecting malformed-but-present .wasm [crates/maos-wasm-host/src/adapter.rs] — fixed: real wasmtime instantiate-probe, timeout-bounded.
- [x] [Review][Patch] AC4's entire T2 (sandbox-kill) column was absent — no forbidden-syscall test, no SIGSYS assertion, no capability negative control [crates/maos-wasm-host/tests/fuel_t2_matrix.rs] — fixed: tests/t2_sandbox_kill.rs + test-fixtures/forbidden-syscall-probe.
- [x] [Review][Patch] fuel_disabled_spin_runs_indefinitely_until_timeout loaded benign.wasm, tested nothing about disabled fuel or spin [crates/maos-wasm-host/tests/fuel_t2_matrix.rs:158] — fixed: loads spin.wasm, uses epoch-interrupt bound.
- [x] [Review][Patch] fuel_ordering_fuel_bound_strictly_less_than_t2 proved precedence via wall-clock less-than-1000ms, not mechanism [crates/maos-wasm-host/tests/fuel_t2_matrix.rs:183] — fixed: dual fuel+epoch config, asserts trap identity not timing.
- [x] [Review][Patch] AC1 composition-root wiring (main.rs:492/main.rs:1683) was entirely absent — SpiritHostPort was constructed nowhere [crates/maos-bin/src/main.rs] — fixed: real wiring, nm-verified.
- [x] [Review][Patch] AC6's export-control leak gate (check_wasm_host_absent_from_default) was dead code, never invoked from any CI-reachable entrypoint [xtask/src/check_export_control.rs] — fixed: wired into run(), mutation-tested RED.
- [x] [Review][Patch] wasm-host feature on maos-bin was empty ([]), gated nothing [crates/maos-bin/Cargo.toml:26] — fixed: ["dep:maos-wasm-host"].
- [x] [Review][Patch] wit/spirit.wit never parsed as valid WIT (result/from reserved keywords, float32 invalid type name) [wit/spirit.wit] — fixed.
- [x] [Review][Patch] frame-origin WIT enum had 2 cases; domain FrameOrigin has 4 [wit/spirit.wit] — fixed to match exactly.
- [x] [Review][Patch] WIT corpus completeness denominator was a hand-maintained const array, not derived from the .wit AST per D5 [crates/maos-wasm-host/tests/wit_corpus.rs] — fixed: wit-parser-based AST parsing.
- [x] [Review][Patch] No test exercised a non-pre-sorted container; codec.rs falsely claimed ciborium canonicalizes by default [crates/maos-wasm-host/src/codec.rs] — fixed: added cbor_non_pre_sorted_container_reveals_insertion_order_not_canonical, corrected doc comment.
- [x] [Review][Patch] invalid_component_fails_closed asserted subprocess exit code, not the typed InvalidComponent error [crates/maos-wasm-host/tests/e2e_roundtrip.rs] — fixed: resolve_launch_rejects_non_conformant_component asserts the typed variant; invalid_component_fails_closed_with_distinct_exit_code asserts the distinct exit code + no leaked partial frame.
- [x] [Review][Patch] E2E tests silently skipped (vacuous pass) when the runner binary was absent [crates/maos-wasm-host/tests/e2e_roundtrip.rs] — fixed: require_runner_binary() panics loudly instead.
- [x] [Review][Patch] wit-bindgen was an unused host-side dependency in the most export-sensitive crate [crates/maos-wasm-host/Cargo.toml] — fixed: removed (guest-side wit-bindgen lives in guests/echo-spirit's own standalone workspace).
- [x] [Review][Patch] New source files committed with executable mode 0755 [crates/maos-host/src/lib.rs, crates/maos-wasm-host/src/*, xtask/src/check_host_surface.rs, xtask/src/check_export_control.rs, xtask/src/check_dependency_closure.rs] — fixed: chmod 644.
- [x] [Review][Patch] docs/adr/ADR-031*/index.md described a "+150 LOC abi-diff" gate that D3/D4 explicitly reject as the wrong gate — fixed: now describes check-kernel-baseline HARD 0 + check-host-surface.
- [x] [Review][Patch] codec.rs had no Content-Length cap, no header-line cap, no blank-separator validation — a guest-triggerable DoS surface — fixed: MAX_FRAME_BYTES, MAX_HEADER_LINE_BYTES, blank-line validation, bounded blank-skip loop.
- [x] [Review][Patch] Runner had no compile-time timeout for Component::new (compile-bomb risk) — fixed: compile_with_timeout on a dedicated thread.
- [x] [Review][Patch] WASM_HOST_LEAK_INDICATORS omitted wasmtime-wasi; cargo tree ran without --edges all (dev/build-dep leaks invisible) — fixed: aligned with check_dependency_closure's list, added --edges all.

---

## File List

### New files
-crates/maos-host/Cargo.toml, src/lib.rs, tests/resolve_launch.rs
-crates/maos-wasm-host/Cargo.toml, src/lib.rs, src/adapter.rs, src/config.rs, src/codec.rs, src/runner.rs, src/frame_bridge.rs, src/conformance.rs, src/host_state.rs, src/wit_guest.rs
-crates/maos-wasm-host/tests/codec_integration.rs, wit_corpus.rs, fuel_t2_matrix.rs, e2e_roundtrip.rs, t2_sandbox_kill.rs, frame_bridge_roundtrip.rs
-crates/maos-wasm-host/guests/echo-spirit/ (Cargo.toml, src/lib.rs — real wasm32-wasip2 maos:spirit@1.0 component)
-crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/ (Cargo.toml, src/main.rs)
-wit/spirit.wit (replaced stub)
-tests/fixtures/wasm/echo.wat, echo.wasm, spin.wat, spin.wasm, benign.wat, benign.wasm, mutator.wat, mutator.wasm, echo_spirit_component.wasm
-abi-baseline/maos-host-v1.txt
-xtask/src/check_host_surface.rs
-docs/compliance/export-counsel-precondition.md

### Modified files
-Cargo.toml (workspace members)
-crates/maos-bin/Cargo.toml (wasm-host feature = ["dep:maos-wasm-host"]; maos-wasm-host optional dep)
-crates/maos-bin/src/main.rs (SpiritHostPort construction next to Loom-lite port at ~line 1683; resolve_launch(NativeSubprocess) call at the BridgeSpawnSpec.program site; run_cli_wrapper_manifest threaded with spirit_host param)
-crates/maos-host/src/lib.rs (doc fix: stale block_on_or_typed reference)
-crates/maos-wasm-host/Cargo.toml (removed unused wit-bindgen; added maos-domain, maos-spirit-abi, wit-parser dev-dep, maos-kernel-core dev-dep, libc dev-dep, smallvec dev-dep)
-xtask/src/main.rs (check-host-surface subcommand)
-xtask/src/check_dependency_closure.rs (wasmtime + maos-domain tree)
-xtask/src/check_export_control.rs (WASM-host absence gate wired into run(); --edges all; wasmtime-wasi indicator added)
-xtask/src/check_host_surface.rs (--all-features on cargo public-api)
-docs/adr/ADR-031, ADR-002, ADR-040, index.md (status + supersession; stale abi-diff gate description corrected)
-.github/workflows/discipline.yml (check-host-surface, check-dependency-closure, wasm-host-tests CI jobs + ship-gate aggregate wiring)
-_bmad-output/test-artifacts/traceability-matrix.md (Murat trace-coverage matrix, 2026-07-01)

---

## Change Log

-2026-06-30: Story 11.1a implementation complete (Tasks 1-9). Kernel-core delta = 0. Task 10 (review) deferred.
-2026-06-30: Code review (§A6 multi-layer + runtime-execution) found the pre-review implementation materially incomplete against AC1/AC3/AC4/AC6. Fixed: real guest round-trip via wasmtime component-model bindgen (was a host-side echo loop); AC4 T2 sandbox-kill cell (was entirely absent); AC1/AC6 composition-root wiring (was entirely absent, nm-verified now real); WIT file parse bugs (reserved keywords, invalid type names, wrong FrameOrigin arity); AC2 corpus completeness now AST-derived. AC1's mutation-red direction ambiguity was resolved by team consensus per spec text + long-term correctness (closed allowlist: added surface → RED, removed surface → GREEN) and `check_host_surface` inverted accordingly, live-mutation-proven. Kernel-core delta remains 0 (22964). Task 10 complete, no open items.
-2026-07-01: Murat trace-coverage audit (TR workflow) initially returned **CONCERNS** — the 71-test suite passed locally but NO CI job ran it, and `frame_bridge::lower/lift` (the WIT conversion for all 15 FrameKinds) had zero direct tests (e2e covered 1 of 15). TA pass closed both P0 gaps: (1) added `wasm-host-tests` CI job (builds the forbidden-syscall-probe fixture + runs `cargo test -p maos-host -p maos-wasm-host`), wired into `v1-0-ship-gate` needs + summary + fail-log; (2) added `tests/frame_bridge_roundtrip.rs` (13 tests: all 15 FrameKinds, all 9 payloads, 3 documented-lossy-field pins). Trace matrix at `_bmad-output/test-artifacts/traceability-matrix.md`; gate upgraded CONCERNS → **PASS**. One advisory caveat carried: the T2 SIGSYS cell self-skips without CAP_SYS_ADMIN (mirrors the kernel's own `sandbox_enforcement_linux.rs` pattern — non-vacuous skip, not a silent gap).
---
