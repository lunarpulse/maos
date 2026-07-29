---
baseline_commit: cb412348
depends_on: 13-6a-authenticated-team-identity, 13-3b-provenance-crosses-the-wall, 13-5d-production-spirit-collective-route
blocked_by: 13-6a-authenticated-team-identity
kernel_grant: NONE — ZERO maos-kernel-core Δ expected, pin stays 23401 (verify at implementation; `maos_kernel_core::iac` is a re-export shim, see D-9). NON-kernel Δ includes maos-cohort COHORT_SCHEMA_V4 per AC6.
preflight: party-mode 2026-07-28 — B2/B4/R1 applied; B1/B3 split OUT to 13-6a-authenticated-team-identity (operator-ratified); 5 ACs
splits_from: 13-6-reza-cortex-journey-closer-nfr-scale-5
---

# Story 13.6b — The cross-team wall has a door, a lock, and a key, and nothing in production ever walks through it

Status: **blocked** — on `13-6a-authenticated-team-identity`

> ⚠ **Renamed 13.6a → 13.6b, 2026-07-28 (operator-ratified split).** The authenticated-identity half moved to **Story 13.6a**, which must land **first**. Reason: `bundle.source_team` is a self-declaration whose signature any seed-holding emitter can forge (D-10), so a crossing built before the identity binding exists is bypassable by impersonation in every commit that contains it. The room rejected that trade in review; it would have been re-created in commit order. **13.6a ships the binding; this story wires the crossing on top of it, and the impersonation negative is live from day one.**

**Kernel-core Δ: ZERO expected — and that is not the same sentence as "zero delta."** `xtask/kernel-core-baseline.toml src_lines = 23401` unchanged; `xtask/fkcs-baseline.toml` stays byte-untouched at the frozen `23081`. Work lands in `maos-bin`, `maos-loom-lite`, `maos-iac`, `maos-domain`, `spirits/` and — per AC6 — **`maos-cohort` (`COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4` + the per-member team binding)**. **No new gate** — legs go on the existing `check-multi-tenant-loom` (ADR-055's gate; the 13.5c **K5(b)** precedent, re-ratified by the 13.5e preflight, says do not create a sibling).

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

### D-10 — ⚠ BLOCKER-GRADE: wiring the crossing turns `base_seed` from a compromise risk into a standing authority

`build_replication_bundle_v2` (`bundle.rs:414`) does:

```rust
let team_seed = derive_team_signing_seed(base_seed, source_region, source_team);
let signing_key = SigningKey::from_bytes(&team_seed);
let signature = signing_key.sign(&sign_payload);
```

and `derive_team_signing_seed(base_seed, region, team)` (`crates/maos-audit/src/sealed_export.rs:72`) works for **any** `(region, team)` in the manifest. T2 requires the emitter host to hold `MAOS_CROSS_TEAM_BASE_SEED`. **Therefore every emitter can sign a valid bundle under every team's key.**

`apply_replication_bundle` then takes `from_team = bundle.source_team` (`bundle.rs:875-879`) and calls `is_granted(from_team, context.to_team, context.intent)` (`:917`). **Consent is decided from a self-declared field whose signature the emitter can forge.**

Nothing binds the authenticated host to the claimed team. `handle_intake_verified` (`router.rs:1222-1232`) binds `request.params.from.host_id == verified_peer` — **host identity only**. And the manifest has no host→team edge: `CohortMember { host_id, fingerprint, roles }` (`manifest.rs:120`) are hosts; `TeamEntry { team_id, region, datname, members: Vec<SpiritId> }` (`:130`) — a team's members are **Spirits**. The only host→team fact in the system is each host's own local `MAOS_LOOM_HOME_TEAM`: unsignable, uncheckable by a peer.

**The attack.** team-a's host — a valid TLS-pinned cohort member holding the seed — wants a row in team-c where `(team-a → team-c)` is **denied** but `(team-b → team-c)` is **allowed**. It stamps `source_team = team-b`, derives team-b's key from the seed, signs. At team-c: transport ✓, signature verifies against the **claimed** pair ✓, `is_granted(team-b, team-c)` ✓. Row lands. **Asymmetric cross-team consent — the property 13.3 exists to provide — bypassed by impersonation.**

**Why the shipped negative does not cover it.** `bundle.rs:1656-1676` / `:1822-1843` forge by **relabel** — sign under team-b, relabel to team-a — which fails because team-a's key was never used. That is a **seed-less** forger. Ours derives the right key and signs correctly. The existing control and this threat do not intersect at any point.

**And the threat model currently claims the opposite.** `docs/loom-threat-model.md` T1 states *"same-region cross-team forgery is cryptographically infeasible"* and **CLOSED on the team axis at ENTRY**. Vex's blast-radius note directly under it scopes the exposure to an attacker who **"recovers"** the seed — a *breach*. 13.6a makes holding the seed a **job requirement**. T1's close was true only while signing was test-only; this story is the introduction.

⚠ **Do not "fix" this by re-scoping AC4 alone.** The room's ratified position (2026-07-28): 13.6a exists so the closer has something real to judge; shipping a knowingly-bypassable crossing defeats the split. **AC6 closes it. AC4's honesty patch stays anyway**, because a seed-holder forging *within its own team*, and an operator who owns a host, remain out of reach.

### D-11 — the fix must bind on the one axis the seed cannot reach, and the canonical form prices it at `COHORT_SCHEMA_V4`

Everything inside the bundle is forgeable, because one key signs all of it — so carrying a stable `SpiritId` in the leaf (v4) does **not** help: the seed-holder forges that too. Note the leaf carries `spirit_pid: i64` (runtime-local, meaningless on the destination host), and `TenantMapPort::team_of(spirit_pid)` (`tenant_map.rs:115`) is local-only.

**The mTLS identity is the exception.** `CohortMember.fingerprint` is operator-pinned and cert-bound; it is **not** derived from `base_seed`. team-a's host cannot present team-b's certificate. That is the axis to bind on.

**Price it honestly.** `to_canonical_bytes` (`manifest.rs:208`) writes per member `host_id` / `fingerprint` / `roles`, and the `TeamEntry` block sits inside the `V2 | V3` arm (`:254`). **Any new field changes the signed pre-image**, so it must be gated behind a new version or v1/v2/v3 signatures break. `COHORT_SCHEMA_V3` and `SIG_DOMAIN_V3` are **already spent** (13.3, `manifest.rs:53`, `SUPPORTED_COHORT_SCHEMAS` at `:54`).

⇒ **`COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4`, with v1/v2/v3 canonical bytes byte-identical by construction** — the 9.2b additive idiom, now over **three** frozen predecessors (13.3b already ran it over two for leaf v3). Do not attempt a "just add an optional field" shortcut: `deny_unknown_fields` plus a shared pre-image means there isn't one.

### D-12 — the evicted-applier case: entailment tested, REFUTED, and re-grounded on the real mechanism

The party-mode preflight asserted that AC6's mTLS team binding would also close the evicted-host residual, and instructed the dev to *"verify that entailment mechanically."* **It was verified. It is false.** Recorded here rather than quietly corrected, because the failure shape is the one this project keeps cataloguing: *a claim that happens to be true is still a claim.*

**Why AC6 cannot close it.** AC6 binds the **sender's** cert → team. The 13.3 finding is about the **receiver** — `state.local_host()` removed from `manifest.members` by a fresh signed reissue. Opposite ends of one connection.

**And the store path is weaker than the finding described.** The crossing applies via:

```
apply_replication_bundle → apply_verified_replication_bundle → store.write_with_source_attested()   (store.rs:553)
```

`team_guard` guards exactly **four** functions — `write:497`, `erase:718`, `read:834`, `scan:956` (13.1's four-site chokepoint). `write_with_source_attested` is a **fifth write path and is not among them** — correctly, because `team_guard` refuses foreign-team rows, which is exactly what a crossing is.

The eviction check lives **inside** `TenantMapAdapter::current_manifest()` (`tenant_map.rs:101-108`, *"local host {} was evicted from the cohort"*), reachable only via:
- `team_of` — called **only** from `team_guard`. The crossing skips it.
- `datname_for` — called **only** from `connection_assignment_guard`, which runs **once at `init_schema`**. Boot-time only; a post-boot eviction is never seen.

And `CrossTeamConsentAdapter::is_granted` (`cross_team_consent.rs:26-46`) is `manifest_if_fresh()` + `cross_team_admits` — **no membership check**, exactly as filed.

⇒ **The eviction check was never a control in its own right — it is a side effect of the tenant-map lookup, and the crossing path drops it as a consequence of being a crossing.**

**What actually closes it.** `handle_intake_verified` (`router.rs:1222`) delegates to `handle_intake` (`:840`), which at `:1108` calls the Story-12.1/12.3 cohort gate. `CohortConsentVerdict::NotCurrent` is documented (`cohort.rs:87-89`) as *"stale **or no longer contains the local member**"* and NACKs at `:1117`. **The A2A cohort gate performs a receiver-side self-membership check on every accepted frame.** Because D-8 puts the applier behind that seam, an evicted applier NACKs before any bundle is applied.

**Four conditions this rests on — all true at `cb412348`, none asserted anywhere. AC6 pins them:**
1. `is_reserved_cohort_intent` (`router.rs:502`) short-circuits the gate for exactly `RESERVED_INTENT_REISSUE` and `RESERVED_INTENT_HALT_RECEIPT` — **on BOTH seams** (`:660` Send, `:1107` Accept). **`collective:share` must never join that set**, or both seams vanish at once.
2. The gate is `Option<Arc<dyn CohortManifestGate>>`; a node constructed without a real gate gets `LegacyCohortManifestGate`, which `Defer`s everything (`cohort.rs:137`).
3. **The emitter side has the same self-check** — verified: `CohortConsentSeam::Send` runs the same chokepoint (`:509`) and `NotCurrent` returns `A2AError::ConfigInvalid` (`:677-682`). An evicted host cannot *send* a crossing either. `NotCurrent`'s condition (`state.rs:555-566`) is freshness **AND** `local_host ∈ members`.
4. ⚠ **`Defer` is evaluated BEFORE the self-membership check, and it is a pass.** This is the one that bites:

```rust
// state.rs:546-554  — counterparty not in roster
if !manifest.members.iter().any(|m| m.host_id == counterparty.as_str()) {
    return CohortConsentVerdict::Defer;      // ← returns BEFORE `current` is computed
}
// state.rs:555-566  — only now is local_host ∈ members evaluated
if !current { return CohortConsentVerdict::NotCurrent; }
```

Two consequences. **(a)** If the counterparty is outside the roster, the local host's own eviction is **never evaluated at all**. **(b)** Worse — an applier whose manifest has dropped the *sender* returns `Defer`, and `:1115` treats `Defer => {}` as a **pass**: a **de-rostered sender can still push a crossing into a healthy applier.** TOFU pin and peer allowlist do not help — those are *config-time* facts, not *manifest-time* facts; the certificate is still valid, only cohort membership was revoked. This is `Defer`'s documented intent (*"a peer outside the roster is a mixed-deployment bilateral path, not a cohort denial"*) — deliberate generality for mixed deployments that is **wrong for a crossing**.

*(Separately confirmed and NOT the hazard: `Defer` is also reachable for a **rostered** peer via `EConsentPeerNotMember` at `state.rs:592` — roster membership and consent-table membership are two different notions. That path sits **after** the `current` check, so it does not bypass self-eviction.)*

**The fix is scoped, not a rollback of 12.1:** on the crossing intent only, `Defer` is a **refusal**, on both seams. General A2A keeps its bilateral fallback; `collective:share` requires cohort membership on both ends. Ratified in the room — Winston withdrew the objection on the ground that `collective:share` postdates 12.1, so this adds a rule to a new intent rather than changing shipped behavior.

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
**And** **no process holds two `LoomLiteStore`s** (D-5) — and this is a **control, not a sentence**: a `Blocking` static leg asserts exactly one production `LoomLiteStore::new` (today `main.rs:2766`), proven-red by adding a second construction. The applier's store is its own home-team store, and `connection_assignment_guard` still passes on both hosts,
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
**Then** each inverted clause is replaced by **(a)** a positive leg naming both endpoints of the now-live path, **and (b)** a *new* negative that only becomes falsifiable **because** the crossing is live — at minimum: an **unconsented crossing is refused at the destination applier** (not merely un-initiated at the source), and a **relabelled `source_team` stamp from a seed-less forger** is refused under the 13.2 derived team key — ⚠ **scoped exactly so, and no wider.** This clause may **not** be written as "forged `source_team` is refused" or "Fork-4 proven": per D-10 a **seed-holding** emitter derives the claimed team's key and signs correctly, so the derived-key check cannot see it. Impersonation is AC6's, not this leg's; overstating it here would be the 27th claim standing in for a control,
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

### ⟶ AC6 moved to Story 13.6a

The authenticated-team-identity AC — manifest `COHORT_SCHEMA_V4` host→team binding, both-seam eviction pins, `Defer`-as-refusal on the crossing intent, and the `loom-threat-model.md` T1 correction — is **Story 13.6a's**, and lands **before** this story.

**What that buys this story:** the impersonation negative is live from this story's first commit rather than being a gap it has to document. D-10, D-11 and D-12 stay here because they are the *measurements that justify the ordering*, and because a dev reading this file must understand why `bundle.source_team` cannot be trusted on its own.

⚠ **Do not re-implement any of it here.** If 13.6a's binding is missing at implementation time, this story is **blocked** — not licensed to ship without it.
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
13. **Never say "Fork-4 proven" about the crossing.** Fork-4's forgery resistance held while signing was test-only. Wiring it hands every emitter the seed, and a seed-holder signs correctly under any team (D-10). AC6 restores a real boundary on the mTLS axis; AC4's derived-key leg covers only the **seed-less relabel** attacker. Two different claims — keep them apart.
14. **`COHORT_SCHEMA_V3` is already spent** (13.3). The team binding is **v4**, and v1/v2/v3 canonical bytes must stay byte-identical — golden-byte tests for all three. There is no "just add an optional field": the pre-image is shared and `deny_unknown_fields` is on.
15. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.

---

## Tasks

- [ ] **T1 (AC1)** — Design the two-process crossing on paper first; write the D-5 sentence into the design note. Decide emitter surface (`MAOS_ONE_SHOT=collective-share` mirroring `main.rs:4883`+) and applier surface (cohort A2A intake arm for intent `collective:share`).
- [ ] **T2 (AC1)** — Implement the emitter on the **seam 13.3b already left**: `originate_team_row(...)` (D-6) to stamp the provenance-carrying row, then `build_replication_bundle_v2(leaves, &region, &home_team, &base_seed)` → emit as a cohort A2A frame under intent `collective:share`. Seed from `MAOS_CROSS_TEAM_BASE_SEED` (D-7). Journal a kernel event (`insert_kernel_event_returning_id`) mirroring the erase verb. ⚠ **This one-shot is an *operator test surface* that exercises the production apply path; it does NOT replace 13.5d's cap-gated Spirit→collective route, which remains the journey-initiated path 13.6 composes.** The D-6 seam doc says *"the Spirit→collective digest publication flow"* — if 13.6's journey needs the Spirit path to be the initiator, that is a **13.6 dependency, not a 13.6a deliverable**; say so rather than letting 13.6 arrive and re-wire the initiator.
- [ ] **T3 (AC1/AC2)** — Implement the applier in **`handle_intake_verified`** (`router.rs:1222`, D-8 — the ratified spoof-proof site; **not** `handle_intake`): → `apply_replication_bundle(bundle, &store, dest_region, Some(CrossTeamApplyContext::new(&home_team, intent)), &base_seed)`. Map `BundleError` → `StoreError` → `CollectivePortError` preserving the five-cause distinguishability. ⚠ **NO new `CollectivePortError` variant — refusal causes ride inside `Transport(TransportCause)` (the 13.3 route).** `collective_port_error` (`crates/maos-kernel-core/src/memory/mod.rs:198-211`) matches `{Unreachable, Timeout, Transport, Memory}` with **no wildcard arm** and lives in **kernel-core**: a new variant makes it non-exhaustive → compile error → the ZERO claim breaks. Cite the `check-kernel-baseline` = **23401** measurement; if it disagrees, **stop for FLAG-Winston**.
- [ ] **T4 (AC3)** — Read side: add the target-team dimension to the recall request path (`LogRecallFilter` or a sibling entry point — `maos-domain`, **not** kernel-core, D-6), and a production surface that supplies it. Confirm plain `recall`'s emitter pin is untouched.
- [ ] **T5 (AC2/AC4)** — Live two-datname integration test: allowed A→B lands; reverse B→A refused `ConsentDenied` with ordered pair + intent; stale lease `MapStale`; unconsented crossing refused **at the applier**; forged `source_team` refused under the derived team key. Physical-absence witness on both.
- [ ] **T6 (AC1/AC3/AC4)** — Rewrite the two dead-wire legs in `check_multi_tenant_loom.rs`: invert, add positive legs naming both endpoints, add the new negatives, one `#[test]` per `--exact` leg, each clause separately named with its inverter. **Close the D-6b hole** and prove it with an `originate_team_row`-from-production fixture.
- [ ] **T6c (AC4)** — Verify at implementation that **13.6a's binding is present and enforcing** before wiring the crossing (peer→team refusal reachable, both-seam eviction pinned, `Defer`-as-refusal live). If any is missing, this story is **blocked**, not licensed to proceed.
- [ ] **T6b (AC1)** — Static `Blocking` leg: exactly one production `LoomLiteStore::new`. Proven-red by adding a second construction. *(D-5 measured it; a measurement left in prose is not a control — the 13.5g shape.)*
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

- **kernel-core:** ZERO expected @ **23401**. If the measurement disagrees, stop for FLAG-Winston. *"kernel-core ZERO" is not the same sentence as "zero delta"* — state both. The **non**-kernel delta is now larger than the first draft: `maos-cohort` gains `COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4` + the per-member team field and its canonical-form arm (AC6). Say both sentences.
- **B2 fence:** no new `CollectivePortError` variant. The kernel mapper is wildcard-free and lives in kernel-core; a variant is the difference between ZERO and a re-pin.
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
2. **Host-axis membership — ⚠ AC6 DOES NOT CLOSE IT. Entailment tested and REFUTED; see D-12.** An earlier draft of this residual claimed the evicted-host case *"falls out"* of AC6's mTLS binding. **It does not.** AC6 binds the **sender's** cert to a team; the 13.3 finding is about the **receiver** (`state.local_host()` evicted). Opposite ends of the same connection. The case is covered — by the Story-12.1/12.3 cohort gate, a mechanism neither room had named — under three currently-true, previously-unasserted conditions that AC6 now pins. **B1 and B3 are therefore NOT one hole with two faces; they are two holes closed by two different mechanisms.**
2b. **Surviving operator-trust limit (OPEN, documented, not closed).** A seed-holder can still forge **within its own team**, and an operator who controls a host controls that host's team. AC6 enforces isolation **against a peer, not against the operator**. True per-team key provisioning stays in the `v25-signed-shard` family (ADR-055 residual #6). This must be a written threat-model line, not an unremarked gap.
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
| 2026-07-28 | **Entailment pressure-test (same session, operator-requested): AC6's evicted-host claim REFUTED — D-12 added.** The preflight had written *"Residual 2 closed in-story by AC6 — same hole, two faces — verify the entailment mechanically."* It was verified and it is **false**: AC6 binds the **sender's** cert→team, the residual is about the **receiver** being evicted. Worse, the crossing writes via `write_with_source_attested` (`store.rs:553`) — a **fifth** write path outside `team_guard`'s four-site chokepoint — and the eviction check exists only **inside** `TenantMapAdapter::current_manifest()`, reachable via `team_of` (only from `team_guard`) or `datname_for` (only from `connection_assignment_guard`, boot-time only). **The eviction check was never a control in its own right; it is a side effect of the tenant-map lookup that the crossing legitimately skips.** The case IS covered — by the Story-12.1/12.3 cohort gate (`router.rs:1108`, `CohortConsentVerdict::NotCurrent` = *"stale or no longer contains the local member"*), a mechanism neither room had named — contingent on three currently-true, unasserted facts now pinned by AC6 and T5b2. **Right conclusion, wrong reason: B1 and B3 are two holes closed by two different mechanisms, not one hole with two faces.** Residual 2 rewritten from CLOSED to REFUTED-and-re-grounded. |
| 2026-07-28 | **Party-mode preflight closed all four review patches.** **B1/B3 (merged, BLOCKER)** — measured D-10: wiring the crossing turns `base_seed` from a compromise risk into a standing authority; every emitter can sign under every team, consent is decided from the forgeable `source_team`, and no host→team edge exists in the signed manifest. `loom-threat-model.md` T1 currently claims the opposite. Room ratified **in-scope, new AC6** (was 5 ACs → **6**, at the epic cap): bind on the **mTLS axis** — the one identity `base_seed` cannot reach — because carrying `SpiritId` in the leaf fails (everything inside the envelope is forgeable). D-11 prices it: `COHORT_SCHEMA_V4` + `SIG_DOMAIN_V4`, v1/v2/v3 bytes frozen (v3 already spent by 13.3). **AC4 re-scoped** to a seed-less relabel forger, with an explicit ban on "Fork-4 proven." Threat-model correction is an in-commit deliverable. Winston dissented on deferring the schema bump; overruled on *"the split exists so 13.6 has something gradeable."* **B2** — T3 fences the kernel mapper (no new `CollectivePortError` variant; ride `Transport(TransportCause)`). **B4** — "no two stores" promoted from prose to a static Blocking leg. **R1** — T2 names the one-shot an operator test surface, not a replacement for 13.5d's Spirit route. Residual 2 now CLOSED in-story by AC6; new residual 2b records the surviving operator-trust limit. |
| 2026-07-28 | Checklist validation pass added D-6 (`originate_team_row` — the seam 13.3b explicitly left for this work, *"This is the seam it will use"*), **D-6b** (the dead-wire leg skips `replication/bundle.rs`, where that seam lives — the negative is one indirection from blind; now an AC4 clause with its own proof obligation), D-7 (`MAOS_CROSS_TEAM_BASE_SEED` is production-wired but verify-side only; this story widens it to the sign side — must be named in the ADR + threat model) and D-8 (applier belongs in `handle_intake_verified`, the 12.3-ratified spoof-proof site). |
