---
Status: Proposed — drafted in the Story 11.0 hold-window de-risk spike (2026-06-29); resolves the `speculative-vNext` ADR-031 ("Cross-Form Spirit Equivalence") placeholder forward-referenced by ADR-002 and ADR-040. Transitions to Accepted at Story 11.1a (WASM form + host) and binding-v2.0 at Story 11.1b (cross-form equivalence gate).
Gate: Story 11.1a — `maos-wasm-host` adapter behind `SpiritHostPort`; launcher-seam re-pin abi-diff proven-red ≤ +150 LOC (→ ~23114) from baseline 22964; WIT `maos:spirit@1.0` byte-equal corpus vs the ADR-032 frame set. Story 11.1b — behavioral-oracle tiered cross-form equivalence (100% on invariant-bearing effects; known-divergent component proven-red) → binding.
Decided: 2026-06-29 (architecture); form + equivalence land at 11.1a / 11.1b
Accepted-in-PR: <PR_NUMBER>
Supersedes: ADR-002 (single-form "subprocess-only" posture) and ADR-040 (`defer-rust-inproc-to-v2.0+` outcome) — see §Supersession. ADR-004 (sandbox tiers), ADR-032 (wire protocol), ADR-038 (kernel ceiling) are PRESERVED unchanged.
Revisits: ADR-002 §revisit; ADR-040 §rollback; §13.1 measurement gate
---

# ADR-031 — WASM Component-Model Spirit form (host-as-adapter; resolves Cross-Form Equivalence)

**Decision.** v2.0 introduces a **second Spirit authoring form** — the **WASM Component Model** — hosted as a **subprocess**: a wasmtime/component runner that *is* `spec.program`, sandboxed by the existing T2 path, speaking the unchanged ADR-032 wire (Content-Length + CBOR over stdio). The host (WIT bindings, component instantiation, fuel/epoch limits) lives in a NEW out-of-kernel crate `maos-wasm-host`, behind a `SpiritHostPort` trait in `maos-domain/src/ports/`, injected at the daemon composition root (the Story-10.4a / ADR-041 pattern). **In-kernel / in-process wasmtime embedding is FORBIDDEN** at v2.0. Because two forms now exist, ADR-031's original reserved topic — **cross-form equivalence** — becomes real and is proven by the Story 11.1b gate.

## Context

ADR-002 (binding-v0.1) committed v0.1 to **subprocess form only**, and explicitly named the future candidate for a second form:

> "A capability-isolation requirement emerges that subprocess's process boundary cannot meet (in which case **WASM-component, not rust-inproc, is the candidate**). ADR-031 (Cross-Form Spirit Equivalence) is `speculative-vNext` and resolves only when this revisit fires."

ADR-040 (binding-v0.5) measured subprocess IPC (J1 P95 = 6 µs against a 25 ms budget) and ruled `defer-rust-inproc-to-v2.0+`, also naming the resolution condition:

> "ADR-031 (`Cross-Form Spirit Equivalence`) **remains** `speculative-vNext` … if the cross-form equivalence requirement becomes binding, rust-inproc measurement becomes mandatory."

The Epic 11 party-mode (2026-06-29, workflow `wyksr4yce`, decisions §1 and §6) ratified the WASM-component form as the v2.0 extensibility vehicle, with the host **as an adapter, not in the kernel**, and ratified that ADR-031 be **drafted first** in the Story 11.0 hold-window spike before Story 11.1a commits ACs. This ADR is that draft.

### Spike grounding (Story 11.0, 2026-06-29)

A launcher-seam survey of the live tree established that the kernel is **already form-agnostic**, which is what makes the host-as-adapter decision affordable:

- **Every kernel launch primitive runs a bare executable path.** `spawn_and_bridge` does `Command::new(&spec.program)` (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:461`); the T3 path does `Command::new(&argv[0])` from `spirit_binary_path` (`security/sandbox/t3/spawn.rs:114`); the generic `spawn_sandboxed(spec, &mut Command)` (`security/sandbox/mod.rs:132`) takes a caller-built `Command`. `SandboxSpec` (`sandbox/mod.rs:32-41`) carries `tier` + caps + scopes but **no `program` field and no form discriminator** — the executable is chosen at the composition root (`crates/maos-bin/src/main.rs:492-505`, resolved from the manifest's `config.command`).
- **The ADR-032 wire the kernel needs already exists.** The `Content-Length: …\r\n\r\n` reader is implemented (`read_content_length`, `runtime.rs:345`, selected by `CliWrapperStdioShape::JsonRpcOverStdio`); `ciborium` is already a `maos-kernel-core` dependency. The runner subprocess produces ADR-032 frames on stdout exactly as the CLI bridge already consumes them.
- **The injection slot already exists.** The Story-10.4a `CollectiveMemoryPort` adapter is constructed and cast to `Arc<dyn …>` at `crates/maos-bin/src/main.rs:1683-1719`; a `SpiritHostPort` plugs in at the same composition root.
- **Greenfield otherwise.** No `SpiritHostPort`, `maos-wasm-host`, or `wasmtime` exists anywhere in the workspace; the Epic-11 doc is the only precursor.

**Validated kernel-core delta: ~ZERO** (baseline `xtask/kernel-core-baseline.toml:175` = `src_lines = 22964`, freshly re-measured = 22964). A WASM Spirit is `program = <component-runner path>` with the `.wasm` module + fuel passed as argv; the existing T2 subprocess path wraps it identically to any other binary. The ratified **≤ +150 LOC** ceiling (→ ~23114) is **headroom** for an *optional* composition-root parameter (e.g. threading a form discriminator or the host port through an existing spawn-spec constructor) — not a target. For the pure "WASM-in-subprocess" option the launcher-seam kernel delta is plausibly **exactly 0**, with all host code in `maos-wasm-host` and all wiring in `maos-bin`. Story 11.1a will commit the final number with an `abi-diff` proven-red.

## Decision

### 1. The second form is WASM-component, hosted as a subprocess

A WASM Spirit is a WASM Component Model artifact run by a wasmtime-based component runner. The runner is an ordinary subprocess: it *is* `spec.program`, it is sandboxed by the **existing T2 path** (OS-level isolation, unchanged), and it speaks the **existing ADR-032 wire** over stdio. The WASM component sandbox (capability gating via WIT imports, fuel/epoch resource limits) composes *on top of* the T2 process boundary — **defense in depth**, which is precisely the "capability-isolation requirement subprocess's process boundary cannot meet" that ADR-002 named as the WASM-component trigger.

### 2. The host is an adapter, not kernel

The host — `wit-bindgen`/WIT bindings, component instantiation, linker, fuel/epoch configuration — lives in a NEW crate `maos-wasm-host`, behind a `SpiritHostPort` trait in `crates/maos-domain/src/ports/`, injected at the daemon composition root. This is the Story-10.4a `CollectiveMemoryPort` / ADR-041 port-trait pattern verbatim: sync trait in `maos-domain`, async work bridged in the adapter via a held `tokio::runtime::Handle` + the `block_on_or_typed` guard (no panic into the kernel). **In-kernel / in-process wasmtime embedding is FORBIDDEN** at v2.0 — it would expand the kernel-core hot-path/crash surface against the ADR-038 ≤ 6 KLOC kernel-core / ≤ 20 KLOC aggregate ceiling, and reintroduce the in-process-form risk ADR-002/040 deferred. In-process embedding stays gated behind a future §13.1 measurement + a superseding ADR.

### 3. WIT `maos:spirit@1.0` is a typed projection of the ADR-032 frame set — not a new protocol

The wire stays ADR-032. WASM Spirits get a typed WIT binding **generated from the same frame set** the native subprocess form uses: the `FrameKind` discriminants (`crates/maos-spirit-abi/src/identity.rs:30-79`) and the `FramePayload` variants + `IacFrame` envelope (`crates/maos-domain/src/frame.rs:26-75`). The WIT interface is the typed face of those records; the bytes on the wire remain Content-Length + CBOR. This preserves the single-wire discipline (ADR-032's "byte-equal golden corpus per frame variant per SDK") — the WIT projection is added to that corpus, it does not replace it.

### 4. Supersession — ADR-002 and ADR-040

- **ADR-002's process-isolation principle is PRESERVED; only its "one form" clause is superseded.** v0.1 shipped one form because a second form would "double the invariant-enforcement surface (two crash recovery semantics, two memory models, two hot-paths)." The WASM-component form does **not** incur that doubling: it reuses the *same* subprocess transport, the *same* T2 sandbox, and the *same* ADR-032 wire and crash/supervision semantics. The only new surface is in user-space (`maos-wasm-host`). So ADR-002's rejection rationale does not apply to *this* second form, and its "subprocess-only" isolation commitment is upheld (the WASM Spirit runs inside a subprocess).
- **ADR-040's `defer-rust-inproc-to-v2.0+` is honored, not contradicted.** ADR-040 deferred **rust-inproc** (in-process). WASM-component is **not** rust-inproc — it keeps the process boundary. rust-inproc **remains deferred**; the §13.1 in-process measurement gate stays **untripped and unchanged** by this ADR (an in-subprocess WASM form is outside that gate entirely). This ADR activates the WASM-component candidate ADR-002/040 explicitly named, without promoting any in-process form.

### 5. Cross-form equivalence becomes real (ADR-031's reserved topic)

With two forms, "do they behave equivalently?" is now a binding question. Story 11.1b proves **behavioral-oracle tiered equivalence**: 100% on invariant-bearing effects (halt, frame sequence, capability denials, region-pin, audit frames) and ≥ 75% slack only on cosmetic/latency surface, scoped to deterministic fixture Spirits, with a **known-divergent component as the proven-red**. (The Epic-11 plan §6 rejects a flat distributional ≥ 75% metric — it hits the `check_cross_form_equiv.rs` U-test NOT-APPLICABLE branch and is vacuous on deterministic Spirits.) ADR-031 transitions to **binding-v2.0** when that gate is green.

## Alternatives considered and rejected

- **In-kernel wasmtime embedding (in-process WASM Spirit).** Rejected at v2.0: expands the kernel-core surface against ADR-038; reintroduces the in-process-form maintenance burden (2× crash/supervision/hot-path) ADR-002 rejected and ADR-040 deferred. Gated behind a future §13.1 measurement + superseding ADR.
- **rust-inproc (ADR-040's deferral subject).** Still deferred: WASM-component meets the capability-isolation need *and* the polyglot/third-party-authoring need without forcing authors into Rust or giving up the process boundary.
- **A new wire protocol / RPC for WASM Spirits.** Rejected: the WIT interface is a typed projection of the ADR-032 frame set, not a replacement. A second wire would fork the byte-equal-corpus discipline and double the codec surface — the exact ABI-doubling ADR-002 warned against.
- **Host embedded in `maos-kernel-core` "for performance."** Rejected: no measured latency requirement justifies it; the subprocess IPC margin (ADR-040: 6 µs P95 vs 25 ms budget) is enormous; it would violate the ADR-038 ceiling and ADR-006/ADR-041 boundary.

## Consequences

- **Two authoring forms, one transport, one wire, one sandbox model, one crash/supervision semantics.** The second form's cost is concentrated in `maos-wasm-host` (user-space), not the kernel.
- **Kernel-core delta ~0** (spike-validated); ≤ +150 LOC is headroom for an optional composition-root parameter, committed with an `abi-diff` proven-red at Story 11.1a and recorded in `kernel-core-baseline.toml` HISTORY with the named surface.
- **ADR-002 and ADR-040 forward-references to ADR-031 are resolved.** The §13.1 in-process measurement gate remains untripped (this ADR does not promote an in-process form).
- **Export-control entanglement (dev-gate).** A WASM runtime can change the 5D002.c.1 export classification. Per the Epic-11 ratification, **Story 11.1a's distributable form must NOT be finalized before export-compliance counsel clears** — one of the two external v1.5 holds gating Epic-11 dev.
- **Story 11.1b's cross-form equivalence gate becomes binding** and is the precondition for ADR-031 → binding-v2.0.

## Gate

- **Story 11.1a** (form + host): `maos-wasm-host` adapter behind `SpiritHostPort`; subprocess launch via the existing T2 path; launcher-seam re-pin **abi-diff proven-red ≤ +150 LOC** (→ ~23114) from baseline 22964 — out-of-surface churn (even a `cargo fmt` reflow, per the 10.5 R3 lesson) is RED; WIT `maos:spirit@1.0` **byte-equal corpus** vs the ADR-032 frame set (extends ADR-032's gate, does not replace it).
- **Story 11.1b** (equivalence): behavioral-oracle **tiered** cross-form equivalence (100% invariant-bearing; known-divergent component proven-red; anti-canned tripwire) → ADR-031 **binding-v2.0**.
- Registered in `docs/adr/index.md`.

## Ratification

Architecture ratified by the Epic 11 party-mode consensus authority (Winston · John · Murat · Amelia + Lunarpulse sign-off, 2026-06-29, workflow `wyksr4yce`, decisions §1 and §6) and de-risked by the Story 11.0 spike (kernel-delta ~0 validated against the live launcher seam). Consistent with ADR-004 (T2 sandbox reused), ADR-006/ADR-041 (host out of kernel), ADR-032 (wire unchanged, WIT as typed projection), and ADR-038 (kernel ceiling protected). The form and the cross-form equivalence gate land at Stories 11.1a / 11.1b; binding-v2.0 follows 11.1b. Drafted during the v1.5 hold-window (a ratified hold-window carve-out: ADR authoring has no Epic-11-dev dependency).
