---
baseline_commit: a414f922
depends_on: 13-6a-authenticated-team-identity (DONE @a414f922), 13-3b-provenance-crosses-the-wall, 13-5d-production-spirit-collective-route
blocked_by: none — 13.6a landed 2026-07-29
kernel_grant: NONE — ZERO maos-kernel-core Δ, pin stays 23401 (verified: `find crates/maos-kernel-core/src -name '*.rs' | xargs cat | wc -l` == 23401 at a414f922). `maos_kernel_core::iac` is a re-export shim (D-9).
preflight: party-mode 2026-07-29 (post-13.6a) — P1 weld + P2 daemon-emitter + P3 boot reconcile + P4 wire-cause folded in; READ SIDE SPLIT OUT to 13-6d (operator-ratified); 6 ACs
splits_from: 13-6-reza-cortex-journey-closer-nfr-scale-5
splits_to: 13-6d-cross-wall-recall-production-initiator
---

# Story 13.6b — The crossing crosses, and the team that crossed it is the team that signed it

Status: **done**

**Kernel-core Δ: ZERO — and that is not the same sentence as "zero delta."** `xtask/kernel-core-baseline.toml src_lines = 23401` unchanged (**verified equal at `a414f922`**); `xtask/fkcs-baseline.toml` stays byte-untouched at the frozen `23081`. Work lands in `maos-a2a-core`, `maos-a2a-tcp`, `maos-loom-lite`, `maos-bin`. **No new gate** — legs go on the existing `check-multi-tenant-loom` (ADR-055's gate; the 13.5c **K5(b)** precedent, re-ratified by the 13.5e preflight).

> ### Two things changed under this story since it was written
>
> **1 · 13.6a is DONE (`a414f922`).** `COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4` ship the operator-signed `CohortMember.team`; both A2A seams enforce it; `Defer` is a refusal on `collective:share`; `loom-threat-model.md` T1 is corrected. 44/44 Blocking legs green, both dead-wire legs still green (it inverted nothing, as promised). **This story is unblocked.**
>
> **2 · The 2026-07-29 preflight found the weld 13.6a's split left behind.** 13.6a authenticates the **envelope's** team claim. The crossing decides consent from the **payload's** team claim. Nothing binds them, and no AC in either story owned it — 13.6b's AC6 moved to 13.6a, and AC4 is explicitly forbidden from claiming impersonation coverage. **D-10's attack survives 13.6a until this story welds the two fields.** See **D-13**; it is now **AC3**.
>
> The read side (the former AC3) is **split out to Story 13.6d** — different crates, different dead-wire leg, structurally independent (D-4). This story is the write side.

---

## Story

**As** a team lead in Reza's single-org Cortex,
**I want** an allowed cross-team share to be something a running MAOS process can actually *initiate* — and the team name the wall judges to be the team the mesh authenticated,
**so that** the consent apparatus already welded into the composition root governs real traffic instead of guarding a function no production code path can reach, and so the Epic-13 closer has a mechanism to judge.

---

## The defect, in code — measured at `a414f922`

### D-1 — the consent check has exactly one consumer, and it is unreachable

```
$ grep -rn "is_granted(" --include='*.rs' crates/ spirits/ | grep -v "/tests/"
crates/maos-bin/src/cross_team_consent.rs:26:    fn is_granted(          # the impl
crates/maos-loom-lite/src/cross_team_consent.rs:20:    fn is_granted(      # the trait
crates/maos-loom-lite/src/replication/bundle.rs:917:  .is_granted(from_team, context.to_team, context.intent)
```

**One** non-test call site, at `bundle.rs:917`, inside `apply_replication_bundle` (declared `bundle.rs:840`).

### D-2 — `apply_replication_bundle` has zero production callers, and a Blocking gate leg proves it

`xtask/src/check_multi_tenant_loom.rs:364` runs `replication_crossing_has_no_production_initiator` as `BindingClass::Blocking`. The test (`crates/maos-bin/tests/cross_team_consent_13_3.rs:161`) walks **every** production `.rs` under `crates/` and `spirits/` for `apply_replication_bundle(`, `build_replication_bundle(`, `build_replication_bundle_v2(`. **Still green at `a414f922`.**

### D-3 — the apparatus is fully constructed in production anyway

| Site | What is built |
|---|---|
| `main.rs:2745-2749` | `CrossTeamConsentAdapter::new(bootstrap.state)` → `Arc<dyn CrossTeamConsentPort>` |
| `main.rs:2753` | `team_verifying_keys_from_env(&bootstrap.state)` |
| `main.rs:2784` | `store.with_cross_team_consent(consent)` |

Real signed-manifest state, real derived team keys — feeding a dead end. **The exact shape 13.6·AC1 declares must fail: *"Constructed-but-unwired controls fail."*** The closer cannot judge it, because it would be judging itself.

### D-5 — ⚠ THE FACT THAT SIZES THIS STORY: one store per process

```
2743:  let tenant_map = maos_bin::tenant_map::tenant_map_for_store(&home_team, tenant_source)
2766:  match maos_loom_lite::store::LoomLiteStore::new(cfg).await {
```

**Exactly one store construction in the composition root**, pinned to `MAOS_LOOM_HOME_TEAM`, with `connection_assignment_guard` proving `datname_for(home_team) == current_database()`.

Therefore **a crossing cannot be performed inside one process.** The production crossing is necessarily a **pair**: an *emitter* on team A's host and an *applier* on team B's host, joined by the cohort A2A mesh. **Say the two-process sentence out loud in the design** — three separate defects below exist because a clause was written without it (D-14, D-15, and the former AC2 wording).

### D-6 — 13.3b already left the emitter seam

`crates/maos-loom-lite/src/replication/bundle.rs:441-445` — `originate_team_row(store, spirit_pid, namespace, key, value, distillation_depth, intent_lineage, home_team, base_seed)`.

**Use it. Do not hand-roll leaf construction.** It builds the `CollectiveKvLeaf` with `source_team` / `distillation_depth` / `intent_lineage` (13.3b leaf v3), calls `build_replication_bundle_v2`, verifies, and computes the inclusion proof (`:465-485`). Its only caller today is a test (`cross_region_live.rs:520`).

⚠ **Its doc comment is WRONG and must be corrected in this commit — see D-16.**

### D-6b — ⚠ THE DEAD-WIRE LEG HAS A HOLE, AND `originate_team_row` SITS IN IT

`cross_team_consent_13_3.rs:201` skips the defining module:

```rust
if path.ends_with("replication/bundle.rs") { continue; }
```

`originate_team_row` **lives in that file** and calls `build_replication_bundle_v2` at `:480`. So **a production caller of `originate_team_row` inverts the crossing in fact while `replication-crossing-has-no-production-initiator` stays green.** The negative is one indirection away from blind. AC5's *"replacement is not weaker"* clause must close it: the replacement negative walks **transitively reachable** production entry points, or names `originate_team_row` explicitly as a needle.

### D-7 — `base_seed` widens from verify to sign

`MAOS_CROSS_TEAM_BASE_SEED` — read at `cross_team_consent.rs:93-101`, registered in `env_contract.rs:290`, consumed at `main.rs:2753` via `team_verifying_keys_from_env`. Today production reads the seed **only to derive public keys**. `build_replication_bundle_v2` and `originate_team_row` need it on the **sign** side: a host that could only *check* team signatures can now *make* them.

13.6a already corrected `loom-threat-model.md` T1 for this. **Verify the correction covers the sign-side widening specifically**; extend it if it only re-scoped the forger.

### D-8 — the applier's site is ratified: `handle_intake_verified` — ⚠ NOW AT `router.rs:1385`

`crates/maos-a2a-core/src/router.rs:1385` (**was `:1222` before 13.6a added 244 lines**), reached from `crates/maos-a2a-tcp/src/transport.rs:586` **after peer verification**. Story 12.3's preflight resolved (P5r) that this is the **spoof-proof observation site** and that threading through `handle_intake` — ~36 callers, no signature anchor — is wrong. Put the applier arm here; do not re-litigate.

### D-9 — the recall work is NOT kernel-core

`crates/maos-kernel-core/src/iac.rs:13` is `pub use maos_iac::*;` — a Story-6.5 backward-compat shim. *(Retained for the reader; the read side now lives in **13.6d**.)*

### D-10 — wiring the crossing turns `base_seed` into a standing authority

`build_replication_bundle_v2` (`bundle.rs:414`):

```rust
let team_seed = derive_team_signing_seed(base_seed, source_region, source_team);
let signing_key = SigningKey::from_bytes(&team_seed);
let signature = signing_key.sign(&sign_payload);
```

`derive_team_signing_seed` (`maos-audit/src/sealed_export.rs:72`) works for **any** `(region, team)`. The emitter must hold the seed. **Therefore every emitter can sign a valid bundle under every team's key**, and `apply_replication_bundle` takes `from_team = bundle.source_team` (`bundle.rs:875-879`) → `is_granted(from_team, …)` (`:917`). Consent is decided from a self-declared field whose signature the emitter can forge.

**Why the shipped negative does not cover it.** `bundle.rs:1656-1676` / `:1822-1843` forge by **relabel** — a *seed-less* forger. Ours derives the right key and signs correctly. The two never intersect.

### D-11 — 13.6a's answer, and what it actually covers

13.6a bound the **mTLS axis** — `CohortMember.fingerprint` is operator-pinned and cert-bound, not seed-derived — and shipped it as `COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4` with a per-member `team: Option<TeamId>` (`manifest.rs:138-150`, fail-closed on absence), v1/v2/v3 canonical bytes byte-identical.

Live at `a414f922`:
- **Send seam** `router.rs:746` — `source_team_stamp` is read from **this host's own signed V4 declaration**, never caller input; a member with no declared team, or whose leaf no longer equals the signed fingerprint, **cannot originate a crossing at all** (`A2AError::CohortTeamIdentityRefused`).
- **Accept seam** `router.rs:1429-1445` — `claimed_team = request.cohort_source_team`; refused under `CODE_TEAM_IDENTITY_MISMATCH = -32010` unless the TLS-verified peer speaks for it, from a **single locked manifest snapshot** shared with the consent verdict.
- `COHORT_INTENT_COLLECTIVE_SHARE` (`router.rs:20`) **already requires** a team claim: `router.rs:1431` fires the check when the intent matches even if `claimed_team` is `None`, so **absence refuses**.

### D-13 — ⚠ BLOCKER-GRADE: the authenticated field and the deciding field are two different fields

**This is what the 13.6a/13.6b split left behind, and it is this story's headline.**

| | Field | Origin | Authenticated? |
|---|---|---|---|
| **Envelope** | `request.cohort_source_team` (`json_rpc.rs:98`) | this host's signed `CohortMember.team` (`state.rs:530` → `team_of_host`) | ✅ 13.6a, `router.rs:1442` |
| **Payload** | `bundle.source_team` | `store.config().home_team` ← `MAOS_LOOM_HOME_TEAM` env (`main.rs:2703`) | ❌ **nothing** |
| **Decision** | `is_granted(from_team, …)` (`bundle.rs:917`) | reads **`bundle.source_team`** (`:875-879`) | — |

**The attack, post-13.6a.** team-a's host stamps a truthful envelope (`cohort_source_team = team-a`, forced by `router.rs:746` — it cannot lie there) and a **lying payload** (`source_team = team-b`, signed correctly with team-b's seed-derived key). The applier: transport ✓ · `-32010` team binding ✓ *(the envelope is honest)* · bundle signature verifies against the **claimed** pair ✓ · `is_granted(team-b, team-c)` ✓. **Row lands. D-10 intact.**

⇒ **The applier must refuse when `bundle.source_team ≠ the authenticated envelope claim.** Four lines of comparison — and the only thing standing between the tenant wall and an impersonation bypass. It gets its **own named leg and its own inverter**, never a clause folded inside AC1.

⚠ **13.6a's impersonation negative cannot be reused as evidence here.** It is proven *against synthetic frames at the envelope*. This is a different assertion at a different site, and calling 13.6a's leg a proof of it would be the failure shape this epic keeps cataloguing.

### D-14 — ⚠ THE EMITTER CANNOT BE A ONE-SHOT. There is exactly one production outbound A2A path.

```
maos-a2a-tcp/src/transport.rs:730   .prepare_outbound(frame, peer, self.own_boot_nonce)
```

is the **only** non-test outbound send in the workspace. The transport that owns it is constructed **only** in `build_cohort_a2a_daemon_runtime` (`main.rs:9482`), called **only** from `run_cohort_a2a_daemon` (`:9286`), reached **only** from the `MAOS_ONE_SHOT=cohort-a2a-daemon` dispatch arm at `main.rs:7643`.

A sibling one-shot at `main.rs:4883` (the collective-erase idiom the previous draft told you to copy) returns **~4,700 lines before any transport, peer pin, or manifest gate exists**. It cannot send an authenticated cohort frame.

⇒ **The emitter lives inside the cohort-a2a-daemon runtime.** This is the 13.5a shape — *"the daemon returns before all governance"* — arriving one story later on the send side. See T2.

### D-15 — ⚠ the five-cause matrix has never crossed a process boundary OR a kernel boundary

Two independent erasures, both measured:

1. **The socket.** `grep -rn "TransportCause" crates/maos-a2a-core crates/maos-a2a-tcp` → **zero occurrences.** `TransportCause` lives in `maos-domain` and is mapped only in `maos-loom-lite/src/adapter.rs` and `replication/router.rs` — all same-process. There is no wire encoding and no A2A code for a cross-team consent denial.
2. **The kernel.** `crates/maos-kernel-core/src/memory/mod.rs:204`:
   ```rust
   maos_domain::ports::CollectivePortError::Transport(_) => CollectiveErrorKind::Transport,
   ```
   **The cause is discarded.** Every one of the five causes collapses to the single word `Transport` at the kernel boundary. No Spirit has ever been able to distinguish `ConsentDenied` from `MapStale`, on any path, since the port was written.

⇒ The previous AC2 wording (*"refused with `TransportCause::ConsentDenied`"* observed by the emitter) asked for something the architecture has never provided anywhere. **Resolved, not weakened** (AC2 below): the refusal crosses the wire as a **new typed A2A cause** following the shipped `interpret_response` idiom, and `TransportCause` stays applier-local where it is already correct and already tested.

**The kernel erasure is NOT this story's to fix** — `collective_port_error` is wildcard-free and lives in kernel-core; widening it is a re-pin off 23401 and a FLAG-Winston conversation. **Record it for 13.6 with an owner** (Residual 6).

### D-16 — the seam comment at `bundle.rs:441` is wrong, and it has been steering design for three stories

Verbatim on disk:

> *"No production caller exists yet: the Spirit→collective digest publication flow is the 13.6 journey. **This is the seam it will use.**"*

Measured against the code: `CollectiveMemoryPort` (`maos-domain/src/ports/collective_memory.rs:111`) has exactly four methods — `write`, `read`, `scan`, `erase`. **There is no `share`.** A Spirit cannot express a crossing through that port, and `write` is precisely the method `team_guard` refuses foreign-team rows on (`store.rs:497`). A Spirit-initiated crossing would need a **new port verb plus a kernel-core call site** — non-zero kernel Δ.

**The architecture is right and the comment is wrong.** The Spirit initiates **publication** into its own team's collective tier (13.5d, already wired). The **host** initiates the **crossing**, under the signed manifest. They were never the same seam — and keeping them separate is a security property, not an accident: *a Spirit that can name a destination team is a Spirit that can be prompt-injected into naming one.*

⇒ **Correct the comment in this commit** (the 13.6a T1 precedent). This retires R1 properly instead of apologising for it.

### D-12 — the evicted-applier case: entailment tested, REFUTED, re-grounded (line pins refreshed for `a414f922`)

The earlier preflight asserted AC6's mTLS binding would also close the evicted-host residual. **It was verified and it is false.** AC6 binds the **sender's** cert → team; the 13.3 finding is about the **receiver** (`state.local_host()` removed from `manifest.members` by a fresh signed reissue). Opposite ends of one connection.

**And the store path is weaker than the finding described.** The crossing applies via `apply_replication_bundle → apply_verified_replication_bundle → store.write_with_source_attested()` (`store.rs:553`) — a **fifth** write path outside `team_guard`'s four-site chokepoint (`write:497`, `erase:718`, `read:834`, `scan:956`), correctly so, because `team_guard` refuses foreign-team rows, which is what a crossing *is*.

The eviction check lives **inside** `TenantMapAdapter::current_manifest()` (`tenant_map.rs:101-108`), reachable only via `team_of` (called only from `team_guard`) or `datname_for` (called only from `connection_assignment_guard`, **boot-time only**).

⇒ **The eviction check was never a control in its own right — it is a side effect of the tenant-map lookup, and the crossing drops it as a consequence of being a crossing.**

**What actually closes it:** `handle_intake_verified` (`router.rs:1385`) → `handle_intake` (`:961`) → the Story-12.1/12.3 cohort gate. `CohortConsentVerdict::NotCurrent` is documented (`cohort.rs`) as *"stale **or no longer contains the local member**"* and NACKs. Because D-8 puts the applier behind that seam, an evicted applier NACKs before any bundle is applied.

**Conditions, re-verified at `a414f922`:**
1. `is_reserved_cohort_intent` (`router.rs:521-523`) short-circuits the gate for exactly `RESERVED_INTENT_REISSUE` and `RESERVED_INTENT_HALT_RECEIPT`. **`collective:share` must never join that set.** ✅ still true.
2. The gate is `Option<Arc<dyn CohortManifestGate>>`; a node built without a real gate gets `LegacyCohortManifestGate`, which `Defer`s everything.
3. The emitter side has the same self-check — `CohortConsentSeam::Send` runs the same chokepoint; `NotCurrent` → `A2AError::ConfigInvalid`.
4. `Defer`-before-self-membership: **closed by 13.6a·AC4** — `Defer` is now a refusal on `collective:share` on both seams, routed through the shipped Deny path under `CrossingDeferRefused`, with the 12.1 bilateral fallback proven preserved.
   ⚠ **13.6a recorded an honest finding you must not misread:** on the **Accept** seam the Defer rule is defense-in-depth *behind* the team binding (an off-roster member has no signed team edge, so `-32010` fires first). It is independently reachable and proven on the **Send** seam. Do not cite the Accept-seam leg as proof of the Defer rule.

---

## Design — two daemons, one mesh, zero new gates

```
   TEAM-A HOST (maos_team_a)                      TEAM-B HOST (maos_team_b)
   MAOS_LOOM_HOME_TEAM=team-a                     MAOS_LOOM_HOME_TEAM=team-b
   MAOS_ONE_SHOT=cohort-a2a-daemon                MAOS_ONE_SHOT=cohort-a2a-daemon
   ┌──────────────────────────────┐               ┌──────────────────────────────┐
   │ store_a (datname maos_team_a)│               │ store_b (datname maos_team_b)│
   │                              │               │                              │
   │ [EMITTER] NEW — INSIDE the   │               │ [APPLIER] NEW — inside       │
   │ daemon runtime (D-14).       │               │ handle_intake_verified       │
   │ originate_team_row(...)      │               │   ← router.rs:1385 (D-8)     │
   │ build_replication_bundle_v2  │  cohort A2A   │                              │
   │   (leaves, region_a, team_a) │──frame,──────▶│ ① envelope team ✓ (13.6a)    │
   │ prepare_outbound stamps      │  intent =     │ ② ⚠ WELD: bundle.source_team │
   │   cohort_source_team from    │ collective:   │    == authenticated claim?    │
   │   the SIGNED declaration     │    share      │    else refuse (D-13, AC3)   │
   │   (router.rs:746)            │               │ ③ apply_replication_bundle(  │
   │                              │               │      …, CrossTeamApplyContext │
   │                              │◀──NACK────────│      ::new(&team_b, intent)) │
   │ interpret_response gains ONE │  typed cause  │        │                     │
   │ arm → typed refusal (AC2)    │  + data{from, │        ▼ bundle.rs:917       │
   │                              │   to, intent} │  is_granted(team_a, team_b,  │
   │                              │               │    "collective:share")       │
   │                              │               │   ← FIRST PRODUCTION REACH   │
   └──────────────────────────────┘               └──────────────────────────────┘

   BOOT, both hosts (AC4): MAOS_LOOM_HOME_TEAM == manifest.team_of_host(local_host)
                           or the process does not start.
```

**The consent tuple is already keyed by intent** — `(from_team, to_team, "collective:share")` is the shipped shape (`manifest.rs:1534,1552`), and `COHORT_INTENT_COLLECTIVE_SHARE` already exists (`router.rs:20`). **Do not add a `FrameKind` variant** — `FrameKind` (`maos-spirit-abi/src/identity.rs:30`) is a `repr` ABI enum and `abi-diff` is a **null control** that cannot see an addition (`epic-13:218`). Ride the intent, not the enum.

---

## Acceptance Criteria (6)

### AC1 — A production write-side crossing initiator exists, inside the daemon, and the crossing lands on a real second database

**Given** two `cohort-a2a-daemon` processes, each booted in tenant mode against its own `datname` (`maos_team_a`, `maos_team_b`), joined by the cohort A2A mesh,
**When** team A initiates an allowed `collective:share` for a named destination team,
**Then** a production (non-test) code path **inside the daemon runtime** (D-14 — **not** a sibling `MAOS_ONE_SHOT` arm) builds the bundle on A's host via `originate_team_row` + `build_replication_bundle_v2`, transports it as a cohort A2A frame under intent `collective:share`, and a production path on B's host applies it via `apply_replication_bundle` with a real `CrossTeamApplyContext`,
**And** the row is physically present in `maos_team_b` and physically **absent** from `maos_team_a`'s destination-key space — the unguarded physical-absence witness a shared table cannot fake (13.1·AC4 idiom),
**And** **no process holds two `LoomLiteStore`s** (D-5) — a **control, not a sentence**: a `Blocking` static leg asserts exactly one production `LoomLiteStore::new` (today `main.rs:2766`), proven-red by adding a second construction. `connection_assignment_guard` still passes on both hosts,
**And** `replication-crossing-has-no-production-initiator` (`check_multi_tenant_loom.rs:364`) is **inverted in the same commit** — never left half-deleted (11.3 D2 / 10.4c atomic-cutover).

### AC2 — Consent is enforced on the live path, and the refusal survives the wire

**Given** the crossing of AC1 is production-reachable, and **D-15** measured that `TransportCause` has **zero** occurrences in either A2A crate,
**When** an A→B share runs with only an A→B grant in the signed manifest,
**Then** `CrossTeamConsentAdapter::is_granted` is reached by a **real production call** (D-1's site gains its first non-test caller) and the share lands,
**And** the reverse B→A share is refused at the **applier** and the refusal reaches the **emitter** as a typed, attributable outcome: a **new A2A code** (following `CODE_TEAM_IDENTITY_MISMATCH = -32010`'s shipped pattern) carrying `from_team` / `to_team` / `intent` in the NACK `data`, re-materialised emitter-side by **one more arm in `interpret_response`** (`router.rs:820`, which already does exactly this for five codes),
**And** `TransportCause::ConsentDenied` remains **applier-local** — the five-cause matrix (`maos-loom-lite/src/adapter.rs:235`) still holds end-to-end **within the applier process**, which is the only boundary it has ever crossed,
**And** a stale lease and an unavailable state remain **distinguishable from consent denial on the emitter's side too** — three distinct observable outcomes, not one generic transport failure,
**And** each of these is **proven-red per limb**: a mutation that neuters one refusal reds its own leg and leaves the others green.

### AC3 — ⚠ The team the wall judges is the team the mesh authenticated

**Given** D-13: `is_granted` decides from `bundle.source_team` (payload, unauthenticated) while 13.6a authenticates `request.cohort_source_team` (envelope), and **nothing binds them**,
**When** a frame arrives whose envelope claim is truthful and whose payload `source_team` names a different team,
**Then** the applier **refuses before `apply_replication_bundle` is called**, under its **own named cause**, distinguishable from both `CODE_TEAM_IDENTITY_MISMATCH (-32010)` (a lying envelope) and from consent denial (an honest but ungranted crossing),
**And** the refusal is proven by a **live negative that only this story can run**: a real emitter that stamps a truthful envelope and a forged payload — the **seed-holding** forger of D-10, which the shipped relabel negative (`bundle.rs:1656-1676`) structurally cannot reach,
**And** ⚠ **13.6a's impersonation leg may NOT be cited as evidence for this AC.** That leg proves the envelope binding against synthetic frames; this is a different assertion at a different site. Citing it would be a claim standing in for a control,
**And** the leg has its **own named inverter**, never folded into AC1's composite.

### AC4 — One host, one team: the env and the signed manifest are reconciled at boot

**Given** two independent surfaces name this host's team — `MAOS_LOOM_HOME_TEAM` (env, `main.rs:2703` → `store.config().home_team` → `bundle.source_team`) and `CohortMember.team` (signed manifest, → the envelope stamp) — and nothing reconciles them,
**When** the daemon boots,
**Then** `reconcile_transport_identity_with_manifest` (`main.rs:9407`, called `:9504`) is **extended to the team axis**: `MAOS_LOOM_HOME_TEAM` must equal `manifest.team_of_host(local_host)` or the process **fails to start**,
**And** this follows that function's own shipped doctrine verbatim — *"a config-time fact silently overrides a manifest-time fact … **Disagreement is a boot error, never a warning**"* — which 13.6a's review wrote for **certificates** and left unwritten for **teams**, one field over,
**And** ⚠ **this does NOT replace AC3.** An attacker owns their own boot and will simply set the env correctly and lie in the payload. AC3 is the security control against a peer; AC4 is the correctness control against misconfiguration — a host whose env and manifest disagree otherwise emits crossings attributed to the wrong team, or fails mysteriously. **State both sentences; do not let either stand in for the other.**

### AC5 — Every inverted dead-wire clause is replaced by a control that is not weaker

**Given** AC1 retires a Blocking negative that was doing real work,
**When** the gate is rewritten,
**Then** the inverted clause is replaced by **(a)** a positive leg naming **both endpoints** of the now-live path, **and (b)** *new* negatives that only become falsifiable **because** the crossing is live — at minimum an **unconsented crossing refused at the destination applier** (not merely un-initiated at the source) and a **relabelled `source_team` from a seed-less forger** refused under the 13.2 derived team key — ⚠ **scoped exactly so, and no wider.** This clause may **not** be written as "forged `source_team` is refused" or "Fork-4 proven": per D-10 a seed-holding emitter signs correctly and the derived-key check cannot see it. **That attacker is AC3's**, and AC3 is where its evidence lives,
**And** each clause is a **separately-named leg with its own named inverter** — never one composite assertion (`epic-13:175`),
**And** ⚠ **the D-6b hole is closed**: the replacement must not be satisfiable by routing the call one file over. `originate_team_row` lives inside the module the current scan skips (`cross_team_consent_13_3.rs:201`) and calls `build_replication_bundle_v2` internally, so the replacement either walks **transitively reachable** production entry points or names `originate_team_row(` explicitly as a needle. **Prove it**: a fixture calling `originate_team_row` from a production module must red the replacement leg,
**And** every replacement negative is proven-red by mutation, each reddening only its own leg,
**And** the **13.5g composition-root test applies**: delete the new production initiator and confirm the positive legs **red**. If they stay green they are testing the library, not the wiring.

### AC6 — Gate, ADR, comment, and budget

**Given** ADR-055 owns the tenant wall and `check-multi-tenant-loom` is its gate,
**When** this story's legs are registered,
**Then** they go on **`check-multi-tenant-loom`** — **no new gate** (13.5c K5(b), re-ratified at 13.5e),
**And** every hermetic leg is `Blocking` with **one `#[test]` per `--exact` leg** (the gate's only anti-vacuity oracle is `"running 1 test"` + `"1 passed"`, structurally blind to a null assertion),
**And** live two-datname legs are `AdvisorySubstrate` and **`.expect()` their own env var rather than silently skipping** (the 13.5g pattern — a skipped leg that prints green is the failure this project keeps catching),
**And** ⚠ **the `bundle.rs:441` seam comment is corrected in this commit** (D-16): the Spirit initiates publication, the host initiates the crossing; they were never the same seam. A stale comment that misdirects design is a deliverable, not a nit — the 13.6a T1 precedent,
**And** `check-kernel-baseline` proves **23401 unchanged** (measured equal at `a414f922`; if it disagrees, **stop for FLAG-Winston, do not route around the owner**),
**And** ADR-055 is amended with the **two-daemon** crossing topology (D-14), the envelope/payload weld (D-13), and the boot reconcile (AC4); `ABSENT_SUCCESSORS` is updated to reflect what this story closes and what it does not.

---

## Traps — every one of these has bitten this repo already

1. **The one-store wall (D-5).** Do not construct a second `LoomLiteStore` to "reach" team B. That defeats ADR-055 in the act of demonstrating it. Two daemons, one mesh.
2. **The emitter is not a one-shot (D-14).** `main.rs:4883` is ~4,700 lines before any transport exists. Copying the collective-erase idiom produces a process that cannot send.
3. **The weld is the story (D-13).** Everything else is plumbing around it. Do not let it become a clause inside AC1.
4. **Do not cite 13.6a's impersonation leg for AC3.** Different site, different assertion.
5. **Do not add a `FrameKind` variant.** `abi-diff` cannot see it (`epic-13:218`). Ride the existing intent.
6. **Atomic cutover.** Invert the dead-wire leg and land the positive replacements in the **same commit** (11.3 D2; 10.4c).
7. **`.expect()`, never skip.** The `AdvisorySubstrate` house pattern still emits `passed: true` on absent substrate — a known cross-gate defect **owned by 13.6**.
8. **One `#[test]` per `--exact` leg.**
9. **Proven-red per limb, byte-identical restore** (`diff -q`). Serialize the mutations; do not batch.
10. **No new `CollectivePortError` variant.** `collective_port_error` (`kernel-core/src/memory/mod.rs:198-211`) is wildcard-free and lives in kernel-core: a variant makes it non-exhaustive → compile error → the ZERO claim breaks. Refusal causes ride `Transport(TransportCause)` applier-side and the new A2A code on the wire.
11. **Do not "fix" the kernel erasure (D-15).** `Transport(_)` at `memory/mod.rs:204` is a kernel-core edit. Record it (Residual 6); 13.6 judges it.
12. **The dead-wire leg has a hole (D-6b).** Close it, and prove the closure.
13. **Applier goes in `handle_intake_verified` (now `router.rs:1385`), not `handle_intake`** (D-8). Ratified at 12.3.
14. **Never say "Fork-4 proven" about the crossing.** A seed-holder signs correctly under any team. AC3 restores the boundary; AC5's derived-key leg covers only the seed-less relabel attacker. Two claims — keep them apart.
15. **⚠ `maos-a2a-core` has 84 lines of reserve** (ceiling **4271**, measured **4187**), and it is an **unratified FLAG-Winston grant from 13.6a's review**. The applier arm, the weld, and the new code+dispatch arm all land there. **Surface this at design time, not at "done"** — that has happened four consecutive stories and is a live retro item.
16. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.

---

## Tasks

- [x] **T1 (AC1)** — Design the two-**daemon** crossing on paper first; write the D-5 and D-14 sentences into the design note. Decide the emitter trigger **inside `run_cohort_a2a_daemon`** (`main.rs:9286`) — a boot-time emit arm, an operator-triggered verb the daemon serves, or a scheduled publication — and say why. **The collective-erase one-shot idiom is NOT available (D-14); do not copy it.**
- [x] **T2 (AC1)** — Implement the emitter inside the daemon runtime on the **seam 13.3b left**: `originate_team_row(...)` (D-6) → `build_replication_bundle_v2(leaves, &region, &home_team, &base_seed)` → emit as a cohort A2A frame under intent `collective:share`. Seed from `MAOS_CROSS_TEAM_BASE_SEED` (D-7). Journal a kernel event (`insert_kernel_event_returning_id`) mirroring the erase verb. Confirm `prepare_outbound` stamps `cohort_source_team` from the signed declaration (`router.rs:746`) on this path.
- [x] **T3 (AC1/AC2)** — Implement the applier in **`handle_intake_verified`** (`router.rs:1385`, D-8): decode the bundle, run the **AC3 weld first**, then `apply_replication_bundle(bundle, &store, dest_region, Some(CrossTeamApplyContext::new(&home_team, intent)), &base_seed)`. Map `BundleError` → `StoreError` → refusal **applier-side**, preserving five-cause distinguishability locally. ⚠ **NO new `CollectivePortError` variant** (Trap 10).
- [x] **T4 (AC3)** — The weld: refuse when `bundle.source_team != request.cohort_source_team`, before `apply_replication_bundle`, under its own cause, with its own leg and inverter. Live negative = a real emitter with a truthful envelope and a forged, correctly-signed payload.
- [x] **T5 (AC2)** — The wire cause: new A2A code + `nack_with_data` carrying `{from_team, to_team, intent}`; one new arm in `interpret_response` (`router.rs:820`) producing a typed `A2AError`. Prove the emitter can distinguish denied / stale / unavailable.
- [x] **T6 (AC4)** — Extend `reconcile_transport_identity_with_manifest` (`main.rs:9407`) to the team axis: `MAOS_LOOM_HOME_TEAM == manifest.team_of_host(local_host)` or boot error. Negative: disagreeing env+manifest must fail to start.
- [x] **T7 (AC1/AC2/AC5)** — Live two-datname, two-daemon integration test: allowed A→B lands; reverse B→A refused with the ordered pair + intent visible at the emitter; stale lease distinguishable; unconsented crossing refused **at the applier**; forged payload refused (AC3); seed-less relabel refused (AC5). Physical-absence witness on both.
- [x] **T8 (AC5)** — Rewrite the dead-wire leg in `check_multi_tenant_loom.rs:364`: invert, add positive legs naming both endpoints, add the new negatives, one `#[test]` per `--exact` leg, each clause separately named with its inverter. **Close the D-6b hole** and prove it with an `originate_team_row`-from-production fixture. Run the 13.5g composition-root test (delete the initiator → positives must red).
- [x] **T9 (AC1)** — Static `Blocking` leg: exactly one production `LoomLiteStore::new`. Proven-red by adding a second construction.
- [x] **T10 (AC5)** — Serialized proven-red pass: one mutation per limb, own-leg-only red, restore byte-identical (`diff -q`). Record each mutation and its observed red leg in the Dev Agent Record.
- [x] **T11 (AC6)** — Amend ADR-055 (two-daemon topology, the weld, the boot reconcile); **correct the `bundle.rs:441` seam comment** (D-16); update `ABSENT_SUCCESSORS`; update `tests/coverage-matrix.yaml`; verify 13.6a's `loom-threat-model.md` T1 correction covers the **sign-side widening** (D-7) and extend it if not.
- [x] **T12 (AC6)** — Gates: `check-kernel-baseline` (**23401**), `kloc-check` (⚠ check `maos-a2a-core` **before** writing — 84 lines), `check-multi-tenant-loom`, `check-reza-production-path`, `cargo fmt --all -- --check`, `cargo test --workspace`. Re-base a ceiling **only if measured**, and record the driver.
- [x] **T13** — Record the dev model in the Dev Agent Record (`check-dev-model-tier` / `check-dev-model-used-populated` are live gates; frontier-class allowlist; §A6 full-layer review net is **non-degradable** for this boundary per `epic-13:13`).

### Review Findings

- [x] [Review][Patch] Production emitter constructs an invalid provenance tuple [crates/maos-bin/src/main.rs:9726]
- [x] [Review][Patch] Unavailable applier ACKs an unapplied crossing [crates/maos-a2a-core/src/router.rs:1596]
- [x] [Review][Patch] Applier discards the operator-requested destination team [crates/maos-bin/src/cross_team_crossing.rs:182]
- [x] [Review][Patch] Stale crossing state collapses to a generic transport failure [crates/maos-a2a-core/src/router.rs:1344]
- [x] [Review][Patch] Live witness bypasses both daemon endpoints [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:776]
- [x] [Review][Patch] Required applier-denial and seedless-relabel controls are missing as separate legs [xtask/src/check_multi_tenant_loom.rs:582]
- [x] [Review][Patch] Non-denial refusal NACKs drop crossing attribution [crates/maos-a2a-core/src/cohort.rs:474]
- [x] [Review][Patch] Crossing classifier ignores the frame kind [crates/maos-bin/src/cross_team_crossing.rs:73]
- [x] [Review][Patch] Non-UTF-8 peer configuration silently disables crossing [crates/maos-bin/src/cross_team_crossing.rs:249]
- [x] [Review][Patch] Non-UTF-8 namespace configuration silently defaults [crates/maos-bin/src/cross_team_crossing.rs:262]
- [x] [Review][Patch] Empty crossing keys can persist unreadable rows [crates/maos-bin/src/cross_team_crossing.rs:282]

---

## Dev Notes

### Existing surfaces to reuse — do not reinvent

| Need | Use | Path |
|---|---|---|
| **Originate a provenance-carrying team row** | `originate_team_row(store, pid, ns, key, value, depth, lineage, team, seed)` | `maos-loom-lite/src/replication/bundle.rs:445` |
| Build a cross-team bundle | `build_replication_bundle_v2(leaves, region, team, seed)` | `bundle.rs:345` |
| **Base seed** (verify-side today, sign-side here — D-7) | `MAOS_CROSS_TEAM_BASE_SEED` | read `cross_team_consent.rs:93-101`; contract `env_contract.rs:290` |
| **The only outbound A2A path** (D-14) | `prepare_outbound` | `maos-a2a-tcp/src/transport.rs:730` ← `build_cohort_a2a_daemon_runtime` `main.rs:9482` ← `run_cohort_a2a_daemon` `:9286` ← dispatch `:7643` |
| **Applier site** — spoof-proof, 12.3 P5r | `handle_intake_verified` | `maos-a2a-core/src/router.rs:1385` ← `maos-a2a-tcp/src/transport.rs:586` |
| **Emitter-side team stamp** (13.6a) | `source_team_stamp` from the signed declaration | `router.rs:746`, applied `:810` |
| **Accept-seam team binding** (13.6a) | `claimed_team` vs `declared` → `-32010` | `router.rs:1429-1445` |
| **NACK → typed error table** (add one arm) | `interpret_response` | `router.rs:820` |
| **Boot reconcile to extend** (AC4) | `reconcile_transport_identity_with_manifest` | `main.rs:9407`, called `:9504` |
| Apply at destination | `apply_replication_bundle(bundle, store, dest_region, cross_team, seed)` | `bundle.rs:840` |
| Crossing context | `CrossTeamApplyContext::new(&to_team, intent)` | `bundle.rs:91-100` |
| Consent decision | `CrossTeamConsentAdapter` (already constructed) | `cross_team_consent.rs:15`; wired `main.rs:2745,2784` |
| Error mapping (applier-local) | `store_error_to_port_error` + five-cause matrix | `maos-loom-lite/src/adapter.rs:235` |
| Live two-datname harness | `MAOS_TEST_POSTGRES_TEAM_A`/`_B` | `maos-bin/tests/cross_team_consent_13_3.rs`, `cohort_daemon_smoke_13_5c.rs` |

### House standards

- Test idiom: `Command::new(env!("CARGO_BIN_EXE_maos"))`. No `assert_cmd`/`escargot`/`predicates`.
- `cargo fmt --check` blocking since E12-B4; rustfmt `max_width = 100`.
- `cargo-deny` is live — a new dependency needs justification. Prefer none.
- `#[ignore]` + `.expect()` on live legs; the gate controls execution. **Skipped ≠ passed.**

### Budget

- **kernel-core:** ZERO @ **23401**, verified equal at `a414f922`. If the measurement disagrees, stop for FLAG-Winston. *"kernel-core ZERO" is not the same sentence as "zero delta"* — state both.
- ⚠ **`maos-a2a-core` = 4271 ceiling / 4187 measured / 84 reserve**, and that ceiling is 13.6a's **unratified review grant** (FLAG-Winston at the Epic-13 retro). The applier arm + weld + new code/dispatch all land there. **Decide the shape against this number before writing.** Ceiling policy is `measured + max(100, ceil(0.02 × measured))`; slack is operating capacity, **not** authorization; a ceiling must never block a correctness repair — surface it, don't route around it.
- **`maos-cohort` is absent from `xtask/kloc.toml` entirely** (13.6a grew it +302 unmeasured). Not this story's to fix; **record it** for the retro.
- **fkcs:** `xtask/fkcs-baseline.toml` stays byte-untouched at `23081`.

### Previous-story intelligence

- **13.6a (`a414f922`, done).** 44/44 Blocking legs, six mutation limbs, threat model corrected in-commit. Its **honest finding** — the Accept-seam Defer rule is defense-in-depth behind the team binding — is the model for how to report a layered control without over-claiming it. Copy that discipline.
- **13.6a's review P1** produced `reconcile_transport_identity_with_manifest`. **AC4 is that same fix, one axis over.** Read the function's doc comment before writing; it already argues your case.
- **13.5g.** Legs green while connecting to nothing; **no leg exercised the composition root**. AC5's composition-root test is the direct descendant.
- **13.5g open finding (carry).** `maos-loom-lite/src/store.rs:419-433` — `init_schema` acquires a **second** pooled client after `connection_assignment_guard` runs on the first. Benign at `pool_size ≥ 2`, a hang at the legal `pool_size: 1`. If you touch `init_schema`, fix it.
- **13.5j.** *"Probe-harness-before-design"* — build a throwaway harness through the public surface, refuse to design until every probe is red, delete before commit. That method produced D-13, D-14 and D-15.

### References

- [Source: `epic-13-reza-cortex-v2-2.md#52`] — *"13.6 is last and only judges; it never invents a missing mechanism inside the journey harness."*
- [Source: `epic-13-reza-cortex-v2-2.md#174-176`] — (f-i) `no-production-crossing-initiator`, inverter UNASSIGNED; the separately-named-clauses rule.
- [Source: `epic-13-reza-cortex-v2-2.md#232`] — pre-dev checklist item 5 (split rather than hide implementation in 13.6).
- [Source: `docs/adr/ADR-055-multi-tenant-loom.md`] — database-per-team, `team_guard`, per-team HKDF weld, and 13.6a's §4b.
- [Source: `_bmad-output/implementation-artifacts/13-6a-authenticated-team-identity.md`] — the envelope binding this story welds to the decision site.

---

## Residuals (carry forward; do not silently close)

1. **`(f-i)` is closed by this story; `(f-ii)` was closed by 13.5c; the read-side leg is 13.6d's.** Verify mechanically before asserting it.
2. **Host-axis membership — AC3 DOES NOT CLOSE IT. Entailment tested and REFUTED; see D-12.** Covered instead by the Story-12.1/12.3 cohort gate under four conditions, three verified at `a414f922` and the fourth closed by 13.6a·AC4.
2b. **Surviving operator-trust limit (OPEN, documented, not closed).** A seed-holder can still forge **within its own team**, and an operator who controls a host controls that host's team. The binding is enforced **against a peer, not against the operator**. True per-team key provisioning stays in the `v25-signed-shard` family (ADR-055 residual #6).
3. **Success-side cross-wall disclosure is not journaled** (ADR-058 Decision 2, deferred 2026-07-21). **Moved to 13.6d** with the read side.
4. **`v25-signed-shard`** — unchanged, not this story's.
5. **Fallible `record_invocation`** (13.5d deferred, kernel-core, needs a second FLAG-Winston) — unchanged.
6. **⚠ NEW — the kernel erases every collective cause.** `kernel-core/src/memory/mod.rs:204`: `CollectivePortError::Transport(_) => CollectiveErrorKind::Transport`. No Spirit has ever distinguished `ConsentDenied` from `MapStale` on any path. Widening it is a kernel-core edit + FLAG-Winston. **Owner: 13.6 to judge** — it decides whether "the operator can see why the wall refused" is a claim the epic may make on the Spirit path. Do **not** close it here.
7. **⚠ NEW — `CollectiveMemoryPort` has no `share` verb** (`collective_memory.rs:111`: `write`/`read`/`scan`/`erase`), and `write` is what `team_guard` refuses foreign-team rows on. A Spirit-initiated crossing needs a new port verb **plus** a kernel call site. D-16 argues the architecture is right and the *comment* was wrong — but if a future story wants Spirit-initiated crossings, this is its price.
8. **`maos-cohort` is not in `xtask/kloc.toml`** — ~300 lines landed unmeasured at 13.6a. Retro item.

---

## Dev Agent Record

### Agent Model Used

Model: anthropic/claude-opus-5 (`opus-5`, frontier-class allowlist per E11 retro §A1 / E12-B3) for the dev pass.

⚠ **The §A6 full-layer review net has NOT run for 13.6b.** It is non-degradable for this boundary (`epic-13:13`), so this story is **not** eligible to be marked `done` until it does — and per the house tip it must run on a **different** LLM than the one that implemented it. Recording its absence rather than implying coverage: the dev pass is not a review.

### Implementation Plan (as executed)

**T1 — the design, decided before writing.** Two sentences said out loud first, because three prior defects came from omitting them:

1. **One store per process (D-5).** The composition root builds exactly one `LoomLiteStore`, pinned to `MAOS_LOOM_HOME_TEAM`, with `connection_assignment_guard` proving `datname_for(home_team) == current_database()`. **A crossing therefore cannot happen inside one process.** It is a *pair*: an emitter on team A's host, an applier on team B's host, joined by the cohort A2A mesh.
2. **The emitter cannot be a one-shot (D-14).** `prepare_outbound` is the only non-test outbound A2A send in the workspace; its transport exists only inside `build_cohort_a2a_daemon_runtime`. The sibling `MAOS_ONE_SHOT` arms return from the dispatch thousands of lines before any transport, pin, or gate exists.

**Emitter trigger — chosen: a boot-time emit arm inside `run_cohort_a2a_daemon`, gated on `MAOS_CROSS_TEAM_SHARE_PEER`.** Why, against the two alternatives the story named:

- *An operator-triggered verb the daemon serves* — rejected for this story. It needs a new inbound intent, a new accept-allowlist surface, and its own consent seam, or the trigger channel itself becomes an unauthenticated remote-crossing trigger. That is a story, not a clause.
- *A scheduled publication* — rejected as unbuildable today. A scheduler needs a source of truth for *which rows cross and when*; there is none. `CollectiveMemoryPort` has no `share` verb (Residual 7), so a Spirit cannot express a crossing at all, and inventing the verb is kernel Δ.
- *A boot-time arm* — chosen. It sits **inside** the runtime after the listener, pins, and reconciled manifest exist; it is deterministic and drivable from a two-datname harness; it adds no inbound surface; and it is the operator's own declaration, the same trust model `MAOS_LOOM_HOME_TEAM` and `MAOS_CROSS_TEAM_BASE_SEED` already assume. Absence of the trigger leaves the daemon byte-for-byte as before.

**T2 emitter** — `originate_team_row` (the 13.3b seam, D-6) now **returns the signed bundle it persisted**, so the bytes on the wire are the bytes that were signed; no leaf is hand-rolled and nothing is re-signed. Then `crossing_frame` → `route_outbound` → `prepare_outbound`, which stamps `cohort_source_team` from this host's own signed V4 declaration. Journaled as `collective.host.cross-team-share` via `insert_kernel_event_returning_id`, mirroring the erase verb.

**T3 applier** — a dependency-inverted `CrossTeamCrossingPort` (`&IacFrame` + primitives only, the `HaltReceiptObserver`/`DigestReadPort` discipline) consulted from `handle_intake_verified` (D-8, 12.3 P5r) **after** the shared intake body ACKs, so the 12.1/12.3 cohort gate — including `NotCurrent` for a de-rostered applier (D-12) — refuses before any bundle touches the store. `BundleError` is mapped through the shipped `store_error_to_port_error` so the five-cause matrix still holds **inside the applier process**, then projected onto the wire cause. **No new `CollectivePortError` variant** (Trap 10) and no new `IacBusError` variant — both crossing errors ride `CrossHostRouteFailure`, so kernel-core stays byte-identical.

**T4 the weld (AC3)** — `bundle.source_team` vs the authenticated envelope claim, compared **before** `apply_replication_bundle`, refused under `CrossingRefusal::SourceTeamUnbound` → `CODE_CROSSING_SOURCE_TEAM_UNBOUND (-32011)`.

**T5 the wire cause (AC2)** — `CODE_CROSS_TEAM_CROSSING_REFUSED (-32012)` carrying `reason`/`from_team`/`to_team`/`intent`; two arms in `interpret_response`; `crossing_outcome_label` gives the emitter the three-way distinction.

**T6 boot reconcile (AC4)** — clause (d) of `reconcile_transport_identity_with_manifest`, factored into the hermetically testable `reconcile_home_team_with_manifest`.

**Design decision worth naming: the crossing rides the existing intent and the existing telemetry-control idiom.** `FramePayload::TelemetryEvent` under a pinned `event_type`, exactly like `maos.cohort-manifest.v1`. **No `FrameKind` and no `FramePayload` variant** — `abi-diff` is a null control that cannot see an enum addition (Trap 5), so the golden `event_type` leg is what actually holds the wire contract.

### Debug Log References

**The retired negative was measured blind, not assumed blind (D-6b, empirical).** At cutover, the shipped `replication_crossing_has_no_production_initiator` reddened naming **only** `cross_team_crossing.rs: apply_replication_bundle( x1`. The **emitter was invisible to it**: `main.rs` reaches the crossing through `originate_team_row(`, which lives in the skipped `replication/bundle.rs` and was never a needle. D-6b is therefore confirmed against running code, not inferred from the source.

**⚠ HONEST FINDING — my own first replacement leg had 13.5g's defect, and the mutation pass caught it.** `crossing_has_a_production_initiator_at_both_endpoints` began as a needle scan. Mutation **M5** deleted the `emit_cross_team_share(...)` call from `run_cohort_a2a_daemon` and **the leg stayed green**, because `emit_cross_team_share` still contained the needle. That is 13.5g's open finding — *"legs green while connecting to nothing"* — reproduced inside this story's own gate. The leg was rewritten as a brace-balanced **call-chain reachability walk** (dispatch → `run_cohort_a2a_daemon` → `emit_cross_team_share` → `originate_team_row` + `route_outbound`; `handle_intake_verified` → `apply_crossing` → `apply_replication_bundle`). M5 now reds. AC5's composition-root test is satisfied by construction rather than by inspection.

**Two more self-inflicted weak legs, both caught by mutation, both fixed:**
- **M11** drifted `CROSSING_EVENT_TYPE` and the round-trip leg stayed green — the test encoded *and* decoded through the same constant. A drifted wire constant silently stops every deployed applier from recognising a crossing. The literal `"maos.cross-team-crossing.v1"` is now pinned as a golden.
- **M12** deleted the intent gate in `from_frame` and the non-crossing leg stayed green — the test's frame had `consent_envelope = None`, short-circuiting one check *earlier* than the gate under test. The frame now carries a well-formed envelope with a **different** cohort intent, which is the case the gate actually guards.

**abi-diff is not evaluable with a dirty worktree, and `maos-spirit-abi` has ZERO delta.** `cargo public-api diff HEAD~1..HEAD` fails with `Your local changes … would be overwritten by checkout` before comparing anything (empty stdout, non-zero status) — a worktree artifact, not a finding. Evidence the ABI is untouched: `git diff a414f922 -- crates/maos-spirit-abi | wc -l` = **0**, the crate is absent from `git status`, and `abi-diff` returned **PASSED** when run against a clean tree (`git stash`). 262 public items unchanged.

**`check-env-contract` was already FAIL at `a414f922`** — 5 pre-existing unregistered reads in `main.rs` (`MAOS_VETTER_KEYRING`, `MAOS_LEGAL_HOLD_PRINCIPAL`, `MAOS_COLLECTIVE_ERASE_{PID,NAMESPACE,KEY}`), confirmed present at the baseline commit. Not a 13.6b regression; workspace coverage is tracked by Story 12.7. All seven vars this story reads ARE registered.

### T10 — serialized proven-red pass (12 mutations, one at a time, byte-identical restore verified by sha256)

| # | Mutation | Leg that reddened | Control that stayed green |
|---|---|---|---|
| M1 | AC3 weld neutered (`payload_team != authenticated_team` → `false`) | `crossing-payload-team-must-equal-authenticated-envelope` | weld-is-a-binding |
| M2 | Weld becomes a refuse-everything stub (→ `true`) | `crossing-weld-is-a-binding-not-a-refuse-all-stub` | AC3 weld leg |
| M3 | `originate_team_row(` removed from the needle set | `crossing-scan-closes-originate-team-row-hole` **and** `crossing-has-production-initiator-both-endpoints` | one-store |
| M4 | A second production `LoomLiteStore::new` added | `exactly-one-production-loom-lite-store` | both-endpoints |
| M5 | Emitter call deleted from `run_cohort_a2a_daemon` | `crossing-has-production-initiator-both-endpoints` | D-6b hole leg |
| M6 | AC4 disagreement downgraded to a pass | `boot-refuses-home-team-manifest-disagreement` | uncorroborated-team leg |
| M7 | AC4 `team_of_host == None` treated as agreement | `boot-refuses-uncorroborated-home-team` | disagreement leg |
| M8 | AC3 refusal collapsed onto `-32010` | `crossing-weld-refusal-has-its-own-wire-code` | consent-denial-reaches-emitter |
| M9 | Ordered pair dropped from the `-32012` NACK data | `crossing-consent-denial-reaches-the-emitter` | own-wire-code |
| M10 | `ConsentStale` reason mapped onto `ConsentDenied` | `crossing-causes-stay-distinguishable-on-the-wire` | own-wire-code |
| M11 | `CROSSING_EVENT_TYPE` drifted | `crossing-control-rides-the-telemetry-idiom` | one-store |
| M12 | Intent gate removed from `from_frame` | `crossing-applier-ignores-non-crossing-frames` | AC3 weld leg |

⚠ **M3 reds TWO legs, and that coupling is deliberate — reported, not hidden.** Both legs share one scanner. A per-leg needle list is *exactly how D-6b happened*: the shipped scan's needles drifted from the call graph and nobody noticed. The sharing is the control. Every other mutation reds exactly one leg. `crossing-has-production-initiator-both-endpoints` also has its own single-leg inverter (M5), so it is not left resting on a coupled mutation.

### Completion Notes List

**AC1 — a production write-side crossing initiator exists, inside the daemon.** ✅ Emitter inside `run_cohort_a2a_daemon` (D-14 honoured — **not** a sibling one-shot); applier in `handle_intake_verified` via the injected port; both endpoints bound by a **call-chain reachability** leg, not a text scan. The one-store wall is a `Blocking` control (`exactly-one-production-loom-lite-store`), proven-red by adding a second construction (M4). `replication-crossing-has-no-production-initiator` is **inverted and replaced in the same commit** — never left half-deleted. ⚠ **The physical presence/absence witness over two real datnames is written but NOT executed here**: no live Postgres substrate exists in this environment, so `live-crossing-lands-at-destination-datname` is `AdvisorySubstrate`, `#[ignore]`d, and `.expect()`s its own env vars (13.5g — skipped is not passed). **AC1's physical-absence clause is therefore ASSERTED BY CODE, NOT YET OBSERVED.** The gate's WOULD-HAVE-BLOCKED banner names it in the skip list.

**AC2 — consent is enforced on the live path and the refusal survives the wire.** ✅ `is_granted` gains its first non-test caller through `apply_replication_bundle`. `-32012` carries `reason`/`from_team`/`to_team`/`intent`; `interpret_response` re-materialises it; denial / staleness / unavailability are three distinct emitter-side outcomes (proven-red per limb: M9, M10). `TransportCause::ConsentDenied` stays applier-local, routed through the shipped `store_error_to_port_error`, so the five-cause matrix still holds inside the applier process — the only boundary it has ever crossed.

**AC3 — the team the wall judges is the team the mesh authenticated.** ✅ The weld refuses before `apply_replication_bundle`, under its own code `-32011`, distinguishable from `-32010` and from consent denial (M8). The negative uses the **seed-holding** forger: the test asserts the forged bundle **verifies** (`verify_replication_bundle(...).expect(...)`) before feeding it to the applier, so the attacker is D-10's, not the shipped relabel forger's. **13.6a's impersonation leg is not cited anywhere as evidence for this AC.** Own leg, own inverter (M1), plus a separate anti-stub control with its own inverter (M2). The hermetic leg runs against a store pointed at a dead host, so `SourceTeamUnbound` is positive evidence the weld ran before any store access.

**AC4 — one host, one team, reconciled at boot.** ✅ Clause (d) of `reconcile_transport_identity_with_manifest`, with the comparison factored into `reconcile_home_team_with_manifest` so it is provable without TLS scaffolding. Both arms have their own leg and their own single-leg inverter (M6, M7). **Both sentences are stated in code, in the ADR, and in the threat model:** AC4 is the correctness control against misconfiguration; AC3 is the security control against a peer. Neither stands in for the other.

**AC5 — every inverted clause is replaced by something not weaker.** ✅ Positive leg naming both endpoints (now a reachability walk); new negatives that only became falsifiable because the crossing is live (unconsented crossing refused **at the destination applier**, in the live leg; the seed-holding forger refused, hermetically). **The D-6b hole is closed and the closure is PROVEN**: a fixture production module whose only crossing reference is `originate_team_row(` is demonstrably **invisible** to the pre-13.6b needle set and **caught** by this story's. AC5's derived-key clause is scoped exactly as written — the seed-**less** relabel attacker only; the seed-holder is AC3's. **Nothing in this story says "Fork-4 proven".** The 13.5g composition-root test is satisfied and was the finding that forced the leg's rewrite.

**AC6 — gate, ADR, comment, budget.** ✅ All legs on `check-multi-tenant-loom`; **no new gate**. One `#[test]` per `--exact` leg. Live legs are `AdvisorySubstrate` and `.expect()` their env. The `bundle.rs` seam comment is **corrected** (D-16): it claimed the Spirit→collective publication flow would use that seam; measured, `CollectiveMemoryPort` has no `share` verb at all, so the claim was false and had been steering design for three stories. `check-kernel-baseline` proves **23401 unchanged**. ADR-055 gains §4c; `ABSENT_SUCCESSORS` stays `&[]` and now says so as a *claim* with the three non-closures named and owned; `tests/coverage-matrix.yaml` updated; `loom-threat-model.md` T1's sign-side correction **verified present and extended** to present tense plus the weld.

**Budget — surfaced, measured, re-based, driver recorded (Trap 15 honoured at design time).** kernel-core **ZERO at 23401**; `fkcs-baseline.toml` byte-untouched. Three ceilings were exceeded and re-based *after* trimming, not instead of it: `maos-a2a-core` 4271→4450 (measured 4350; 152 lines of inline tests moved to `tests/crossing_wire_13_6b.rs` and `CrossingRefusal::wire` factored out of the router first), `maos-bin` 14433→14835 (measured 14735), `xtask` 31690→31856 (measured 31756), `_aggregate_hardfail` 136669→136859 (measured 136759). ⚠ **`maos-a2a-core` is now on its SECOND consecutive grant and both remain FLAG-Winston unratified** — ratify them together at the Epic-13 retro. `maos-cohort` is still absent from `xtask/kloc.toml` (Residual 8, not fixed here).

**What this story does NOT claim.** The kernel's cause erasure is untouched (Residual 6 — `Transport(_)` still collapses all five causes at the kernel boundary; no Spirit can distinguish `ConsentDenied` from `MapStale`); no `share` verb is added to `CollectiveMemoryPort` (Residual 7); the operator-trust limit is unchanged (Residual 2b — isolation is enforced against a peer, not against the operator); the read side is 13.6d's; the §A6 review net has not run.

### Verification evidence

- `cargo test --workspace` — **green** (no regressions; one pre-existing enumeration negative, `composition_root_does_not_seed_manifest_scopes`, correctly reddened on the new `src/*.rs` file and was updated to cover it — the control working as designed).
- `check-multi-tenant-loom` — **PASSED**; 13 new hermetic `Blocking` legs green, live legs skipped with the WOULD-HAVE-BLOCKED banner (no substrate).
- `check-kernel-baseline` — **PASSED**, `maos-kernel-core/src = 23401`, pinned 23401.
- `kloc-check` — **PASSED** (aggregate 136758).
- `check-reza-production-path` — **PASSED** (live substrate advisory).
- `check-fkcs` — PASS (advisory, unchanged oracle state).
- `cargo fmt --all -- --check` — clean.
- `abi-diff` — **not evaluable with a dirty worktree** (see Debug Log); `maos-spirit-abi` diff vs `a414f922` = 0 lines, and the gate returns PASSED on a clean tree.
- T10 proven-red harness — 12/12 mutations reddened their own legs, all controls green, every file restored byte-identically (sha256-verified).

### File List

**Added**
- `crates/maos-bin/src/cross_team_crossing.rs`
- `crates/maos-bin/tests/cross_team_crossing_13_6b.rs`
- `crates/maos-a2a-core/tests/crossing_wire_13_6b.rs`

**Modified**
- `crates/maos-a2a-core/src/cohort.rs`
- `crates/maos-a2a-core/src/error.rs`
- `crates/maos-a2a-core/src/lib.rs`
- `crates/maos-a2a-core/src/router.rs`
- `crates/maos-a2a-core/src/transport/json_rpc.rs`
- `crates/maos-a2a-tcp/src/transport.rs`
- `crates/maos-bin/src/cross_team_consent.rs`
- `crates/maos-bin/src/env_contract.rs`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/cross_team_consent_13_3.rs`
- `crates/maos-loom-lite/src/adapter.rs`
- `crates/maos-loom-lite/src/replication/bundle.rs`
- `docs/adr/ADR-055-multi-tenant-loom.md`
- `docs/loom-threat-model.md`
- `tests/coverage-matrix.yaml`
- `xtask/kloc.toml`
- `xtask/src/check_multi_tenant_loom.rs`
- `_bmad-output/implementation-artifacts/13-6b-production-cross-team-crossing-initiators.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created from the Story 13.6 grounding pass (D-1…D-9). |
| 2026-07-28 | Entailment pressure-test: AC6's evicted-host claim REFUTED — D-12 added. The eviction check was never a control in its own right; it is a side effect of the tenant-map lookup that the crossing legitimately skips. Covered instead by the 12.1/12.3 cohort gate. |
| 2026-07-28 | Party-mode preflight closed four review patches; D-10/D-11 measured; identity half split OUT to 13.6a (operator-ratified) — ordering, not deferral. |
| 2026-07-29 | **Post-13.6a preflight — re-baselined `cb412348` → `a414f922`, UNBLOCKED, and three measured defects folded in.** **D-13 (BLOCKER):** 13.6a authenticates the **envelope** (`request.cohort_source_team`, `router.rs:1429-1445`) while the crossing decides consent from the **payload** (`bundle.source_team` → `is_granted`, `bundle.rs:875-917`). **Nothing binds them, and no AC in either story owned it** — 13.6b's AC6 moved to 13.6a and AC4 is explicitly barred from claiming impersonation coverage, so the weld fell into the seam the split created. D-10's attack survives 13.6a. Now **AC3**, with its own leg, own inverter, and an explicit ban on citing 13.6a's synthetic-frame leg as evidence. **D-14 (BLOCKER):** the emitter cannot be a one-shot — the sole production outbound path is `prepare_outbound` (`a2a-tcp:730`), whose transport is built only in `build_cohort_a2a_daemon_runtime` (`main.rs:9482`) ← `run_cohort_a2a_daemon` (`:9286`) ← the `cohort-a2a-daemon` dispatch (`:7643`); a sibling arm at `main.rs:4883` returns ~4,700 lines earlier. T2 rewritten; the collective-erase idiom is withdrawn. **D-15:** `TransportCause` has **zero** occurrences in either A2A crate, *and* `kernel-core/src/memory/mod.rs:204` discards the cause (`Transport(_)`) — the five-cause matrix has never crossed a process boundary or a kernel boundary, so the old AC2 asked for something the architecture never provided. AC2 **resolved, not weakened**: a new typed A2A cause carrying the ordered pair + intent, one more arm in `interpret_response` (a table that already does this five times), `TransportCause` staying applier-local; the kernel erasure recorded as **Residual 6 with 13.6 as owner**. **New AC4:** extend `reconcile_transport_identity_with_manifest` (`main.rs:9407`) to the team axis — `MAOS_LOOM_HOME_TEAM` vs signed `CohortMember.team` is the identical config-vs-manifest asymmetry 13.6a's own review fixed for certificates one field over; operator ratified **both** AC3 and AC4 (security control + correctness control, neither standing in for the other). **D-16:** `bundle.rs:441`'s seam comment (*"the Spirit→collective digest publication flow … This is the seam it will use"*) is **wrong** — `CollectiveMemoryPort` has no `share` verb and `write` is what `team_guard` refuses foreign-team rows on, so a Spirit-initiated crossing needs a new port verb + a kernel call site. The Spirit initiates publication; the host initiates the crossing; keeping them apart is a security property (*a Spirit that can name a destination team can be prompt-injected into naming one*). Comment corrected in-commit (13.6a T1 precedent), retiring R1 properly. **READ SIDE SPLIT OUT to 13.6d** (operator-ratified, fifth split of the epic): different crates, different dead-wire leg, structurally independent per D-4. Line pins refreshed for 13.6a's +244 router lines (`handle_intake_verified` `1222`→**1385**, `is_reserved_cohort_intent` `502`→**521**). Kernel-core verified **23401 == 23401**. Budget flag raised at design time: **`maos-a2a-core` has 84 lines of reserve on an unratified FLAG-Winston grant**, and that is where the applier, the weld and the new dispatch arm all land. 6 ACs. |
| 2026-07-29 | **IMPLEMENTED (dev pass, `opus-5`) — status → review.** Two-daemon crossing wired end to end: emitter inside `run_cohort_a2a_daemon` (D-14, boot-time arm gated on `MAOS_CROSS_TEAM_SHARE_PEER`, using the 13.3b `originate_team_row` seam which now returns the bundle it signed), applier in `handle_intake_verified` behind a new dependency-inverted `CrossTeamCrossingPort` (D-8). **AC3's weld shipped**: `bundle.source_team` must equal the authenticated envelope claim, refused before `apply_replication_bundle` under `CODE_CROSSING_SOURCE_TEAM_UNBOUND (-32011)`, negative driven by the **seed-holding** forger whose signature is asserted valid first. AC2's `-32012` carries `reason`/`from_team`/`to_team`/`intent` with two `interpret_response` arms, keeping denied/stale/unavailable distinguishable at the emitter; `TransportCause` stays applier-local through the shipped `store_error_to_port_error`. AC4 extends `reconcile_transport_identity_with_manifest` to the team axis. Dead-wire clause (f-i) **inverted and replaced in the same commit**; D-6b closed and the closure proven by fixture. **kernel-core ZERO at 23401**, fkcs byte-untouched, no `CollectivePortError`/`IacBusError`/`FrameKind`/`FramePayload` variant added. 13 new hermetic `Blocking` legs + 1 `AdvisorySubstrate` live leg; 12/12 proven-red mutations with byte-identical restores. |
| 2026-07-29 | **Three self-inflicted weak legs found by the T10 mutation pass and fixed, not narrated.** (1) The first replacement positive was a needle scan and **stayed green when the emitter call was deleted** — 13.5g's exact finding reproduced inside this story's own gate; rewritten as a brace-balanced call-chain reachability walk. (2) The wire-idiom leg round-tripped through `CROSSING_EVENT_TYPE` on both sides and survived drifting it; the literal is now a pinned golden. (3) The non-crossing leg used a frame with no consent envelope, short-circuiting *before* the intent gate it claimed to test; it now uses a well-formed envelope with a different cohort intent. |
| 2026-07-29 | **Budget surfaced and re-based on measurement after trimming (Trap 15).** `maos-a2a-core` 4271→4450, `maos-bin` 14433→14835, `xtask` 31690→31856, `_aggregate_hardfail` 136669→136859; drivers recorded in `xtask/kloc.toml`. 152 lines of inline a2a-core tests were moved to `tests/` and `CrossingRefusal::wire` factored out of the router **before** asking for the bump. ⚠ `maos-a2a-core` is on its second consecutive grant; both remain FLAG-Winston unratified — ratify together at the Epic-13 retro. |
| 2026-07-29 | **Honest non-closures, recorded rather than implied.** §A6 full-layer review net has **not** run (non-degradable for this boundary; must run on a different LLM). AC1's physical presence/absence witness is **written but not executed** — no live Postgres substrate in this environment, so the live leg is `AdvisorySubstrate` + `#[ignore]` + `.expect()` and appears in the gate's WOULD-HAVE-BLOCKED skip list. `abi-diff` is not evaluable with a dirty worktree; `maos-spirit-abi` has **zero** delta vs `a414f922` and the gate passes on a clean tree. `check-env-contract` was already FAIL at `a414f922` (5 pre-existing unregistered reads, Story 12.7). Residuals 2b, 6, 7, 8 remain OPEN and unclaimed. |
| 2026-07-29 | **§A6 adversarial review complete on a different model (`openai-codex/gpt-5.6-sol`) — 11/11 findings patched and verified; status → done.** Fixed the invalid production provenance tuple, fail-open missing-applier ACK, destination-team discard, stale-cause collapse, blank refusal attribution, frame-kind type confusion, unreadable/empty environment inputs, and the composite/missing AC5 controls. The AdvisorySubstrate witness now starts two real `cohort-a2a-daemon` processes and drives the production emitter → mTLS route → verified intake → applier path; deterministic debug-only boot nonces make the two independently generated process pins reproducible. Verification: `maos-a2a-core` crossing unit/wire suites green, `maos-bin` crossing suite 16 passed / 2 substrate-ignored, 13.6a identity compatibility 11/11, `check-multi-tenant-loom` PASSED, `cargo fmt --all -- --check` clean. The live two-Postgres daemon witness remains unexecuted locally because `MAOS_TEST_POSTGRES_TEAM_A/_B` are absent; the gate reports it explicitly as WOULD-HAVE-BLOCKED rather than green. `check-env-contract` remains at its exact five pre-existing Story-12.7 violations and reports no new crossing/test-nonce read. |
