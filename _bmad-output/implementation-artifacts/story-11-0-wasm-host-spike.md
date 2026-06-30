# Story 11.0 — WASM host de-risk spike (findings)

**Status:** DONE (spike, NO-MERGE) — 2026-06-29. Ratified hold-window carve-out (no Epic-11-dev dependency). Feeds the Story 11.1a preflight.
**Model:** claude-opus-4-8. **Kernel-Δ:** 0 (verified). **Spike code:** `spikes/story-11-0-wasm-host/` (outside the workspace `members` list → never compiled/merged).

This spike validates that a WASM Component-Model Spirit can be hosted as a subprocess behind an out-of-kernel `SpiritHostPort` with **~zero kernel-core change**, and drafts the governing ADRs, **before** Story 11.1a commits its ACs and its FLAG-Winston re-pin number.

---

## AC outcomes

| AC | Outcome | Evidence |
|----|---------|----------|
| **AC1 — Prototype `SpiritHostPort` + `maos-wasm-host`** | ✅ | `spikes/story-11-0-wasm-host/{spirit_host_port.rs, wasm_host_adapter.rs, wit/spirit.wit, composition_root.rs.txt}` — the trait (a form→launch-plan resolver), the adapter (mirrors the 10.4a `block_on_or_typed` bridge), the `maos:spirit@1.0` WIT projection of the ADR-032 frame set, and the two-touch-point wiring sketch (both in `maos-bin`). |
| **AC2 — Validate the FLAG-Winston re-pin ceiling vs 22964** | ✅ — **delta ~0; ≤+150 is headroom** | `find crates/maos-kernel-core/src -name '*.rs' \| xargs wc -l` = **22964** = `kernel-core-baseline.toml:175`. The launcher seam is composition-root + adapter only (see §Kernel-delta). |
| **AC3 — Draft ADR-031 + ADR-002/040 supersession** | ✅ | `docs/adr/ADR-031-wasm-component-model-spirit-form.md` (proposed; supersedes ADR-002 single-form + ADR-040 defer-outcome; preserves the §13.1 in-process gate). Plus `docs/adr/ADR-024-out-of-kernel-sandbox-escape-structural-detector.md` for 11.4b. Both indexed in `docs/adr/index.md`. |

---

## Kernel-delta validation (AC2) — the load-bearing finding

**The kernel is already form-agnostic.** Every launch primitive runs a bare executable path; none branch on Spirit form:

- `spawn_and_bridge` → `Command::new(&spec.program)` — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:461`. `BridgeSpawnSpec.program: String` (`runtime.rs:242`) is "a resolved path or PATH name."
- `spawn_sandboxed(spec, &mut Command)` — `crates/maos-kernel-core/src/security/sandbox/mod.rs:132`. `SandboxSpec` (`mod.rs:32-41`) carries `tier` + caps + scopes but **no `program` field and no form discriminator**.
- T3: `Command::new(&argv[0])` from `spirit_binary_path` — `security/sandbox/t3/spawn.rs:114`.
- The program value is chosen at the **composition root**, not the kernel — `crates/maos-bin/src/main.rs:492-505`, resolved from the manifest's `config.command`.

The ADR-032 wire the kernel must read already exists: `read_content_length` (`runtime.rs:345`, selected by `CliWrapperStdioShape::JsonRpcOverStdio`) + `ciborium` (already a `maos-kernel-core` dependency).

**Therefore** a WASM Spirit is just `program = <wasmtime component-runner path>` with `--component <module.wasm> --fuel <n>` as argv, sandboxed by the unchanged T2 path, emitting ADR-032 frames the existing bridge already consumes. **The realistic launcher-seam kernel-core delta is 0–tens of LOC, plausibly exactly 0** for the ratified "WASM-in-subprocess" option. The ratified **≤ +150 LOC (→ ~23114)** ceiling is **headroom** — the only way to spend it is an *optional* 11.1a choice to thread a form discriminator through a kernel spawn-spec constructor instead of resolving `program` entirely at the composition root. Not required.

**Recommendation for 11.1a:** target **0 kernel-core lines**; the host port + adapter live in `maos-domain` + `maos-wasm-host`, the wiring in `maos-bin`. If a kernel touch is proposed, it must be justified against this finding and committed with an `abi-diff` proven-red against the 22964 baseline; out-of-surface churn (even `cargo fmt` reflow, per the 10.5 R3 lesson) is RED.

---

## Design (for the 11.1a preflight to lift)

- **`SpiritHostPort` = a form → launch-plan resolver** (sync trait in `maos-domain/src/ports/`, zero async deps, mirrors `CollectiveMemoryPort`). The composition root asks it to resolve a `SpiritLaunchRequest {form, artifact, form_config}` into a `SpiritLaunchPlan {program, argv, env, wire}`; the kernel's existing `spawn_and_bridge` launches the plan unchanged. Native form → identity; WASM form → runner + argv.
- **`maos-wasm-host` adapter** holds a `tokio::runtime::Handle` and offloads blocking wasmtime work (component compile / WIT-world conformance) via the guarded `block_on_or_typed` bridge ported verbatim from `maos-loom-lite/src/adapter.rs:64` — no panic into the kernel.
- **WIT `maos:spirit@1.0`** is a typed projection of the ADR-032 frame set (`FrameKind` `identity.rs:30-79`; `FramePayload`/`IacFrame` `frame.rs:26-75`), **not a second wire**. The bytes stay Content-Length + CBOR; the WIT projection is added to ADR-032's byte-equal golden corpus at 11.1a.
- **Injection** at `maos-bin/src/main.rs:~1683`, next to the Loom-lite `CollectiveMemoryPort`, gated on operator config (`MAOS_WASM_RUNNER`). `None` → native-only (the kernel default, unchanged).

---

## What the spike did NOT prove — 11.1a must still establish

The spike establishes the **seam** by code survey; it did **not** build a running wasmtime stack. 11.1a owns the runtime proof. Honest gaps:

1. **A real component runner speaking ADR-032 end-to-end.** `validate_component` is a stub; no `.wasm` Spirit was built or run. 11.1a's **runtime proven-red**: a real wasmtime component-runner subprocess that decodes a Content-Length+CBOR frame, drives the `maos:spirit@1.0` world, and emits a conformant frame — plus a malformed/non-conformant component that fails closed (`InvalidComponent`).
2. **WIT byte-equal corpus.** The spike projects the envelope + kind enum + representative payloads. 11.1a must project **every** routed payload and prove byte-equality against the native form's CBOR (ADR-032 gate, extended).
3. **Fuel/epoch ↔ T2 interaction.** How the in-runner fuel/epoch limit composes with the T2 sandbox CPU-rate/Job-Object caps (defense-in-depth, ADR-031 §1) — measured, not assumed.
4. **The final abi-diff number.** Commit the launcher-seam re-pin (target 0) with an `abi-diff` proven-red and a `kernel-core-baseline.toml` HISTORY entry naming the surface — only if any kernel line is genuinely required.
5. **Export-control entanglement (dev-gate).** A WASM runtime can change the 5D002.c.1 classification. Per the Epic-11 ratification, **11.1a's distributable form must NOT be finalized before export-compliance counsel clears** (one of the two external v1.5 holds gating Epic-11 dev). The spike is design-only and does not finalize a distributable.

---

## Feeds into

- **Story 11.1a** (WASM form + host + WIT): lift the `SpiritHostPort` + `maos-wasm-host` design; target 0 kernel-core delta; build the runtime proven-red (gap #1) + WIT byte-equal corpus (gap #2); finalize ADR-031 → Accepted.
- **Story 11.1b** (cross-form equivalence): the tiered behavioral-oracle gate that takes ADR-031 → binding-v2.0.
- **ADR-031 / ADR-024** drafted here; binding deferred to 11.1b / 11.4b respectively.
