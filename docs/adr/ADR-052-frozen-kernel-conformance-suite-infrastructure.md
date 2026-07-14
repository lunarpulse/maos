---
Status: binding-v2.0 (architecture + mechanism ratified 2026-07-07 party-mode; AMENDED 2026-07-07 — literal AC3: the two-axis model is superseded by at-admission negative-control rejection; binding at Story 11.5 gate green)
Gate: Story 11.5 — `check-fkcs` (8 per-leg-independent legs: frozen-tag-consistency, diff-oracle-derives, negative-control-rejects, proxy-cohort-fkcs-score, fault-inject-falsifiers, admission-path-unmodified, release-graph-absence, kernel-abi-diff); `{ v1_0 = advisory, v1_5 = advisory, v2_0 = blocking }`; absent/unmeasured → BLOCK@v2.0
Decided: 2026-07-07 (party-mode preflight; AC3 amendment same day — see §3, Consequences, Gate)
Accepted-in-PR: Story-11.5
Supersedes: none
Revisits: ADR-006 (kernel learns nothing); ADR-010 (hexagonal port/adapter ring); ADR-031 (WASM component-model Spirit form); ADR-032 (Spirit wire protocol); ADR-049 (v2.0 cross-region gate discipline)
---

# ADR-052 — Frozen-Kernel Conformance Suite infrastructure

**Decision.** MAOS v2.0 gets FKCS infrastructure, not a populated genuine-external cohort. The infrastructure consists of an annotated `frozen-kernel-v2.0` tag, `xtask/fkcs-baseline.toml` with the pinned surface triple, an `xtask`-resident diff oracle that derives zero frozen-surface change from the existing kernel/ABI/host gates, a dev-only `maos-fkcs` proxy cohort harness, and a `check-fkcs` ship gate. **The negative-control fourth Spirit is rejected at admission by `maos_registry::admission::admit_spirit`** — a journaled, falsifiable `AdmissionError::OffFrozenSurface` whose reason string contains `off-frozen-surface` — because it declares an off-frozen-surface / `pub(crate)`-style kernel internal; the three in-house conformance proxy Spirits admit green through the same real admission path. (AC3 amendment, 2026-07-07: this supersedes the earlier two-axis model in which the negative control failed only at an FKCS-internal conformance gate while the admission path stayed byte-identical to the 11.4c baseline.)

## Context

NFR-Test-5 requires a Frozen-Kernel Conformance Suite: diff oracle, test harness, kernel-frozen-vN.0 commit tagging, a negative-control fourth Spirit, and a score floor. Epic 11 split this into v2.0 infrastructure and v2.5 populated genuine external Spirits. Story 11.5 is therefore the falsifiability story: it must prove the mechanism is not a self-reported `abi_unchanged` flag, a compile-time artifact, a summary count, or a proxy cohort mislabeled as external.

The frozen surfaces are the `maos-kernel-core/src` line baseline (`23081`), the `maos-spirit-abi` public API baseline (`abi-baseline/v1-pre-bump.txt`), and the `maos-host` closed allowlist (`abi-baseline/maos-host-v1.txt`). These are read-only for this story. The zero-delta claim is scoped to those frozen crates; `xtask` and `maos-fkcs` are expected infrastructure edits, and — per the AC3 amendment — `maos-registry` admission gains the literal `OffFrozenSurface` rejection (see §3 and Consequences).

## Decision

### 1. Frozen tag and pinned surface triple

`frozen-kernel-v2.0` is an annotated git tag at commit `1f5d57295f4f8fbe58a4d1bbf16cf99f5cfdcdd8`. `xtask/fkcs-baseline.toml` records the machine-readable triple:

- `src_lines = 23081`
- `abi_baseline = "abi-baseline/v1-pre-bump.txt"`
- `host_baseline = "abi-baseline/maos-host-v1.txt"`

The tag is the auditable freeze. The TOML file is the oracle input. The `frozen-tag-consistency` leg reconciles both against the live gate values and the tag target.

### 2. Diff oracle derives, never trusts

The FKCS oracle lives in `xtask/src/check_fkcs.rs`, beside the gates it composes. It calls the Rust gate functions directly rather than shelling out to `cargo run -p xtask`; the existing `abi-diff` and `check-host-surface` gates may still invoke `cargo-public-api` internally.

`kernel_unchanged` is derived from before/after `FkcsSurfaceSnapshot` values:

- kernel source line count unchanged;
- `maos-spirit-abi` remains additive-only;
- `maos-host` remains within the closed allowlist.

A forged `kernel_unchanged=true` or `abi_unchanged=true` input is not an oracle input and is ignored. Real capture (the existing gate APIs over the live tree) is used for the green path; synthetic snapshots are used only for the mutation/fault-injection falsifiers.

### 3. Literal at-admission negative-control rejection (redefines L4 + the admission baseline)

The runtime admission path performs the literal AC3 rejection itself. `admit_spirit` parses the manifest's **optional** `[fkcs].internal_references` array inline (lenient line-based, mirroring the existing `extract_manifest_tier`) and rejects any package with a **non-empty** declaration via a new `AdmissionError::OffFrozenSurface { symbols: Vec<String> }` — whose reason string contains `off-frozen-surface` — **before** tier/signature/ComplianceClaim resolution. There is **no** frozen-surface allowlist, **no** `AdmissionConfig` change, and **zero** call-site impact across the workspace: `internal_references` is semantically defined to name off-frozen-surface / `pub(crate)`-style internals, so a non-empty declaration is a conformance violation regardless of trust tier. (There is no separate `required_symbols` field.) The negative-control fourth Spirit declares such an internal and is rejected; the three in-house conformance proxy Spirits omit the declaration and admit green. The real static frozen-surface measurement stays in the `check-fkcs` gate (the diff-oracle legs); admission now **also** rejects on the manifest declaration — that is the literal AC3 ("rejected by `admit_spirit`, journaled/falsifiable, not only `FrozenSymbolGate`"). The `FrozenSymbolGate` in `maos-fkcs` is kept `pub` as an oracle/fixture helper (so tests still compile) but is demoted out of the production admission path — it is no longer THE AC3 mechanism; the real `admit_spirit` is.

This **redefines L4** (the admission-path-unmodified landmine). The admission path is no longer byte-identical to the 11.4c baseline, because it now carries the literal AC3 rejection. The **frozen admission baseline** is therefore redefined to include the new rejection surface: the `[fkcs]`-section parsing + the `OffFrozenSurface` arm in `crates/maos-registry/src/admission.rs`, plus the `[fkcs]`-fixture emission in `crates/maos-fkcs/src/lib.rs::signed_local_package`. The corresponding `check-fkcs` leg (`admission-path-unmodified`) is repurposed to a **content-hash (SHA-256) match over the declared admission files** against this **redefined** baseline — proving the literal AC3 rejection is in place — rather than asserting byte-identity to the 11.4c admission crates.

The **frozen crate triple** (`maos-kernel-core/src`, `maos-spirit-abi`, `maos-host`) remains zero-delta and is unaffected by this amendment: the kernel learns nothing and gains no new internal-symbol-exposure path. Admission's rejection is over the Spirit's own manifest declaration (`internal_references`), not introspection of Spirit binaries; the static surface truth is measured independently by `check-fkcs`.

### 4. Proxy cohort score is proof-of-mechanism only

The in-house Chinese-wall proxy cohort computes per-Spirit FKCS scores and aggregate score using the same harness v2.5 will run against genuine external Spirits. At v2.0 that score is advisory proof of mechanism only. It does **not** satisfy the genuine-external NFR-Test-5 floor. FKCS-populated at v2.5 owns the binding floor over three future externally-authored Spirits.

### 5. Gate enrollment and phase disposition

`check-fkcs` is enrolled as a ship gate with `{ v1_0 = advisory, v1_5 = advisory, v2_0 = blocking }`. A missing result is a v2.0 blocker. Attempted-but-zero tests are hard failures at every phase.

## Alternatives considered and rejected

- **Self-reported `abi_unchanged` / `kernel_unchanged` flag.** Rejected: this is the exact fabrication class FKCS exists to eliminate. The oracle derives from surfaces.
- **Parallel or forked admission queue.** Rejected: admission stays on `SkillAdmissionQueue` + `admit_spirit`; FKCS observes and tests beside it. The AC3 amendment augments the existing `admit_spirit` with the `OffFrozenSurface` arm; it does not fork or wrap the queue.
- **Negative control fails by compiling a `pub(crate)` call.** Rejected: compile failure is a harness artifact, not a conformance gate result. (This alternative is the reason the two-axis model was originally introduced; literal AC3 rejects it by performing the rejection at admission instead.)
- **Keep the two-axis model — admission stays unmodified and the negative control is rejected only at the FKCS-internal `FrozenSymbolGate`.** Rejected (user-selected literal AC3, 2026-07-07): an FKCS-internal gate rejection is not a real admission-path rejection, leaves the negative control silently admissible on the production admission path, and is the exact canned-green this story exists to kill. The harness-built gate looked identical to a real rejection until falsified.
- **Proxy cohort reported as genuine external.** Rejected: v2.0 uses in-house Chinese-wall proxy authors; genuine externals are v2.5.
- **Weakening existing gates or baselines.** Rejected: `check-kernel-baseline`, `abi-diff`, and `check-host-surface` keep their existing semantics and baselines.

## Consequences

- New dev-only crate: `maos-fkcs` for proxy cohort, admission fixtures, symbol-gate fixture, and score harness.
- New xtask gate: `check-fkcs` for FKCS oracle and ship-gate orchestration.
- New baseline artifact: `xtask/fkcs-baseline.toml`.
- New annotated git tag: `frozen-kernel-v2.0`.
- No source edits under `crates/maos-kernel-core/src`, `crates/maos-spirit-abi/src`, or `crates/maos-host/src`.
- `maos-registry` admission is **modified** by the AC3 amendment: it gains the `OffFrozenSurface { symbols }` variant + the early `[fkcs].internal_references` gate in `crates/maos-registry/src/admission.rs` (before tier/signature/ComplianceClaim resolution). Within the frozen-admission-baseline set (`maos-skill` / `maos-registry` / `maos-compliance`), **only** `admission.rs` changed; `maos-skill` and `maos-compliance` are unchanged. (L4 is redefined — see §3.)
- The frozen admission baseline (the content-hash `admission-path-unmodified` leg) now pins the `[fkcs].internal_references` parse + `OffFrozenSurface` surface in `crates/maos-registry/src/admission.rs` and the `[fkcs]`-fixture emission in `crates/maos-fkcs/src/lib.rs`; it no longer asserts byte-identity to the 11.4c admission crates.
- NFR-Test-5 is annotated as infrastructure delivered at v2.0 and populated genuine-external floor deferred to v2.5 (the N=12 black-box trial is owned by NFR-Test-8, not FKCS-populated).

## Gate

`check-fkcs` has eight independently falsifiable legs:

1. `frozen-tag-consistency` — tag target and TOML triple reconcile to live baselines.
2. `diff-oracle-derives` — forged self-report flags are ignored and real drift reds (real capture for green; synthetic snapshots only for mutation/fault injection).
3. `negative-control-rejects` — the off-frozen-surface negative control is **rejected at admission by `admit_spirit`** (`AdmissionError::OffFrozenSurface`, reason contains `off-frozen-surface`); a conformance proxy twin admits green at the same path; an always-admit blind (`AdmissionHarness::always_admit_for_test()`) reds the leg.
4. `proxy-cohort-fkcs-score` — three in-house proxy Spirits admit and derive/reconcile scores (itemized checklist; max 30, per-Spirit floor 27, aggregate floor 85/90).
5. `fault-inject-falsifiers` — kernel-line, ABI, host, and self-report falsifiers red the derived oracle.
6. `admission-path-unmodified` (repurposed — see §3) — content-hash (SHA-256) match over the declared admission files against the **redefined** admission baseline (the FKCS-parsing + `OffFrozenSurface` surface), proving the literal AC3 rejection is in place; the kernel-core/abi/host triple stays zero-delta.
7. `release-graph-absence` — `maos-fkcs` is absent from the release binary normal dependency graph.
8. `kernel-abi-diff` — `check-kernel-baseline` stays green at `23081`.

## Ratification

Ratified by the Story 11.5 preflight (Winston · Murat · John · Amelia · Vex, 2026-07-07) and **amended the same day for literal AC3** (user overruled the preflight two-axis model): F1 whole-story opus-4-8 + full §A6; F2 annotated tag + TOML triple; **F3 (amended): literal at-admission negative-control rejection by `admit_spirit` (`OffFrozenSurface`), with L4 redefined and the frozen admission baseline redefined to include the new rejection surface**; F4 oracle in `xtask` (real capture for green, synthetic only for mutation), proxy fixtures in `maos-fkcs`; F5 proxy score advisory at v2.0 and genuine-external floor at v2.5.
