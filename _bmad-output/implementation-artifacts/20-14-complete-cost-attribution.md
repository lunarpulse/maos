---
story_id: '20.14'
story_key: 20-14-complete-cost-attribution
created: 2026-11-03
context_commit: 8c809f9c
baseline_commit: 8c809f9c
security_sensitive: true
kernel_grant: "RULED (operator 2026-11-03, ask d_batch4; D-20-14-02 B + D-20-14-03 B; FLAG-Winston, `20-14-flag-winston-decision.md`): KT token-bound cost scope measured at 8c809f9c from 26841 physical / 21042 tokei, +35 / +25. Creation-priced ceilings (STOP above): kernel ≤ 26911 physical / 21092 tokei; L2 kernel last. Binding at dev start only after re-base onto then-landed pins and cumulative room; the later kernel lander re-bases. Recorded cumulative measured 21203; this story at ×2 21228; both 20-9 and 20-14 at ×2 21249. D-20-14-12 C (ask d2014_12) grants joint targeted STOP 21250 for 20-9 L2 + 20-14 L2 only; shared STOP 21209 for everything else; D-20-9-15 A retains 21230 as 20-9 single-story ceiling; ADR-069 alarm 22000 untouched."
kloc_grant: "RULED (operator 2026-11-03, ask d_batch4) (D-20-14-09 A; D02 A ×2 measured / ×3 unmeasured). Creation-priced ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1; each ceiling is HEAD + allowance and re-bases onto the exact pins at landing): kernel ≤ 26911 / 21092; maos-domain ≤ 10101; maos-cli ≤ 6524; maos-audit ≤ 7779; maos-bin ≤ 34441; aggregate ≤ 192974. Binding at dev start only after re-base onto then-landed pins; creation-wide pricing below is historical evidence, not a reservation. Every other crate 0."
landing_1_grant: "D02 A rebase: domain 10198 + 2 = 10200; CLI 6429 + 95 = 6524; audit 8179 + 58 = 8237; bin 36225 + 250 = 36475; aggregate 198914 + 455 = 199369; kernel unchanged 26841/21042. Orchestrator-adjudicated overdrafts, disclosed to the operator: domain 10200 -> 10215 (d1408 wrapper, initial 10207 + 8); CLI 6524 -> 6545 (formatted report and new in-source witness, measured 6528 + 17). Review corrections (security F1-F8 + triage P1-P4), orchestrator-adjudicated overdraft D02 A, disclosed to the operator: CLI ceiling 6545 -> 6610 (before: pin 6529 / ceiling 6545; after: measured 6599 formatted, ceiling 6610); audit stays inside 8237 (measured 8227); iac 8728 -> 8770 for round 3 (R2-1/R2-2, escaped-value discovery, digest form): orchestrator-adjudicated D02 A overdraft, disclosed to the operator (before: pin 8726 / ceiling 8728; after: measured 8770, ceiling 8770); round-3 CLI measured 6609 inside 6610; domain 10203 inside the standing 10215 overdraft; aggregate measured 199146 inside the rebased 199378, hardfail 199147. Later 20-9 and 20-5 domain draws re-base onto the exact landed pin."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): a kernel FLAG-Winston row (D-20-14-02), an attesting audit-record writer (the cost frame, rule 11) and an anti-spoof principal source (AC3). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC2.3 (no Spirit reaches the scope binding), AC3 (principal source, SR-3, SR-4) and D-20-14-04, Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.14: Complete cost attribution — per task, per session principal

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


Status: in-progress

<!-- Created 2026-11-03 at 8c809f9c (main; `git log -1 -- crates xtask` = 8c809f9c) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30). D-20-14-01…12 were ruled as recommended by the operator on 2026-11-03 (ask `d_batch4`); D-20-14-12 C carries the joint targeted exception (ask `d2014_12`). Grants bind at dev start after re-base onto then-landed pins and cumulative room. No cargo ran (parent rule): cites re-read at HEAD (`creation/cites-8c809f9c.md`); kernel and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); the epic's AC1 scene run on the prebuilt binaries in an isolated home (`creation/runtime/`). Stale E20 §20-14 cites are corrected in place and recorded in `creation/records-owed.md` §A. -->

**Creation rationale — ready-for-dev at filing, not blocked.** Every AC is hermetic: replay cassette, isolated home, no network,
no paid key. The kernel row is a ruled measured FLAG-Winston grant (D-20-14-02 B), binding after dev-start re-base, and lands last; L1 needs no kernel line.
Preflight places this story at creation-sequence step 5, before 20-5 (PF:142); it serializes with 20-4's binding-site
and `inference/mod.rs` edits (Overlap).

> **The capability:** *`maosctl audit cost-reconcile` shows the operator what each task cost and on whose behalf —
> one row per task, session principal, Spirit, provider and model — and says which rows rest on a bound session
> principal and which on the write-target proxy, instead of pooling two sessions into `host-unallocated`.*

**Closes:**
- Epic-20 §20-14 (E20@HEAD:319-332) as amended 2026-10-30 (D01 A; PF §3.6): epic AC1 → AC1 + AC4; AC2 → AC2; AC3 → AC3;
  AC4 → AC5.
- FR64's v1.0 half (`prd/functional-requirements.md:107`, DELTA-2026-10-25): token spend per provider attributed per
  Spirit, per task and per session principal — for spend through the Inference Port (D-20-14-06). Owner row
  `deferred-work.md:80`.
- NFR-Cost-1's per-Spirit / per-task / per-principal clause (`prd/non-functional-requirements.md:179`); owner row
  `deferred-work.md:91` re-scoped (AC6).
- ADR-046 R3 / SR-5 debt ("task_ref dropped", "per-call principal binding is the durable fix", `ADR-046:38-40,79-81,88`).
- `tests/coverage-matrix.yaml:899-904` (FR64 `gates: []`) and `:1103-1108` (NFR-Cost-1) name witnesses (AC6).

**Does NOT close, and says so:**
- CPU-time and storage-I/O dimensions (v1.1, PRD DELTA-2026-10-25; ADR-046 R6; `cost.rs:18-30`).
- NFR-Cost-1's ≥ 98 % monthly reconciliation against provider invoices — operational (ADR-046 §7); replay tokens
  prove the arithmetic, never a vendor bill (D-20-14-11).
- Worker CLI spend through the egress proxy: it never crosses the Inference Port and no usage parser exists
  (`crates/maos-egress/src`, K10) — labelled in the report and the doc, routed (D-20-14-06).
- Multi-tenant cost isolation (NFR-Tenancy-1, v2.2); an authenticated end-user identity per request (D-20-14-04 B/C).
- Uninstall does not scrub cost frames (20-11 PF:129); `maosctl forget` does (SR-3, AC3.4).

## Story

As **an operator accounting for inference spend per tenant on a single-tenant root**,
I want **every cost frame to name the task it ran under and the session principal I bound, and the reconcile report
to group by task, principal and Spirit and say which attribution source each row rests on**,
so that **FR64's "no enterprise deployment without per-tenant cost accounting" is a report I can read, not a
`host-unallocated` pool.**

**Contract:**
- Epic §20-14 (E20@HEAD:319-332) as amended 2026-10-30 (D01 A; preflight §3.6, rows `:87`, `:131`, `:258-261`) —
  create-story may tighten, not add uncited ACs: AC4 is epic AC1's report half, AC5 epic AC4, AC6 the records.
- Tracker `sprint-status.yaml:316`; E20 confidence rules 4, 6, 8–11 (E20:59-61); proof contract
  `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review, tiers).
- ADR-046 (posture: observability, not invoice; R4 no money in the frame; R5 sum-then-round; SR-1…SR-5) stays in
  force; R3 and §10 are amended in place, never contradicted silently.

## Inventory at HEAD — measured, not assumed

Method: every cite read at `8c809f9c` (`creation/cites-8c809f9c.md` C1–C19, K1–K10); the epic's AC1 scene run on the
prebuilt binaries in an isolated home (`creation/runtime/`); no cargo.

| # | Charter item | Disposition | Code at HEAD | Witness / measurement | AC |
|---|---|---|---|---|---|
| 1 | Task on the cost frame | **missing** | payload `cost.rs:85-106` (no task field); negative test `no_task_ref_in_payload` `:247-268` | measured: two turns → two frames, no task key | AC2, AC4 |
| 2 | Task source "SCB marker set from the dispatched `task.assign`" | **no production writer possible as written** | no Inference Port caller is ever dispatched a `task.assign` (delegation spawns a Worker subprocess, `delegation.rs:10-14,540-560`; installed dispatch `installed_spirits.rs:141-176`); ledger written for Workers only (`supervision.rs:2290-2309`); port callers mint per turn/pass (`shell_host.rs:112-122`; seam mint `main.rs:4179-4200`) | C13 | AC2 (D-01, D-02) |
| 3 | `InferenceRequest` correlation id, "Added-only, `abi-diff`" | **abi-diff blind to it** | `inference.rs:28-45` (`#[non_exhaustive]`); `abi-diff` reads `maos-spirit-abi` only (`xtask/src/abi_diff.rs:8`) | C12 | D-03 |
| 4 | Session principal | **missing** (constructed nowhere) | `AttributionSource::SessionPrincipal` "Future" `cost.rs:60-61`; emission hard-codes `WriteTargetProxy` `inference/mod.rs:448`; proxy `:404-425` over `transparency_log.rs:1495-1517` (principal writes authorized per pid, any principal: `memory/mod.rs:405-412,734`) | measured: `MAOS_SESSION_PRINCIPAL=alice` read by nothing → `Unattributed` | AC3 |
| 5 | Report axes | **task and source absent; Spirit = boot-local pid** | key `(principal, pid, provider, model)` `subcommands.rs:4850-4851`; fold `:4868-4873` | measured: two sessions → **one** row `host-unallocated pid=1 100/60` | AC1, AC4 |
| 6 | Pre-20-14 frames readable | **holds** (no `deny_unknown_fields`) | `cost.rs:85-106`; unknown dimension key is an `Err` (`:270-311`) | — | AC4.1 |
| 7 | SR-3 erasure of a new principal field | **covered by the existing body scan** | `transparency_log.rs:1519-1525,1885-1953`; cost frames in `forget_cascade_18_6.rs:184,432` | measurement 7 owed | AC3.4 |
| 8 | Operator cost page | **absent** | only `docs/adr/ADR-046-…md` names `cost-reconcile` | C18 | AC5 (D-10) |
| 9 | Worker CLI spend | **not attributed** | egress has no usage parser (K10) | — | D-06 |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-8c809f9c.md`. Applied in place to E20 §20-14 (`creation/records-owed.md` §A).

| Premise (epic / preflight / tracker) | Fact at `8c809f9c` and consequence |
|---|---|
| Kernel pin 26361/20594 (E20:321), 26409/20618 (E20:323, PF:261) | **26841 / 21042** (C1). |
| Proxy `inference/mod.rs:375` / `:383`; emission `:406-426` / `:424-427` | call `:404-407`, fold `:408-425`, emission `:437-461`, `WriteTargetProxy` `:448` (C2–C4). |
| CLI `cli.rs:667` / `:729`; `build_cost_report` `:4614`, `:4622-4645` / row `:4722`, builder `:4790` | `cli.rs:746-763`; row `subcommands.rs:4772-4782`, `build_cost_report` `:4842-4983`, entry `:5023-5100` (C5, C6). |
| "SCB current-task marker set from the dispatched `task.assign`" (E20:328, PRD `project-scoping-phased-development.md:412`) | No Inference Port caller receives a `task.assign`; a marker set from one would be always-None in production — ADR-046 R3's own objection. The unit of work is the root's turn/pass mint (C13) → D-20-14-01, -02. |
| "`InferenceRequest` gains a correlation id (Added-only, `abi-diff`)" (E20:328) | `abi-diff` cannot see `maos-domain` (C12); a Spirit-supplied id is not authoritative → D-20-14-03. |
| "the operator cost page" (E20:330) | none exists (C18) → D-20-14-10. |
| "Principal is never caller-supplied" (E20:329, FR64 errata) | still true of the request; the proxy itself is writable by the attributed Spirit (per-pid write authorization, C15) — why the proxy stays tagged and fallback-only. |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) run
from a `git archive 8c809f9c` copy root per prototype; the untouched copy reproduces the tree exactly (26841 / 95 files
/ set hash `db39bc4a…`; kernel 21042 tokei; aggregate 192519). Detail and diffs: `creation/measurements.md`,
`creation/kloc-kernel-8c809f9c-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, ruled 2026-11-03; `20-14-flag-winston-decision.md`):**

| Shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| **KT** scope bound to the token at mint, read at emission by the verified token (D-02 B, **recommended**) | **+35 (26876)** | **+25 (21067)** | — | `kt-token-bound-scope.diff` |
| KM SCB marker + `ScbTable` alias + adapter `with_scbs` (D-02 A, epic literal) | +44 (26885) | +30 (21072) | both construction sites wire it | `km-scb-marker.diff` |
| KM without the alias | +59 (26900) | +47 (21089) | — | `km-naive-scb-marker.diff` |
| KP root `CostScopePort` read at emission (D-02 C) | +31 (26872) | +25 (21067) | maos-domain +7 | `kp-root-cost-scope-port.diff` |
| K0 token bytes on the cost row, report-side join (D-02 D) | 0 (content hash moves) | 0 | task/principal never on the frame | `k0-token-on-cost-row.diff` |
| KR add-on: `correlation_id` → `request_ref` (D-03 A) | +1 | +1 | maos-domain +9 | `kr-request-correlation-id-on-km-naive.diff` |

Cumulative (STOP 21209): recorded expectation 21125 (20-1b +68, 20-4 +9, 20-11 +6) → 21150 with KT. The other kernel
rows filed PROPOSED the same day at the same HEAD — 20-8 +33 (`20-8-flag-winston-decision.md`), 20-9 R +21
(`20-9-flag-winston-decision.md`), 20-13 KT3 −1 (`20-13-flag-winston-decision.md`) — bring the measured cumulative to
**21203**, 6 under the STOP (797 under the 22000 alarm). At this story's ×2 ceiling (+50) it is **21228**; with 20-9 at
its ×2 ceiling too, 21249 — **above the STOP** → **D-20-14-12** (split / exchange / bump); 20-9 raised the same
question as D-20-9-15 (targeted STOP 21230). These creation calculations are historical: D-20-14-12 C now grants joint targeted STOP 21250 for 20-9 L2 + 20-14 L2 only (ask `d2014_12`); shared STOP 21209 otherwise, D-20-9-15 A single-story ceiling 21230, and ADR-069 alarm 22000 are unchanged. Not compiled ([INFERENCE]; measurement 1).

**kloc forecast** (tokei; HEAD = every pin; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · KT (AC2, AC3) | **+25** (physical +35) | prototype `kt` |
| maos-domain · `task_ref`, `CostScope`, R3 test replaced (AC4.1) | **−1** | prototype `dom` |
| maos-cli · report axes, rows, plain line (AC4.2) | **+25** | prototype `nk` |
| maos-cli · Worker-CLI note, `task: none` legend (AC4.3) | +5…+15 [INFERENCE] | `subcommands.rs:4986-5021` |
| maos-audit · `spirit_names` (AC4.2, D-05 A) | **+29** | prototype `nk` |
| maos-bin · `MAOS_SESSION_PRINCIPAL` row + parse, shell turn binding (AC2.2, AC3.1) | **+35** | prototype `bin` slice |
| maos-bin · seam mint per pass, one-shot hello, `maos run` boot threading (AC2.2) | +20…+60 [INFERENCE] | `main.rs:4179-4200,9601-9615` |
| maos-iac, xtask | 0 | K5; no gate change |
| tests, ADR-046, docs ×3, coverage matrix | uncharged | `xtask/src/kloc_check.rs` |
| **aggregate** | **+113 measured; +138…+188 total** | prototype `rec` = 192632 |

**Envelope (D-20-14-09 A, ruled 2026-11-03; ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | +35 / +25 | **26911 / 21092** | 0 | **yes — FLAG-Winston** |
| maos-domain | 10099 | −1 | **10101** | 0 | yes |
| maos-cli | 6429 | +25 meas, +5…+15 | **6524** (+50 + 45) | 427 numeric (pin 6856 reserved; STOP 6666, D-20-1-05 A) | yes (the reserve is 20-1's) |
| maos-audit | 7721 | +29 meas | **7779** | 0 | yes |
| maos-bin | 34191 | +35 meas, +20…+60 | **34441** (+70 + 180) | 0 | yes |
| aggregate | 192519 | +138…+188 | **192974** (+455); hardfail = measured + 1 | 1 | yes |

Interaction: other unlanded stories claim the same crates — 20-4 (bin L1 measured 34363 in the working tree, ceiling
34643; domain 10107, ceiling 10115), 20-11 (audit ≤ 8261, bin ≤ 34698), 20-1b (kernel, cli). Each ceiling above is a
delta re-based onto the exact pins at this story's landing (the 20-4 L1 precedent: 34191 + 145 × 2 + 54 × 3 = 34643).
Each figure lands exact after `cargo fmt --all`, with a HISTORY row whose first segment cites this key with `->`
(D11 A); a lower measurement lowers the pin.

**Operator-ruling list (ruled as recommended 2026-11-03, ask `d_batch4`; binding after dev-start re-base):** (1) FLAG-Winston kernel row KT (D-20-14-02 + -03; KM/KP/K0/KR priced);
(2) maos-domain ≤ 10101; (3) maos-cli ≤ 6524; (4) maos-audit ≤ 7779; (5) maos-bin ≤ 34441; (6) aggregate ≤ 192974,
hardfail measured + 1; (7) D-20-14-01…12 (12: the cumulative kernel STOP).

## Decisions — ruled 2026-11-03 (operator, ask `d_batch4`)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-14-01** | What a "task" is on the cost frame | **A** the root's unit of work, bound where the root mints the Spirit's `ProviderInfer` token: one shell turn (`shell.turn:<ulid>`, `shell_host.rs:112-122`), one seam pass (`<spirit>.pass:<ulid>`, the shared mint `main.rs:4179-4200` used by Butler `spirits/butler/src/lib.rs:1175`, Orchestrator `orchestrator_root.rs:90`, Researcher `researcher_root.rs:204`), the one-shot hello run (`main.rs:9601-9615`); a `task.assign` that reaches a port caller names its frame id (`task.assign:<hex>`) — none does at HEAD · **B** the `task.assign` frame id only (epic literal): every Inference Port caller reports `task: none` at HEAD, AC1 reachable only through a test-only dispatch · **C** the capability token id itself, no new id | **A.** B is ADR-046 R3's "structurally always-None" field again (C13): no port caller is dispatched a `task.assign`. C equals A's granularity at every HEAD site but ties the report's task axis to token TTL and loses the kind. A names the units an operator actually runs; T0 measurement 3/4 classifies every mint site and whether a pass mints once. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-02** | Where the scope lives and how emission reads it (◆ FLAG-Winston) | **A** KM SCB marker (`set_cost_scope`/`cost_scope`), adapter reads the SCB table — +44/+30 · **B** KT scope bound to the token in the capability registry, read by the verified token id — +35/+25 · **C** KP root `CostScopePort` consulted at emission — +31/+25, domain +7 · **D** K0 token bytes on the cost row, the report joins a root-written context row — 0/0 lines | **B.** Every port caller already mints per turn/pass (C13); the token is verified before emission (`inference/mod.rs:250-272`); a token-keyed scope cannot go stale after a failed turn or leak across two concurrent tasks of one Spirit (the ledger is a `Vec` for that reason, `control_block.rs:278`) — a single SCB marker has no end-of-pass hook to clear it at HEAD; neither adapter construction site (`main.rs:4073,4100`) needs wiring, where A and C fall back silently to the proxy at any site that forgets `with_*`. C moves the table into the root for the same kernel cost and more bin lines; D leaves the PRD DELTA's "on the cost frame" false. Charter wording ("SCB marker") is amended in the records (§A). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **B** |
| **D-20-14-03** | `InferenceRequest` correlation id | **A** `correlation_id` via a builder (`#[non_exhaustive]`, `new` unchanged), journaled as Spirit-declared `request_ref`, never the task (+1/+1 kernel, domain +9) · **B** no request field; the root-bound scope is the only task source; E20 AC2 / PRD DELTA wording amended · **C** the request id as the task source | **B.** Nothing consumes a sub-task id at HEAD; a Spirit-supplied value on a billing surface is what ADR-046 R3 refused ("no synthesized weak key"); C lets the attributed party choose its task. A remains open as an additive later change. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **B** |
| **D-20-14-04** | Session principal source (never caller-supplied) | **A** boot-bound `MAOS_SESSION_PRINCIPAL` (new UserFacing env row; the D-19-2-08 A rule: unset/blank none, `:`/control/non-UTF-8 refuse the boot), bound by the root into every scope it opens that boot; two principals = two sessions · **B** per-dispatch principal (shell flag, door body field) bound per task — operator-asserted per request · **C** IdP-verified only: `AuthenticatedPrincipal` from `MAOS_SSO_ASSERTION` (`worker_spawn.rs:418-430`, `enterprise_identity.rs:307-338`) · **D** A, and when SSO is configured the verified subject is the session principal (a different `MAOS_SESSION_PRINCIPAL` refuses the boot) | **A.** Single tenant per kernel through v2.0 (NFR-Tenancy-1): the operator who boots the root is the authority; the shape and validation are 19-2's ruled `MAOS_BUTLER_PRINCIPAL` (`main.rs:4132-4150`); measured slice +35 bin. B widens the ADR-062 door surface; C makes the v1.0 claim unreachable without an IdP; D is A plus +10…+20 bin if the operator wants IdP strength now. `MAOS_BUTLER_PRINCIPAL` (picks namespace) is unchanged and does not imply a session principal. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-05** | The "per Spirit" axis | **A** read-time name from `(boot_nonce, pid)` via kind-19/kind-7 rows (`maos_audit::spirit_names`, +29; legacy frames included); pid kept as a column · **B** stamp the Spirit id on the payload at emission (root puts it in the scope; kernel +1; legacy frames unnamed) · **C** pid only (documented) | **A.** Measured: two boots admit hello-spirit at pid 1 and the report pools them (`creation/runtime/`); boot-local pids repeat across boots (`deferred-work.md:1329`). The identity rows exist for the shell path (measured) and are already the name authority (`maos-audit/src/lib.rs:1679-1767`). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-06** | Worker CLI spend (egress proxy) | **A** out of scope here, labelled: the report and the doc say Worker CLI spend is not in cost frames; a `deferred-work.md` row asks the operator to assign a destination (proposed: the v1.1 cost-dimensions work, ADR-046 R6) · **B** in scope: parse provider usage from proxied responses (JSON and SSE) and journal cost frames at the Worker's pid and task — +250…+500 [INFERENCE], maos-egress/maos-bin · **C** narrow FR64's v1.0 text to Inference Port spend by a PRD DELTA | **A.** The ruled DECIDE-2 charter is the cost frame's task/principal axes (E20:319-330); no E20 story touches the egress proxy, so rule 10 has no (a)/(b) destination and the operator must name one; the label keeps the report from implying completeness (X5 in the 2026-10-28 research). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-07** | Payload schema version | **A** `schema_version: 2` for frames written by this story; readers accept 1 and 2 · **B** keep 1 (the field is additive and skipped when absent) | **A.** ADR-046 R6 stamps the schema so a reader can tell a frame that could carry a task from one that could not; `task: none` on a v2 frame means "no scope bound", on v1 "pre-20-14". | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-08** | Landing shape | **A** one key, two landings: L1 kernel-free (AC4, AC5, AC6 partial), L2 kernel + root binding last (AC1–AC3, AC6 close); `done` after L2 · **B** one landing · **C** split key | **A.** Kernel last (20-1a/1b, 20-4 precedent); L1 ships the reader of the new field before any writer, so frames written by L2 are never unreadable; KT's root binding calls a kernel API, so it lands with L2. C adds a key and the doc-gate churn 20-7 recorded. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-09** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants, every correction re-asks · **C** ×1.3 (rule 6) | **A.** 20-3b/20-3c landed at 2–3× creation forecasts; 20-4 L1 rebased its bin ceiling the same way. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-10** | Where the operator cost page lives (AC5) | **A** a "Cost attribution" section in `run-maos.md` (`docs/maos.dev/`, `docs-site/docs/`, `docs-site/i18n/ko/…/current/`) · **B** a new runbook `docs/runbooks/cost-attribution.md` · **C** a new docs-site route (en + ko, route manifest) | **A.** The page does not exist (C18); `run-maos.md` is where the operator surfaces live and where 20-4/20-10 add theirs; C adds a route and the manifest-parity/ko gates for one section. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-11** | NFR-Cost-1's ≥ 98 % operational half | **A** stays "Ownerless and open" in `deferred-work.md:91` (the clause this story closes is split out), the doc states the limit · **B** a new operator-lane row `ops-cost-billing-reconciliation` (ADR-046 §7 weekly sampling runbook) | **A.** The epic excludes it (E20:332); a new tracker key is an operator act and touches the doc gates. B is the operator's to add. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-14-12** | Cumulative kernel room (rule: cumulative > 21209 is an operator row) — measured 21203 with every ruled row, 21228 with this story's ×2 ceiling, 21249 with 20-9's too | **A split** — no allowance here: 20-14's kernel ceiling is its exact measured +25, re-based; any review correction above it is STOP · **B exchange** — K0 (0 kernel lines, D-02 D) instead of KT, freeing 25 tokei at the cost of the task/principal not riding the frame · **C bump** — one joint targeted cumulative STOP with D-20-9-15 sized to both ×2 allowances, **21250** (750 under the 22000 alarm, ADR-069 envelope 23000 untouched) | **C**, joint targeted exception for 20-9 L2 + 20-14 L2 (ask `d2014_12`); D-20-9-15 A remains 20-9’s single-story ceiling of 21230. 20-14 lands last (PF:142) and would otherwise meet 6 tokei of room with a +25 row whose review history (20-3b/20-3c landed at 2–3× creation) makes a STOP at landing likely; A turns every review correction into a new ask; B gives up the charter's "task reference on the cost frame". C still leaves 750 under the alarm. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **C**; ask `d2014_12`: joint targeted STOP **21250** for 20-9 L2 + 20-14 L2 only; shared STOP **21209** otherwise; ADR-069 alarm **22000** unchanged; D-20-9-15 A retains **21230** as 20-9's single-story ceiling |
| **D-20-14-13** | Kernel-free frame schema | **A** keep the bare `CostAttributionPayload` unchanged; add `CostFrameBody` flattening it and carrying optional `task_ref`, plus `CostScope`; reader parses the wrapper. L2 wraps its existing literal at emission. | The exhaustive kernel literal otherwise requires a kernel edit in L1. Operator ruled **A**, ask `d1408` (date not recorded). Domain 10200 -> 10215 and CLI 6524 -> 6545 are orchestrator-adjudicated overdrafts, disclosed to the operator; wrapper and formatted test costs were not in the creation forecast. |

## Acceptance Criteria

*(Epic §20-14 as amended 2026-10-30; clauses marked (D-…) carry the operator-ruled recommendation (2026-11-03, ask `d_batch4`). Every AC names a failing-before witness and a plant. **local** = hermetic on the dev host;
**hosted** = the discipline run.)*

### L1 — kernel-free

#### AC4 — the report has task, attribution-source and Spirit axes; old frames still read (epic AC1, report half; D-20-14-05, -06, -07)

1. The frame body gains `task_ref: Option<String>` through the additive `CostFrameBody`
   wrapper, flattening the unchanged `CostAttributionPayload` (D-20-14-13 A),
   and `maos_domain::cost::CostScope { task_ref, session_principal }` is added.
   `no_task_ref_in_payload` is replaced by wrapper and legacy-read vectors; SR-4
   and R6 tests remain unchanged. In L2 the kernel wraps its existing literal as
   `CostFrameBody { payload: CostAttributionPayload { ... }, task_ref: scope.task_ref }`;
   the original KT +35/+25 prototype did not price this wrapper hunk and the
   final kernel lander must re-measure it against the rebased grant.
2. `build_cost_report` (`subcommands.rs:4842-4983`) groups by `(task, principal, attribution_source, spirit, spirit_pid,
   provider, model)`; a row carries `task` (`none` when absent), `attribution_source` (`session_principal` |
   `write_target_proxy`) and `spirit` — the name at `(boot_nonce, spirit_pid)` from `maos_audit::spirit_names`
   (kind 19 / kind 7 rows), else `pid:<n>` (D-05 A; prototype `nk`). The SR-1/SR-2 rule (only `Resolved` is attributed;
   `Ambiguous`/`Unattributed` → `host-unallocated`) and the R5 single division are unchanged.
3. (D-06 A) Both output formats carry a fixed note that Worker CLI spend through the egress proxy is not in cost frames.
4. **Failing-before (measured at HEAD):** two shell sessions on one log report **one** row,
   `host-unallocated pid=1 replay/hand-authored-seed in=100 out=60` (`creation/runtime/reconcile-head.*`). Witness: the
   in-src test `subcommands.rs:5862-5906` updated, plus vectors — two frames with different `task_ref` → two rows; a
   schema-1 frame → `task: none`; two boots, same pid, two names → two `spirit` values.
5. **Plant (local):** restore the `(principal, spirit_pid, provider, model)` key ⇒ the per-task vector reds.

#### AC5 — the operator page and ADR-046 say what is true (epic AC4; D-20-14-10, -11)

1. (D-10 A) A "Cost attribution" section in `docs/maos.dev/run-maos.md`, `docs-site/docs/run-maos.md` and the ko mirror:
   the three axes; the source values and their honesty boundary (session principal = bound by the operator at boot,
   not an authenticated end user unless D-04 D; proxy = write-target reverse lookup the attributed Spirit can write,
   C15); what `task: none` means on schema 1 and 2; the Worker-CLI label; the v1.1 CPU-time / storage-I/O plan
   (ADR-046 R6); NFR-Cost-1 restated over the token dimensions, the ≥ 98 % check operational (ADR-046 §7).
2. ADR-046 amended in place: R3 (`:38-40`) — `task_ref` restored, sourced from the root-bound scope (D-01/D-02 as
   ruled); §10 (`:79-81`) and Consequences (`:88`) — per-call binding landed, proxy kept as tagged fallback; Context
   `:15` annotated "superseded by 20-14", not deleted.
3. Docs gates in `prepush` stay green (no new route under D-10 A; ko glossary/coverage).

### L2 — kernel last

#### AC2 — per-task attribution (epic AC2; D-20-14-01 A, -02 B, -03 B, -07 A) ◆ FLAG-Winston

1. KT (`kt-token-bound-scope.diff`): `CapabilityRegistryAdapter::bind_cost_scope(token_id, CostScope)` (prunes scopes of
   tokens the ring no longer holds, measurement 2) and `cost_scope(&token_id)`; the emission (`inference/mod.rs:398-461`)
   reads the scope by the verified token id and writes `task_ref`, `schema_version: 2`. Any other
   `crates/maos-kernel-core/src` change is STOP.
2. Root binding (D-01 A): each production Inference Port mint binds a scope at mint — shell turn (`shell_host.rs:112-122`),
   the seam's per-pass mint (`main.rs:4179-4200`), one-shot hello (`main.rs:9601-9615`); T0 measurement 3 classifies
   every mint site and records production vs smoke; a production site that does not bind says why in the record.
3. Anti-spoof (security): the binding is reachable only from the root — Spirit crates receive mint closures, never the
   registry (`spirits/butler/src/lib.rs:1175`, `spirits/orchestrator/src/lib.rs:447`); a token minted without a scope emits no
   `task_ref` and the proxy; no request field feeds the task (D-03 B).
4. **Failing-before (local):** `crates/maos-kernel-core/tests/cost_attribution_20_14.rs` (adapter harness of
   `inference_fallback_authz_18_1.rs`): a minted `ProviderInfer` token, a bound `{task_ref: "t-1"}`, one completion ⇒
   the journaled payload carries `"task_ref":"t-1"`; an unbound token ⇒ no `task_ref`. At HEAD the frame has no task
   (measured, `creation/runtime/tl-rows-head.txt`).
5. **Plant (local):** read `CostScope::default()` at emission ⇒ the `t-1` vector reds. (The epic's "remove the marker
   write" is AC1.4's second plant under KT.)

#### AC3 — session-principal attribution (epic AC3; D-20-14-04 A) ◆ security

1. `MAOS_SESSION_PRINCIPAL` — UserFacing row in `crates/maos-bin/src/env_contract.rs` (`check-env-contract` green), read
   once at boot by `maos shell` and `maos run` under the D-19-2-08 A rule, bound into every scope the root opens this
   boot (prototype `bin` slice). Never read from a request, manifest, Spirit or frame.
2. Emission: a bound principal ⇒ `PrincipalRef::Resolved`, `AttributionSource::SessionPrincipal`, `Exact`, and the
   proxy is not queried; none bound ⇒ the proxy (`transparency_log.rs:1495-1517`) with today's
   `Unattributed`/`Resolved`/`Ambiguous{count}` fold, tagged `WriteTargetProxy`.
3. SR-4: a kernel vector with two principals written by the pid and no session principal ⇒ `Ambiguous { count: 2 }`, no
   member id anywhere in the payload; `cost.rs:238-245` green.
4. SR-3: `forget(alice)` scrubs `alice` from her session-principal cost frames with the existing body scan (K5;
   measurement 7), dimensions kept; bob's frame byte-identical after (harness `forget_cascade_18_6.rs`).
5. **Failing-before (measured):** with `MAOS_SESSION_PRINCIPAL=alice` the HEAD frame is `Unattributed` /
   `write_target_proxy`. Witnesses in the AC2.4 file and `crates/maos-bin/tests/cost_attribution_20_14.rs`: a value with
   `:` or `\u{1}` refuses the boot; blank binds none.
6. **Plant (local):** emit the proxy while a principal is bound ⇒ the attribution-source assertion reds.
7. (D-04 D only) SSO configured ⇒ the verified subject is the session principal; a different env value refuses the boot.

#### AC1 — the command (epic AC1)

1. **The command (local, isolated `HOME`, from the repo root, after AC2–AC4):**
   `maos init`; then for `p` in `alice`, `bob`:
   `printf '@hello-spirit hello\n' | MAOS_SESSION_PRINCIPAL=$p MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j0/shell-intro.json maos shell`;
   then `maosctl audit cost-reconcile --month <YYYY-MM> --format ndjson --pricing xtask/provider-pricing.toml` exits 0
   with exactly two rows — distinct `task` values `shell.turn:<ulid>`, principals `alice` and `bob`,
   `attribution_source: session_principal`, `spirit: hello-spirit`, `replay / hand-authored-seed`, 50 in / 30 out each —
   and no `host-unallocated` row; a seeded schema-1 frame in the same month reports under `task: none`. (Money is 0 for
   the seed cassette; the assertion is on tokens and rows, K9.)
2. **Measured at HEAD:** one `host-unallocated` row, 100 / 60 (`creation/runtime/`).
3. **Witness (local):** `crates/maos-bin/tests/cost_attribution_20_14.rs` runs the real `maos` and `maosctl`
   (`PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl")`, the `audit_query_20_3c.rs:21-25` locator);
   no network, no cargo in the test; wall time against `xtask/test-budgets.toml` (measurement 6).
4. **Plants (local, two):** the AC4.5 key ⇒ the per-task row assertion reds; drop the bind in `issue_turn_token` ⇒ the
   rows read `task: none` / `write_target_proxy` and red.

#### AC6 — records

1. `tests/coverage-matrix.yaml:899-904` (FR64) and `:1103-1108` (NFR-Cost-1) name the AC1–AC4 witnesses; no gate is
   claimed that does not exist.
2. At `done`: `deferred-work.md:80` (FR64) `CLOSED <date> by 20-14 @<commit>`; `:91` (NFR-Cost-1) re-scoped — the
   per-Spirit/task/principal clause closed by 20-14, the ≥ 98 % half as D-11 rules; the D-06 row filed.
3. Landing: exact pins, HISTORY first segment citing this key with `->`; L2 records the FLAG-Winston grant in
   `20-14-flag-winston-decision.md` and both kernel pins with the `capability/mod.rs` and `inference/mod.rs` content
   hashes; revert `intent-lineage-coverage-report.md` if a run modifies it.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D-20-14-01…11 recorded; tracker → `in-progress`. Run
  `creation/measurements.md` "Not measured here" 1–7. Re-measure every cite this story touches (rule 9). A figure above
  the ruled envelope, or a T0 result contradicting a ruled option, is STOP + D-row.
- [ ] **T1 — Payload and domain (AC4.1).** `task_ref`, `CostScope`, R3 test replaced, literals updated.
- [ ] **T2 — Report (AC4.2–4.5).** Key, rows, `spirit_names`, Worker note, vectors, plant.
- [ ] **T3 — Docs and ADR-046 (AC5).** Three `run-maos.md` files, ADR amendments.
- [ ] **T4 — L1 landing (D-20-14-08 A).** Exact pins inside the ruled envelope; `check-kernel-baseline` unchanged;
  `kloc-check`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; one hosted discipline run on
  the final tree + plants. No commit or push without the operator.
- [ ] **T5 — Kernel KT (AC2.1, AC3.2–3.3)** under the ruled row: red-before kernel vectors, KT, plants.
- [ ] **T6 — Root binding and session principal (AC2.2–2.3, AC3.1, AC3.4–3.7).** Env row, boot parse (`maos shell`,
  `maos run`), shell turn, seam per-pass mint, one-shot hello, mint-site classification record, forget vector.
- [ ] **T7 — The command (AC1).** `cost_attribution_20_14.rs`, two plants.
- [ ] **T8 — L2 landing + records (AC6).** Both kernel pins + content hashes (re-base onto the then-landed pins of
  20-1b/20-4/20-11; cumulative STOP 21209); `cargo test -p maos-kernel-core --lib`; hosted run; ledger closures;
  coverage; FLAG-Winston record.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Domain | `crates/maos-domain/src/cost.rs:53-106` (source doc, `CostScope`, `task_ref`), tests `:197-311` | AC4 |
| CLI | `crates/maos-cli/src/subcommands.rs:4772-4796` (row, report), `:4842-4983` (`build_cost_report`), `:4986-5021` (plain), `:5023-5100` (entry: names map), test `:5862-5906`; `cli.rs:746-750` doc | AC4 |
| Audit | `crates/maos-audit/src/lib.rs` (`spirit_names` beside `resolve_spirit_name` `:1679-1767`) | AC4 |
| Kernel ◆ (L2) | `crates/maos-kernel-core/src/capability/mod.rs:124-181,273-276` (map, bind, read); `src/inference/mod.rs:398-461` (emission); NEW `tests/cost_attribution_20_14.rs`; pins `xtask/kernel-core-baseline.toml` (`src_lines`, two content hashes, EOF HISTORY), `xtask/kloc.toml:264` | AC2, AC3 |
| Bin (L2) | `crates/maos-bin/src/env_contract.rs` (row), `shell_host.rs:39-122` (field, builder, bind at mint), `main.rs:4132-4200` (boot parse beside `MAOS_BUTLER_PRINCIPAL`, seam mint), `:4515-4526` (shell host), `:9601-9615` (one-shot); NEW `tests/cost_attribution_20_14.rs` | AC1–AC3 |
| Docs / records | `docs/maos.dev/run-maos.md`, `docs-site/docs/run-maos.md`, `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`; `docs/adr/ADR-046-cost-attribution-and-reconciliation.md:15,38-40,79-81,88`; `tests/coverage-matrix.yaml:899-904,1103-1108`; `deferred-work.md:80,91` | AC5, AC6 |

No new `maos-bin/src` file (so `SCANNED_SOURCE_FILES` in `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` is
unchanged); a new one must be enrolled there.

### Overlap (serialize; re-measure)

- **`crates/maos-bin/src/main.rs` binding sites and `shell_host.rs`** — 20-4 L1 (uncommitted in the working tree at
  creation) rewrites the provider builder, `declared_providers` and the shell binding (`main.rs:890-904,4374`,
  `orchestrator_root.rs:221-237`); land after it and re-measure every cite.
- **`inference/mod.rs`** — 20-4 L2 (KA, `bucket_key` `:96-101`) and this story's emission edit the same file: one
  integration owner per pin update, the later lander re-bases the content hash.
- **Kernel pins** — 20-1b, 20-4, 20-11, 20-8, 20-9, 20-13 forecast or hold kernel rows; cumulative STOP 21209.
- **`maos-audit`** — 20-11 (erasure inventory, orphan-halt query) edits `lib.rs` beside `resolve_spirit_name`.
- **`run-maos.md` ×3** — 20-4, 20-10 add sections; the later lander re-bases.

### Current state that must be preserved

- ADR-046 posture: the report is observability, not an invoice; no money in the frame (R4); integer sum-then-round
  (R5, `subcommands.rs:4899-4966`); only `Resolved` is attributed (SR-1/SR-2); `Ambiguous` carries a count (SR-4).
- Pre-20-14 frames read and report (`task: none`); an unknown dimension key stays an `Err` (R6 test).
- SR-3: forget scrubs every principal-bearing frame by body scan; uninstall does not scrub cost frames (20-11).
- 19-2 `MAOS_BUTLER_PRINCIPAL` semantics (picks namespace) unchanged; FR51 re-check after the call
  (`inference/mod.rs:469-475`) unchanged; the 6.4 rate-limit path unchanged.
- The cost frame stays kernel-origin (`FrameOrigin::Kernel`) and tokenless in its column (FR4 non-call kind, K6).

### Guardrails

- No kernel line without the ruled FLAG-Winston row; any change outside KT's two kernel files is STOP.
- The task and the session principal come from the root only — never from `InferenceRequest`, a manifest, a Spirit or
  a frame; no `bind_cost_scope` handle reaches `spirits/*` or a WASM/T3 Spirit.
- No member identifiers for an ambiguous set; no N-way split; no fabricated task key (`spirit_pid + timestamp`).
- No `unwrap`/`expect` after a serde parse in production code; no test spawns cargo without the expensive marker;
  new env vars registered in `env_contract.rs`.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean
  it; plants byte-restored in the real tree (`proof-contract.md:24-31`).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Report task axis (AC4) | old grouping key | per-task vector | local |
| Task on the frame (AC2) | default scope at emission | `t-1` vector | local |
| Session-principal source (AC3) | proxy while bound | attribution-source assertion | local |
| The command (AC1) | old key; drop the shell-turn bind | per-task rows; `session_principal` rows | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-domain -p maos-cli -p maos-audit
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test cost_attribution_20_14 --test forget_cascade_18_6   # L2
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib                                                          # L2
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test cost_attribution_20_14                                   # L2 (needs maosctl built)
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-env-contract
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-exit-commands
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-decision-register
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC2.3, AC3 (source, SR-3, SR-4) and the
D-04 ruling, Test Infrastructure (glm-5.3). Focused suites per touched crate + `cargo test -p maos-kernel-core --lib`
at L2; the reviewer reads the plant record, never re-runs plants except for a disputed finding.

### Previous-story intelligence

- 20-4 (ready-for-dev, L1 in progress): two-landing shape, kernel last; its L1 rebased the bin ceiling onto the landed
  pin (34643) — this story's ceilings re-base the same way; it owns the binding sites this story binds scopes at.
- 20-11 (ready-for-dev): cost frames are never scrubbed by uninstall (PF:129); `(boot_nonce, pid)` is the only safe key
  for a Spirit across boots (`deferred-work.md:1329`) — D-05 A applies the same rule to the report.
- 19-2: `MAOS_BUTLER_PRINCIPAL` is the ruled shape for a boot-bound principal (D-19-2-08 A) — D-04 A reuses its rule.
- 18-1: one seam mint per pass serves Butler, Orchestrator and Researcher (`main.rs:4170-4200`) — one binding site.
- 9.3b: built the cost frame, the proxy and SR-3; ADR-046 named this story's two primitives as its successor.
- 20-7: a key rename broke two doc gates `prepush` does not run — this story renames no key.

### Git intelligence

`8c809f9c` (20-10 hosted corrections; bin 34191, aggregate 192519), `16c35ef0`, `ac1fc12a`, `75538bf7`, `d3e3f586`
(20-1a exit leg; 20-2/20-4/20-10/20-11 created and ruled). `crates/maos-domain/src/cost.rs`,
`crates/maos-kernel-core/src/inference/mod.rs` emission and `crates/maos-cli` cost code are unchanged since 9.3b's
landing apart from 19-2's re-check (`inference/mod.rs:469-475`) and 18-1's fallback refusal (`:291-293`).

### Library / framework

No new dependency: `serde`/`serde_json` (payload), `ulid` (already a maos-bin dependency, `installed_spirits.rs:142`),
`rusqlite` read-only (audit), `std::sync::RwLock` + `HashMap` (KT map).

### References

E20@HEAD:59-61,87,319-332; preflight `epic-20-preflight-2026-10-30.md:87,131,142,251-261`; tracker
`sprint-status.yaml:316`; `deferred-work.md:80,91,1329`; routing audit `:69,130`; requirements inventory `:570,619`;
PRD `functional-requirements.md:107,122`, `non-functional-requirements.md:179`, `project-scoping-phased-development.md:406,412`,
`user-journeys.md:359`; ADR-046; ADR-069; ADR-062; 19-2 D-19-2-08; research
`technical-agent-orchestration-ux-and-techniques-for-maos-research-2026-10-28.md:316,389,402`.

## Dev Agent Record

### Story context creation

Created 2026-11-03 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract; `Create2014`) at
`8c809f9c`, without cargo (parent rule).

**Inputs:** E20 §20-14 at HEAD (amended 2026-10-30, PF §3.6) — stale cites corrected in place
(`creation/records-owed.md` §A); preflight rows; tracker `:316`; `deferred-work.md` rows naming 20-14; PRD FR64 /
NFR-Cost-1 DELTA-2026-10-25; ADR-046; direct reads of every cited range (`creation/cites-8c809f9c.md`); kloc and kernel
by the prebuilt gate binary on `/tmp/proto2014` `git archive` copies (`creation/kloc-kernel-8c809f9c-and-prototypes.json`);
prototypes KT, KM, KM-naive, KP, K0, KR, NK, BIN and the combined REC saved as `rustfmt`'d diffs, never applied; the
epic AC1 scene on the prebuilt binaries in an isolated home (`creation/runtime/`).

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–7 (5 answered for the shell path).

**Records owed:** `creation/records-owed.md` (§A epic, §B planning records, §C routed observations).

**Revert:** no crate source modified (prototypes only under `/tmp/proto2014`, deleted); no cargo/test ran;
`intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS
CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree, read-only; `creation/gates/*.json`): `check-epic-close-coherence` rc 0
(21 epics, pin 26841); `check-decision-register` rc 0 (23 rows, 6 open); `check-exit-commands` rc 0 (2 blocks, 10
tokens); `check-dev-record-completeness` rc 0 (197 done stories, 172 deferred-owner assertions, 0 violations; a first run
red on a sibling 20-8 row at `deferred-work.md:1390`, fixed by its creator and re-run green; a later sibling 20-9 note at `deferred-work.md:1261` red it once, was fixed by its creator, and the re-run is green). The
`intent-lineage-coverage-report.md` hash is unchanged across the runs. `kloc-check` / `check-kernel-baseline` were
measured on the `git archive` copies only (the working tree carries a sibling's uncommitted 20-4 L1 edits).

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

### Completion Notes List

L1 implementation evidence: `20-14-complete-cost-attribution-evidence/landing-1/evidence.md`.
The reader, wrapper, name lookup and guides are implemented; current writers
still emit schema 1 and proxy attribution. `CostFrameBody` is permanent; the
new in-source report test cannot move to `tests/` without exposing private
report internals. New supplementary serde vectors were moved to `tests/` without
moving existing counted tests. Domain 10198 -> 10203, CLI 6429 -> 6529,
audit 8179 -> 8209, aggregate 198914 -> 199049; kernel unchanged 26841/21042.
The explicit schema-1 `task: none` assertion was added after the workspace run
and has not been run; its one-line formatted source is measured in the pins.
20-9 forecasts domain 0..6 and 20-5 +8; cumulative measured-forecast domain
10211..10217 [INFERENCE], before later rebasing and review corrections.
AC1–AC3 and AC6 closure remain L2; hosted recipe unverified. Focused suites
were interrupted by missing Python cryptography; the provisioned subsequent
workspace run exited 100 with 5879 passed and two failures (no-data-home and
20-11's 1M pending observation), without retry. Frozen prepush timed out at
3000 s. Full commands, plant outcomes and same-line failures are in evidence.

**L1 review corrections (2026-11-10; triage P1–P4, security F1–F8; evidence
`20-14-complete-cost-attribution-evidence/landing-1/review/corrections.md`).** P1/F1: `spirit_names` now
accepts only identity rows (kind 19, or kind 7 `lifecycle.load` with a NULL capability token) and returns timestamped events; the
report uses the latest identity at or before the frame, else `pid:<n>`. P2: boot-separated and unnamed-pid vectors in
`crates/maos-cli/tests/support/cost_report_review_20_14.rs`. P3: ADR-046 keeps the SR-1 write-path-authorization follow-up
beside SR-5/R3 with a section-scoped test. P4: the unsupported D-13 calendar date is gone (ask `d1408`, date not recorded); status
`in-progress`. F2: kinds 28/29 were first narrowed to identity paths (round 1), which wrongly exempted kind-28 governance receipts as having "no principal
nexus" (false: ADR-045 has no blanket D5 exemption; that note is ModelProvenancePayload-only, `governance.rs:117-126`); corrected in round 2 below; F3: plain output escapes control characters and caps fields at 256;
F4: origin=Kernel query filter, row pid must equal payload pid, `task_ref` over 256 skipped with a warning, distinct `none (schema 1)` /
`none (schema 2)` / `literal:` labels; F5: `per_spirit_attributable_fraction` keyed `<boot_nonce>:<pid>` (closes review defer D1; the
JSON key changed); F6: one warning per unpriced provider/model; F7: saturating/checked u128 and u64 arithmetic; F8: guides and ADR wording.
Every code patch has an observed red plant (restored by copy-back, `cmp`, `touch`) in `corrections.md`. Pins: CLI 6529 -> 6599 under
an orchestrator-adjudicated overdraft, disclosed to the operator (ceiling 6545 -> 6610); audit 8209 -> 8227 (ceiling 8237); iac 8728, domain 10203, bin 36234,
kernel 26841/21042 unchanged; aggregate 199146, hardfail 199147 (<= 199378). One workspace nextest: 5889 passed, the known 20-11 1M timing
case failed (not retried; owner `20-7`, `deferred-work.md` row 58). `prepush` was not run (3000 s timeouts); hosted/Apple/Windows legs unverified.
Still open: T0 measurements, T4 and all L2 work; AC1–AC3 and AC6 closure remains the kernel landing. Deferred (owner this story, landing 2):
R6 wording (review D2) and schema/source cross-check (F4 residual).
The schema-1 absent-task assertion that had not been run now reads `none (schema 1)` and ran green
(`tests::cost_report_reads_payload_not_intent`, in the workspace run and the focused run).

**L1 correction round 2 (2026-11-10; independent reviews `review/correction-review.md` CR1–CR3 and GLM `Sec2014Corr` F-1/F-2; evidence
`20-14-complete-cost-attribution-evidence/landing-1/review/correction-round2.md`, raw red/plant/green logs under `review/correction-round2/`).**
Round 1 had swapped over-erasure for under-erasure. CR1: kind-28 governance frames are not exempt (ADR-045 has no blanket D5; the note is
`ModelProvenancePayload`-only); kinds 28/29 now share one value-only scrub for discovery, body replacement and the late-window certification predicate
(round-2 wording "keys, enum tags, hashes and fixed receipt discriminators never touched" was false for typed governance tags/status/manifest fingerprint; corrected in round 3 below; kind 28 scrubs every other nested string value). CR2/F-1: a kind-29 frame is
selected by whole-token value matches in `principal_id`, `task_ref`, `session_principal`, `provider` or `model`, and provider/model values are scrubbed
too (priced as unpriced afterwards); a principal equal to a real provider name erases that value in every frame carrying it, documented in ADR-046 §8 and
the guides. CR3/F-2: `cost_terminal_field` escapes backslashes and bidirectional-format ranges; guides document the `<boot_nonce>:<pid>` fraction keys and
"input characters"; the report test pins the exact keys and fractions. Witnesses: a real T2 hostile-twin session receipt (`spawned_scope_17_3c.rs`),
kernel forget vectors, late-append hooks in `erasure_races_20_11.rs`, CLI vectors. Pins: CLI 6599 -> 6603 (inside the 6610 orchestrator-adjudicated overdraft,
disclosed to the operator), iac 8728 -> 8726, audit 8227, aggregate 199148, hardfail 199149. The existing F2 fixture values changed (`provider`/`model`
values now `anthropic`/`claude-x`) because key-name values would now legitimately match as values; its assertions are unchanged.

**L1 correction round 3 (2026-11-11; GPT `correction-review-r2.md` R2-1/R2-2 + inherited limits, GLM `Sec2014R2` F-1..F-4; evidence
`20-14-complete-cost-attribution-evidence/landing-1/review/correction-round3.md`, raw red/plant/green logs under `review/correction-round3/`).**
R2-1/R2-2: kind-28 typed fields are preserved by envelope and value form (never by key name): `/event/governance_type` (valid variant), the ABI `/event/status`
(valid enum), the ManifestDeclaration `/event/manifest_sha256` and `/event/publisher` (canonical colon-hex); witnessed through the real producers
(`emit_vetter_key_event`, `emit_abi_extension_event`, `emit_manifest_declaration_event`) and the consuming decoders `maos_audit::load_ratification_frames` and
`decode_declaration_bytes` (`crates/maos-bin/tests/forget_governance_20_14.rs`). Inherited: kinds 28/29 bypass the SQL `instr` prefilter so escaped spellings
(`\u0061lice`, `ali\"ce`) reach decoded selection in discovery, replacement and certification (initial snapshot and late hook); exempt hash fields (`sha256`, `supersedes`,
`schema_content_hash`) are exempt only as 64-hex digests and `register_schema_lifecycle` (operator door and CLI) rejects non-canonical digests. GLM F-4: `cost_terminal_field`
escapes the Cf class, U+061C, U+180E, U+FEFF, tag characters and variation selectors. The round-2 real-session plants were re-run after rebuilding `maosctl` so the receipt
assertion is what fails. **Operator-visible privacy limits, deferred (owner this story's landing 2 / ADR-045 follow-up, `deferred-work.md`):** the principal matcher is
case- and normalization-sensitive for every frame kind (GLM F-1), Spirit-name columns are outside erasure (GLM F-3), whole-value label replacement is accepted policy (GLM F-2).
Pins: CLI 6603 -> 6609 (ceiling 6610), iac 8726 -> 8770 under an **orchestrator-adjudicated D02 A overdraft, disclosed to the operator: iac ceiling 8728 -> 8770** (before: pin 8726 /
ceiling 8728; after: measured 8770, zero headroom), audit 8227, aggregate 199198, hardfail 199199 (<= 199378). The existing kernel hash fixtures changed from `deadbeef`/`cafe` to 64-hex digests.

### File List

- `crates/maos-domain/src/cost.rs`, `crates/maos-domain/tests/cost_frame_20_14.rs`,
  `crates/maos-audit/src/lib.rs`, `crates/maos-audit/tests/spirit_resolution.rs`,
  `crates/maos-cli/src/subcommands.rs`, `crates/maos-cli/tests/cost_docs_20_14.rs`:
  frame reader, report and witnesses.
- `crates/maos-kernel-core/tests/forget_cascade_18_6.rs`: T0 session-principal
  erasure vector; no kernel source change.
- `docs/maos.dev/run-maos.md`, `docs-site/docs/run-maos.md`,
  `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md`,
  `docs/adr/ADR-046-cost-attribution-and-reconciliation.md`: operator limits
  and the pending final binding.
- `tests/coverage-matrix.yaml`, `xtask/kloc.toml`, `deferred-work.md`, this story,
  its FLAG-Winston record, tracker and
  `20-14-complete-cost-attribution-evidence/{landing-1,ci}`: scoped coverage,
  exact pins, rulings and observed evidence.
- Review corrections: `crates/maos-audit/{src/lib.rs,tests/spirit_resolution.rs}`, `crates/maos-cli/{src/subcommands.rs,tests/cost_docs_20_14.rs,tests/support/cost_report_review_20_14.rs}`,
  `crates/maos-iac/{src/adapter/transparency_log.rs,src/adapter/transparency_log/erasure_sweep.rs,tests/support/erasure_races_20_11.rs}`,
  `crates/maos-kernel-core/tests/forget_cascade_18_6.rs` (test only), the three `run-maos.md` guides, ADR-046, `xtask/kloc.toml`,
  `deferred-work.md`, tracker, FLAG-Winston record and `20-14-complete-cost-attribution-evidence/landing-1/review/corrections*`.
- Round 2: `crates/maos-bin/tests/spawned_scope_17_3c.rs` (helper parametrized by directory/seed; real-session forget witness), `crates/maos-iac/src/adapter/transparency_log.rs`
  (shared value-only scrub), the erasure-race, forget-cascade and CLI report test files, the three guides, ADR-046 §8, `xtask/kloc.toml`,
  `20-14-complete-cost-attribution-evidence/landing-1/review/correction-round2*`.
- Round 3: `crates/maos-iac/src/adapter/transparency_log.rs` (typed keep-list, canonical digest form, prefilter bypass, ingress validation), `crates/maos-cli/src/subcommands.rs`
  (invisible-character table), new `crates/maos-bin/tests/forget_governance_20_14.rs`, kernel/iac/CLI test files, guides, ADR-046 §8, `xtask/kloc.toml`,
  `20-14-complete-cost-attribution-evidence/landing-1/review/correction-round3*`.

## Change Log

- 2026-11-03: Story created at `8c809f9c` (ready-for-dev; D-20-14-01…11 PROPOSED). Measured: two shell sessions under
  two principals pool into one `host-unallocated` row (100/60, both at pid 1); no Inference Port caller is ever
  dispatched a `task.assign`, so the epic's SCB marker has no production writer — the unit of work is the root's
  turn/pass mint; `abi-diff` cannot see `InferenceRequest`. Kernel KT +35/+25 recommended (KM +44/+30, KP +31/+25,
  K0 0/0, KR +1/+1 priced by reverted prototype); cumulative 21150 (21203 with the 20-8/20-9/20-13 PROPOSED rows; 21228 at the ×2 ceiling → D-20-14-12). Non-kernel measured: domain −1, cli +25,
  audit +29, bin +35.
- 2026-11-03: D-20-14-12 added after Create209 reported 20-9's PROPOSED R +21 (and 20-8 +33, 20-13 −1 at the same HEAD): measured cumulative 21203 ≤ 21209, ×2 ceiling 21228 > STOP; recommendation C (joint targeted STOP 21250 with D-20-9-15). Status remains ready-for-dev.

- 2026-11-03: Operator ruled D-20-14-01…12 as recommended (ask `d_batch4`): 01 A, 02 B, 03 B, 04–11 A, 12 C. D-20-14-12 C (ask `d2014_12`) grants joint targeted STOP 21250 for 20-9 L2 + 20-14 L2 only; shared STOP 21209 otherwise, 20-9 D-20-9-15 A single-story ceiling 21230, and ADR-069 alarm 22000 remain unchanged. Grants bind at dev start after re-base onto then-landed pins and cumulative room; no implementation or gates run.

- 2026-11-10: L1 review corrections applied (see Completion Notes); status `in-progress`; CLI ceiling 6545 -> 6610 (orchestrator-adjudicated overdraft, disclosed to the operator); pins CLI 6599, audit 8227, hardfail 199147.

- 2026-11-10: L1 correction round 2 (CR1–CR3, GLM F-1/F-2): value-only scrub for kinds 28/29 incl. provider/model values, backslash and bidi escaping, guide and ADR trade-off text, false kind-28 "no principal nexus" claim withdrawn; pins CLI 6603 (inside the 6610 overdraft), iac 8726, hardfail 199149.

- 2026-11-11: L1 correction round 3 (R2-1, R2-2, escaped-value discovery, digest form, F-4): see Completion Notes; iac ceiling 8728 -> 8770 (orchestrator-adjudicated D02 A overdraft, disclosed to the operator); pins CLI 6609, iac 8770, hardfail 199199.
