---
story_id: '20.13'
story_key: 20-13-pluggable-crypto-provider-selection
created: 2026-11-03
context_commit: 8c809f9c
baseline_commit: 8c809f9c
security_sensitive: true
kernel_grant: "Ruled 2026-11-03 (operator, ask d_batch4; D-20-13-04 A, KT3); binding at dev start after re-measurement against the landed tree; no kernel edit before that binding: SecurityManagerAdapter takes the selected provider as a required constructor argument; measured by reverted prototype at 8c809f9c from 26841 physical / 21042 tokei: +1 / −1. Ruled ceilings (STOP above; re-base at L2 start): kernel ≤ 26843 physical / 21042 tokei; kernel lands last (L2). Every later kernel landing re-bases; expected cumulative with the ruled unlanded rows 20-1b +68, 20-4 +9, 20-11 +6 is 21124 tokei; with the concurrent PROPOSED rows 20-8 KS +33, 20-14 KT +25 and 20-9 R +21 it is 21203 (6 below the STOP 21209; D-20-13-16), ADR-069 alarm 22000. Priced alternatives: KT1 +20/+14, KTd +11/+6, KI +19/+18 (domain +6), KO +38/+37 (domain +7), none 0/0."
kloc_grant: "Ruled 2026-11-03 (operator, ask d_batch4; D-20-13-13 A; D02 A ×2 measured / ×3 unmeasured); binding at dev start after re-measurement against the landed tree. Ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1): maos-bin ≤ 34687; maos-shell ≤ 737; maos-registry ≤ 4350; maos-compliance ≤ 2106 (pin 2117 kept); maos-domain ≤ 10114; xtask ≤ 44644; kernel ≤ 26843 / 21042 (see kernel_grant); aggregate ≤ 193564. maos-secrets, maos-audit, maos-cli, maos-capability 0."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): a kernel FLAG-Winston row (D-20-13-04), signature verification and key-material routing on every FR48 surface, a refusing control (unknown provider, structural check) and a compatibility rule on ComplianceClaim drift (D-20-13-06). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC1.3, AC3, AC4 and AC7, Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.13: Pluggable crypto provider selection

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


Status: ready-for-dev

<!-- Created 2026-11-03 at 8c809f9c (main; `git log -1 -- crates xtask` = 8c809f9c) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30); DECIDE-1 split (2026-10-23). D-20-13-01…16 were ruled as recommended (all A) by the operator on 2026-11-03 (ask `d_batch4`); grants bind at dev start after re-measurement against the landed tree. No cargo ran: cites re-read on a `git archive 8c809f9c` copy (`creation/cites-8c809f9c.md`), kernel and kloc by the prebuilt gate binary on reverted `git archive` prototypes (`creation/measurements.md`), bypass inventory (`creation/bypass-inventory-8c809f9c.md`). Stale E20 §20-13 cites are corrected through `creation/records-owed.md` §A. -->

**Status rationale — ready-for-dev.** Every AC is hermetic: the routing proof uses a test-only provider compiled
into a fixture build; no FIPS module, no paid key, no live substrate beyond the existing T3 harness. Two serializations
are named, not blockers: 20-1b (backlog) edits the same registry/compliance admission files (D-20-13-15), and the D:1379
endpoint half reads 20-4 L1's endpoint table (in flight, D-20-13-07). The kernel row lands last (D-20-13-12).

> **The capability:** *an operator names the crypto module once, at `maos init`; the daemon builds exactly that module,
> once, and every kernel signature, capability token and at-rest seal goes through it — and the Transparency Log says
> which module it was.*

**Closes:**
- Epic-20 §20-13 (E20@HEAD:304-317) as amended 2026-10-30 (D01 A; PF §3.6): epic AC1 → AC1 + AC2; AC2 → AC3 + AC4 + AC7;
  AC3 → AC5; AC4 → AC6.
- FR48 (`prd/functional-requirements.md:32`) and NFR-Sec-15's provider-trait half (`prd/non-functional-requirements.md:48`,
  DELTA-2026-10-23): one selection point, the three kernel surfaces through the selected provider, unknown provider refused.
- `deferred-work.md:1379` (FR38 runtime drift check vacuous; owner 20-13 by D-20-4-10 B): the crypto half → AC4; the
  endpoint half per D-20-13-07.

**Does NOT close, and says so:**
- The FIPS 140-3-validated provider (v1.5, PRD DELTA-2026-10-23; `deferred-work.md:110` stays open). v1.0 ships `ring` only.
- Export classification (RELEASE-HOLDS Hold 2).
- Audit-artifact signatures (sealed-export bundle, erasure proof, receipts, release verify, Loom replication, cohort pins),
  publisher/vetter tooling and the rustls TLS provider — each named outside FR48's three surfaces (D-20-13-10;
  `creation/bypass-inventory-8c809f9c.md` §C–D).
- `maosctl`-side claim checks (`maosctl spirit install`, `maosctl import`) unless D-20-13-05 rules B.

## Story

As **an operator of a regulated deployment**,
I want **to choose the daemon's crypto module at `maos init`, have every kernel crypto operation go through that one
instance, and see the module named in the Transparency Log**,
so that **a FIPS-validated or hardware-backed module can replace `ring` later by configuration and a rebuild of the
root, never by recompiling a Spirit — and an auditor can tell which module signed.**

**Contract:**
- Epic §20-13 (E20@HEAD:304-317) as amended 2026-10-30 (D01 A; preflight `epic-20-preflight-2026-10-30.md` §3.6, PF:86,130);
  create-story tightens, adds no uncited AC: AC4.3–4.4 are the received D:1379 row.
- Tracker `sprint-status.yaml:315`; E20 confidence rules 4, 6, 8–11; proof contract `docs/testing/proof-contract.md:10-31`
  (one final-tree run + one plant per AC), `:33-71` (review, tiers).
- "One persisted selection and one shared instance, not a second port or initialization convention" (E20:308).

## Inventory at HEAD — measured, not assumed

Method: every cite re-read at `8c809f9c` on a `git archive` copy (`creation/cites-8c809f9c.md` C1–C18); constructions by the
gate simulation (`creation/gate-sim-base.txt`); bypasses by hand (`creation/bypass-inventory-8c809f9c.md`); no cargo.

| # | Charter item | Disposition at HEAD | Code | AC |
|---|---|---|---|---|
| 1 | Selection point | **missing**: `maos init` takes no flag (`main.rs:1906-1913`); `config.toml` is written (`maos-shell/src/lib.rs:59,118`) and read by nothing | `default_config_toml` `lib.rs:784-801` | AC1 |
| 2 | One instance | **false**: two production instances in the root (`main.rs:2218`, `:3238-3239`), one per Worker T3 launch (`worker_spawn.rs:888`), one per kernel T3 admission (`security/mod.rs:350`), one in a smoke arm (`main.rs:8403`) | `gate-sim-base.txt` (5) | AC1, AC7 |
| 3 | Unknown provider refused | **absent** (nothing is selected) | — | AC1 |
| 4 | Capability-token signing | **routed** through the injected provider | `cap_tokens/mod.rs:176-179`; root `main.rs:2261-2262` | AC1 (origin), AC2 |
| 5 | CRL verify | **routed** through the port, but on the root's second instance | `revocation/parser.rs:58`, `poller.rs:20`; door `operator_door.rs:441,1999` | AC1, AC2 |
| 6 | T3 image lock | port-shaped (`image_lock.rs:152-185`), callers construct their own `ring` | `worker_spawn.rs:888`; **kernel** `security/mod.rs:350` | AC1, AC7 ◆ |
| 7 | ComplianceClaim + publisher verify | **ring-direct** | `evaluator.rs:100-106,317-321`; `maos-registry/src/admission.rs:549-562`; daemon caller `installed_spirits.rs:25` → `install.rs:111-150` | AC4 |
| 8 | Sealed-export encryption | **routed** (`seal_at_rest` → `seal_for_export`), instance from the root | `maos-secrets/src/lib.rs:462-499`; `enterprise_identity.rs:290,477`; `main.rs:2339-2340` | AC1, AC2 |
| 9 | Audit sealed export | **signed, not encrypted** (ed25519-dalek, `maosctl`) | `maos-audit/src/sealed_export.rs:296-334`; `maos-cli/src/subcommands.rs:3268` | D-10 |
| 10 | Identity journaled | **absent**: an `eprintln!` names `ring-default` (`main.rs:2219`); the trait has no identity (`crypto.rs:50-97`) | — | AC5 |
| 11 | Runtime context names the module | **vacuous**: contexts take the manifest's own value (`install.rs:135-142`; `admission.rs:142-145,161-163`); no shipped manifest declares `crypto_provider` (only `maos-corpus-gen/seeds/ccac-seeds-v1.0.toml`) | D:1379 | AC4 |
| 12 | Structural refusal | **absent**: `check-composition-root-completeness` only counts `*Adapter` in `main.rs` (`check_composition_root_completeness.rs:1-14`) | blocking job `discipline.yml:431-443` | AC3 |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-8c809f9c.md`.

| Premise (epic / preflight / tracker) | Fact at `8c809f9c` and consequence |
|---|---|
| "≈ 12 production sites", "16 bypass files" (E20:310; S:315) | 3 maos-bin production + 1 smoke + **1 kernel** constructions; 21 direct sign/verify/seal files besides the provider (C8, C9). The kernel site was never inventoried → AC7 ◆. |
| Init `main.rs:1896-1948`; boot `:2190-2191`; CRL `:3211`; smoke `:8202`; later `:15081/:15342/:15817/:15846`; enterprise import `:648` (E20:308; PF:256-257) | `:1906-1913`; `:2218-2219`; `:3238-3239`; `:8403`; `:15354/:15615/:16090/:16119` (all test); `:649` (C1–C7). |
| "the audit sealed export" uses `seal_for_export` (E20:313) | It signs with ed25519-dalek in `maosctl`; nothing in maos-audit encrypts (C10) → D-20-13-10. |
| "capability-token signing must be routed" (E20:313) | Already through the port (C11); only the instance origin changes. |
| "visible in `maosctl` posture" (E20:314) | `maosctl posture` is a per-Spirit write verb (C13) → D-20-13-14. |
| "`runtime_context.rs` already names `ring`" (E20:314) | Doc only; production contexts are manifest-derived (C14) → AC4. |
| "Kernel: Yes; unmeasured" (PF:130) | Measured: KT3 +1 / −1 (C15). |
| maos-secrets / maos-audit "measured requests" (E20:306) | Both measure **0** under the recommended rows (seal already routed; audit artifacts outside FR48). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) run from a
`git archive 8c809f9c` copy root per prototype; the untouched copy reproduces the pins exactly (26841 / 95 files / set hash
`db39bc4a…`; kernel 21042 tokei; aggregate 192519). Detail and diffs: `creation/measurements.md`,
`creation/kloc-kernel-8c809f9c-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, ruled 2026-11-03; `20-13-flag-winston-decision.md`):**

| Shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| **KT3 required `new(policy, crypto)`, `Default` removed (D-04 A, recommended)** | **+1 (26842)** | **−1 (21041)** | 22 callers +1 arg (4 in `main.rs`) | `kt3-required-ctor-param.diff` |
| KT1 builder, `ring` default kept (D-04 B) | +20 (26861) | +14 (21056) | — | `kt1-builder-ring-default.diff` |
| KTd builder + `Debug` supertrait (D-04 B variant) | +11 (26852) | +6 (21048) | domain trait; 20 impls gain `Debug` | `ktd-builder-debug-supertrait.diff` |
| KI `identity()` on the trait (D-03 B) | +19 (26860) | +18 (21060) | domain +6; 20 impls | `ki-trait-identity.diff` |
| KO `open_sealed` on the trait (D-11 B) | +38 (26879) | +37 (21079) | domain +7; 20 impls | `ko-trait-open-sealed.diff` |
| none (D-04 C) | 0 | 0 | kernel T3 admission stays `ring` | — |

**Cumulative (ADR-069, `docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-48`):** 21042 + 20-1b 68 + 20-4 9 + 20-11 6
= 21125 expected before this story; **with KT3: 21124** (physical 26943); KT1 21139; KT3+KI 21142; KT3+KI+KO 21179. All under
the STOP 21209 and the 22000 alarm on their own. **With the concurrent PROPOSED rows** created the same day at the same HEAD —
20-8 KS +33 (`20-8-flag-winston-decision.md`), 20-14 KT +25 (`20-14-flag-winston-decision.md`) and 20-9 R +21
(`20-9-flag-winston-decision.md`) — the expectation is 21125 + 33 + 25 + 21 = 21204 before this story and **21203 with KT3**
(6 below 21209). Every costlier shape here crosses the STOP: KT1 21218, KTd 21210, KT3+KI 21221, any KO ≥ 21240 → D-20-13-16.

**kloc forecast** (tokei; HEAD = every pin, 1 line aggregate room; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · KT3 (AC7) | **−1** (physical +1) | `kt3` |
| maos-bin · `crypto_provider.rs` selection, table, journal (AC1, AC5) | **+95** | `nk` SEL |
| maos-bin · `crypto-recording` provider (AC2) | **+50** | `nk` REC |
| maos-bin · root wiring, Worker/host-B parameter, installed boot, init flag (AC1, AC4) | **+43** | `nk` WIRE |
| maos-bin · smoke/one-shot roots through `select`, help row, messages | +10…+40 [INFERENCE] | `gate-sim-nk.txt`; `verbs.rs:90-95` |
| maos-shell · `run_init` writes/rewrites `[crypto]` (AC1) | **+46** | `nk` INIT |
| maos-shell · messages | 0…+10 [INFERENCE] | — |
| maos-compliance · `evaluate_envelope_with`, core, unpinned rule (AC4) | **+22** | `nk` COMP |
| maos-compliance · drift detail | 0…+10 [INFERENCE] | — |
| maos-registry · `verify_publisher_sig_with`, `verify_installed` (AC4) | **+21** | `nk` REG |
| maos-registry · endpoint comparison (AC4.4, D-07 A) | +5…+20 [INFERENCE] | `install.rs:135-142` |
| maos-domain · port doc (comments) / helper | 0…+5 [INFERENCE] | `crypto.rs:1-49` |
| xtask · structural scan (AC3) | **+59** | `nk` GATE |
| xtask · `build-test-fixtures` entry, gate refinements | +12…+40 [INFERENCE] | `build_test_fixtures.rs:70-260` |
| maos-secrets, maos-audit, maos-cli, maos-capability | 0 | D-10 A, D-11 A, D-05 A |
| tests, fixtures, docs, data | uncharged | `kloc_check.rs` |
| **aggregate** | **+335 measured; +352…+460 total** | sum |

**Envelope (D-20-13-13 A, ruled 2026-11-03; D02 A ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | +1 / −1 | **26843 / 21042** | 0 | **yes — FLAG-Winston** |
| maos-bin | 34191 | +188 meas, +10…+40 | **34687** | 0 | yes |
| maos-shell | 615 | +46 meas, 0…+10 | **737** | 0 | yes |
| maos-registry | 4248 | +21 meas, +5…+20 | **4350** | 0 | yes |
| maos-compliance | 2032 (pin 2117) | +22 meas, 0…+10 | **2106** (pin kept) | 85 numeric | yes (aggregate) |
| maos-domain | 10099 | 0…+5 | **10114** | 0 | yes |
| xtask | 44406 | +59 meas, +12…+40 | **44644** | 0 | yes |
| aggregate | 192519 | +352…+460 | **193564** (+670 + 375); hardfail = measured + 1 | 1 | yes |

**Cumulative with other pending stories (stated, not granted):** each pending ceiling was computed from its own HEAD and
each lander re-measures against the exact landed pins of the one before. Forward draws on maos-bin from 34191: 20-4 ≤ +452
(L1 in flight measured 34363), 20-11 ≤ +507, this story ≤ +496 → worst case 35646; 20-2 (≤ 33534) and 20-1a/1b (≤ 34410) were
computed from earlier HEADs and are at or below today's pin — their landers re-ask. Aggregate forward draws: 20-4 ≤ 193526,
20-11 ≤ 194058, this story ≤ 193564 → worst case 196110 if every ceiling were drawn in full.

**Operator-ruling list (ruled 2026-11-03, ask `d_batch4`; binding at dev start after re-measurement):** (1) FLAG-Winston kernel row KT3 (D-20-13-04; KT1/KTd/KI/KO/none priced); (2) the
envelope above (D-20-13-13); (3) D-20-13-01…16. Each figure lands exact, with a HISTORY row whose first segment cites this
key with `->` (D11 A).

## Decisions — ruled 2026-11-03 (operator, ask `d_batch4`)

Options list the recommendation first only where it is A; evidence is cited; prototypes are in `creation/`.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-13-01** | Where the one persisted selection lives | **A** `[crypto] provider = "<name>"` in `<home>/config.toml`, written only by `maos init` (maos-shell), read once by `maos run` (maos-bin, `toml`) · **B** a dedicated `<home>/crypto-provider.json` read through maos-domain (the `control.json` pattern, `maos-domain/src/operator_door.rs`) · **C** an `[crypto]` section of `~/.config/maos/operator.toml` through the kernel's `operator_config` resolvers (env + disk precedence) | **A.** `config.toml` is already the init-owned operator file and has no reader to conflict with (C1); one file, one writer, one reader. B adds a second init-written file for one value. C spends kernel lines (`operator_config` is in maos-kernel-core, `main.rs:2228`) and adds an env-override convention the epic forbids (E20:308). Prototype `nk` INIT/SEL: shell +46, bin +95. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-02** | Re-selection and homes initialised before 20-13 | **A** `maos init --crypto-provider X` on an initialized home rewrites only `[crypto]` (stage + rename), prints `old -> new`, effective at the next boot; a home without `[crypto]` boots `ring` and journals `source: default` · **B** refuse re-selection (operator edits `config.toml`); refuse to boot a home without `[crypto]` until re-init · **C** first init only; later changes by hand | **A.** Init's early return keeps an initialized home's `config.toml` (`lib.rs:86-110`); every existing home (J0's first command) lacks `[crypto]` — B refuses all of them at upgrade. Nothing persistent is bound to the provider except ComplianceClaim attestations, which drift and refuse at the next boot by design (AC4). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-03** | Source of the journaled identity (name + version) | **A** the maos-bin provider table row (`name`, `version`), pinned to the locked `ring` (`Cargo.lock:4464-4465`, 0.17.14) by a test; the trait is unchanged · **B** KI: `CryptoProvider::identity()` (kernel +19/+18, domain +6, 20 impls) · **C** stderr only | **A.** The table row is the selection point: the identity and the instance come from the same row and cannot diverge; zero kernel lines. B makes every test double declare an identity for no reader the table lacks. C fails epic AC3. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-04** | Kernel T3 admission's inline `ring` (`security/mod.rs:350`) ◆ FLAG-Winston | **A** KT3: `SecurityManagerAdapter::new(policy, crypto)` required, `impl Default` removed, manual `Debug` (+1/−1; 22 callers, 4 in `main.rs`) · **B** KT1: builder `with_crypto_provider`, `ring` default kept (+20/+14; KTd +11/+6 with a `Debug` supertrait) · **C** none: kernel T3 admission stays `ring`, a permanent named gate exception and a labelled FR48 gap | **A.** Smallest and fail-closed by construction: a root that forgets the provider does not compile. B leaves a silent `ring` default inside the kernel — exactly the bypass AC3 refuses. C leaves FR48 false on the one kernel-internal verification the epic names (T3 image lock). Measured `creation/measurements.md`. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-05** | Scope of ComplianceClaim / publisher verification routing | **A** daemon only: boot re-verification of installed Spirits (`installed_spirits.rs:25` → `verify_installed`) verifies publisher and claim through the selected provider (`verify_publisher_sig_with`, `evaluate_envelope_with`); `maosctl spirit install`/`import` keep ring-direct, labelled "install-time pre-check; the daemon re-verifies before anything runs" · **B** also `maosctl`: move `RingCryptoProvider` out of the kernel into a crate `maosctl` can link and construct from the same selection · **C** none routed; claim verification recorded outside FR48 | **A.** FR48 names kernel signature verification; the daemon's boot re-verification is the admission a package passes before it executes (20-1a, `install.rs:108-110`). `maos-cli` has no kernel or maos-secrets dependency (`crates/maos-cli/Cargo.toml:22-25`); B is a crate move plus a second process reading the selection. Prototype `nk` COMP/REG: +22/+21. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-06** | `runtime_crypto_provider` in the runtime context (D:1379 crypto half) | **A** the selected name; an attested empty value is "unpinned" (compared as empty, as today), a non-empty one exactly · **B** strict: empty refused; manifests and templates gain `crypto_provider = "ring"`, every package re-published · **C** keep manifest-derived (vacuous) | **A.** No shipped manifest declares `crypto_provider` (C14), so B refuses every installed package at the next boot; A is strictly stronger than HEAD (a pinned claim now drifts on a different module). CCAC unchanged: drift seeds attest non-empty (`expansion.rs:240`), missing fields stay `MalformedClaim` (`ccac-seeds-v1.0.toml:259-275`) — measurement 4. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-07** | D:1379 endpoint half (runtime provider endpoint) | **A** boot re-verification compares the claim's `provider_endpoint` with the configured endpoint of its `provider_id` (20-4 L1's endpoint table; same unpinned-empty rule); `maosctl` callers labelled as in D-05 A; lands after 20-4 L1 · **B** all three callers; `maosctl` rebuilds endpoints from `maos_providers` defaults + `MAOS_OLLAMA_URL` · **C** re-route ownerless (rule 10) with the reason | **A.** Only the daemon has a runtime endpoint; B re-implements the daemon's table out of process and drifts from it. If 20-4 L1 has not landed at T0, AC4.4 waits for it (STOP + note), the rest of L1 proceeds. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-08** | Host of the structural refusal (epic AC2) | **A** extend `check-composition-root-completeness` (already blocking, `discipline.yml:431-443`, `aggregate.needs` `:6534`) with a crate-wide production scan for `RingCryptoProvider` (allowlist with reasons; test modules cut) · **B** a new gate `check-crypto-provider-routing` (gate-registry row; ship-gate count and coverage rows change) · **C** clippy `disallowed-types` | **A.** Same subject (one instance per root), no new job or registry row; prototype +59 xtask; failing-before measured (5 sites, `gate-sim-base.txt`). B costs gate-count churn for one scan. C is advisory (`clippy-advisory`). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-09** | Test-only `recording` provider | **A** maos-bin feature `crypto-recording` (`compile_error!` without `debug_assertions`, the `kms-fault-inject` precedent `maos-secrets/src/lib.rs:9-10`), log `<home>/crypto-recording.log` (no new env var), built once by `build-test-fixtures`; `RecordingCryptoProvider` joins `check-mock-not-in-release`'s forbidden symbols (`check_mock_not_in_release.rs:29`) · **B** one TL frame per call · **C** in-process only (unit test of the table) | **A.** Proves the composition root end to end (epic AC1) with the 20-12 no-cargo-in-test rule; B writes TL frames on the token-issue hot path and changes journal shape; C does not observe the root. Prototype +50. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-10** | Audit-artifact signatures (bypass inventory §C) | **A** outside FR48's three surfaces: each row dispositioned in the inventory and named in the operator page (the audit "sealed export" is a signature, C10) · **B** route the daemon's erasure-proof signature through the provider (needs a generic sign op: trait + kernel impl, KI-sized) · **C** route all (`maosctl` cannot build a provider, D-05) | **A.** FR48/NFR-Sec-15 name kernel signature verification, sealed-export encryption and capability-token signing; these are operator-tooling artifacts signed outside the kernel. A records them so v1.5's FIPS work starts from a named list. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-11** | At-rest open path (`open_at_rest`, ring-direct) | **A** keep, with the existing rationale (`maos-secrets/src/lib.rs:516-528`): FR48 names encryption; AES-256-GCM's inverse interoperates across providers · **B** KO: `open_sealed` on the trait + `ring` impl (kernel +38/+37, domain +7) and route open | **A.** 37 kernel tokei for a path FR48 does not name; a v1.5 module that cannot open `ring`'s standard AES-256-GCM output is a v1.5 finding, not a v1.0 one. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-12** | Landing shape | **A** one key, two landings: L1 kernel-free (AC1–AC6; the kernel T3 line is a named pending gate entry) · L2 kernel last (AC7; entry removed); `done` after L2 · **B** one landing · **C** split keys | **A.** Kernel-last precedent (20-1a/1b, 20-4, 20-11); L1 alone makes every daemon-side surface true. Interim, stated: between L1 and L2 kernel T3 admission verifies with `ring` whatever is selected (v1.0 ships only `ring`, so no behaviour differs); no release tag between the landings (20-5 runs last). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-13** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** 20-3b and 20-3c landed at 2–3× creation forecasts; 20-7 L1 review corrections added +387 xtask inside its ceiling (`xtask/kloc.toml:339` history). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-14** | Where an operator sees the selection (epic AC3 "maosctl posture") | **A** the boot TelemetryEvent `crypto.provider.selected` `{provider, version, source}`, read with the existing `maosctl audit query --frame-kind TelemetryEvent --intent-contains crypto.provider`; it enters audit bundles with the TL · **B** a door route + new `maosctl` read verb · **C** stderr only | **A.** No new verb (verb-table and `check-exit-commands` churn), the TL is what auditors already export; precedent `secret.source.fallback` (`main.rs:3782-3789`). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-15** | Ordering against 20-1b (PF:130,141 "after 1's signing/attestation decisions") | **A** ready-for-dev now: 20-1's decisions are ruled (D-20-1-*), 20-1a landed boot re-verification; file-level serialization with 20-1b on `install.rs`, `admission.rs`, `evaluator.rs` (one integration owner, the later lander re-measures) · **B** wait for 20-1b `done` | **A.** This story changes which instance verifies, not the claim or vetting shape 20-1b AC6 binds; waiting idles a kernel-free L1. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-16** | Cumulative kernel STOP (21209) with the concurrent creation-priced rows 20-8 KS +33, 20-14 KT +25 and 20-9 R +21 | **A** no action: under KT3 the expectation is 21203 (6 below), this story lowers it by 1 · **B** split: any costlier ruled shape here (KT1, KTd, KI, KO — each crosses 21209) moves to a separate key landing after 20-8/20-9/20-14 re-base · **C** exchange: a costlier shape is granted only against an equal measured reduction elsewhere (e.g. KO deferred to v1.5 with the FIPS module) · **D** bump: the operator raises the internal STOP — the joint targeted STOP **21250** proposed by D-20-9-15 A / D-20-14-12 (covers their ×2 allowances, 21249); at measured figures KT1 21218, KTd 21210, KT3+KI 21221 fit, KO ≥ 21240 fits narrowly (ADR-069 alarm 22000 unchanged, `ADR-069:35-48`) | **A** with the recommended rows (D-04 A, D-03 A, D-11 A). If any of those is ruled otherwise, the crossing figures above apply and the operator picks B, C or D in the same ruling; ceiling sums are not the test (`20-4-flag-winston-decision.md` §Kernel grant). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-13-17** | Empty attestation scope in the shared evaluator | Preserve HEAD's exact comparisons for ring-direct `evaluate_envelope` / `evaluate_envelope_at`; only selected-provider daemon boot `evaluate_envelope_with` treats an empty attested crypto name or endpoint as unpinned. Non-empty attestations compare exactly on both paths. | The real install/import callers can pass non-empty manifest-derived values; applying unpinned behavior to the shared core would weaken their previous refusal. See `landing-1/evaluator-callers.md`. | RULED 2026-11-10 by orchestrator adjudication of D-20-13-06 A; disclosed to the operator in the landing report. |

## Acceptance Criteria

*(Epic §20-13 as amended 2026-10-30; clauses marked (D-…) carry the ruled recommendation and are rewritten by a
different ruling. Every AC names a failing-before witness and a plant. **local** = hermetic on the dev host; **hosted** =
the discipline run.)*

### L1 — kernel-free

#### AC1 — one selection, one instance (epic AC1; D-01, D-02) — the command

1. `maos init --crypto-provider <name>` validates `<name>` against `crypto_provider::PROVIDERS` and records
   `[crypto] provider = "<name>"` in `<home>/config.toml`; plain `maos init` on a fresh home records `ring`; on an
   initialized home the flag rewrites only `[crypto]` and prints `old -> new` (D-02 A). Air-gap `maos init` takes the
   same flag (`main.rs:1575-1581`). Unknown name: non-zero exit, message names the known providers, `config.toml` unchanged.
2. `maos run` calls `crypto_provider::select(home)` once and builds exactly one `Arc<dyn CryptoProvider>`; the capability
   registry (`main.rs:2261`), enterprise runtime (`:2339`), CRL poller and door (`:3238-3245`, `operator_door.rs:441`),
   Worker T3 lock (`worker_spawn.rs:888`, callers `main.rs:5504,6088`, host-B `delegation.rs:846-875`), smoke arm
   (`:8403`) and installed-package boot verification (AC4) receive clones of it. Every other process root (one-shot and
   smoke fns) calls `select` itself — one instance per process, never a direct construction.
3. A recorded name this build cannot construct (or a malformed `[crypto]`) refuses boot with exit 78 and a message naming
   the known providers — never a silent fall-back to `ring`. A home with no `[crypto]` boots `ring` (D-02 A).
4. **Failing-before (local):** at HEAD `maos init --crypto-provider ring` is ignored (no `[crypto]` written) and
   `[crypto] provider = "bogus"` boots with `ring-default` (C1, C2). Witness `crates/maos-bin/tests/crypto_provider_20_13.rs`
   (isolated `HOME`/`MAOS_HOME`; the prebuilt binary via `MAOS_FIXTURE_DIR`/`CARGO_BIN_EXE_maos`): init writes, re-selection
   rewrites, unknown name refused at init and at boot (exit 78, stderr names `ring`).
5. **Plant (local, one):** make `select` fall back to `ring` on an unknown name ⇒ the exit-78 vector reds.

#### AC2 — the routing is proven, not asserted (epic AC1's recording clause; D-09)

1. Feature `crypto-recording` adds the `recording` row (delegates to `ring`, appends one op name per call to
   `<home>/crypto-recording.log`); `compile_error!` without `debug_assertions`; `RecordingCryptoProvider` is added to
   `FORBIDDEN_PRODUCTION_SYMBOLS` (`check_mock_not_in_release.rs:29`); the fixture `maos` built with the feature by
   `cargo run -p xtask -- build-test-fixtures` (new entry, `build_test_fixtures.rs:70-260`), never by `cargo` inside a test.
2. With `recording` selected, one hermetic boot records: `sign_capability_token` for a token issue; `verify_signature` for
   a CRL import over the door; `seal_for_export` for an at-rest seal (T0 measurement 3 names the trigger); two
   `verify_signature` for one installed package's boot re-verification (AC4). The Worker T3 lock records
   `verify_signature` on the T3 harness leg (hosted, `provision-t3-test-substrate`; skips locally with a named reason).
3. **Failing-before (local):** the feature and the row do not exist at HEAD; recorded against L1 before AC1.2's wiring:
   the CRL leg writes nothing (second instance).
4. **Plant (local, one):** restore the CRL poller's own `RingCryptoProvider` ⇒ the CRL `verify_signature` line is absent
   ⇒ red.

#### AC3 — no FR48 surface bypasses the selection (epic AC2; D-08, D-10)

1. `check-composition-root-completeness` (D-08 A) refuses any production `RingCryptoProvider` construction outside
   `crates/maos-bin/src/crypto_provider.rs`, the provider's own module and test modules; the allowlist names each other
   entry with a reason (`maos-bench/src/*` harnesses; at L1 the kernel line `security/mod.rs:350` as "pending 20-13 L2").
2. The bypass inventory is the story's record (`creation/bypass-inventory-8c809f9c.md`): §A rows routed or allowed, §B
   routed (AC1, AC4) or recorded (D-11), §C/§D recorded outside FR48 (D-10); the dev re-runs it at T0 and at landing.
3. **Failing-before (measured):** the scan reds 5 sites at HEAD (`creation/gate-sim-base.txt`).
4. **Plant (local):** add `let _c = maos_kernel_core::security::crypto::RingCryptoProvider;` to `main.rs`'s root ⇒ red.

#### AC4 — claims and packages are verified by the selected module (epic AC2 "admission-time ComplianceClaim verify"; D:1379; D-05, D-06, D-07)

1. `maos_compliance::evaluate_envelope_with(envelope, ctx, crypto)` and
   `maos_registry::admission::verify_publisher_sig_with(pkg, crypto)` exist (one evaluator core, no duplicated logic); the
   boot re-verification (`installed_spirits.rs:25` → `verify_installed`) uses both with the selected instance (D-05 A).
2. The runtime context carries the selected provider name (`install.rs:135-142`); an attested empty `crypto_provider` is
   unpinned, a non-empty one is compared exactly (D-06 A). `maosctl spirit install`/`import` keep ring-direct verification
   and say "install-time pre-check" in their output docs (D-05 A).
3. **Failing-before (local):** an installed package whose claim attests `crypto_provider = "fips-module"` boots and admits
   at HEAD (manifest-derived context, C14); after: refused `ContextDrift { CryptoProvider }` and the Spirit is not loaded.
   A package attesting `""` still admits. Witness in `crypto_provider_20_13.rs` (publish/install fixture shape of 20-1a).
4. (D-07 A) The claim's `provider_endpoint` is compared with the configured endpoint of its `provider_id` (20-4 L1's
   table), unpinned-empty rule as above; **failing-before:** a claim attesting `https://evil.example` for `anthropic`
   admits at HEAD. If 20-4 L1 has not landed at T0, this clause waits (D-07) and the record says so.
5. `deferred-work.md:1379` → `CLOSED <date> by 20-13 @<commit>` for the halves that land; any half left open is re-labelled
   with its owner and reason.
6. **Plant (local):** pass `fields.crypto_provider` (manifest-derived) again ⇒ AC4.3 admits ⇒ red.

#### AC5 — the selection is journaled once and visible (epic AC3; D-03, D-14)

1. After the Transparency Log opens, the root writes one TelemetryEvent `crypto.provider.selected` with
   `{"provider","version","source": "config"|"default"}`; the `ring-default` eprintln (`main.rs:2219`) names the selection.
2. `maosctl audit query --frame-kind TelemetryEvent --intent-contains crypto.provider` shows it (D-14 A); a test pins the
   table's `ring` version to `Cargo.lock`'s.
3. **Failing-before (local):** no such frame at HEAD. **Plant:** skip the journal call ⇒ the frame vector reds.

#### AC6 — docs and records (epic AC4)

1. `docs/maos.dev/run-maos.md`, `docs-site/docs/run-maos.md` and the ko page gain a "Crypto provider" section: the selection
   point and file, the default, re-selection effect, the exit-78 refusal, the journal frame, the v1.5 FIPS plan and the claim
   boundary (v1.0 ships `ring` only; pluggability, not validation; audit artifacts, publisher tooling and TLS are outside).
2. Stale docs rewritten: `maos-domain/src/ports/crypto.rs:1-49` (C16), `maos-kernel-core/src/security/crypto.rs:20-27` (L2,
   kernel physical counted), `tests/coverage-matrix.yaml:740-754` (FR48 names AC1–AC5 witnesses and the gate),
   `:1733-1738` (NFR-Sec-15: provider-trait half witnessed, FIPS half v1.5).
3. At `done`: D:1379 per AC4.5; `deferred-work.md:110` unchanged (v1.5); epic corrections via `creation/records-owed.md` §A.

### L2 — kernel last

#### AC7 — kernel T3 admission verifies with the selected provider (D-20-13-04 A) ◆ FLAG-Winston

1. KT3 (`kt3-required-ctor-param.diff`): `SecurityManagerAdapter::new(policy, crypto)`; the T3 arm (`security/mod.rs:349-366`)
   uses `self.crypto`; `impl Default` removed; every caller passes the root's instance (production) or a test provider.
2. **Failing-before (local):** `crates/maos-kernel-core/tests/t3_admission_crypto_20_13.rs`: a validly signed lock and trust
   anchor via `with_t3_image_verification`, an adapter built with a rejecting provider ⇒ T3 admission must refuse
   `T3AdmissionFailed`; at L1 the inline `ring` admits it (observed before KT3, recorded). With `recording` (bin leg),
   L1 records no kernel `verify_signature`; L2 records one.
3. The L1 pending gate entry is removed; the scan finds no production construction outside the allowlist.
4. **Plant (local):** restore the inline `RingCryptoProvider` ⇒ the rejecting-provider vector admits ⇒ red, and the scan reds.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D-20-13-01…16 recorded; tracker → `in-progress`. Run `creation/measurements.md`
  "Not measured here" 1–7; re-run the bypass inventory and gate simulation; re-measure every cite this story touches (rule 9).
  A measured figure above the ruled envelope, or a T0 result contradicting a ruled option, is STOP + D-row.
- [ ] **T1 — Selection (AC1).** `crypto_provider.rs` (+ `lib.rs`, `SCANNED_SOURCE_FILES` 37 → 38 in
  `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:931`), `run_init` writer, init flag both builds, root wiring, Worker /
  host-B parameter, one-shot roots through `select`, witness, plant.
- [ ] **T2 — Recording provider (AC2).** Feature, `compile_error!`, forbidden symbol, `build-test-fixtures` entry +
  `xtask/test-budgets.toml` row if needed, routing vectors, plant.
- [ ] **T3 — Structural check (AC3).** Scan + allowlist, failing-before record, plant.
- [ ] **T4 — Claims (AC4).** `evaluate_envelope_with`, `verify_publisher_sig_with`, `verify_installed`, unpinned rule, endpoint
  half per D-07, CCAC re-run, witnesses, plant.
- [ ] **T5 — Journal (AC5).** Frame, eprintln, `Cargo.lock` pin test, plant.
- [ ] **T6 — Docs/records (AC6).**
- [ ] **T7 — L1 landing (D-20-13-12 A).** Exact pins inside the ruled envelope; `kloc-check`, `check-kernel-baseline`
  (unchanged), `check-composition-root-completeness`, `check-mock-not-in-release`, `check-service-boundary`, `check-env-contract`,
  `check-corpus`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`,
  `check-decision-register`, `prepush`; one hosted discipline run on the final tree + plants. Revert
  `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.
- [ ] **T8 — Kernel T3 (AC7)** under the ruled row: red-before test, KT3, callers, gate entry removed, plant.
- [ ] **T9 — L2 landing.** Kernel pins + `security/mod.rs` content hash (`kernel-core-baseline.toml:505` + EOF HISTORY;
  `kloc.toml:264`), re-based on whichever of 20-1b/20-4/20-11 landed first; kernel-surface baseline per measurement 2;
  FLAG-Winston record updated to GRANTED/LANDED; `cargo test -p maos-kernel-core --lib`; hosted run; ledger; coverage.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Bin | NEW `crates/maos-bin/src/crypto_provider.rs`; `src/lib.rs`; `Cargo.toml` (`crypto-recording`); `src/main.rs:1575-1581,1906-1913` (init flag), `:2218-2219` (select), `:2278-2281` (security), `:3238-3245` (CRL), `:2404`→journal, `:4332-4345` (installed boot), `:5504,6088` (Worker), `:8403` (smoke), `:9304`, `:11874-11930`, `:11948` (cohort/host-B), `:14813` + `:15321,16107` (smoke/test `new` callers); `src/worker_spawn.rs:585-599,888`; `src/delegation.rs:846-875,925-955`; `src/installed_spirits.rs:16-30`; `tests/cohort_daemon_smoke_13_5c.rs:931`; NEW `tests/crypto_provider_20_13.rs` | AC1, AC2, AC4, AC5 |
| Shell | `crates/maos-shell/src/lib.rs:54-155,784-801` | AC1 |
| Compliance | `crates/maos-compliance/src/evaluator.rs:77-180,317-321`; `src/lib.rs:42` | AC4 |
| Registry | `crates/maos-registry/src/admission.rs:549-562`; `src/install.rs:111-150` | AC4 |
| xtask | `xtask/src/check_composition_root_completeness.rs`; `xtask/src/check_mock_not_in_release.rs:29`; `xtask/src/build_test_fixtures.rs:70-260`; `xtask/test-budgets.toml` (data) | AC2, AC3 |
| Kernel ◆ (L2) | `crates/maos-kernel-core/src/security/mod.rs:130-141,201-235,349-366,479-487`; `src/security/crypto.rs:20-27` (doc); kernel tests constructing `SecurityManagerAdapter` (`tests/provider_switched_journal.rs`, `sandbox_admission.rs`, `operator_posture_ceiling_16_6.rs`, `abi_stability_admit.rs`); NEW `tests/t3_admission_crypto_20_13.rs`; pins `xtask/kernel-core-baseline.toml:505` + HISTORY, `xtask/kloc.toml:264` | AC7 |
| Other callers (uncharged) | `crates/maos-bin/tests/*` (13 files with `SecurityManagerAdapter::new`), `crates/maos-escape-detector/tests/common/mod.rs`, `xtask/tests/story_10_4a_ac1_proven_red.rs:996`, xtask fixtures `p1-*` | AC7 |
| Domain | `crates/maos-domain/src/ports/crypto.rs:1-49` (doc) | AC6 |
| Docs / records | `docs/maos.dev/run-maos.md`; `docs-site/docs/run-maos.md` + ko; `tests/coverage-matrix.yaml:740-754,1733-1738`; `deferred-work.md:1379` | AC6 |

### Overlap (serialize; re-measure)

- **`main.rs` root / `admission.rs`** — 20-4 L1 (in flight: provider builder, endpoint table, `AdmissionDeps`), 20-1b (yank
  policy, admission identity), 20-11 (uninstall cascade). One integration owner per file per landing; rebase on 20-4 L1.
- **`maos-registry/src/{install.rs, admission.rs}`, `maos-compliance`** — 20-1b AC6 (claim binds artifact + publisher key),
  20-11 AC4.3 (`storage.rs`, additive) — D-20-13-15.
- **Kernel pins** — 20-1b, 20-4, 20-11 ruled rows; 20-8/9/14 forecast; the later lander re-bases (cumulative above).
- 20-11's overlap note says 20-13 touches `maos-iac` `transparency_log.rs`: it does not (the TL adapter signs nothing;
  the journal call uses `insert_frame_event`).

### Current state that must be preserved

- Capability-token issue/verify semantics and the < 5 µs verify hot path (`cap_tokens/mod.rs:216-258`): the provider is used
  at issue only.
- CRL replay protection and the boot-captured door anchor (`main.rs:3222-3237`, D-16-1-X).
- T3 fail-closed admission: no development fallback (`worker_spawn.rs:882-893`, `security/mod.rs:349-366`); the shipped
  placeholder lock is never verifiable (`image_lock.rs:171-196`).
- At-rest seal format `maos.at-rest.v1` and `open_at_rest` compatibility (`maos-secrets/src/lib.rs:462-540`).
- `maos init` idempotency and the one-writer rule for `control.json` (`maos-shell/src/lib.rs:44-53,157-180`).
- CCAC N=600 verdicts (`check-corpus`); 20-1a boot re-verification refusals (`installed_spirits.rs:25-45`).

### Guardrails

- No kernel line without the ruled FLAG-Winston row; any `crates/maos-kernel-core/src` change outside AC7's shape is STOP —
  including a trait method (D-03 B / D-11 B would be separate ruled rows).
- No second selection file, env override or provider registry; no `RingCryptoProvider` construction outside the table.
- The recording provider never reaches a release build (`compile_error!` + forbidden symbol) and is never selectable there.
- No `unwrap`/`expect` after serde/toml parsing in production code; no test spawns `cargo` without the expensive marker
  (20-12); a new maos-bin `src` file joins `SCANNED_SOURCE_FILES`; any new env var joins `env_contract.rs` (none planned).
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory; plants byte-restored in the real
  tree (`proof-contract.md:24-31`).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Unknown provider refused (AC1) | `select` falls back to `ring` | exit-78 vector | local |
| One instance (AC2) | CRL poller's own `RingCryptoProvider` restored | CRL `verify_signature` line absent | local |
| Structural check (AC3) | a construction in the root | gate red | local |
| Claim drift on crypto (AC4) | manifest-derived context restored | `fips-module` claim admits | local |
| Journal (AC5) | journal call skipped | frame vector | local |
| Kernel T3 (AC7) | inline `ring` restored | rejecting-provider vector admits; gate red | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- build-test-fixtures
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test crypto_provider_20_13
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-shell -p maos-compliance -p maos-registry -p xtask
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test t3_admission_crypto_20_13      # L2
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib                                        # L2
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-composition-root-completeness --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-corpus
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-exit-commands
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-decision-register
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

`check-mock-not-in-release` runs in CI on the release binary (`discipline.yml:400-415`).

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC1.3 (no silent fall-back), AC3 (allowlist),
AC4 (drift rule, unpinned semantics) and AC7, Test Infrastructure (glm-5.3). Focused suites per touched crate +
`cargo test -p maos-kernel-core --lib` at L2; the reviewer reads the plant record and the hosted T3 run, never re-runs plants
except for a disputed finding.

### Previous-story intelligence

- 20-4 (creation 2026-11-02, L1 in flight): the root's provider builder and an endpoint table at admission — D-07's input;
  its hosted/live workflow cut but not run; rebase main.rs edits on it.
- 20-1a: boot re-verification of installed packages (`8143326b`, `47631284`) is where AC4 routes verification; its hosted run
  found faults local runs missed — run the hosted discipline once on the final tree.
- 20-11: kernel-last two-landing shape and the re-base rule; this story's kernel row re-bases on it.
- 20-7: kernel landed at exactly the prototype's figure; a key rename broke two doc gates `prepush` does not run — this story
  renames no key.
- 18-0 D-18-0-03: the zbus advisories once routed here were re-owned by 20-5 (`deferred-work.md:1288`) — not this story.

### Git intelligence

`8c809f9c` (20-10 hosted fixes; bin 34191, aggregate 192519), `16c35ef0`, `ac1fc12a`, `75538bf7`, `d3e3f586` (20-2/4/10/11
created and ruled). `security/mod.rs` last changed `18082789` (19-4/19-5); `security/crypto.rs` `3d751b4e` (5-4);
`ports/crypto.rs` `0a439b7a` (1b.2); `install.rs`/`installed_spirits.rs` 20-1a (`8143326b`, `47631284`); `evaluator.rs`
`cf6abb59` (7-3).

### Library / framework

No new crate. `ring` 0.17.14 (locked; `Ed25519KeyPair::from_seed_unchecked`, `UnparsedPublicKey`, `AES_256_GCM`);
`toml` (already a maos-bin dependency) for `config.toml`; Ed25519 signatures are deterministic (RFC 8032), so routing a
verification through the provider changes no byte of any signed artifact.

### References

E20@HEAD:86,304-317; preflight `epic-20-preflight-2026-10-30.md:86,130,141,251-257`; tracker `sprint-status.yaml:315`;
`deferred-work.md:110,372,1379`; routing audit `routing-audit-2026-10-22.md:68,128`; requirements inventory
`epics/requirements-inventory.md:17,139,371,477,564,617`; PRD `functional-requirements.md:32`, `non-functional-requirements.md:48`;
`compliance-claim-schema-review.md:321`; ADR-069 (`:35-48`); review scout `review-2026-09-04/05-fr1-20-scout.md:24`.

## Dev Agent Record

### L1 preflight — 2026-11-09

At HEAD `37baeedf` the baseline is kernel 26,841 physical / 21,042 tokei,
bin 35,914, shell 615, registry 6,151, compliance 2,077, domain 10,198,
xtask 44,700, aggregate 198,235. Re-applying D02 A's ruled allowances gives
ceilings bin 36,410, shell 737, registry 6,253, compliance 2,151, domain
10,213, xtask 44,938, aggregate 199,280. L1 does not edit the kernel; rebase
KT3 again at L2. Previous baseline and the AC1 observed-red run are recorded in
`20-13-pluggable-crypto-provider-selection-evidence/landing-1/preflight.md`.
20-4 L1's `ProviderCatalog` is the runtime endpoint source.

### L1 implementation candidate — 2026-11-09

The kernel-free candidate selects a configured provider once in the composition
root, shares it across the L1 consumers, persists the init selection, records one
boot journal frame, and supplies real runtime values to installed-package
admission. The debug-only recording provider and the structural gate's live
negative control exercise the boundaries; see `landing-1/preflight.md` and
`landing-1/bypass-inventory.md`. T3 kernel admission remains an explicit L2
exception; no AC7 or hosted-substrate success is claimed by L1.

After formatting, measured bin 36,209, shell 673, registry 6,194, compliance
2,107, domain 10,198, xtask 44,874, kernel 21,042 and aggregate 198,835;
all are within the rebased ceilings above. Exact pins and hardfail 198,836
replace the baseline pins without reserved headroom. The L1 candidate is
uncommitted and the story is not marked complete.

The new integrated witness is placed under `tests/support/` per the 14-2b
test-location convention, not moved from an existing counted test. Tokei
excludes that file; the measured bin source total **without the test body** is
36,209 (baseline 35,914, growth +295), 201 below the rebased 36,410 ceiling.
This total still includes the test-feature recorder's identity instrumentation
and inline shared harness code. A temporary inline form measured 36,431 before
the authorized new-test placement; it was never re-pinned.

**D-05 boundary / pending:** only `verify_installed` daemon boot
re-verification routes publisher and claim verification through the selected
provider. Ring-direct `admit_spirit` (`maos-registry/src/admission.rs:152,
296,313`), `maos-bin/src/yank_policy.rs:100`, `maos-bin/src/admission.rs:578`
and `maos-bin/src/main.rs:193,10769` remain separately recorded for the
L2 / v1.5 FIPS integration under `deferred-work.md` (rule 10(b)); no claim is
made that all admission paths use the selected provider. FKCS admission
fingerprint was re-pinned only after the independent `landing-1/review/
fkcs-admission-review.md` and the orchestrator's explicit authorization.

**L1 file list.** `crates/maos-bin/{Cargo.toml,src/{crypto_provider.rs,
delegation.rs,installed_spirits.rs,lib.rs,main.rs,worker_spawn.rs},
tests/{crypto_provider_20_13.rs,crypto_provider_t3_20_13.rs,
cli_wrapper_exhaustion_17_9.rs,cohort_daemon_smoke_13_5c.rs,
nfr_perf_8_real_spawn_fanout_20_7.rs,support/{worker_revoke_root.rs,
shell_world_16_2.rs,crypto_integrated_20_13.rs},two_host_delegation_2b.rs,
worker_terminal_facts_17_7/unload_scenes.rs,
erasure_uninstall_13_5b.rs}}`; `crates/maos-compliance/{src/{evaluator.rs,lib.rs},
tests/evaluator_test.rs}`; `crates/maos-domain/src/ports/crypto.rs`;
`crates/maos-registry/{Cargo.toml,src/{admission.rs,install.rs},
tests/{yank_auth_20_1b.rs,publisher_provider_20_13.rs}}`;
`crates/maos-shell/{Cargo.toml,src/lib.rs}`; `Cargo.lock`,
`.config/nextest.toml`, `tests/coverage-matrix.yaml`,
`xtask/{kloc.toml,fkcs-baseline.toml,gate-registry.toml,test-budgets.toml,
src/{build_test_fixtures.rs,check_composition_root_completeness.rs,
check_mock_not_in_release.rs}}`; `docs/maos.dev/run-maos.md`,
`docs-site/docs/run-maos.md`, the Korean `docs-site` run page; this story,
`sprint-status.yaml` (decision comment only), `deferred-work.md`, and L1
evidence `{preflight.md,verification.md,bypass-inventory.md,evaluator-callers.md}`
plus the branch-only `ci/ci-20-13-verify.yml`.

**Hosted fixes carried by 20-13 landing (not 20-13 scope).** Main reported
discipline run 42722028970 on `be3b9763`: stale 18-pass recall workflow-shape
assertions and an ARM cgroup descendant that was already dead (`X`) rather than
zombie (`Z`). `xtask/tests/ci_shape_18_0.rs` derives recall and recall-wire
counts/names from the Rust source and plants each missing token independently;
`crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs` accepts reaped,
`Z`, `X`, or `x`, but not a live state. The original `X` and subsequent `x`
vectors failed before their respective fixes (exit 101, `artifact://15717` and
`artifact://16016`). Both changes are test-only;
no kernel source or kernel pin is changed.

**Hosted correction to review F1/P7.** The review allowed either refusing an
unresolvable home or rejecting a non-absolute configuration path. Its refusal
fix conflicted with the earlier D-18-7-R1 contract: unavailable home must not
abort the daemon or independent offline children with explicit data roots.
When neither `MAOS_HOME` nor `HOME` resolves, provider selection now defaults
to `ring` without constructing or reading any configuration path; the
selection frame records `source: default`. An explicitly resolved `MAOS_HOME`
still controls configuration even with `HOME` unset. The unchanged 18-7 journey
and `absent_home_never_reads_working_directory_config` witness both behaviors,
including an ignored bogus CWD configuration.
The formatted correction measures bin 36,225 -> 36,234 (+9; ceiling
36,410), and aggregate 199,049 -> 199,058 (+9; hardfail re-pinned to
199,059). The 20-14 baseline is retained; kernel remains 26,841 / 21,042.

**Review Defer 1 adjudication (2026-11-10).** Dismissed: I2 fail-stop already
guarantees no unjournaled selection. Main corrected the premise of the operator
`d1w` ruling: `insert_frame_event` returns `LogBeforeDeliver<()>`, not a
`Result`; SQLite write failure panics before consumers are served. Do not
catch the panic or introduce a fallible append API. The real-boot SQLite
failure-injection witness is `selection_journal_failure_stops_real_boot`.

The recording-provider compile guard is `not(debug_assertions)`: a custom
release profile enabling debug assertions can compile it. Release symbol
inspection (`check-mock-not-in-release`) is the separate backstop; no such
profile override was found by review.

**P4 HEAD-equivalence adjudication (2026-11-10).** At `be3b9763`,
`run_uninstall_cascade_inner` calls `has_frames()` with no frame exemption
(`main.rs:10823-10826`) and `run_uninstall_cascade` appends the terminal
Decision even for NotFound (`main.rs:10581-10589`). The existing
`empty_transparency_log_is_not_found_even_for_unknown_spirit` test covers the
first call only; no repeated-unknown test was found in the two uninstall
binaries. The HEAD results below are inferred from those code paths, not
measured by running the old tree:

| Empty home, same unknown Spirit | HEAD (code-derived) | Corrected L1 |
|---|---|---|
| First uninstall | 4 / NotFound | 4 / NotFound, ignoring kernel selection rows |
| Second uninstall | 5 / Failed: first NotFound Decision made the log populated | 5 / Failed for the same Decision row |

Selection rows from every boot are excluded only when kind is TelemetryEvent,
origin Kernel, pid 0, and intent exactly `crypto.provider.selected`. A
Spirit-origin frame with that intent remains populated evidence.

**P5 real-mode refinement (Main adjudication of `p5w`, 2026-11-10).**
The literal one-boot four-surface witness remains the in-process
`crypto_integrated_20_13` daemon harness. Real binaries are witnessed per mode:
`crypto_real_shell_20_13` covers installed boot re-verification, a live-door
CRL import and token issuance in one shell root; `crypto_real_boot_20_13`
covers a cohort root's governed token and at-rest seal (and live-door CRL).
No single production mode reaches all four: cohort one-shot mode deliberately
skips the installed inventory. This security question is ownerless-and-open,
dispositioned in `deferred-work.md` for the first story touching cohort boot.
Both real witnesses and the in-process witness run in the blocking
composition-root discipline job.

Review corrections additionally change `.github/workflows/discipline.yml`,
`crates/maos-bin/tests/{crypto_endpoint_drift_20_13.rs,
crypto_real_shell_20_13.rs,crypto_real_boot_20_13.rs}`, the existing
`crypto_provider_20_13.rs` and erasure witness, composition-root guard,
`xtask/gate-registry.toml`, test budgets and nextest serial routing, and the
operator docs / evidence / deferred records listed above. No kernel source or
FKCS-declared admission file is modified by these corrections.

After review corrections and `cargo fmt --all`, exact `kloc-check` measurement:
bin 36,225 (+16 from candidate; ceiling 36,410), shell 675 (+2; ceiling 737),
compliance 2,107 (unchanged), registry 6,194 (unchanged), domain 10,198
(unchanged), xtask 44,935 (+61; ceiling 44,938), kernel 21,042 (unchanged),
aggregate 198,914 (+79; ceiling 199,280). The exact pins and aggregate
hardfail 198,915 are re-pinned together; `tests/` witness bodies are excluded.

### L2 candidate — kernel T3 admission (AC7), 2026-11-12 — uncommitted; story stays in-progress

**Ruling (operator, 2026-11-12, ask `kt3`):** the LITERAL KT3 shape. `SecurityManagerAdapter` carries the selected
provider as a field of the public struct (`new(policy, crypto)`, `impl Default` removed, manual `Debug`), and
**exactly one** kernel surface hash is refreshed: `maos_kernel_core::security::SecurityManagerAdapter`
(`76:bb:bb:10:3f:98:d9:4e:…` -> `60:35:04:56:09:f4:28:36:…`, full values in `landing-2/evidence.md`). `check-service-boundary`
was run before the refresh and flagged that one symbol and nothing else. The refresh is also pre-approved by the
2026-11-12 standing rule `prepaid` (a) (a symbol named in this measured FLAG-Winston grant); both are recorded in
`20-13-flag-winston-decision.md` §L2 and the tracker comment.

**Change.** `security/mod.rs`: the field, `new(policy, crypto)`, manual `Debug`, the T3 arm verifies through
`self.crypto` on both the explicit-lock and the environment branch, `Default` removed; `security/crypto.rs`: module
doc rewritten line-neutral. Callers migrated: `maos-bin/src/main.rs` (the supervised root passes `Arc::clone(&crypto)`,
the same instance the capability registry receives; the 7.5a smoke arm takes the root's provider as a parameter; two
test-module roots), 14 `maos-bin` integration/support files (shared selected provider in
`support/crypto_integrated_20_13.rs`, test-local ring elsewhere), `maos-escape-detector/tests/common/mod.rs`, four
kernel integration files and `xtask/tests/story_10_4a_ac1_proven_red.rs`. The xtask structural check drops the
pending-T3 exception and its visitor state (`pending_t3`, `security_impl`, `visit_item_impl`), keeping the
`cfg(test)` skip and negative controls; direct ring construction in the T3 arm is now rejected.

**Witnesses** (`crates/maos-kernel-core/tests/t3_admission_crypto_20_13.rs`; full logs under `landing-2/`):

| Vector | Red before (HEAD, pre-KT3 helper) | Plant (inline ring restored) | Final |
|---|---|---|---|
| explicit lock, selected provider rejects every signature -> `T3AdmissionFailed`, no `Load` journaled | RED: admitted | RED | green |
| explicit lock, recording provider sees exactly one `verify_signature` on the selected instance | RED: 0 calls | RED | green |
| environment lock (`MAOS_T3_IMAGE_LOCK_PATH` + trust anchor hex): ring admits; rejecting refuses; recording counts 1 | RED | RED | green |
| controls: valid lock + ring admits; tampered signature and wrong trust anchor refuse | green | green | green |
| gate unit: kernel `admit_spirit` ring construction is a violation (2 of 2, no exemption) | RED (left 1, right 2) | — | green |
| `check-composition-root-completeness` on the restored inline line | — | RED (`security/mod.rs:363`) | green |

The red-before form differs from the final file in one helper line (`adapter_with`: HEAD ignores the provider and
builds `SecurityManagerAdapter::new(policy)`), recorded as the transitional pre-routing state.

**Measured (after `cargo fmt --all`, live base `456793ca`):** kernel **27044 physical / 21202 tokei** (+1 / −1: the
priced KT3 figure, under the shared STOP 22000), maos-capability 1454, maos-domain 10274, maos-iac 8771, maos-cli
6612 (all unchanged), **maos-bin 38563 (+7 over the live pin 38556**: four call sites; overdraft for Main
adjudication; the L1 ceilings in the frontmatter are stale absolutes, the cumulative L2 draw is +7), xtask 45355
(−24), aggregate 202917, hardfail 202918. Pins are re-fixed in `kernel-core-baseline.toml` (src_lines 27044, two file
digests, set hash), `kloc.toml` (kernel, bin, xtask, aggregate; the xtask row's first HISTORY segment cites this live
key with `->`) and the two pin-constant tests.

**Not claimed.** No Worker T3 container run, no hosted run, no `abi-diff` (it refuses a dirty tree). The bin-side
recording leg of AC7.2 (a daemon T3 admission recorded as a kernel `verify_signature`) needs the hosted T3 substrate;
the same-instance property is covered locally by the counting provider on the adapter and, in the root, by the
compiler-required argument at `main.rs` (`Arc::clone(&crypto)`, the registry receives the same clone).


### Story context creation

Created 2026-11-03 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `8c809f9c`, without cargo
(parent rule).

**Inputs:** E20 §20-13 at HEAD (amended 2026-10-30, PF §3.6) — stale cites corrected here and owed to E20
(`creation/records-owed.md` §A); preflight rows; tracker `:315`; `deferred-work.md` rows naming 20-13 or NFR-Sec-15; PRD FR48 /
NFR-Sec-15; direct reads of every cited range (`creation/cites-8c809f9c.md`); bypass inventory; kloc and kernel by the prebuilt
gate binary on `/tmp/proto2013` `git archive` copies (`creation/kloc-kernel-8c809f9c-and-prototypes.json`); prototypes KT3,
KT1, KTd, KI, KO and the L1 bundle saved as diffs (`git apply --check` clean on HEAD), `rustfmt`'d, never applied to the tree.

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–7.

**Records owed:** `creation/records-owed.md`.

**Revert:** no crate source modified (prototypes only under `/tmp/proto2013`, deleted); no cargo/test ran;
`intent-lineage-coverage-report.md` untouched.

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

### Completion Notes List

- 2026-11-12 (L2 landing `998b36e6`, independent GPT + GLM reviews at `20-13-pluggable-crypto-provider-selection-evidence/landing-2/review/`, 0 P0-P2): the maos-bin +7 overdraft (38,556 -> 38,563) is ADJUDICATED by Main 2026-11-12 as the cost of migrating the four `main.rs` `SecurityManagerAdapter` call sites. Operator severity ruling: the two P3 hardening findings (F1 adapter exempt from the exactly-one-construction gate; F2 naming-ban scan coverage) are recorded as rule-10 rows in `deferred-work.md`. Story NOT complete; owed per the GPT review: AC7.2 bin-side recording proof of kernel admission (the Worker recording test is not a substitute); hosted T3/container admission, final-tree hosted discipline, `abi-diff` and `prepush` unverified; the bin/escape suite is not wholly green (million-row erasure failure persists after the dependency-corrected rerun). Evidence precision: the surface reports contain 401 items, not 400.

### File List

## Change Log

- 2026-11-03: Operator ruled D-20-13-01…16 as recommended (all A), ask `d_batch4`; kernel KT3 and the D02 A per-crate envelope are ruled, binding at dev start after re-measurement against the landed tree. Kernel lands last (L2), with exact rebased pins; status remains ready-for-dev.

- 2026-11-03: Story created at `8c809f9c` (ready-for-dev; D-20-13-01…16 PROPOSED). Inventory measured: two production
  provider instances in the root plus one per Worker T3 launch and one per kernel T3 admission (never inventoried); 21 direct
  sign/verify/seal files besides the provider; the audit sealed export signs, it does not encrypt; the FR38 crypto drift check
  is vacuous. Kernel KT3 +1/−1 recommended (KT1 +20/+14, KTd +11/+6, KI +19/+18, KO +38/+37 priced by reverted prototype);
  out-of-kernel measured bin +188, shell +46, compliance +22, registry +21, xtask +59.

- 2026-11-12: L2 kernel landing `998b36e6` reviewed (GPT, GLM; 0 P0-P2, two P3 deferred rows); maos-bin +7 overdraft adjudicated by Main as the cost of migrating the four `main.rs` call sites; AC7.2 bin-side recording proof, hosted admission, `abi-diff` and `prepush` still owed; surface reports hold 401 items, not 400.
