---
baseline_commit: cb412348
depends_on: 13-3b-provenance-crosses-the-wall, 13-5d-production-spirit-collective-route, 13-5g-tl-stage2-datname-inversion-defense-in-depth
kernel_grant: NONE — ZERO maos-kernel-core Δ expected, pin stays 23401 (verify at implementation; `maos_kernel_core::iac` is a re-export shim, see D-9)
splits_from: 13-6-reza-cortex-journey-closer-nfr-scale-5
---

# Story 13.6a — The cross-team wall has a door, a lock, and a key, and nothing in production ever walks through it

Status: **ready-for-dev**

**Kernel-Δ: ZERO expected.** `xtask/kernel-core-baseline.toml src_lines = 23401` unchanged; `xtask/fkcs-baseline.toml` stays byte-untouched at the frozen `23081`. Work lands in `maos-bin`, `maos-loom-lite`, `maos-iac`, `maos-domain` and `spirits/`. **No new gate** — legs go on the existing `check-multi-tenant-loom` (ADR-055's gate; the 13.5c **K5(b)** precedent, re-ratified by the 13.5e preflight, says do not create a sibling).

> **Read this first — this story exists because Story 13.6's grounding disproved its own AC2.**
>
> 13.6 was filed as the Epic-13 closer that *"composes 13.1–13.5e"* under the epic's own binding rule: **"13.6 is last and only judges; it never invents a missing mechanism inside the journey harness"** (`epic-13-reza-cortex-v2-2.md:52`).
>
> The 2026-07-28 grounding pass measured every mechanism 13.6 must compose. Five of six are production-reachable. **The cross-team crossing is not** — and it is the one 13.6·AC2 and 13.6·AC3 are built on. The apparatus is fully constructed in production and terminates in a function that a *Blocking* gate leg proves nothing in production calls (D-1…D-4 below, all measured).
>
> The epic anticipated exactly this and refused to paper over it: `(f-i) no-production-crossing-initiator — inverter UNASSIGNED` (`epic-13:174`), *"named as an open gap, not given a fictional successor"*, mechanically re-confirmed at the 13.5d preflight (`epic-13:176`). The dead-wire test says so in its own failure message: *"production caller owner is UNASSIGNED and must be assigned before inversion."* It is **still unassigned** after 13.3, 13.3b, 13.5c and 13.5d.
>
> **Ratified by the operator 2026-07-28: split.** This story assigns the owner. It is a mechanism story — 13.6 stays a judge. This is the epic's own written policy (pre-dev checklist item 5: *"If grounding reveals another independently shippable mechanism, split it rather than hiding implementation in 13.6"*), applied for the third time after H1→13.3b and 13.5c→13.5c/d/e.
>
> **What this story does NOT claim.** It does not close the *host-axis* membership gap (an evicted host holding a fresh lease still consents — deferred-work 2026-07-20, owner was named as 13.5c and 13.5c closed without a membership answer; see Residual 2). It does not journal success-side cross-wall disclosure (ADR-058 Decision 2 scoped that out; Residual 3). It does not build the journey — 13.6 does.

---

## Story

**As** a team lead in Reza's single-org Cortex,
**I want** an allowed cross-team share and an allowed cross-wall traceback to be things a running MAOS process can actually *initiate*, not just things the wall is capable of refusing,
**so that** the consent apparatus already welded into the composition root governs real traffic instead of guarding a function no production code path can reach — and so the Epic-13 closer has a mechanism to judge.

---

## The defect, in code — measured at `cb412348` before a line was designed

### D-1 — the consent check has exactly one consumer, and it is unreachable

```
$ grep -rn "is_granted(" --include='*.rs' crates/ spirits/ | grep -v "/tests/"
crates/maos-bin/src/cross_team_consent.rs:26:    fn is_granted(          # the impl
crates/maos-loom-lite/src/cross_team_consent.rs:20:    fn is_granted(      # the trait
crates/maos-loom-lite/src/replication/bundle.rs:917:  .is_granted(from_team, context.to_team, context.intent)
```

**One** non-test call site. It lives at `bundle.rs:917`, inside `apply_replication_bundle` (declared `bundle.rs:840`).

### D-2 — `apply_replication_bundle` has zero production callers, and a Blocking gate leg proves it

`xtask/src/check_multi_tenant_loom.rs:363` runs `replication_crossing_has_no_production_initiator` as `BindingClass::Blocking`. The test (`crates/maos-bin/tests/cross_team_consent_13_3.rs:161`) walks **every** production `.rs` under `crates/` and `spirits/` — skipping `tests`/`benches`/`examples`/`target` and the defining module — for `apply_replication_bundle(`, `build_replication_bundle(`, `build_replication_bundle_v2(`. It is **green**. Every caller in the workspace is a test.

### D-3 — the apparatus is fully constructed in production anyway

| Site | What is built |
|---|---|
| `main.rs:2745-2749` | `CrossTeamConsentAdapter::new(bootstrap.state)` → `Arc<dyn CrossTeamConsentPort>` |
| `main.rs:2753` | `team_verifying_keys_from_env(&bootstrap.state)` |
| `main.rs:2784` | `store.with_cross_team_consent(consent)` |
| `main.rs:2958` | `LogRecallAdapter::with_cross_wall_consent(CrossWallRecallConsentAdapter::new(…))` |

Four production wiring sites, real signed-manifest state, real derived team keys — feeding a dead end. **This is precisely the shape 13.6·AC1 declares must fail: *"Constructed-but-unwired controls fail."*** The closer cannot judge it, because the closer would be judging itself.

### D-4 — the read side is unreachable for a structural reason, not a missing call

`LogRecallPort::recall_cross_wall(spirit_pid, team: &TeamId, filter)` — `crates/maos-domain/src/ports/log_recall.rs:35`, implemented at `crates/maos-iac/src/adapter/log_recall.rs:347`. It is a **default-implemented** trait method whose default returns `ECrossWallRecallDenied { NoConsentProvider }`.

Production dispatch calls plain `.recall(`:
- `spirits/researcher/src/lib.rs:1030` — `port.recall(spirit_pid, filter)`
- `crates/maos-bin/src/main.rs:5881` — inside a one-shot smoke path

**No production surface supplies the `&TeamId`.** `LogRecallFilter` (`crates/maos-domain/src/log_recall.rs:14`) has `kind`/`since_ns`/`until_ns`/`limit`/… and **no team field**. The gap is not "add a caller" — it is "the request cannot express the question." Blocking leg `cross-wall-recall-no-production-caller` (`check_multi_tenant_loom.rs:524`) pins it.

### D-5 — ⚠ THE FACT THAT SIZES THIS STORY: one store per process

```
$ grep -n "LoomLiteStore::new\|tenant_map_for_store" crates/maos-bin/src/main.rs
2743:  let tenant_map = maos_bin::tenant_map::tenant_map_for_store(&home_team, tenant_source)
2766:  match maos_loom_lite::store::LoomLiteStore::new(cfg).await {
```

**Exactly one store construction in the composition root**, pinned to `MAOS_LOOM_HOME_TEAM`, with `connection_assignment_guard` proving `datname_for(home_team) == current_database()`.

Therefore **a crossing cannot be performed inside one process.** A source-team process physically cannot open the destination team's database — that is the wall working as designed (ADR-055 database-per-team). The production crossing is necessarily a **pair**: an *emitter* on team A's host and an *applier* on team B's host, joined by the cohort A2A mesh.

Anyone scoping this as *"call `apply_replication_bundle` from `main.rs`"* discovers D-5 mid-implementation and either builds a wall-breaking second store or stalls. **Say the two-process sentence out loud in the design.**

### D-6 — 13.3b already left the emitter seam, and it names this work

`crates/maos-loom-lite/src/replication/bundle.rs:441-445`, verbatim:

```rust
/// No production caller exists yet: the Spirit→collective digest
/// publication flow is the 13.6 journey. This is the seam it will use.
#[allow(clippy::too_many_arguments)]
pub async fn originate_team_row(
    store, spirit_pid, namespace, key, value,
    distillation_depth, intent_lineage, home_team, base_seed,
) -> Result<(), BundleError>
```

**Use it. Do not hand-roll leaf construction.** It builds the `CollectiveKvLeaf` with `source_team` / `distillation_depth` / `intent_lineage` (13.3b leaf v3), calls `build_replication_bundle_v2`, verifies, and computes the inclusion proof (`:465-485`). Its only caller today is `crates/maos-loom-lite/tests/cross_region_live.rs:520`.

It is the **origination** half — it stamps a team-owned row in the store it is given. The **crossing** half (transport + apply at the destination) is still this story's.

### D-6b — ⚠ THE DEAD-WIRE LEG HAS A HOLE, AND `originate_team_row` SITS IN IT

`cross_team_consent_13_3.rs:196-198` skips the defining module:

```rust
if path.ends_with("replication/bundle.rs") { continue; }
```

`originate_team_row` **lives in that file** and calls `build_replication_bundle_v2` at `:480`. So **a production caller of `originate_team_row` inverts the crossing in fact while `replication-crossing-has-no-production-initiator` stays green.** The negative is one indirection away from blind.

This is not hypothetical — it is the exact seam D-6 says to use. AC4's *"replacement is not weaker"* clause must close it: the replacement negative walks **transitively reachable** production entry points, or names `originate_team_row` explicitly as a needle. A leg that can be satisfied by moving a call one file over is a claim standing in for a control.

### D-7 — `base_seed` has a production source, and this story widens its use from verify to sign

`MAOS_CROSS_TEAM_BASE_SEED` — read at `crates/maos-bin/src/cross_team_consent.rs:93-101` (64 hex chars → `[u8; 32]`), registered in `crates/maos-bin/src/env_contract.rs:290`, consumed at `main.rs:2753` via `team_verifying_keys_from_env`.

⚠ Note the function's own doc (`cross_team_consent.rs:87`): today production reads the seed **only to derive public keys** — the verify side. `build_replication_bundle_v2` and `originate_team_row` need it on the **sign** side. Same variable, materially different authority: a host that could only *check* team signatures can now *make* them. **Name this in the ADR amendment and in the threat model** (`docs/loom-threat-model.md`); do not let it land as an unremarked widening.

### D-8 — the applier's site is already ratified: `handle_intake_verified`

`crates/maos-a2a-core/src/router.rs:1222`, reached from `crates/maos-a2a-tcp/src/transport.rs:586` **after peer verification**. Story 12.3's preflight resolved (P5r) that this is the **spoof-proof observation site** and that threading through `handle_intake` — ~36 callers, no signature anchor — is wrong. Put the applier arm here; the precedent is ratified, do not re-litigate it.

### D-9 — the recall work is NOT kernel-core

`main.rs:2950` says `maos_kernel_core::iac::log_recall::LogRecallAdapter`, which reads like kernel-core. It is not: `crates/maos-kernel-core/src/iac.rs:13` is `pub use maos_iac::*;` — a Story-6.5 backward-compat shim. The adapter lives in `maos-iac`. **Adding a public item to `maos-iac` does not move `count_rs_lines` over `maos-kernel-core/src`.** Verify with the gate, do not assume.

---

## Design — two initiators, one mesh, zero new gates

```
   TEAM-A HOST (maos_team_a)                      TEAM-B HOST (maos_team_b)
   MAOS_LOOM_HOME_TEAM=team-a                     MAOS_LOOM_HOME_TEAM=team-b
   ┌──────────────────────────────┐               ┌──────────────────────────────┐
   │ store_a (datname maos_team_a)│               │ store_b (datname maos_team_b)│
   │                              │               │                              │
   │ [WRITE-SIDE INITIATOR]  NEW  │               │ [APPLIER] NEW, inside        │
   │ originate_team_row(...)      │               │ handle_intake_verified       │
   │   ← seam 13.3b left (D-6)    │               │   ← router.rs:1222 (D-8)     │
   │ build_replication_bundle_v2  │  cohort A2A   │ apply_replication_bundle(    │
   │   (leaves, region_a, team_a) │──frame,──────▶│   bundle, &store_b, "reg-b", │
   │                              │  intent =     │   Some(CrossTeamApplyContext │
   │                              │ collective:   │     ::new(&team_b,           │
   │                              │    share      │       "collective:share")),  │
   │                              │               │   &base_seed)                │
   │                              │               │        │                     │
   │                              │               │        ▼ bundle.rs:917       │
   │                              │               │  is_granted(team_a, team_b,  │
   │                              │               │    "collective:share")       │
   │                              │               │   ← FIRST PRODUCTION REACH   │
   └──────────────────────────────┘               └──────────────────────────────┘

   [READ-SIDE INITIATOR]  NEW: a production surface that names a target team
   → LogRecallPort::recall_cross_wall(pid, &team_b, filter)   (maos-iac:347)
   → CrossWallRecallConsentAdapter  (already wired, main.rs:2958)
```

**The consent tuple is already keyed by intent** — `(from_team, to_team, "collective:share")` is the shipped shape (`crates/maos-cohort/src/manifest.rs:1534,1552`). So the crossing rides an existing intent-carrying A2A frame; **do not add a `FrameKind` variant.** `FrameKind` (`crates/maos-spirit-abi/src/identity.rs:30`) is a `repr` ABI enum with explicit discriminants — a new variant is an ABI surface change, and the epic already recorded that `abi-diff` is a **null control** that cannot see it (`epic-13:218`). Ride the intent, not the enum.

**Operator-verb precedent for the emitter:** the collective-erase one-shot at `main.rs:4883`+ (`MAOS_ONE_SHOT`, env-driven args at `:4920-4943`, port call at `:4947`, then `insert_kernel_event_returning_id`). Copy that shape — it is the ratified idiom (13.5b AC4(a)), and every other one-shot verb has it.

---

## Acceptance Criteria (5)

### AC1 — A production write-side crossing initiator exists, and the crossing lands on a real second database

**Given** two processes, each booted in tenant mode against its own `datname` (`maos_team_a`, `maos_team_b`), joined by the cohort A2A mesh,
**When** team A initiates an allowed `collective:share` for a named destination team,
**Then** a production (non-test) code path builds the bundle on A's host, transports it as a cohort A2A frame under intent `collective:share`, and a production code path on B's host applies it via `apply_replication_bundle` with a real `CrossTeamApplyContext`,
**And** the row is physically present in `maos_team_b` and physically **absent** from `maos_team_a`'s destination-key space — the unguarded physical-absence witness a shared table cannot fake (13.1·AC4 idiom),
**And** **no process holds two `LoomLiteStore`s** (D-5): the applier's store is its own home-team store, and `connection_assignment_guard` still passes on both hosts,
**And** `replication-crossing-has-no-production-initiator` is **inverted in the same commit** (D-2) — never left half-deleted (the 11.3 D2 / 10.4c atomic-cutover lesson).

### AC2 — Consent is enforced on the live path, not merely constructed

**Given** the crossing of AC1 is production-reachable,
**When** an A→B share runs with only an A→B grant in the signed manifest,
**Then** `CrossTeamConsentAdapter::is_granted` is reached by a **real production call** (D-1's site gains its first non-test caller), and the share lands,
**And** the reverse B→A share is refused with `TransportCause::ConsentDenied` carrying the **ordered pair and the intent** — asymmetry enforced on the live path, not only in `bundle.rs`'s unit tests,
**And** a stale lease refuses as `MapStale` and an unavailable state as `StateUnavailable`, remaining distinguishable from `ConsentDenied` (the five-cause matrix at `crates/maos-loom-lite/src/adapter.rs:235` must still hold end-to-end, not just over hand-built `StoreError`s),
**And** each of these three outcomes is **proven-red per limb**: a mutation that neuters one refusal reds its own leg and leaves the others green.

### AC3 — A production read-side cross-wall recall initiator exists, and refusal is first-class

**Given** `recall_cross_wall` is unreachable because no production request can name a team (D-4),
**When** a production surface issues a traceback against a named remote team,
**Then** `LogRecallPort::recall_cross_wall` is reached in production with a real `&TeamId`, consent is proven fresh through the already-wired `CrossWallRecallConsentAdapter` (`main.rs:2958`), and an allowed recall returns the remote page,
**And** an unconsented or stale-lease recall surfaces `ECrossWallRecallDenied` as an **observable operator outcome**, never folded into an empty-page success (ADR-049 §7 provenance-presence discipline),
**And** ⚠ **the emitter-scope pin on plain `recall` is not widened as a side effect.** 13.3b·AC2 ratified the citer-auth control (`distillate.rs:330-359`, Story 8.10 AC2b) precisely so a digest-of-digest cannot launder a cross-principal raw frame. Cross-wall reach is granted **only** through the team-named path with its own consent proof; a mutation that lets plain `recall` cross the wall must red,
**And** `cross-wall-recall-no-production-caller` is **inverted in the same commit**, with this story named as its owner in the leg's replacement.

### AC4 — Every inverted dead-wire clause is replaced by a control that is not weaker

**Given** AC1 and AC3 each retire a Blocking negative that was doing real work,
**When** the gate is rewritten,
**Then** each inverted clause is replaced by **(a)** a positive leg naming both endpoints of the now-live path, **and (b)** a *new* negative that only becomes falsifiable **because** the crossing is live — at minimum: an **unconsented crossing is refused at the destination applier** (not merely un-initiated at the source), and a **forged `source_team` stamp on a live crossing** is refused under the 13.2 derived team key,
**And** each clause is written as a **separately-named leg with its own named inverter** — never one composite assertion (the ratified rule at `epic-13:175`; a composite leg here is exactly what went wrong before the (f-i)/(f-ii) split),
**And** ⚠ **the D-6b hole is closed**: the replacement negative must not be satisfiable by routing the call one file over. `originate_team_row` lives inside the module the current scan skips and calls `build_replication_bundle_v2` internally, so the replacement either walks **transitively reachable** production entry points or names `originate_team_row(` explicitly as a needle. **Prove it**: a fixture that calls `originate_team_row` from a production module must red the replacement leg,
**And** the replacement negatives are proven-red by mutation, each reddening only its own leg.

### AC5 — Gate, ADR and budget

**Given** ADR-055 owns the tenant wall and `check-multi-tenant-loom` is its gate,
**When** this story's legs are registered,
**Then** they go on **`check-multi-tenant-loom`** — **no new gate** (13.5c **K5(b)**, re-ratified at the 13.5e preflight: that gate already keys the `MAOS_TEST_POSTGRES_TEAM_A`/`_B` substrate, already models both `BindingClass` values with the non-vacuity guard, and already owns the `ABSENT_SUCCESSORS` surface that must be rewritten anyway),
**And** every hermetic leg is `Blocking` with **one `#[test]` per `--exact` leg** (the gate's only anti-vacuity oracle is `"running 1 test"` + `"1 passed"`, structurally blind to a null assertion),
**And** live two-datname legs are `AdvisorySubstrate` and **`.expect()` their own env var rather than silently skipping** (the 13.5g pattern — a skipped leg that prints green is the failure this project keeps catching),
**And** `check-kernel-baseline` proves **23401 unchanged** (ZERO kernel-core Δ — D-6; if the measurement disagrees, **stop for FLAG-Winston, do not route around the owner**),
**And** ADR-055 is amended (or ADR-058 extended) with the two-process crossing topology of D-5 and its honest limit; `check-multi-tenant-loom`'s `ABSENT_SUCCESSORS` is updated to reflect what this story closes and what it does not.

---

## Traps — every one of these has bitten this repo already

1. **The one-store wall (D-5).** Do not construct a second `LoomLiteStore` to "reach" team B. That defeats ADR-055 in the act of demonstrating it, and `connection_assignment_guard` will be telling the truth when it refuses. Two processes, one mesh.
2. **Do not add a `FrameKind` variant.** `abi-diff` cannot see it (`epic-13:218` — const signatures are value-erased, the diff fails only on *removed* lines). Ride the existing intent-carrying frame; `collective:share` is already the manifest's key.
3. **Atomic cutover.** Invert the dead-wire legs and land the positive replacements in the **same commit**. A half-deleted negative is worse than the negative (11.3 D2; 10.4c 7-file cutover).
4. **`.expect()`, never skip.** The `AdvisorySubstrate` house pattern still emits `passed: true` on absent substrate (deferred-work 2026-07-25, `check_reza_production_path.rs:474-545`). That is a known cross-gate defect **owned by 13.6**; do not rely on it here — the live legs must `.expect()` their own var so an unset substrate is loud.
5. **One `#[test]` per `--exact` leg.** Two tests behind one leg name defeats the `"running 1 test"` oracle.
6. **Proven-red per limb, byte-identical restore.** Mutate one limb, confirm it reds *its own* leg and leaves siblings green, restore with `diff -q` / empty `git diff`. Serialize the mutations; do not batch.
7. **Do not "fix" `main.rs:2402` / `SpiritMemoryView`.** It forwards only to the trait path, a permanent `CapabilityDenied` in production **by design** (`memory/mod.rs:721-739`). 13.5d already recorded this trap.
8. **`ranged_recall` is forbidden** for the read side — path-addressed, capability-free, and compile-pinned out by `spirits/researcher` (13.3b·AC3). Use the consented `recall_cross_wall` path.
9. **Measure the baseline in an isolated `git worktree`**, not a dirty tree (the 13.5i error, fixed at 13.5j). "Code first, then the pin."
10. **The dead-wire leg has a hole (D-6b).** Its scan skips `replication/bundle.rs`, and `originate_team_row` — the very seam you are told to use — lives there. Inverting the crossing *in fact* while the leg stays green is one file move away. Close it, and prove the closure.
11. **The base seed widens from verify to sign (D-7).** Say it in the ADR and the threat model rather than letting it land unremarked.
12. **Applier goes in `handle_intake_verified`, not `handle_intake`** (D-8). ~36 callers, no signature anchor. Ratified at 12.3; do not re-litigate.
13. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.

---

## Tasks

- [ ] **T1 (AC1)** — Design the two-process crossing on paper first; write the D-5 sentence into the design note. Decide emitter surface (`MAOS_ONE_SHOT=collective-share` mirroring `main.rs:4883`+) and applier surface (cohort A2A intake arm for intent `collective:share`).
- [ ] **T2 (AC1)** — Implement the emitter on the **seam 13.3b already left**: `originate_team_row(...)` (D-6) to stamp the provenance-carrying row, then `build_replication_bundle_v2(leaves, &region, &home_team, &base_seed)` → emit as a cohort A2A frame under intent `collective:share`. Seed from `MAOS_CROSS_TEAM_BASE_SEED` (D-7). Journal a kernel event (`insert_kernel_event_returning_id`) mirroring the erase verb.
- [ ] **T3 (AC1/AC2)** — Implement the applier in **`handle_intake_verified`** (`router.rs:1222`, D-8 — the ratified spoof-proof site; **not** `handle_intake`): → `apply_replication_bundle(bundle, &store, dest_region, Some(CrossTeamApplyContext::new(&home_team, intent)), &base_seed)`. Map `BundleError` → `StoreError` → `CollectivePortError` preserving the five-cause distinguishability.
- [ ] **T4 (AC3)** — Read side: add the target-team dimension to the recall request path (`LogRecallFilter` or a sibling entry point — `maos-domain`, **not** kernel-core, D-6), and a production surface that supplies it. Confirm plain `recall`'s emitter pin is untouched.
- [ ] **T5 (AC2/AC4)** — Live two-datname integration test: allowed A→B lands; reverse B→A refused `ConsentDenied` with ordered pair + intent; stale lease `MapStale`; unconsented crossing refused **at the applier**; forged `source_team` refused under the derived team key. Physical-absence witness on both.
- [ ] **T6 (AC1/AC3/AC4)** — Rewrite the two dead-wire legs in `check_multi_tenant_loom.rs`: invert, add positive legs naming both endpoints, add the new negatives, one `#[test]` per `--exact` leg, each clause separately named with its inverter. **Close the D-6b hole** and prove it with an `originate_team_row`-from-production fixture.
- [ ] **T7 (AC4)** — Serialized proven-red pass: one mutation per limb, confirm own-leg-only red, restore byte-identical (`diff -q`). Record each mutation and its observed red leg in the Dev Agent Record.
- [ ] **T8 (AC5)** — Amend ADR-055 (two-process crossing topology + honest limit); update `ABSENT_SUCCESSORS`; update `tests/coverage-matrix.yaml` if the leg set changes traceability.
- [ ] **T9 (AC5)** — Gates: `cargo run -q -p xtask -- check-kernel-baseline` (**23401**), `kloc-check`, `check-multi-tenant-loom`, `check-reza-production-path`, `cargo fmt --all -- --check`, `cargo test --workspace`. Re-base the kloc ceiling **only if measured**, and record it — this has been re-based at "done" for four consecutive stories and is a live retro item.
- [ ] **T10** — Record the dev model in the Dev Agent Record (`check-dev-model-tier` / `check-dev-model-used-populated` are live gates; frontier-class allowlist, §A6 full-layer review net is **non-degradable** for this boundary per `epic-13:13`).

---

## Dev Notes

### Existing surfaces to reuse — do not reinvent

| Need | Use | Path |
|---|---|---|
| **Originate a provenance-carrying team row** — the seam 13.3b left for this work | `originate_team_row(store, pid, ns, key, value, depth, lineage, team, seed)` | `crates/maos-loom-lite/src/replication/bundle.rs:445` (doc at `:441`) |
| Build a cross-team bundle | `build_replication_bundle_v2(leaves, region, team, seed)` | `crates/maos-loom-lite/src/replication/bundle.rs:345` |
| **Base seed** (already production-wired; verify-side today, sign-side here — D-7) | `MAOS_CROSS_TEAM_BASE_SEED` | read `crates/maos-bin/src/cross_team_consent.rs:93-101`; contract `env_contract.rs:290` |
| **Applier site** — spoof-proof, ratified at 12.3 P5r | `handle_intake_verified` | `crates/maos-a2a-core/src/router.rs:1222` ← `crates/maos-a2a-tcp/src/transport.rs:586` |
| Apply at destination | `apply_replication_bundle(bundle, store, dest_region, cross_team, seed)` | `bundle.rs:840` |
| Crossing context | `CrossTeamApplyContext::new(&to_team, intent)` | `bundle.rs:91-100` |
| Consent decision | `CrossTeamConsentAdapter` (already constructed) | `crates/maos-bin/src/cross_team_consent.rs:15`; wired `main.rs:2745,2784` |
| Cross-wall read consent | `CrossWallRecallConsentAdapter` (already constructed) | `cross_team_consent.rs:50`; wired `main.rs:2958` |
| Cross-wall read | `LogRecallPort::recall_cross_wall` | trait `maos-domain/src/ports/log_recall.rs:35`; impl `maos-iac/src/adapter/log_recall.rs:347` |
| Operator verb idiom | `MAOS_ONE_SHOT` collective-erase | `crates/maos-bin/src/main.rs:4883`, args `:4920-4943`, call `:4947` |
| Error mapping | `store_error_to_port_error` + five-cause matrix | `crates/maos-loom-lite/src/adapter.rs:235` |
| Live two-datname harness | `MAOS_TEST_POSTGRES_TEAM_A`/`_B` | `crates/maos-bin/tests/cross_team_consent_13_3.rs`, `cohort_daemon_smoke_13_5c.rs` |

### House standards

- Test idiom: `Command::new(env!("CARGO_BIN_EXE_maos"))`. No `assert_cmd`/`escargot`/`predicates`.
- `cargo fmt --check` blocking since E12-B4; rustfmt `max_width = 100`.
- `cargo-deny` is live — a new dependency needs justification. Prefer none.
- `#[ignore]` + `.expect()` on live legs; the gate controls execution. **Skipped ≠ passed.**

### Budget

- **kernel-core:** ZERO expected @ **23401**. If the measurement disagrees, stop for FLAG-Winston. *"kernel-core ZERO" is not the same sentence as "zero delta"* — state both.
- **kloc:** re-check `xtask/kloc.toml` ceilings for `maos-bin`, `maos-loom-lite`, `maos-iac`, `maos-domain` **before** writing, so a ceiling breach is a design input rather than a "done"-time re-base. Ceiling policy is `measured + max(100, ceil(0.02 × measured))`; slack is operating capacity, **not** authorization; a ceiling must never block a correctness repair.
- **fkcs:** `xtask/fkcs-baseline.toml` stays byte-untouched at `23081`.

### Previous-story intelligence

- **13.5g (`cb412348`, done).** Landed the TL tenant binding two-phase design. Its review found: legs green while connecting to nothing, and **no leg exercised the composition root** — deleting the Phase A block left all 15 Blocking legs green. Repairs included two real-binary boot legs. **Apply the same test:** after writing T6's legs, delete the new production initiator and confirm the positive legs red. If they stay green, they are testing the library, not the wiring.
- **13.5g open finding (carry).** `crates/maos-loom-lite/src/store.rs:419-433` — `init_schema` acquires a **second** pooled client after `connection_assignment_guard` runs on the first, while its own new comment asserts they are the same client. Benign at `pool_size ≥ 2` (repo min 2, default 16) but a hang at the legal `pool_size: 1` (deadpool's default wait timeout is `None`). If you touch `init_schema`, fix it; otherwise leave it and let 13.6 judge it.
- **13.5j.** *"Probe-harness-before-design"* — build a throwaway harness through the public surface, refuse to design until every probe is red, delete before commit. That method produced this story's D-1…D-6 and should produce T1's design.
- **13.3 deferred (2026-07-20).** *"An evicted host holding a fresh lease still consents."* Owner was named as 13.5c; 13.5c closed without a membership answer. **The crossing going live makes this reachable for the first time** — see Residual 2. Do not silently absorb it; record it.

### Git intelligence (last 6 commits)

`cb412348` 13.5g · `c2e55a25` 13.5j · `04a6e72d` 13.5i · `dd4a908e` 13.5h · `5ccd862c` 13.5b · `595e8453` 13.5a. Pattern: each lands one mechanism, adds `--exact` Blocking legs to an existing gate, re-bases kloc at close, and carries an explicit residual list. Follow it.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-13-reza-cortex-v2-2.md#52`] — *"13.6 is last and only judges; it never invents a missing mechanism inside the journey harness."*
- [Source: `epic-13-reza-cortex-v2-2.md#174-176`] — (f-i) `no-production-crossing-initiator`, inverter UNASSIGNED; the separately-named-clauses rule.
- [Source: `epic-13-reza-cortex-v2-2.md#232`] — pre-dev checklist item 5 (split rather than hide implementation in 13.6).
- [Source: `epic-13-reza-cortex-v2-2.md#171`] — K5(b): do not create a sibling gate.
- [Source: `docs/adr/ADR-055-multi-tenant-loom.md`] — database-per-team, `team_guard`, per-team HKDF weld.
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md#527-529`] — evicted-host consent gap.

---

## Residuals (carry forward; do not silently close)

1. **`(f-i)` is closed by this story; `(f-ii)` was closed by 13.5c.** After this story, no Epic-13 dead-wire clause should remain unassigned. Verify that claim mechanically before asserting it.
2. **Host-axis membership (OPEN, becomes reachable here).** `CrossTeamConsentAdapter::is_granted` checks lease freshness + grant, **not** `state.local_host() ∈ manifest.members`. Until now the gap was unreachable because the apply path was dead. This story makes it reachable. If not closed in-story, it must be re-filed with a named owner — not re-deferred to a story that already shipped.
3. **Success-side cross-wall disclosure is not journaled** (ADR-058 Decision 2, deferred-work 2026-07-21). A successful `recall_cross_wall` journals as a plain local `log.recall`. This story makes it reachable; the recommendation on file is to extend the refusal-journaling clause to success-side disclosure.
4. **`v25-signed-shard`** (13.5e/13.5g residual #6) — unchanged, not this story's.
5. **Fallible `record_invocation`** (13.5d deferred, kernel-core, needs a second FLAG-Winston) — unchanged.

---

## Dev Agent Record

### Agent Model Used

_(record the frontier-class model; `check-dev-model-used-populated` is a live gate)_

### Debug Log References

### Completion Notes List

### File List

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created from the Story 13.6 grounding pass. Split ratified by the operator: 13.6a owns the production crossing initiators (mechanism), 13.6 stays a judge. D-1…D-5 + D-9 measured at `cb412348`; 5 ACs, ZERO kernel-core Δ expected @23401. |
| 2026-07-28 | Checklist validation pass added D-6 (`originate_team_row` — the seam 13.3b explicitly left for this work, *"This is the seam it will use"*), **D-6b** (the dead-wire leg skips `replication/bundle.rs`, where that seam lives — the negative is one indirection from blind; now an AC4 clause with its own proof obligation), D-7 (`MAOS_CROSS_TEAM_BASE_SEED` is production-wired but verify-side only; this story widens it to the sign side — must be named in the ADR + threat model) and D-8 (applier belongs in `handle_intake_verified`, the 12.3-ratified spoof-proof site). |
