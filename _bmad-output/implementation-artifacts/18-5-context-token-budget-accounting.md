---
story_id: '18.5'
story_key: 18-5-context-token-budget-accounting
created: 2026-10-25
context_commit: b50ea844
baseline_commit: b50ea844
security_sensitive: true
kernel_grant: "◆ FLAG-Winston, **RATIFIED 2026-10-26 by the operator (Lunarpulse), ADR-069 envelope** (`18-5-flag-winston-decision.md` §Rulings): D-18-5-02 A + 03 B + 10 A = **+27 physical / +23 tokei**: `check-kernel-baseline` 26387 → **≤ 26414** (95 files, unchanged), `maos-kernel-core` 20617 → **≤ 20640**, exact measured figure at landing (after `cargo fmt --all`). Files: `capability/mod.rs` (ADR-016 ledger built inside `CapabilityRegistryAdapter::new`, issuance gate for an exhausted pid outside the distil/hand-off exits, `context_budget()` accessor, slot drop in `revoke_all_for_pid`; +17 / +13), `mcp/mod.rs` (MCP use-time check; +6 / +6), `inference/mod.rs` (the `RateLimited` emitter spawns its delivery; +4 / +4). Any other kernel file or any figure above the grant stops the story and returns as a FLAG request. Each figure was measured at creation by a throwaway prototype at b50ea844 (fmt-clean, `cargo check --locked -p maos-kernel-core` clean), then reverted; after the revert `check-kernel-baseline` PASSED 26387 / 95. Alternatives measured, not taken: the adapter in the kernel +137 / +120; ledger and adapter in the kernel +259 / +214. ADR-069 after the grant: 2,360 to 23,000, 1,360 to the alarm."
kloc_grant: "RULED 2026-10-26 (operator, Lunarpulse): D-18-5-09 A — **Epic 18's named maos-bin budget 28558 → 28655** (forecast +40–75 [INFERENCE] × 1.3; measured so far +2 for the `hook_cap_override` literal under D-06 A); exact re-pin at landing; above 28655 → re-ask before landing. The other rows over their ceilings are **measured requests returned to the operator at review with exact figures**; the prototype figures are their expected values, and a dev figure **more than +25 % above its prototype value → STOP at dev** and re-ask: maos-capability 1347 → 1442 (+95; `xtask/kloc.toml:265`; STOP above +118); maos-iac 7769 → 7891 (+122 prototype; the unprototyped TL fallback and channels +12–24 [INFERENCE] fit; `:406`; STOP above +152); maos-domain 9805 → 9843 (+38; `:425`; STOP above +47); maos-manifest 4540 → 4572 (+32; `:415`; STOP above +40); maos-audit 7410 (`:409`) +4–8 [INFERENCE, not prototyped] (STOP above +10); maos-spirit-abi 1038 → 1042 (+4; ceiling 3000, `:270`, fits); maos-wasm-host 2255 (`:703`) +30–60 [INFERENCE, not prototyped] under D-05 A (STOP above +75); `_aggregate_hardfail` (`:718`, 184707) = measured + 1 at landing (prototype aggregate 185010). `spirits/*` is ungoverned (`xtask/src/kloc_check.rs:310-311`)."
review: "bmad-code-review under `docs/testing/proof-contract.md`. Dev and reviewer model `anthropic/claude-opus-5-5` only (fable only with operator permission, `proof-contract.md:57`). Security-critical: AC2 (the report port is fixed to the admitted pid and the manifest window; a Spirit cannot report for another pid or choose its own window), AC3 (guest-forged kernel kinds refused; the new kinds are an ABI extension) and AC5 (the issuance gate and the MCP use-time check: the exits set, the order policy → context → quota, typed refusal on every governed path). The review record names a security-focused layer on opus-5-5 for all three. FLAG-Winston sign-off on the kernel diff."
---

# Story 18.5: Context-token budget accounting (ADR-016)

Status: ready-for-dev

<!-- Created 2026-10-25 at b50ea844 by bmad-create-story under the lighter creation contract (no party-mode preflight). 18-0/18-1/18-8/18-2 done; 18-3 patched with landing fixes; 18-4 under review and treated as landing. Three read-only scouts (budget surfaces, report/refusal seams, governance), then direct re-reads of every range a decision rests on. The host's only cargo process (CARGO_TARGET_DIR=/mnt/build/cargo, `env -u CI -u GITHUB_ACTIONS`) ran `check-kernel-baseline` and `kloc-check --json` at HEAD, five throwaway prototypes (measured, then reverted) and `cargo check --locked --workspace` for the ripple. FLAG record `18-5-flag-winston-decision.md`. RULED 2026-10-26 by the operator (Lunarpulse): D-18-5-01…10 all as recommended (01 A, 02 A, 03 B, 04 A, 05 A, 06 A, 07 A, 08 A, 09 A, 10 A); kernel grant +27 / +23 ratified (ADR-069) → `ready-for-dev`. Rulings applied docs-only (no cargo). -->

> **The capability:** *a Spirit that fills its context window finds out before it runs out. It reports its own token count, the kernel never counts for it, and at 80 % and 95 % of the window its manifest declares it receives a typed `ContextPressure`, then a typed `ContextLimit`, once per crossing. Past 100 % its next tool call is refused `EContextExhausted` on every governed path, while the routes ADR-016 leaves open (distil through inference and memory, hand off over IAC) stay open. Distilling back under the window lifts the refusal.*

**Closes:**
- ADR-016 (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md:252-272`; decision `:256`, gate `:254`, 17-9 implementation note naming this story `:264-272`).
- PRD `prd/domain-specific-requirements.md:52`, reconciled with `:62` per D-18-5-07.
- Architecture §4.6's context-token clause (`4-kernel-design.md:418,431-437`) and the inventory rows that name this story (`epics/requirements-inventory.md:367,381,557`).
- The consumer that manifest `[budget].context_window_size` has lacked since Story 1b.5c (`crates/maos-manifest/src/manifest.rs:1113-1120`).
- Under D-18-5-10 A, the `RateLimited` frame that is never delivered (`crates/maos-kernel-core/src/inference/mod.rs:191-195`).
- No `deferred-work.md` row names this story (grep for `18-5`, `ADR-016`, `context_window` and `context-token` returns zero rows).

**Does NOT close, and says so:**
- **Honest reporting.** Counts are Spirit-self-reported by PRD design (`:52`). A Spirit that under-reports is never refused. The control is the refusal once a report crosses the window, not an audit of the number (rule 11: stated as a limit).
- **Spirits that call no inference.** Butler and Researcher report from their completions. A native Spirit with no port wired reports nothing and is never exhausted. Any Spirit can use the port; only these two are wired (labelled).
- **Native in-proc frame delivery.** Native Spirits learn through the synchronous typed status; the frame is journaled, but no mailbox pump exists for them (D-18-5-04).
- **An I12 digest-ref cardinality cap** (D-18-5-07: no such quantity is specified).
- 17-9's issuance window and its `cap.quota:*` rows, a different unit that is unchanged here except that a context refusal is checked before 17-9's charge.

## Story

As **a director who runs Spirits on models with finite context windows**,
I want **each Spirit told, in typed signals, when its self-reported context nears the window its manifest declares, and its new tool calls refused once it exceeds it — with distillation and hand-off left open**,
so that **a Spirit decides to distil, hand off or halt before it degrades, the kernel never guesses at tokens, and an overfull Spirit cannot keep acting on the world.**

**Contract:**
- Epic-18 §18-5 (`_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:185-212`, corrected in place at creation; creation corrections and rulings `:193-203`).
- Kernel-Δ (`:32`; the 18-5 grant `:36`), the kloc asks and rulings (`:40-48`; 18-5 budget `:48`).
- §R: R18-18 (`:79`, re-anchored), R18-20 (`:81`).
- Stories row `:106`; confidence rules 9–11 (`:50-52`); Dependencies `:257` (18-1 → 18-5), budget continuation `:259`.
- Epic-17 R17-204 (`epic-17-workers-and-third-party-form-w2.md:754`); D-17-9-FW-09 A and D-17-9-PF-R1 (`17-9-flag-winston-decision.md:21,61,71`; Spirit-facing typed frames stay ADR-016's).
- ADR-016 (`12-architecture-decision-records.md:252-272`); ADR-013 `:102-107` and ADR-030 `:54-58` (the name `CapError::ContextExhausted` reserved for this story).
- ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:20-25,35-46`); PRD `domain-specific-requirements.md:52,62`.
- `routing-audit-2026-10-22.md:3,71,107`.
- `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `b50ea844` by direct reads, scout maps re-verified by hand, `kloc-check --json`, `check-kernel-baseline`, five reverted prototypes and a ripple `cargo check --workspace`.

| Premise (epic / §R / routing) | Fact at `b50ea844` and consequence |
|---|---|
| `[budget].context_window_size` "parsed and validated … and enforced by nothing" | **Holds.** `Budget` `manifest.rs:1116-1120`, `RawBudget` `:1129-1134` (`deny_unknown_fields`), `validate` `:1136-1166` (window in [1, 2²⁴]). It reaches the SCB as `SpiritManifestBundle.budget` (`crates/maos-kernel-core/src/scheduler/control_block.rs:213-218`: "the kernel holds the value but never learns what it bounds"). Only `time_cap_seconds` is read (`scheduler/hook_dispatch.rs:357-361`, `crates/maos-bin/src/spirit_session.rs:135-141`). Manifest windows: Butler 8192 (`spirits/butler/manifest.toml:63`), Researcher 16384 (`spirits/researcher/manifest.toml:93`). |
| "`context_pressure_threshold`/`context_limit_threshold` exist in no crate" | **Holds**, and `deny_unknown_fields` means the `5-spirit-abi.md:62-65` sketch is refused today. → D-18-5-06. |
| "No `ContextPressure`/`ContextLimit` type exists" | **Holds** (code grep: only the doc comment `crates/maos-capability/src/cap_quota/mod.rs:28-30`). `CapError` has no `ContextExhausted` (`crates/maos-domain/src/ports/capability.rs:113-146`; 17-9's `IssuanceQuotaExhausted` `:135-137`); the name is reserved by ADR-013 `:106`, ADR-030 `:58` and ADR-016 `:271-272`. |
| "18-1's inference seam is where Spirit-self-reported counts originate" | **Half true.** The seam returns provider `usage` to the Spirit (`InferenceResponse` `crates/maos-domain/src/ports/inference.rs:83-91`, `TokenUsage` `:110-116`; cassette entries carry `usage`, `crates/maos-bin/src/cassette_replay.rs:92-101`), but both Spirits discard it (Butler `spirits/butler/src/lib.rs:1080-1081`, Researcher `spirits/researcher/src/lib.rs:979-985`). No Spirit → kernel path exists: `Ctx` has no report surface (`crates/maos-spirit-abi/src/ctx.rs:65-135`), and the WASM world imports only `recall` (`wit/spirit.wit:343-346`). → D-18-5-01, 05. |
| AC1 "it receives a typed `ContextPressure`" | Native in-proc Spirits have **no mailbox** in production: only the WASM/subprocess session calls `register_spirit_typed` (`spirit_session.rs:282`), and `fire_on_frame` has no production caller (`hook_dispatch.rs:306-309`). → D-18-5-04 (synchronous typed status + journaled frame). |
| AC3 "ADR-016's typed IAC frame … for native and WASM" | Kernel → Spirit typed kinds exist: `BudgetWarning` 12 / `BudgetExceeded` 13 (`crates/maos-spirit-abi/src/identity.rs:53-58`) and `RateLimited` 23. They are forwarded to WASM sessions (`spirit_session.rs:376-399`), and guest forgery is refused (`frame_refusal` `:769-780`; payload/kind check `:828-840`, both compiled only with `--features wasm-host`, `crates/maos-bin/src/lib.rs:70-71`). TL discriminants 26–29 are taken (`crates/maos-iac/src/adapter/transparency_log.rs:112-130`), so the new kinds are **30/31**. WIT `maos:spirit@2.1.0` has no context kinds, and `frame_kind_to_wit` is exhaustive and infallible (`crates/maos-wasm-host/src/frame_bridge.rs:71-92`; `BridgeError` `:25-33`). → D-18-5-05. |
| AC1 "its next tool call above 100 % returns `EContextExhausted`" | "Tool call" is undefined. Every governed path mints per call: Butler MCP `crates/maos-bin/src/main.rs:1202-1214`, Researcher MCP `crates/maos-bin/src/researcher_root.rs:220-224`, WASM frames `spirit_session.rs:713-752`, recall via `MediatedLogRecall`. Issuance is therefore the one kernel chokepoint, as 17-9's charge already shows (`crates/maos-kernel-core/src/capability/mod.rs:188-216`). MCP also has a use-time gate (`crates/maos-kernel-core/src/mcp/mod.rs:47-76`). → D-18-5-03. |
| (unstated) the kernel can emit a frame from a sync port | `IacBusAdapter::deliver_typed` is `async` (`crates/maos-iac/src/adapter.rs:328`). The only sync-port precedent drops the future unpolled: `let _ = iac.deliver_typed(frame, 0, None);` (`inference/mod.rs:191-195`), so `RateLimited` frames are never journaled or delivered [code-read; no test asserts the frame]. → D-18-5-04, D-18-5-10. |
| AC4 reconcile `:52` (above 100 %) with `:62` (above 95 %, I12 cardinality) | No I12 cardinality cap exists. The decorator only attaches refs (`crates/maos-iac/src/adapter/decision_logger.rs:52-56`), and a ref count is not a token count. → D-18-5-07. |
| "a zero literal at `crates/maos-bin/src/supervision.rs:1917`" | **Holds.** It is the `hook_cap_override` seam's placeholder `Budget` (`supervision.rs:642`, `:1914-1922`). Window 0 must mean accounting off, never a 0-token window. → D-18-5-08. |
| Routing audit `:107` cites §18-5 at `epic-18:104-107` | **Stale.** §18-5 is `epic-18-spirits-that-think-w3.md:185-212`. The audit file is outside this story's doc set and is not edited here (recorded). |
| ADR-069 `:20-25`: 18-5 among the unpriced ◆ stories | Priced here and ratified 2026-10-26: +27 / +23 (FLAG record). |
| FR4 / ABI | New kernel-written kinds need `NON_CALL_KINDS` and `kind_to_string` names (`crates/maos-audit/src/fr4_classifier.rs:59-70`), and a disposition for any new writer site (doorbell `crates/maos-audit/tests/fr4_classifier_16_2.rs:838`). New `FrameKind`s are an FR62 ABI extension (precedent: the 9.3b ratification record, `crates/maos-domain/src/governance.rs:215-219`). |
| Holds | PRD `:52`, `:62`; ADR-016 `:254-262`; `5-spirit-abi.md:62-65`; `requirements-inventory.md:367,381,556,557`; R17-204 `epic-17…w2.md:754`; `spirit_test/manifest.rs:163-165`; tracker row `sprint-status.yaml:281`. |

**Epic cites corrected in place (OLD → NEW @`b50ea844`):**
- ADR-016 `12-architecture-decision-records.md:252-260` → `:252-272` (decision `:256`, 17-9 note `:264-272`).
- `4-kernel-design.md:418,427` → `:418,431-437`. `:427` is the `cap-quota` issuance row; the context-token clause is the "Separate unit" paragraph.
- `manifest.rs:1117-1165` @15b02d98 **holds** (struct `:1116-1120`, `validate` `:1136-1166`). `supervision.rs:1917` **holds** (the `hook_cap_override` seam). `spirit_test/manifest.rs:164` **holds** (`:163-165`).
- AC2 `spirits/researcher/src/lib.rs:941` → completion `:979-981`, request `:966-976`. `:941` is now the `survey_bound` call inside `infer_survey` `:936-942`.
- "18-1's NEW `Butler::with_deferred_inference_port`" → delivered, `spirits/butler/src/lib.rs:1320-1327`; completion `:1080`.
- `crates/maos-bin/src/main.rs:5504` / `:5531` @15b02d98 → shared port builder `:3901-3934`, Butler seam fill `:3950-3997`, Researcher wiring `:4037-4065` (`researcher_root.rs` `construct` `:94-135`, `bind` `:161-181`).
- AC1 "a test named at creation" → `crates/maos-bin/tests/context_budget_18_5.rs`.
- R18-18 NEW gains the same re-anchors. The stories row's kernel-Δ now carries the measured figure (ratified 2026-10-26).

## Kernel and kloc pricing (E16-A7) — measured at creation

**Prototypes (throwaway, at `b50ea844`, `cargo fmt --all`, reverted):**
- Domain types (port, `CapError::ContextExhausted`, `FramePayload`/`ContextEnvelope`, `FrameKindLabel`), abi kinds 30/31, the maos-iac matches and the `maos-capability::context_budget` ledger were shared by every option.
- The kernel diff varied by D-18-5-02/03/10. Full figures and diff hashes are in `18-5-flag-winston-decision.md` §Measured figures.

| Option | Kernel physical | Kernel tokei | Files |
|---|---:|---:|---|
| **D-02 A** (ledger maos-capability, adapter maos-iac; gate + accessor + slot drop) | 26387 → **26404 (+17)** | 20617 → **20630 (+13)** | `capability/mod.rs` |
| **+ D-03 B** (MCP use-time) | **26410 (+23)** | **20636 (+19)** | + `mcp/mod.rs` |
| **+ D-10 A** (RateLimited polled) | **26414 (+27)** | **20640 (+23)** | + `inference/mod.rs` (each increment measured over A; disjoint files, so additive) |
| D-02 B (adapter in kernel) | 26524 (+137) | 20737 (+120) | + `capability/context_port.rs` |
| D-02 C (ledger + adapter in kernel) | 26646 (+259) | 20831 (+214) | + `capability/context_budget.rs`, `context_port.rs` |
| D-02 D (kernel 0) | 26387 | 20617 | none: by construction, not prototyped |

**kloc at `b50ea844`** (`kloc-check --json`, `passed: true`, `over_budget: []`):

| Crate | Measured / ceiling | 18-5 (measured by prototype / [INFERENCE]) | Consequence |
|---|---|---|---|
| maos-kernel-core | 20617 / 20617 (`xtask/kloc.toml:264`); pin 26387 (`kernel-core-baseline.toml:505`) | +23 tokei / +27 physical (recommended) | FLAG grant (D-02/03/10) |
| maos-bin | 28558 / 28558 (`:517`); Epic 18 named budget fully used | +2 measured (D-06 literal) + wiring +40–75 [INFERENCE] | **D-18-5-09** |
| maos-capability | 1347 / 1347 (`:265`) | +95 measured (ledger) | measured request |
| maos-iac | 7769 / 7769 (`:406`) | +122 measured (16 ripple + 106 adapter) + fallback/channels +12–24 | measured request |
| maos-domain | 9805 / 9805 (`:425`) | +36 measured (+2 for D-03 B) | measured request |
| maos-spirit-abi | 1038 / 3000 (`:270`) | +4 measured | fits |
| maos-manifest | 4540 / 4540 (`:415`) | +32 measured (D-06 A) | measured request |
| maos-audit | 7410 / 7410 (`:409`) | +4–8 (FR4 names) | measured request |
| maos-wasm-host | 2255 / 2255 (`:703`) | D-05 A +30–60; B +8–15; C +10–15 | measured request |
| aggregate | 184706; `_aggregate_hardfail` 184707 (`:718`) | prototype 185010 (D-02 A + D-06 A) | re-pin = measured + 1 |

`spirits/*` is ungoverned, so Butler's and Researcher's reporting and their typed handling of the status go there. The maos-bin remainder is root composition: per-Spirit adapter construction over the kernel registry and the IAC bus on every construction path, and the typed refusal mapping on each governed path.

## Decisions — RULED 2026-10-26 by the operator (Lunarpulse)

**Ruling record.** D-18-5-01 **A**, 02 **A**, 03 **B**, 04 **A**, 05 **A** (WIT 2.1.0 → 2.2.0 with `context.report`, in this story), 06 **A**, 07 **A** (one quantity, refuse above 100 %; PRD `:62` amended as a dated DELTA), 08 **A**, 09 **A** (Epic 18 named maos-bin budget → 28655), 10 **A** — every row as recommended. Kernel grant D-02 A + D-03 B + D-10 A = **+27 physical / +23 tokei** ratified with these rulings (ADR-069 envelope). Out-of-kernel rows over their ceilings are measured requests returned at review (prototype figures = expected values; > +25 % above expected → STOP at dev). Evidence and measured figures for each row: `18-5-flag-winston-decision.md`. CR-n are creation defaults carried into the rulings.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-18-5-01** | Where does the Spirit's own count enter? | **A — sync `ContextBudgetPort::report_context_used(used_tokens) -> Result<ContextStatus, ContextReportError>` (maos-domain), root-built per Spirit and fixed to its pid (lazy `pid_of`) and its manifest window/thresholds; Butler and Researcher report `usage.input_tokens + usage.output_tokens` after each completion** · B — `InferenceRequest.context_used` · C — the kernel reads provider `usage` (`inference/mod.rs:431-455`) · D — a reserved `EpistemicScalarPort` tag | **A.** It is Spirit-self-reported (PRD `:52`) and kernel-held. The pattern is the root-built port precedent (18-3 scalar adapter `main.rs:565-643`, 18-4 `PidBoundPrincipalPort`). B covers inference callers only, C is not self-reported, D routes a budget through the halt-policy path. | **RULED 2026-10-26 — A** |
| **D-18-5-02** | Where do the ledger, adapter and frame emitter live (kernel figure)? | **A — ledger `maos_capability::context_budget`; adapter `maos_iac::adapter::context_signal`; kernel: the registry builds the ledger in `new`, gates issuance, exposes `context_budget()`, drops the slot in `revoke_all_for_pid` (+17 / +13 measured)** · B — adapter in the kernel (+137 / +120) · C — all in the kernel (+259 / +214) · D — kernel 0, gate at maos-bin call sites | **A.** It keeps ADR-016's "Capability Registry" placement (the ledger beside `cap_quota`, read by the registry's own issuance path). maos-iac already depends on maos-capability and maos-spirit-abi, so no new edge. Every `CapabilityRegistryAdapter` construction path carries the ledger and the constructor is unchanged (the one-binding lesson). One slot-drop hook serves unload, crash and revocation (`scheduler_loop.rs:563`, `supervision/crash_detector.rs:127`, `revocation/applier.rs:125`). D is a bypassable per-path gate. | **RULED 2026-10-26 — A** (kernel +17 / +13 within the +27 / +23 grant) |
| **D-18-5-03** | What is a "new tool call", and where is it refused? (security-critical) | A — issuance gate after policy and before 17-9's charge, every scope except the exits `ProviderInfer`, `MemWrite`, `IacSend` → `CapError::ContextExhausted { spirit_id }` · **B — A + MCP use-time check → `McpError::ContextExhausted { server, tool }` (+6 / +6 over A)** · C — MCP use-time only · D — every scope | **B.** Issuance covers every governed path at one kernel point, because all of them mint per call. The use-time check closes the gap where an MCP token minted just before the crossing is spent just after. The exits are ADR-016's own routes: distil (`ProviderInfer` + `MemWrite`), hand off (`IacSend`); halting needs no token. A refused mint consumes no 17-9 issuance capacity. New scope kinds are refused by default. | **RULED 2026-10-26 — B** |
| **D-18-5-04** | How does the signal reach the Spirit? | **A — synchronous typed `ContextStatus { level, crossed }` to the reporter, plus a kernel-journaled `ContextPressure` (30) / `ContextLimit` (31) frame with `ContextEnvelope { spirit_pid, context_used, context_window_size, threshold_permille }`, one per threshold crossed upward, delivered to a registered mailbox, else a TL row; delivery spawned on the current runtime** · B — frame only + a native frame pump (first `fire_on_frame` caller) · C — status only | **A.** Native Spirits have no mailbox (`spirit_session.rs:282` is the only registration). The frame keeps ADR-016's typed IAC artifact and the audit trail, and reaches WASM through the existing forwarding (`:376-399`). The TL fallback follows `emit_budget_frame` (`hook_dispatch.rs:247-268`). Spawned delivery avoids the dropped future of `inference/mod.rs:195`. | **RULED 2026-10-26 — A** |
| **D-18-5-05** | The WASM form | **A — in this story: WIT `maos:spirit@2.1.0` → `2.2.0` (kinds, `context-envelope`, a `context` import `report: func(used-tokens: u32) -> context-status` mirroring `recall`), bridge and linker, four provenance-pinned components rebuilt, FR62 ratification record** · B — in this story, kinds only; the guest reports through an intercepted `TelemetryEvent` frame (maos-bin) · C — native only; the WASM half goes to a new named story `18-9-wasm-context-budget-surface` (operator ruling vs PRD amendment (b)) | **A.** The epic's AC2/AC3 name both forms. 17-3d's `recall` import is the precedent for a host-mediated sync call, and WASM sessions already receive kernel frames. Kernel 0 under every option. C is honest but leaves a routed obligation unmet. | **RULED 2026-10-26 — A** (WIT 2.2.0 with `context.report`, in this story) |
| **D-18-5-06** | Thresholds | **A — optional `[budget].context_pressure_threshold` / `context_limit_threshold` (f64 in (0, 1], pressure < limit, defaults 0.80 / 0.95), stored as permille so `Budget` keeps `Eq` (maos-manifest +32, maos-bin +2 measured)** · B — fixed ⌈80 %⌉ / ⌈95 %⌉ | **A.** PRD `:52` tracks a per-Spirit `context_pressure_threshold` with a "default 80%", and `5-spirit-abi.md:62-65` sketches the fields. | **RULED 2026-10-26 — A** |
| **D-18-5-07** | PRD `:52` vs `:62` | **A — one quantity, refusal above 100 % (ADR-016 gate `:254`, `:52`); `:62` amended to "above 100 %" as a dated PRD DELTA (T8), its I12 sentence read as the same Spirit-reported tokens** · B — refusal above 95 % (ADR-016 and `:52` amended) · C — a separate I12 cardinality cap | **A.** A digest-ref count is not a token count; turning refs into tokens is estimation, which `:52` forbids. No cap or figure exists for C. Docs only. | **RULED 2026-10-26 — A** (PRD `:62` amended as a dated DELTA) |
| **D-18-5-08** | Occupancy semantics | **A — absolute: the last report wins; latches re-arm below their threshold; exhaustion clears on a report ≤ window; window 0 = accounting off; a hot-swap successor keeps the pid's slot until its first report** · B — monotonic per incarnation | **A.** ADR-016 lets the Spirit decide to distil, which must be able to lift the refusal without a restart. | **RULED 2026-10-26 — A** |
| **D-18-5-09** | kloc outside the kernel | **A — Epic 18 named maos-bin budget 28558 → 28655; exact re-pin at landing; above → re-ask; other crates exact measured requests at landing; `_aggregate_hardfail` = measured + 1** · B — hold 28558; T0 returns a measured maos-bin request first · C — none possible (root-only composition) | **A.** The root work is construction and mapping: adapter fill ×2 (+12–20), window/thresholds from the bundle (+4–8), `GovernedMintError` variant/Display/mapping (+5), session verdict + refusal kinds (+3), Butler/Researcher MCP mapping (+6–12), MCP receipt reason (+4–8), CLI-wrapper guard (+2–4), literal (+2 measured) = +40–75 [INFERENCE], × 1.3 = +97. | **RULED 2026-10-26 — A** (maos-bin ≤ 28655; other rows measured requests at review, > +25 % over the prototype figure → STOP at dev) |
| **D-18-5-10** | Finding: the `RateLimited` frame is a dropped future | **A — fix at origin: spawn the delivery (+4 / +4 measured) and witness a kind-23 row on a 429** · B — route to `20-3c-audit-truth-and-halt-receipt-residuals` with a `deferred-work.md` row at the ruling | **A.** This story establishes the sync-port emission pattern (D-04) that the same defect violates; one review, four lines. | **RULED 2026-10-26 — A** |
| CR-1 | Ledger arithmetic | Per pid one slot `{window, pressure_at, limit_at, used, level}` in a `DashMap`. `pressure_at = ⌈window·p/1000⌉`, `limit_at = ⌈window·l/1000⌉` in u64. Level: `used > window` → Exhausted; `≥ limit_at` → Limit; `≥ pressure_at` → Pressure; else Within. `crossed` = the highest level above the previous one. Frames: one per threshold crossed upward, Pressure before Limit (a jump from Within to Exhausted emits both); entering Exhausted emits no frame (its signal is the refusal). Butler 8192 → 6554 / 7783; Researcher 16384 → 13108 / 15565. | — |
| CR-2 | Gate order | `issue_with_mediation`: policy → **context** → 17-9 charge → tokens. A context refusal writes no `cap.quota:*` row and consumes no issuance. `McpClientAdapter::check_capability`: context → verify → scope. | — |
| CR-3 | Construction paths (18-1 / 18-3 lesson) | `ButlerSeam` (`spirits/butler/src/lib.rs:280-303`, attach `:329-349`) gains a context-port factory taking `pid_of`. `ResearcherWiring` (`researcher_root.rs:71-80`) gains the same, bound in `bind` (`:161-181`). The boot arm, the door `load` and the hot-swap successor (`build_spirit_obj`, `main.rs:1035-1044` class arms) bind alike. The window and thresholds come from the admitted bundle, never from the Spirit. | — |
| CR-4 | Typed refusals per path | `GovernedMintError::ContextExhausted` (`worker_spawn.rs:348-370`, mapping `:426-436`). MCP refusal receipt `mcp.call.refused:context-exhausted` (beside `:517`). Session `kernel_verdict: "context-exhausted"` (`spirit_session.rs:743-749`). CLI-wrapper spawn refuses on both paths (as `worker_spawn.rs:1005-1016`). Butler and Researcher keep the refusal typed (no collapse into `ButlerMcpError::TokenIssuanceFailed`, `main.rs:1214`). Serde results in src are never `.unwrap/.expect/.unwrap_or*`. | — |

## Acceptance Criteria

*(Epic §18-5 as corrected at creation and ruled 2026-10-26 (D-18-5-01…10 all as recommended). AC6 is D-18-5-10 A's fix at origin. One plant per control AC, `docs/testing/proof-contract.md:26`.)*

**Inherited deferred rows owned by this story:** none (`deferred-work.md` grep: zero rows). Nothing filed at the rulings (D-18-5-10 A keeps the `RateLimited` fix here).

### AC1 — command: a Spirit learns it is near its window, and its next tool call above 100 % is refused (D-01, D-03, D-04, D-08)

1. Process leg (daemon-level), on a keyless runner (scratch `HOME`, `MAOS_OLLAMA_URL` unset):
   - The command `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/context-budget-18-5.json maos run spirits/butler/manifest.toml --clock compressed:4d --acceptance tests/fixtures/j-butler/acceptance-21d-steady.jsonl` exits 0. The `--clock`/`--acceptance` scene is 18-4's, landing.
   - The cassette is re-authored from the scene's day 1–4 prompts as 18-4 CR-3 did: real hashes, `provenance: seed`, and `usage` totals 4000 / 6600 / 7800 / 8300.
   - Each simulated day's line carries `"context": {"used", "window": 8192, "level", "crossed"}`, emitted Spirit-side.
   - Exactly one `crossed: "pressure"` (day 2), one `"limit"` (day 3) and one `"exhausted"` (day 4) appear.
   - `maos audit query` lists exactly one kind-30 row and one kind-31 row for the Butler pid, each a `ContextEnvelope`.
2. Refusal leg, `crates/maos-bin/tests/context_budget_18_5.rs`. It uses the `j_researcher_survey_18_3.rs` harness: a real `CapabilityRegistryAdapter`, `McpClientAdapter` over the replay MCP fixture, `LiveResearcherMcpPort`, and the maos-iac adapter.
   - After the Researcher reports 17204 / 16384, its next MCP mint is refused `CapError::ContextExhausted`. The Researcher returns a typed error (no fallback), and `mcp.call.refused:context-exhausted` is journaled.
   - A token minted before the report is refused at use with `McpError::ContextExhausted`.
   - `ProviderInfer`, `MemWrite` and `IacSend` still issue for that pid.
   - A report of 8000 lifts the refusal.
3. **Proven red (1 plant):** drop the latch (every report at or above pressure counts as a crossing) → the process leg prints two `pressure` crossings → red.

### AC2 — Spirit-self-reported, kernel-held, bound on every construction path (D-01, D-02, D-08, CR-3; security-critical)

1. `ContextBudgetPort` (maos-domain, `/// Class: universal-arithmetic`); ledger `maos_capability::context_budget` (CR-1); `maos_iac::adapter::context_signal::ContextSignalAdapter`, fixed to `pid_of`, window and thresholds. An unresolved pid → `ContextReportError::NotAdmitted`.
2. `CapabilityRegistryAdapter::new` builds the ledger (constructor unchanged); `context_budget()` exposes it; `revoke_all_for_pid` drops the slot.
3. Butler and Researcher report `input_tokens + output_tokens` after every completion (Butler `lib.rs:1080`; Researcher per claim, `:979-981`) and keep the last `ContextStatus`. Both are bound on the boot arm, the door `load` and the hot-swap successor (CR-3).
4. The kernel adds no read of `InferenceResponse.usage` for this purpose; the `CostAttribution` read (`inference/mod.rs:431-455`) is unchanged.
5. Witnesses:
   - Daemon-level door leg in `context_budget_18_5.rs`, using the 18-1 `butler_daemon_18_1` door pattern: a door-loaded Butler crosses pressure and its kind-30 row names the door pid; `unload` then `load` starts `Within`.
   - Library tests: a report for an unadmitted pid is refused; window 0 never exhausts.
6. **Proven red (1 plant):** skip the context-port binding on the door-load arm → the door leg finds no kind-30 row → red.

### AC3 — the signal reaches the Spirit, typed, and cannot be forged (D-04, D-05; security-critical)

1. New kinds `FrameKind::ContextPressure = 30` and `ContextLimit = 31` in maos-spirit-abi, the maos-iac TL enum, the span labels, the TL mapping and the recall labels. The payloads are `FramePayload::ContextPressure` / `ContextLimit` (`ContextEnvelope`). Per-kind mailbox channels (Mpsc, 32) sit beside `RateLimited` (`adapter/channels.rs:40`, `adapter/mailbox.rs:288-290`). Delivery is spawned on the current runtime; with no registered mailbox, the same payload is written as a TL row.
2. WASM (D-05 A):
   - WIT `2.2.0`: `frame-kind` + `frame-payload` variants and the `context` import. Bridge both directions (`frame_bridge.rs:71-114,224-320`); linker (`crates/maos-wasm-host/src/runner.rs:242-249`).
   - The four provenance-pinned components are rebuilt and re-pinned (`tests/fixtures/wasm/equiv-fixtures.provenance.toml`).
   - A WASM fixture Spirit reports through `context.report` and receives `context-pressure` through `handle-frame`.
3. A guest-sent `ContextPressure`/`ContextLimit` is refused `kernel_reserved_kind` (`spirit_session.rs:773-780`), and the payload/kind match (`:828-840`) gains both arms. The kinds are recorded as an FR62 ABI extension (governance record; `check-abi-ratification` green).
4. FR4: `context.pressure` / `context.limit` are added to `NON_CALL_KINDS` and `kind_to_string`, and any new `insert_frame_event` site gets a `WRITER_SHAPES` disposition (the doorbell test `fr4_classifier_16_2.rs:838` is green).
5. **Proven red (1 plant):** remove the two kinds from `frame_refusal` → a WASM guest's forged `ContextPressure` reaches its peer → red.

### AC4 — thresholds and the PRD reconciliation (D-06, D-07)

1. `RawBudget` accepts the optional `context_pressure_threshold` / `context_limit_threshold`:
   - Each must be in (0, 1] and pressure must be below limit; violations are typed `ManifestError`s.
   - Defaults are 800 / 950 permille.
   - Fixtures: `crates/maos-kernel-core/tests/fixtures/manifest/budget/` (well-formed / edge / rejected); fuzz seed `crates/maos-manifest/fuzz/corpus/manifest_parser/seed_budget.toml`.
   - The `spirit_test` harness surfaces both (`crates/maos-spirit-sdk/src/spirit_test/manifest.rs:27-28,77-80,163-165`).
2. Docs (D-07 A, T8):
   - PRD `domain-specific-requirements.md:62` amended by a **dated PRD DELTA** (operator ruling 2026-10-26, D-18-5-07 A) to "above 100 %", its I12 sentence and signals recorded as ADR-016's single Spirit-reported token quantity.
   - ADR-016 gains an 18-5 implementation note: exits, per-incarnation slot, self-report limit.
   - §4.6 `4-kernel-design.md:418,431-437` and `requirements-inventory.md:367,381,557` say "delivered by 18-5" at landing.
3. **Proven red (1 plant):** accept pressure ≥ limit → the rejection test admits a 0.95 / 0.80 manifest → red.

### AC5 — new tool calls above 100 % are refused, typed, on every governed path (D-03 B, CR-2, CR-4; security-critical)

1. `issue_with_mediation`: policy → context → charge → tokens. An exhausted pid is refused `CapError::ContextExhausted { spirit_id }` for every scope except `ProviderInfer`, `MemWrite` and `IacSend`. A refused mint leaves the 17-9 window untouched.
2. `McpClientAdapter::check_capability` refuses `McpError::ContextExhausted` before the wire, so no `McpInvocation` row is written.
3. CR-4 holds on every path: governed mint, MCP receipt, session verdict, CLI-wrapper spawn, Butler and Researcher.
4. Kernel witness `crates/maos-kernel-core/tests/context_budget_gate_18_5.rs`:
   - An exhausted pid is refused for `McpCall`, `FsRead`, `NetHttps`, `ProcExec` and `CliSubprocessSpawn`.
   - The exits issue.
   - The quota usage is unchanged by refusals.
   - `revoke_all_for_pid` clears the slot.
   - Another pid is unaffected.
5. Ledger tests `crates/maos-capability/tests/context_budget_18_5.rs`: ⌈⌉ thresholds at 8192 / 16384, re-arm, clear, window 0.
6. **Proven red (1 plant):** delete the issuance gate → AC1.2's MCP mint issues → red.

### AC6 — the `RateLimited` frame is delivered (D-18-5-10 A)

1. `emit_rate_limited_frame` spawns `deliver_typed` on the current runtime (no unpolled future). A kernel test drives a 429 through the rate limiter and finds one kind-23 TL row and, for a registered mailbox, the frame.
2. **Proven red (1 plant):** restore `let _ = iac.deliver_typed(…)` → no kind-23 row → red.

## Tasks / Subtasks

- [ ] **T0 — Preflight at the dev HEAD.** D-18-5-01…10 RULED 2026-10-26 (all as recommended); tracker `ready-for-dev` → `in-progress`. Re-anchor drift after 18-4's landing: `git diff --stat b50ea844 -- crates/maos-kernel-core/src/{capability/mod.rs,mcp/mod.rs,inference/mod.rs} crates/maos-capability crates/maos-iac/src crates/maos-domain/src crates/maos-spirit-abi/src crates/maos-manifest/src/manifest.rs crates/maos-bin/src/{main.rs,worker_spawn.rs,researcher_root.rs,spirit_session.rs,supervision.rs} spirits/butler/src spirits/researcher/src wit crates/maos-wasm-host/src`. Re-measure `kloc-check --json` / `check-kernel-baseline` (expect 26387 / 20617 unless something landed; if the pin moved, the +27 / +23 grant applies over the landed values and is re-stated in the record).
- [ ] **T1 — Domain, ABI, bus types** (AC3.1): port, `CapError::ContextExhausted`, `McpError::ContextExhausted`, `FramePayload` + `ContextEnvelope`, `FrameKindLabel`; abi kinds 30/31; maos-iac TL enum, span labels, TL map, recall labels, channels.
- [ ] **T2 — Ledger + kernel** (AC2.2, AC5.1–5.2): `maos-capability::context_budget` (CR-1); registry field, gate, accessor, slot drop; MCP use-time check.
- [ ] **T3 — Adapter** (AC2.1, AC3.1): `maos_iac::adapter::context_signal` (spawned delivery, TL fallback row).
- [ ] **T4 — Manifest thresholds** (AC4.1): `Budget`/`RawBudget`, validation, fixtures, fuzz seed, `spirit_test`; the `hook_cap_override` literal.
- [ ] **T5 — Spirits** (AC2.3): Butler and Researcher report after each completion, keep `ContextStatus`, surface it in the scene line (Butler) and in their pass outcome; typed handling of `ContextExhausted` refusals; seam fields.
- [ ] **T6 — Root** (AC2.3, AC5.3, CR-3, CR-4): adapter construction on every construction path; `GovernedMintError::ContextExhausted`; MCP receipt; session verdict + `frame_refusal` + payload/kind arms (`--features wasm-host`); CLI-wrapper guard. No new maos-bin src file is expected. If one is added, list it in `SCANNED_SOURCE_FILES` (`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:927`, 30 → 31).
- [ ] **T7 — WASM** (AC3.2, D-05 A): WIT 2.2.0, bridge, linker, component rebuild + provenance re-pin, fixture Spirit.
- [ ] **T8 — FR4, ABI, docs** (AC3.3–3.4, AC4.2): `NON_CALL_KINDS`, `kind_to_string`, `WRITER_SHAPES`; FR62 ratification record; PRD `:62`, ADR-016 note, §4.6, inventory; docs-site en + ko where 17-9 edited them.
- [ ] **T9 — Witnesses + data** (AC1, AC2.5, AC3.2, AC5.4–5.5, AC6): `context_budget_18_5.rs` (process leg, door leg, refusal leg), kernel gate test, ledger tests, maos-iac adapter test (one frame per crossing; TL fallback), WASM fixture test; the context cassette; a test pinning its entry count and usages. Σ ≤ 30 s per test binary (`xtask/test-budgets.toml:26`), no exception planned.
- [ ] **T10 — Plants and records:** one plant per control AC (AC1–AC6) in `18-5-evidence/plants.md` (command, output, wall-clock); a CI plant for AC5's gate (the refusal leg runs in `workspace-test-suite`, `discipline.yml:5569`).
- [ ] **T11 — Governance:** `cargo fmt --all`; the FLAG record's re-pin set in order (`kernel-core-baseline.toml:952-957`: classes, surface + enumerated diff incl. the `CapabilityRegistryAdapter` private-field reshape, `src_lines` + HISTORY, digests + `set_hash`, pin-test literals, `kloc.toml:264`, `RATIFIED_AT_EASING`). Kernel ≤ 26414 / ≤ 20640 exact (ratified grant). maos-bin exact ≤ 28655 (D-18-5-09 A). The other over-ceiling rows (maos-capability, maos-iac, maos-domain, maos-manifest, maos-audit, maos-wasm-host) are exact measured requests returned to the operator at review; any figure > +25 % above its expected value in `kloc_grant` → STOP and re-ask. Then `check-service-boundary`, `check-kernel-baseline`, `kloc-check`, `check-abi-ratification`, `check-test-cost`, `check-serde-error-handling`, `check-dev-record-completeness`.
- [ ] **T12 — Regression and record:** the suites below; revert `intent-lineage-coverage-report.md` if a run modifies it; populate the Dev Agent Record; reconcile story and tracker.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Domain | `crates/maos-domain/src/ports/{mod.rs,capability.rs:113-146,mcp.rs:136-138}`, new `ports/context_budget.rs`; `frame.rs:61-89`; `log_recall.rs:50-97` | no port, no kinds → port, error variants, payloads |
| ABI | `crates/maos-spirit-abi/src/identity.rs:30-121` | kinds ≤ 25 → + 30/31 |
| Bus | `crates/maos-iac/src/adapter.rs:350-372,512-561`; `adapter/transparency_log.rs:55-170`; `adapter/log_recall.rs:194-271`; `adapter/channels.rs:40`; `adapter/mailbox.rs:288-290`; new `adapter/context_signal.rs` | — → adapter + kinds |
| Ledger | `crates/maos-capability/src/lib.rs:18-21`, new `context_budget.rs` | — → per-pid ledger |
| Kernel | `crates/maos-kernel-core/src/capability/mod.rs:17-20,122-131,148-172,188-216,392-397`; `mcp/mod.rs:47-76`; `inference/mod.rs:191-195` (D-10) | no context gate → gate, accessor, slot drop, MCP check |
| Manifest | `crates/maos-manifest/src/manifest.rs:1113-1166` | window only → + thresholds |
| Spirits | `spirits/butler/src/lib.rs:280-349,1054-1082,1320-1327`; `spirits/researcher/src/lib.rs:359-363,650-677,936-987` | `usage` discarded → reported |
| Root | `crates/maos-bin/src/main.rs:1095-1245,3901-4065`; `researcher_root.rs:71-181,220-224`; `worker_spawn.rs:348-436,471-520,1005-1016`; `spirit_session.rs:743-749,769-780,828-840`; `supervision.rs:1914-1922` | — → bindings + typed refusals |
| WASM | `wit/spirit.wit:29-51,220-242,343-355`; `crates/maos-wasm-host/src/{frame_bridge.rs,runner.rs}`; `tests/fixtures/wasm/` | 2.1.0 → 2.2.0 |
| Audit | `crates/maos-audit/src/fr4_classifier.rs:59-70`, `lib.rs` `kind_to_string` | — → names |

### Current state that must be preserved

- **17-9's issuance window**: semantics, `cap.quota:*` rows and order (only a context refusal precedes the charge); `CapError::IssuanceQuotaExhausted` and its mappings.
- **17-11 entitlement** (`cap_policy`) and refusal vocabulary: `PolicyDenied` before any context check.
- **18-1 / 18-3 seams**: per-pass mint, boot refusal for an unregistered provider, `pass_verdict`; a Spirit with no window or no port behaves byte-identically.
- **18-4 scene** (landing): its lines gain the `context` field only. Exit line 4 and its steady twin are otherwise unchanged.
- **Hook time budget** (`BudgetWarning`/`BudgetExceeded`, `hook_dispatch.rs:186-268`): a different unit, untouched.
- **ADR-064 lattice**; the unset mode stays deterministic.

### Guardrails

- The kernel never estimates tokens; only Spirit reports move the ledger.
- The port is fixed to the pid and window at the root. A Spirit never receives the ledger or the registry.
- No silent loss: a frame that cannot be delivered is journaled; a refusal is typed on every path; no proceed-on-refusal (17-9 FW-06 posture).
- Kernel diff exactly the ruled files and figure; anything else returns as a FLAG request.
- One target dir; `env -u CI -u GITHUB_ACTIONS`; no cargo inside a `#[test]`; plants restored with `git checkout -- <file>`.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Once per crossing (AC1) | drop the latch | two `pressure` crossings |
| Binding on every path (AC2) | no binding on door load | door leg: no kind-30 row |
| Forgery refusal (AC3) | remove kinds from `frame_refusal` | forged frame delivered |
| Threshold validation (AC4) | accept pressure ≥ limit | 0.95 / 0.80 admitted |
| Issuance gate (AC5) | delete the gate | MCP mint issues |
| RateLimited delivery (AC6) | `let _ =` restored | no kind-23 row |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-capability -p maos-domain -p maos-spirit-abi -p maos-manifest -p maos-iac -p maos-audit
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test context_budget_gate_18_5 && cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test context_budget_18_5 --test butler_scene_18_4 --test j_researcher_survey_18_3 --test butler_daemon_18_1 --test cli_wrapper_exhaustion_17_9
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --features wasm-host -p maos-wasm-host
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p butler -p researcher
cargo run -q --locked -p xtask -- check-kernel-baseline && cargo run -q --locked -p xtask -- check-service-boundary && cargo run -q --locked -p xtask -- kloc-check --json
cargo run -q --locked -p xtask -- check-abi-ratification && cargo run -q --locked -p xtask -- check-test-cost && cargo run -q --locked -p xtask -- check-serde-error-handling
cargo fmt --all -- --check
```

### Review plan (proof contract `:31-57`)

- **Model:** `anthropic/claude-opus-5-5` (fable only with operator permission).
- **Security layer (AC2, AC3, AC5):**
  - The exits set and its default-deny for new scopes.
  - Gate order and quota accounting.
  - Pid/window fusion and the unadmitted-pid refusal.
  - Guest forgery of kinds 30/31.
  - The MCP use-time path.
  - Typed refusals with no fallback.
  - No personal content in envelopes.
- **FLAG-Winston:** kernel diff = the three ruled files; figures exact.
- **Focused suites:** as above; kernel `--lib` (kernel source changes).
- **Daemon-level witnesses** for the door and process legs.

### Previous-story intelligence

- **18-1 (done):**
  - The Butler seam and per-pass mint (`ButlerSeam`, root fill `main.rs:3950-3997`).
  - The review lesson that a seam is bound on every construction path (D-18-1-R1); refused passes are typed.
- **18-3 (patched):**
  - `researcher_root::{construct, bind}` and `pid_of`.
  - The `j_researcher_survey_18_3` harness the refusal leg reuses.
  - The 47 s test-budget exception (`xtask/test-budgets.toml`): keep process legs short.
  - The FR4 doorbell scanner fix (reads the kind argument).
- **18-4 (review):**
  - The scene driver and Spirit-side scene step (kloc C).
  - Serde results never unwrapped in src.
  - New maos-bin src files listed in the cohort smoke roster.
- **18-8 (done):** `McpError` additions and the kernel `io` grant pattern.
- **17-9 (done):**
  - Issuance quota, the policy → charge → issue order, and typed mappings on every path.
  - PF-05 freed `ContextExhausted`; PF-R1 kept Spirit-facing frames for this story.
- **17-11 (done):** `Scope::entitles` and the `PolicyDenied` vocabulary.

### Git intelligence

| Commit | Content |
|---|---|
| `b50ea844` | 18-4 dev pass + 18-3 landing fixes |
| `b96d123a` | 18-3 review patches (kernel +10 / +9) |
| `419f051f` | 18-1/18-8/18-2 done + 18-3 dev |
| `140b906b` | 18-8 (kernel +14 / +13) |

None touched `cap_quota`, `issue_with_mediation`'s order, `McpClientAdapter::check_capability` or `emit_rate_limited_frame` since 17-9.

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-5 `:185-212` (creation corrections and rulings `:193-203`), Kernel-Δ `:32` (18-5 grant `:36`), kloc `:40-48`, rules `:50-52`, §R R18-18 `:79`, R18-20 `:81`, stories row `:106`, Dependencies `:257,259`.
- `12-architecture-decision-records.md:252-272`; `4-kernel-design.md:416-437`; `5-spirit-abi.md:62-66`; `prd/domain-specific-requirements.md:52,62`; `requirements-inventory.md:367,381,556,557`; `epic-17…w2.md:754`.
- `routing-audit-2026-10-22.md:3,71,107`; `docs/adr/ADR-013-…md:102-107`; `docs/adr/ADR-030-…md:54-58`; `docs/adr/ADR-069-…md:20-25,35-48`; `docs/testing/proof-contract.md:10-29,31-57`.
- `17-9-flag-winston-decision.md:21,61,71`; `17-11-flag-winston-decision.md:42-44` (re-pin set format); `18-5-flag-winston-decision.md`.

## Dev Agent Record

### Story context creation

Created 2026-10-25 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `b50ea844`, clean tree.

**Inputs:**
- Three read-only scouts (budget surfaces; report/refusal seams; governance and deferrals).
- Direct re-reads of every decision-bearing range.
- `check-kernel-baseline` (PASSED 26387 / 95) and `kloc-check --json` (`passed: true`, aggregate 184706) at HEAD.

**Prototypes** (one cargo process; fmt-clean; kernel `cargo check --locked` clean):
- D-02 B (adapter in kernel): +137 / +120.
- D-02 A, adapter in maos-capability with a frame sink: +17 / +13.
- D-02 C (all in kernel): +259 / +214.
- D-02 A as recommended (adapter in maos-iac): +17 / +13.
- D-03 B MCP increment: +6 / +6.
- D-06 A manifest: maos-manifest +32, maos-bin +2.
- D-10 A: +4 / +4.
- `cargo check --locked --workspace` found the ripple: maos-iac ×3 matches, maos-wasm-host ×2; `spirit_session.rs` sits behind `wasm-host`.

**Revert:** all prototypes reverted: `git diff --quiet HEAD -- crates xtask spirits Cargo.lock Cargo.toml` exit 0, `check-kernel-baseline` PASSED 26387. After the revert `git status` showed only this story's docs.

### Agent Model Used

anthropic/claude-opus-5-5 (creation, 2026-10-25)

### Debug Log References

### Completion Notes List

### File List

- `_bmad-output/implementation-artifacts/18-5-context-token-budget-accounting.md` (A)
- `_bmad-output/implementation-artifacts/18-5-flag-winston-decision.md` (A)
- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md` (M: §18-5 creation corrections, R18-18 re-anchor, stories row)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (M: 18-5 row comment)

### Change Log

- 2026-10-25: Story created (backlog; D-18-5-01…10 PROPOSED) at `b50ea844`. Findings:
  - The inference seam returns `usage`, but both Spirits discard it, and no Spirit → kernel report path exists (D-01).
  - Native Spirits have no mailbox (D-04).
  - "Tool call" is undefined, and issuance is the universal chokepoint (D-03).
  - The `RateLimited` frame is an unpolled future (D-10).
  - PRD `:62`'s I12 cardinality is not a token quantity (D-07).
- Kernel priced by five reverted prototypes. Recommended +27 / +23 (26414 / 20640); alternatives +137 / +120 and +259 / +214.
- Epic §18-5, R18-18 and the stories row re-anchored in place; tracker comment refreshed; FLAG record `18-5-flag-winston-decision.md`.
- 2026-10-26: RULED (operator, Lunarpulse): D-18-5-01 A, 02 A, 03 B, 04 A, 05 A (WIT 2.1.0 → 2.2.0 with `context.report`, in this story), 06 A, 07 A (one quantity, refuse above 100 %; PRD `:62` amended as a dated DELTA), 08 A, 09 A (Epic 18 named maos-bin budget 28558 → 28655; exact re-pin; above → re-ask), 10 A (`RateLimited` delivery fixed here). ◆ FLAG-Winston kernel grant **+27 physical / +23 tokei** (26387 → ≤ 26414, 20617 → ≤ 20640) ratified, ADR-069 envelope. Out-of-kernel rows over their ceilings: measured requests returned at review (prototype figures = expected values; > +25 % → STOP at dev). AC6 unconditional; AC4.2 PRD amendment as a dated DELTA. Applied docs-only (no cargo). Status → `ready-for-dev`.
