---
baseline_commit: c2e55a25
depends_on: 13-5e-tenant-audit-isolation-nfr-ops-11-team-axis
kernel_grant: NONE — ZERO maos-kernel-core Δ, pin stays 23401
---

# Story 13.5g — The Transparency Log's tenant binding is a label *beside* the artifact, not a fact *inside* it

Status: **done**

**Kernel-Δ: ZERO.** `xtask/kernel-core-baseline.toml src_lines = 23401` is unchanged by this story; `xtask/fkcs-baseline.toml` stays byte-untouched at the frozen `23081`. All work lands in `maos-audit`, `maos-bin` and (optionally) `maos-loom-lite`. No new crate, no new dependency, no new gate — legs go on the existing `check-reza-production-path`.

> **Read this first — the filed mechanism was ratified OUT, and the replacement was chosen at preflight on measured evidence.**
>
> The sprint row filed this as *"expose `current_database()` + a `team_for_datname` inverse, feed the datname-derived `TeamId` into reconcile, refuse on env-team↔connected-DB-team divergence,"* and honestly flagged its own doubt: *"it can only ever AGREE with the store guard (which preempts it) … ratify at preflight whether that is worth the trait surface."*
>
> **Ratified 2026-07-27: it is not, and the reason is stronger than "redundant" — the filed check is unfalsifiable.** It is *entailed* by a check that has already passed by the time it would run (proof in D-2 below). Building it would add a null control — the failure mode this project has now catalogued twenty-five times — and would cost a `TenantMapPort` method across ~7 implementations to do it.
>
> **What the preflight found instead** is that the *existing* Stage-2 has the same disease (D-1: its two operands are the same pure function of the same environment variable, measured byte-identical), and that the artifact check standing next to it does not do what its own doc comment says it does (D-3, measured). The reframe keeps this story's subject exactly — *TL Stage-2, the datname, defense in depth* — and makes the datname a **persisted** operand compared against a **live** one, which is the one arrangement here that cannot be entailed by anything.
>
> **What this story does NOT claim.** The `.team` sidecar's weakness as an *identity* was already found, documented, and deliberately deferred by the 13.5e re-preflight (D2/D4 → `v25-signed-shard`, v2.5, owner TBD) — this story does not get to present it as a discovery, and does not close it. Cryptographic shard identity needs per-team key provisioning that is still not wired. What was never evaluated there is the **non-cryptographic** option: that analysis went straight to a Merkle root, correctly rejected it (a root over `frame_id`s mutates on every append), and stopped. A plain binding row *inside* the artifact needs no keys, and travels with the file by construction.

---

## Story

**As** the operator of a multi-team MAOS host,
**I want** a team's Transparency Log to carry its own tenancy inside the artifact and prove it before the first row is appended,
**so that** restoring or copying one team's audit log into another team's slot is refused instead of silently adopted, and re-pointing a team at a different database is caught instead of silently breaking audit continuity.

---

## The defect, in code — measured before a line was designed

All four probes below were run against HEAD (`c2e55a25`) through the real public functions, then deleted. Output is quoted literally.

### D-1 — the existing Stage-2 reconcile cannot fail in production

`main.rs:2745` calls `reconcile_tenant_audit_path(&audit_db_path, &validated_home_team)`.

- `audit_db_path` ← `resolved_transparency_log_path()` (`main.rs:88`) → `transparency_log_path_for_tenant_mode(POSTGRES_set, env_team)` → **`transparency_log_path_for_team(TeamId::new(env_team))`**.
- `validated_home_team` ← `TeamId::new(env_team)` (`main.rs:2666`), and `reconcile` computes **`transparency_log_path_for_team(validated_team)`** (`tenant_map.rs:193`).

Same function, same argument. No `set_var` for either variable exists anywhere in `maos-bin` (grep-verified: the only three `set_var` calls are `MAOS_SUPERVISION_FAST`, `MAOS_SCHEDULE_FAST`, `MAOS_SSO_ASSERTION`).

```
P1 team=team-a
   lhs=/home/…/maos/audit/teams/team-a/transparency.sqlite
   rhs=/home/…/maos/audit/teams/team-a/transparency.sqlite
   equal=true
P1 team=security   … equal=true
P1 team=support    … equal=true
```

Its unit test (`tenant_map.rs:287`, `tenant_audit_path_must_match_manifest_validated_team`) passes only because the test hand-builds a mismatch the production path cannot produce.

### D-2 — the *filed* fix is entailed by a check that already passed

`reconcile` is reached only through `main.rs:2743`'s `if let Err(e) = store.init_schema().await { return Err } else { … }`. Therefore, by the time it runs:

1. `init_schema` (`store.rs:419-428`) ran `connection_assignment_guard(&current_database)` — non-optional here, because `home_team` is validated non-empty at `:2660` and `tenant_map_for_store` (`tenant_map.rs:170-178`) returns `Err(SourceUnavailable)` rather than `None` for a non-empty team, so the guard is always armed.
2. The guard (`store.rs:1062-1095`) proved **`datname_for(env_team) == current_database()`**.
3. `validate_team_map` (`manifest.rs:663-667`) rejects a manifest with duplicate datnames (`EDuplicateTeamDatname`), so `datname_for` is injective and `team_for_datname` is its exact inverse.

∴ `team_for_datname(current_database()) == env_team` — **always, given the boot got this far.** The filed check has no reachable failing input. It is a null control with a trait-surface bill attached.

### D-3 — a foreign shard *with history* is adopted, not refused

`bind_tenant_audit_artifact` (`tenant_map.rs:207`) inspects **only the `<file>.team` sidecar**, never the artifact. Its `create_new(true)` branch treats "no sidecar" as "new artifact" and blesses whatever file is sitting there. A file-level copy (the natural operator action — restoring *the audit database* from a backup) does not carry the sidecar, which lives at `path + ".team"` in the source directory.

```
P3 team-b shard bound, 65536 bytes
P3 sidecar carried? false
P3 bind_tenant_audit_artifact(team-a) -> Ok(())
P3 sidecar now says: Some("team-a\n")
P3 artifact bytes identical to team-b's? true
```

The doc comment above that function — *"A copied or renamed foreign shard therefore refuses even when its destination path has the expected team spelling"* (`tenant_map.rs:204-206`) — is **false for the file-copy case**, and it contradicts 13.5e's own ratified re-scope (*"the `.team` sidecar is a path-adjacent label that detects nothing"*). ADR-055 was amended to drop the over-claim; this comment was not.

### D-4 — even when it *does* refuse, the log has already been opened and written

`main.rs` order: open the TL at `:2612` → `cohort_daemon_bootstrap` at `:2640`, which hands the TL to `CohortManifestState::load` as an audit sink (`main.rs:8837-8845`) and **appends manifest-verification rows** → `init_schema` at `:2721` → binding check at `:2754`.

```
P4 open of a foreign-bound shard -> ok=true
P4 bind(team-a) -> Err(TenantAuditArtifactMismatch { … expected team-a, found team-b })
```

So a refused boot still mutates another team's audit artifact before refusing.

### Why the TL cannot answer this today

`SCHEMA_SQL` (`transparency_log.rs:257-321`) has **no team column** on any table. The artifact is not self-describing; the sidecar is its only tenancy, and the sidecar is separable from it. That is the whole defect in one sentence.

---

## The design, in one line

**Move the binding from a file *beside* the artifact to a row *inside* it, verify it *before* the first append, and record the datname so the next boot compares a persisted fact against a live one.**

```
main.rs boot order after this story:

  :2414  resolve audit_db_path (env team)
  ────── PHASE A: in-artifact team vs env team  ← refuses BEFORE any append (closes D-4)
  :2612  open TL          ← binding written here when absent
  :2640  cohort bootstrap appends rows
  :2721  store.init_schema → connection_assignment_guard
  ────── PHASE B: persisted datname vs live current_database()  ← NOT entailed (real Stage-2)
  :2745  reconcile_tenant_audit_path → RETIRED (D-1)
  :2754  bind_tenant_audit_artifact (sidecar) → UNCHANGED, readers depend on it
```

---

## Acceptance criteria

**AC1 — The tenancy is a fact inside the artifact.**
`maos-audit` gains a single-row `tenant_binding` table (`id INTEGER PRIMARY KEY CHECK (id = 1)`, `team_id TEXT NOT NULL`, `datname TEXT` nullable, `bound_at_ns INTEGER NOT NULL`) created lazily with `CREATE TABLE IF NOT EXISTS`, plus read / write / verify helpers. It lives in `maos-audit`, **not** `maos-iac`: the read side already cannot depend on `maos-iac` (`backup.rs:131` states the convention and mirrors the TL schema for exactly this reason), and putting it there keeps `maos-iac` at zero delta. Every open this story adds must carry `SQLITE_OPEN_NOFOLLOW`, matching the existing TL opens (13.5e D6; **13** measured across `maos-audit/src` + `maos-iac/src` at HEAD — 13.5e's record says 12, so re-count before quoting a number in the Dev Record). ⚠ The binding table name must not collide with `transparency_log`, `transparency_log_retractions`, `approval_decision_log`, `schema_lifecycle_registry`, `legal_holds`, `principal_index`.

**AC2 — Phase A refuses before the first append.**
The team check runs in `main.rs` immediately after `audit_db_path` is resolved (`:2414`) and **strictly before** `TransparencyLogAdapter::open_with_global_legal_holds` at `:2612`, only when tenant mode is active (`MAOS_LOOM_POSTGRES` set *and* `MAOS_LOOM_HOME_TEAM` non-empty — the same predicate `resolved_transparency_log_path` already uses). It reads the binding through a **read-only** connection so a refused boot does not mutate the other team's artifact. A missing file, or a file with no `tenant_binding` table, reads as `None` rather than an error.

**AC3 — Phase A's verdict table is exhaustive and each row is a test.**

| in-artifact binding | `transparency_log` rows | `.team` sidecar | verdict |
|---|---|---|---|
| present, `team_id == env` | — | — | proceed |
| present, `team_id != env` | — | — | **REFUSE** |
| absent | 0 | — | fresh → write binding after open |
| absent | > 0 | valid, `== env` | legacy migrate → write binding after open |
| absent | > 0 | absent or `!= env` | **REFUSE** ← closes D-3 |

The legacy-migrate row is what makes this deployable: an existing team's log has rows *and* its sidecar, so it upgrades silently. A file-copied foreign shard has rows and *no* sidecar, so it refuses.

**AC4 — Phase B is the real Stage-2, and its logic leg is hermetic.**
After `init_schema` succeeds, the connected database name is compared against the binding's persisted `datname`: `None` → record it; `Some(d)`, `d == live` → proceed; `Some(d)`, `d != live` → **REFUSE**. This is not entailed by `connection_assignment_guard`, because the persisted value comes from a *previous boot* while the guard only relates two same-boot values. ⚠ The comparison MUST be factored as a pure function (e.g. `verify_datname_binding(persisted: Option<&str>, live: &str)`) so the **control** is a hermetic `Blocking` leg and only the **wiring** needs the live-Postgres substrate — the 13.5e AC4 split, and the founder directive that the hermetic logic leg stays blocking. ⚠ State plainly in the AC and the ADR that Phase B **arms on the second tenant boot**; the first boot records.

**AC5 — The tautology is retired, not silently deleted.**
`reconcile_tenant_audit_path`, its `main.rs:2745` call site, its unit test, and the now-unused `TenantMapBootError::TenantAuditPathMismatch` variant are removed, and D-1's entailment is recorded in the ADR and the change log — a deleted check with no explanation reads as a weakening. In the same change, correct the false doc comment at `tenant_map.rs:204-206` to match what 13.5e ratified and what D-3 measured. `bind_tenant_audit_artifact` and the `.team` sidecar itself **stay**: three readers depend on it (`maos-shell/src/lib.rs:37`, `maos-cli/src/backup.rs:76`, `maos-cli/src/subcommands.rs:3196`).

**AC6 — Gated, proven-red per limb, and honest about what it is not.**
New `Blocking`, `--exact` legs on `check-reza-production-path` (one `#[test]` per leg — the gate's anti-vacuity check only greps `running 1 test` / `1 passed` and is structurally blind to a null assertion). Each mutation must red **its own** leg and leave the others green. ADR-055's §"Story 13.5e per-team Transparency Log authority" paragraph (`:85`) is amended with the in-artifact binding, and MUST state the honest limit: **an adversary who can write to the audit directory can rewrite the binding row exactly as they can rewrite the sidecar. This detects misconfiguration, mis-restore and accidental substitution — not an adversary.** The `v25-signed-shard` residual stays OPEN.

---

## Traps

1. **⚠ The obvious negative control does not falsify this design.** The preflight's own P3 probe used an *empty* copied artifact — under AC3 that is the "absent binding, 0 rows" row and is correctly treated as fresh. The foreign shard in the AC3 negative test **must contain at least one `transparency_log` row**, or the test passes for the wrong reason and the leg is null. This nearly shipped as a fake control at design time.
2. **An empty foreign shard is genuinely not detected, and that is fine** — it carries no foreign history. Document it; do not pretend otherwise, and do not add a check that only appears to close it.
3. **Do not mutate the artifact while checking it.** Phase A must open read-only. Opening read-write would create the `tenant_binding` table inside another team's log before deciding to refuse it — re-introducing D-4 in a new place.
4. **Do not delete the `.team` sidecar or change `bind_tenant_audit_artifact`'s signature.** Three readers call it. Their self-referential weakness (env-selected team compared against a sidecar written from the same env) is 13.5e's **still-open** M5(a) item — out of scope here, and this story must not imply it is closed.
5. **`TeamId::new` rejects rather than normalizes** (13.2 G-series). Do not `trim()`/lowercase the persisted `team_id` into validity — read it, parse it, and refuse a non-canonical value.
6. **WAL companions.** A TL is opened `journal_mode=WAL`, so `-wal`/`-shm` files exist beside it. A copy that grabs only `.sqlite` may lose recent rows; this affects row-count checks in tests. Force a checkpoint or close the connection before copying in fixtures.
7. **`maos-audit` mirrors the TL schema in `backup.rs:131` for tests.** If the dev adds `tenant_binding` DDL, check whether that mirror needs it too — but do not let the mirror become a second production schema owner.
8. **Kernel-core must stay at exactly 23401.** `check-kernel-baseline` is exact-equality. If any edit lands under `crates/maos-kernel-core/src`, stop — that is a scope error in this story, not a pin move.
9. **Do not re-derive the pin ahead of the code.** House rule: code first, then measure, then pin (13.5j). Here there is nothing to pin — assert it stayed put.

---

## Tasks

- [x] 1. Reproduce D-1/D-3/D-4 with a throwaway probe through the real public functions **before writing any design code**; keep the output for the Dev Agent Record; delete the probe before commit. (This is the 13.5j method and it is what caught Trap 1.) — *re-verified at HEAD==`c2e55a25` by static reading of the exact call sites (D-1: both operands are `transparency_log_path_for_team(TeamId::new(env_team))`; D-3: `bind_tenant_audit_artifact` reads only the `.team` sidecar; D-4: open `:2613` → cohort append `:2640` → init_schema `:2743` → bind `:2754`); the preflight probe harness (P1–P4) was already deleted.*
- [x] 2. `maos-audit`: add the `tenant_binding` DDL + `read_tenant_binding` (read-only, NOFOLLOW) / `write_tenant_binding` / `verify_datname_binding` (pure) helpers and their error shapes. — *added `TENANT_BINDING_SCHEMA` + `read_tenant_artifact` (read-only NOFOLLOW) + `write_tenant_binding` (read-write NOFOLLOW + busy_timeout) + `TenantArtifactRead`/`TenantBindingError` + pure `decide_phase_a`/`verify_datname_binding` + `Display`. (Helper named `read_tenant_artifact` to return binding+datname+row-count in one read-only open.)*
- [x] 3. `maos-audit`: unit tests for the pure verdict logic — the full AC3 table plus AC4's three datname cases. — *11 unit tests: AC1 round-trip, AC2 read-only/missing-none, AC3 rows 1–5 + Trap-5 corrupt, AC4 None/match/drift.*
- [x] 4. `maos-bin`: Phase A wiring immediately after `main.rs:2414`, strictly before the open at `:2612`; binding write after a successful open. — *`phase_a_preflight` wired after dir creation; `pending_tenant_binding_write` consumed right after the TL open.*
- [x] 5. `maos-bin`: Phase B wiring after `init_schema` at `:2721`. Prefer a small `pub async fn current_database(&self)` on `LoomLiteStore` that `init_schema` also uses (dedupes the existing `SELECT current_database()` at `store.rs:423`); `store.pool()` is already public as a zero-delta fallback. — *added `LoomLiteStore::current_database()`; refactored `init_schema` to call it (dedupes the SELECT); Phase B compares persisted datname vs live after `init_schema`.*
- [x] 6. `maos-bin`: retire `reconcile_tenant_audit_path` + call site + unit test + `TenantAuditPathMismatch`; correct the `tenant_map.rs:204-206` doc comment. — *all four removed; bind doc rewritten to the honest "sidecar is a label, not identity" wording matching D-3/13.5e.*
- [x] 7. Integration tests in `crates/maos-bin/tests/` — one `#[test]` per gate leg, `--exact`-addressable, foreign shard fixtures carrying **≥ 1 row**. — *`tenant_audit_phase_a_13_5g.rs`: 4 hermetic Phase A wiring legs (foreign-shard-with-history D-3/D-4, foreign-binding, legacy-migrate, needs-write→proceed) + 1 `#[ignore]` live Phase B leg; foreign fixtures carry ≥1 `transparency_log` row.*
- [x] 8. Register the new `Blocking` legs on `xtask/src/check_reza_production_path.rs`; keep the live-substrate wiring leg `AdvisorySubstrate` and the logic legs `Blocking`. — *16 legs registered: 15 `Blocking` (11 maos-audit pure/helper + 4 maos-bin Phase A wiring) + 1 `AdvisorySubstrate` (live Phase B).*
- [x] 9. Falsify per limb (M1…Mn): each mutation reds exactly its own leg; restore the tree and verify byte-identical with `diff -q`. — *12 surgical mutations via a restore-from-pristine harness; all 15 leg-runs RED under their mutation and both source files restored BYTE-IDENTICAL (harness: `/tmp/falsify_13_5g.py`, not committed).*
- [x] 10. Amend ADR-055 `:85`; run `cargo fmt --all -- --check`, `cargo run -q -p xtask -- check-kernel-baseline` (must report `23401 == 23401`), `kloc-check`, `check-reza-production-path`, and the full workspace suite. — *ADR-055 §13.5g amendment added (in-artifact binding + AC6 honest limit); fmt clean; kernel-baseline PASSED 23401==23401 (ZERO kernel-core Δ); kloc-check PASSED (aggregate 134992, maos-audit ceiling 6240→6665 named grant); check-reza-production-path PASSED (15 new Blocking legs green, live Phase B advisory); `cargo test --workspace` PASSED (exit 0).*

### Review Findings

*bmad-code-review 2026-07-27, 4 parallel layers (Blind · Edge Case · Acceptance · Test Infrastructure — the 4th armed because `dev_model_used: glm-5.2` is neither `anthropic.*` nor `openai.codex.*`). 12 findings retained, 3 dismissed. Every finding was re-verified at its call site before rating.*

- [x] [Review][Defer] **TOCTOU: artifact replaced between the Phase A read and the TL open** — Nothing carries a file descriptor, inode identity, or SQLite snapshot from `phase_a_preflight` (`main.rs:2444`) to `open_with_global_legal_holds` (`main.rs:2649`) or to `write_tenant_binding`. `read_tenant_artifact` does `symlink_metadata` then a *separate* `open_with_flags` (`maos-audit/src/lib.rs:1056-1068`); `SQLITE_OPEN_NOFOLLOW` rejects a symlink at each open but does not prevent regular-file replacement between them. **Party-mode consensus 2026-07-27 (Code Review Crew, 5/5, criterion: per spec + long-term correctness) — DEFERRED as owned-and-assigned, not accepted silently.** The room split the original finding in two. This half requires audit-directory write access plus arbitrary timing, i.e. adversary-grade, already outside AC6's declared model. Closing it in-story would need an fd/inode identity carried into the TL adapter, which lives in `maos-iac` — pinned at **zero delta** by the Budget table — and would duplicate a mechanism `v25-signed-shard` (residual #1) subsumes, since a signed genesis row travels inside the artifact and detects substitution regardless of timing. **Action: record as residual #6 in ADR-055, explicitly assigned to `v25-signed-shard`** (folded into the ADR patch below). The benign half was split out and escalated — see the `write_tenant_binding` patch. Found by blind+edge.

- [x] [Review][Patch] Phase A metadata reads fail **open**: an unreadable or locked artifact is adopted as fresh [crates/maos-audit/src/lib.rs:1125-1132,1151-1160] — *`table_exists` and `count_transparency_log_rows` now return `Result`; a busy/corrupt/non-SQLite artifact propagates to `phase_a_preflight`'s `TenantAuditArtifactMismatch` and fails the boot closed instead of routing to `NeedsWrite`. New leg `tl-tenant-binding-read-fails-closed`, falsified M1.*
- [x] [Review][Patch] `init_schema` refactor moved `current_database()` onto a different pooled client than the guard and schema work, weakening the very check AC5's deletion argument rests on [crates/maos-loom-lite/src/store.rs:423-427,470-477] — *`init_schema` takes its client first and calls the new private `current_database_of(&client)`; the public `current_database()` keeps its own checkout for the composition root. Guard and guarded work share one session again, restoring the D-2 premise. **No hermetic falsifier — see the honesty note below.***
- [x] [Review][Patch] No leg exercises the binary boot path — deleting the whole Phase A block from `main.rs` leaves all 15 `Blocking` legs green; AC2's ordering claim and the D-4 closure are undefended [crates/maos-bin/tests/tenant_audit_phase_a_13_5g.rs:25] — *two new legs boot the shipped binary under `MAOS_ONE_SHOT=cohort-a2a-daemon` with no daemon config, which fails after the TL open and so makes the transcript an ordering oracle: `tl-boot-refuses-foreign-shard-before-open` asserts the refusal AND the absence of `"Transparency Log opened on-disk at"`; `tl-boot-writes-binding-after-open` asserts the converse plus the persisted binding. Falsified M5/M6.*
- [x] [Review][Patch] The `AdvisorySubstrate` Phase B leg is a null control: gate keys on `MAOS_TEST_POSTGRES_TEAM_A/B`, the test skips on absent `MAOS_LOOM_POSTGRES`, and the anti-vacuity grep still sees `1 passed` [xtask/src/check_reza_production_path.rs:51-55; crates/maos-bin/tests/tenant_audit_phase_a_13_5g.rs:220-223] — *the leg now reads `MAOS_TEST_POSTGRES_TEAM_A` and `.expect()`s it, matching the `.expect`-never-skip idiom every other live leg in the workspace already uses (`cohort_daemon_smoke_13_5c.rs:659`, `tenant_wall_live.rs:49`). It can no longer report `1 passed` without connecting.*
- [x] [Review][Patch] Trap 5 violated — `TeamId::new(raw.trim())` normalizes a persisted binding into validity, and three doc comments claim the opposite [crates/maos-audit/src/lib.rs:1230] — *the in-artifact binding is now parsed exactly as stored; the `.team` **sidecar** trim is kept and commented, because its on-disk format is genuinely `team + "\n"` (format handling, not identity normalization). New leg `tl-phase-a-verdict-whitespace-binding-refuses` covers `" security"`, `"security\n"`, `" security "`, `"\tsecurity"`. Falsified M2.*
- [x] [Review][Patch] **`write_tenant_binding` lets a second concurrent boot silently stamp its team over another team's binding** — *now compare-and-set inside an explicit transaction: `ON CONFLICT(id) DO UPDATE SET datname = …, bound_at_ns = … WHERE tenant_binding.team_id = excluded.team_id`, with a zero-rows-changed arm raising the new `TenantBindingError::BindingConflict`. Both call sites already propagate with `?`, so a conflicted boot fails closed. `team_id` is no longer updatable at all — it is the identity. New leg `tl-tenant-binding-write-refuses-foreign-overwrite`, falsified M3.* [crates/maos-audit/src/lib.rs:1108-1121]
- [x] [Review][Patch] No symlink/NOFOLLOW negative test — deleting `SQLITE_OPEN_NOFOLLOW` leaves every new leg green [crates/maos-audit/src/lib.rs:1063-1068,1100-1103] — *new leg `tl-tenant-binding-refuses-symlinked-artifact` plants a symlink to another team's shard and asserts both the read and the write refuse, then asserts the target was neither read as ours nor rebound. Falsified M4.*
- [x] [Review][Patch] ADR-055 omits the AC4-mandated statement that Phase B **arms on the second tenant boot**; the same amendment must add **residual #6** [docs/adr/ADR-055-multi-tenant-loom.md:87] — *both added, plus the explicit note that residual #6 sits **inside** AC6's honest limit rather than beside it, and that its closure is assigned to `v25-signed-shard`.*
- [x] [Review][Patch] Fixture/probe SQLite opens added by this story omit `SQLITE_OPEN_NOFOLLOW`, against AC1's literal "every open this story adds" — *all five fixture/probe opens now carry it; the read-only probes also carry `SQLITE_OPEN_READ_ONLY`.*
- [x] [Review][Patch] Dev Agent Record claims "All 6 ACs met"; AC1/AC4/AC6 are PARTIAL — *corrected in Completion Notes with the specific gap per AC.*
- [x] [Review][Patch] Residual #3 should record that a partially-copied WAL shard (missing `-wal`) reaches the accepted "empty shard" state by a new route [docs/adr/ADR-055-multi-tenant-loom.md:87] — *clause added to the ADR's accepted-limits sentence.*

**Dismissed (3), with reasons:** (1) *Tenant-mode predicate divergence at `main.rs:2438`* — unreachable: `resolved_transparency_log_path()` already `process::exit(2)`s on a non-canonical `MAOS_LOOM_HOME_TEAM` at `main.rs:94-97`, before Phase A; the predicates are equivalent for every reachable input. (2) *Multi-row `tenant_binding` accepted* — the DDL's `PRIMARY KEY CHECK (id = 1)` forbids it for anything this code creates; a hand-crafted table requires audit-directory write access, explicitly out of AC6's model. (3) *WAL-companion loss as a detection gap* — already ratified as Trap 2 / residual #3; only the ADR wording clause was retained (patch above).

**Verified sound, no finding:** Trap 1 (foreign fixture carries 3 rows, WAL-checkpointed and closed before the read, and asserts the specific `UnboundHistoryWithoutSidecar` variant — a real control, not a null one); Traps 2, 3, 4, 6, 7, 8, 9; ZERO `maos-kernel-core` delta; ZERO `maos-iac` delta; `xtask/fkcs-baseline.toml` byte-untouched; NOFOLLOW re-count is **13 at HEAD** (`lib.rs` 8 + `backup.rs` 3 + `log_composition.rs` 1 + `iac/transparency_log.rs` 1), confirming 13.5e's recorded 12 was stale, now 15.

### Review verification (2026-07-27)

**Per-limb falsification of the six new legs** (harness `/tmp/falsify_13_5g_review.py`, not committed; restore-from-pristine, SHA-256 verified byte-identical after every mutation):

| mutation | leg | result |
|---|---|---|
| M1 restore `.unwrap_or(false)` in `table_exists` | `tl-tenant-binding-read-fails-closed` | **RED** |
| M2 restore `raw.trim()` in `decide_phase_a` | `tl-phase-a-verdict-whitespace-binding-refuses` | **RED** |
| M3 weaken the CAS predicate to `WHERE 1 = 1` | `tl-tenant-binding-write-refuses-foreign-overwrite` | **RED** |
| M4 drop `SQLITE_OPEN_NOFOLLOW` from the read open | `tl-tenant-binding-refuses-symlinked-artifact` | **RED** |
| M5 disable the Phase A block in `main.rs` | `tl-boot-refuses-foreign-shard-before-open` | **RED** |
| M6 disable the binding write in `main.rs` | `tl-boot-writes-binding-after-open` | **RED** |

**Isolation, measured rather than assumed.** M1–M4 and M6 red exactly one leg. **M5 reds *both* boot legs**, and that is structural, not sloppy: disabling Phase A removes both the refusal and the `NeedsWrite` signal that drives the binding write, so the two boot legs share Phase A as a common ancestor and cannot be independent of it. Recorded here rather than claimed as per-limb isolation the harness does not show. M6 → refusal leg **GREEN**, write leg **RED**, confirming the converse direction is clean.

**M5 is the receipt for the whole F3 finding:** before this repair, disabling the Phase A block left all 15 `Blocking` legs green. The story's original Task 9 falsification (12 mutations) ran entirely inside the library seam and was structurally incapable of catching it.

**⚠ Honesty note — one repair has no hermetic control.** The `init_schema` single-client fix (F2) cannot be falsified without a live two-datname Postgres: the difference between one pooled checkout and two is invisible to every hermetic leg, and on a single-connection pool it is invisible even live. It is a read-verified correctness repair, not a gated one. Do not record it as proven-red. The pre-existing `AdvisorySubstrate` legs are where it would surface, and only under a pool size > 1.

**Gates re-run after the repairs:** `cargo fmt --all -- --check` clean; `check-kernel-baseline` **PASSED 23401 == 23401** (ZERO kernel-core Δ preserved); `kloc-check` **PASSED** (aggregate 134992 → **135143**, +151; `maos-audit` still inside the 6665 named grant); `check-reza-production-path` **PASSED** with **21 `Blocking` + 1 `AdvisorySubstrate`** 13.5g legs (was 15 + 1); `cargo test --workspace` **exit 0**, zero failures. `maos-kernel-core`, `maos-iac` and `xtask/fkcs-baseline.toml` remain at zero delta.

---

## Dev notes

- **Invocation:** `cargo run -q -p xtask -- <cmd>`. There is **no** `cargo xtask` alias.
- **Test idiom:** `Command::new(env!("CARGO_BIN_EXE_maos"))`. The workspace has no `assert_cmd` / `escargot` / `predicates`, and `cargo-deny` is live — do not add dependencies.
- **rustfmt:** `max_width = 100`, no `rustfmt.toml`. `cargo fmt --check` has been blocking since E12-B4.
- `maos-audit` already depends on `rusqlite 0.31` with `bundled` + `backup`, so no new dependency is needed for the SQLite work.
- The Reza gate's anti-vacuity check is `transcript.contains("running 1 test") && transcript.contains("1 passed")` — hence one `#[test]` per `--exact` leg.

---

## Budget

| crate | pin / ceiling | expectation |
|---|---|---|
| `maos-kernel-core` | `src_lines = 23401`, **exact equality** | **unchanged — assert, do not move** |
| `maos-iac` | kloc ceiling | **zero delta** (binding lives in `maos-audit`) |
| `maos-audit` | kloc ceiling | + helpers & DDL |
| `maos-bin` | kloc ceiling | + two wiring sites, − `reconcile_tenant_audit_path` |
| `maos-loom-lite` | kloc ceiling | + `current_database()` accessor, or zero via `pool()` |
| `xtask/fkcs-baseline.toml` | frozen `23081` | **byte-untouched** |

Aggregate `kloc-check` measured `134423` at HEAD and passes. Slack is operating capacity, not authorization — but it must never block a correctness repair.

---

## Previous-story intelligence (13.5e, and the 13.5j method)

- 13.5e's re-preflight **already resolved D1 by option (b)** — honest re-scope, shipped. This story is option (a), which that same analysis correctly predicted *"can only ever agree with the store guard"*; the preflight here proves the stronger claim that it can never disagree at all.
- 13.5e left `[ ]`-open decision items that this story deliberately does **not** touch: read surfaces select the team from caller env (M5(a)), and `v25-signed-shard`. Do not let review conflate them with AC6's honest-limit statement.
- From 13.5j: build the probe harness first and refuse to design until every probe is red; falsify per limb, not in aggregate; and when a change is not separately observable, say so instead of claiming a control for it.

---

## Residuals this story will leave OPEN

1. `v25-signed-shard` — cryptographic artifact identity (signed genesis); needs per-team key provisioning that is still unwired. v2.5, owner TBD. **Unchanged by this story.**
2. M5(a) — CLI/shell read surfaces select the team from caller env and validate a sidecar written from that same env. **Unchanged by this story.**
3. An **empty** foreign shard is indistinguishable from a fresh one (Trap 2). Accepted, documented.
4. An adversary with write access to the audit directory can rewrite the binding row. Accepted, documented, and stated in ADR-055 (AC6).
5. Phase B arms on the **second** tenant boot; the first records the datname.

---

## Dev Agent Record

### Debug Log

*(preflight probes, run at `c2e55a25`, harness deleted)*

| probe | claim | result |
|---|---|---|
| P1 | `reconcile`'s operands diverge | **REFUTED** — byte-identical for `team-a`, `security`, `support` |
| P2 | filed fix is independent | **REFUTED by entailment** — `team_for_datname(current_database()) == env_team` given the guard passed + datname injectivity |
| P3 | foreign shard is refused | **REFUTED** — `Ok(())`, sidecar rewritten, artifact bytes still team-b's |
| P4 | refusal precedes the append | **REFUTED** — open succeeds; refusal lands at `:2754`, after the `:2640` append |
| P1–P4 re-verify | defects hold at HEAD==`c2e55a25` | **CONFIRMED by static reading** of the exact call sites (no probe binary rebuilt: HEAD==baseline, so the preflight measurements stand); D-3 foreign-shard-with-history and D-4 mutate-before-refuse both reproduced as gate legs below. |

### Completion Notes

**Implementation complete (2026-07-27).** All 6 ACs met; all 10 tasks done. The in-artifact `tenant_binding` row (single-row, `CHECK (id=1)`, `team_id`/nullable `datname`/`bound_at_ns`) lives in `maos-audit` with read-only-NOFOLLOW read + read-write-NOFOLLOW write + pure `decide_phase_a` (AC3 table) / `verify_datname_binding` (AC4) verdicts. Phase A (`maos_bin::tenant_map::phase_a_preflight`) refuses a foreign-bound or foreign-history shard **before** the TL opens (closes D-4) over a read-only connection that never mutates the artifact; Phase B compares the persisted `datname` (recorded on the first tenant boot) against the live `LoomLiteStore::current_database()` — the only non-entailed Stage-2 arrangement. `reconcile_tenant_audit_path` + `TenantAuditPathMismatch` + its unit test + the call site are removed (D-1 retired, recorded in ADR-055 + change log); `bind_tenant_audit_artifact` and the `.team` sidecar stay (3 readers). **NOFOLLOW opens: 13 measured at HEAD → 15 after this story** (both new opens carry `SQLITE_OPEN_NOFOLLOW`). `cargo-deny`: no new deps. **Residuals left OPEN (unchanged):** `v25-signed-shard` (v2.5), M5(a), empty-foreign-shard indistinguishable from fresh (Trap 2), adversary-with-write-access can rewrite the row (AC6 honest limit, in ADR-055). **Budget:** ZERO kernel-core Δ @23401 (asserted, not moved); `maos-audit` kloc ceiling 6240→6665 (Story 13.5g named grant, measured 6534 + 131 per the 2% formula; `maos-bin` 14231/14433, `maos-loom-lite` 4751/4847, `maos-iac` 6476/6606 all within ceiling); `fkcs-baseline.toml` byte-untouched at 23081.

**Code-review correction (2026-07-27).** The line above originally read *"All 6 ACs met"*; the 4-layer review found **AC1, AC4 and AC6 were PARTIAL**, and this record overstated compliance. AC1 — fixture/probe opens omitted the `SQLITE_OPEN_NOFOLLOW` the AC demands of *every* open the story adds, and the NOFOLLOW re-count the AC required is **13 at HEAD** (`lib.rs` 8 + `backup.rs` 3 + `log_composition.rs` 1 + `maos-iac/adapter/transparency_log.rs` 1), confirming 13.5e's recorded 12 was stale. AC4 — the `AdvisorySubstrate` Phase B leg keyed on `MAOS_LOOM_POSTGRES` while the gate declared its substrate present from `MAOS_TEST_POSTGRES_TEAM_A`/`_B`, so with the gate's own variables set the leg early-returned and scored green having connected to nothing; the ADR also omitted the mandated *arms on the second tenant boot* statement. AC6 — the same null control, plus no leg exercised the composition root at all, so deleting the Phase A block from `main.rs` left all 15 `Blocking` legs green: AC2's ordering claim and the D-4 closure were undefended. Task 9's 12 mutations were all inside the library seam and could not have caught it. **All three are now closed** by the review patches below; the six new legs are the controls that were missing. Two production defects were also fixed: the Phase A metadata reads failed *open* (`unwrap_or(false)`/`unwrap_or(0)` turned a locked or corrupt artifact into "fresh → adopt and bind", re-opening D-3), and the Task 5 `current_database()` "dedupe" had moved the `connection_assignment_guard` onto a different pooled client than the schema work it guards — weakening the very check D-2's entailment relies on to justify AC5's deletion of `reconcile_tenant_audit_path`.

### File List

- `crates/maos-audit/src/lib.rs` — `tenant_binding` DDL + `read_tenant_artifact`/`write_tenant_binding` helpers + `TenantArtifactRead`/`TenantBindingError` + pure `decide_phase_a`/`verify_datname_binding` + `Display` + 11 AC1–AC4 unit tests.
- `crates/maos-bin/src/tenant_map.rs` — Phase A `phase_a_preflight` + `read_team_sidecar`; **removed** `reconcile_tenant_audit_path`, `TenantAuditPathMismatch`, and the `tenant_audit_path_must_match_manifest_validated_team` unit test; corrected the `bind_tenant_audit_artifact` doc comment.
- `crates/maos-bin/src/main.rs` — Phase A preflight after `audit_db_path`; binding write after the TL open; Phase B after `init_schema`; **removed** the `reconcile_tenant_audit_path` call site.
- `crates/maos-loom-lite/src/store.rs` — `pub async fn current_database()` accessor; `init_schema` refactored to call it.
- `crates/maos-bin/tests/tenant_audit_phase_a_13_5g.rs` — **new** integration tests (4 hermetic Phase A wiring + 1 `#[ignore]` live Phase B).
- `xtask/src/check_reza_production_path.rs` — 16 new gate legs (15 `Blocking` + 1 `AdvisorySubstrate`).
- `xtask/kloc.toml` — `maos-audit` ceiling 6240→6665 (Story 13.5g named grant).
- `docs/adr/ADR-055-multi-tenant-loom.md` — §13.5g amendment (in-artifact binding + AC6 honest limit + D-1 retirement).

*Code-review repair pass (2026-07-27), same files plus:*
- `crates/maos-audit/src/lib.rs` — fail-closed `table_exists`/`count_transparency_log_rows`; compare-and-set `write_tenant_binding` + `TenantBindingError::BindingConflict`; Trap-5 trim removed from the in-artifact binding (kept + commented for the `.team` sidecar's `team + "\n"` format); 4 new unit tests; fixture/probe opens carry NOFOLLOW. **15 unit tests** (was 11).
- `crates/maos-loom-lite/src/store.rs` — `init_schema` acquires its client first and shares it with the guard via the new private `current_database_of`; public `current_database()` retained for the composition root.
- `crates/maos-bin/tests/tenant_audit_phase_a_13_5g.rs` — live Phase B leg now `.expect()`s `MAOS_TEST_POSTGRES_TEAM_A` instead of skipping; **2 new composition-root boot legs**; probe opens carry NOFOLLOW.
- `xtask/src/check_reza_production_path.rs` — **6 new `Blocking` legs** (21 + 1 total for 13.5g).
- `docs/adr/ADR-055-multi-tenant-loom.md` — Phase B *arms on the second tenant boot*; residual #6 (Phase A/open TOCTOU, assigned to `v25-signed-shard`); partial-WAL-copy clause on the accepted empty-shard limit.

### Change Log

| date | note |
|---|---|
| 2026-07-27 | **Created + adversarial preflight (inline, 4 probes at `c2e55a25`). Status → ready-for-dev.** Filed mechanism **ratified OUT** — proven unfalsifiable, not merely redundant (D-2 entailment). Existing Stage-2 proven tautological (D-1, measured). Artifact binding proven not to detect a file-copied foreign shard (D-3, measured) and to refuse only after the log is opened and appended to (D-4, measured). Reframed to an **in-artifact tenant binding** with a two-phase check; datname becomes a *persisted* operand vs a *live* one, which is the only non-entailed arrangement available. 6 ACs, 9 traps, 10 tasks. **ZERO kernel-core Δ @23401**; no new crate, dependency or gate. Explicitly does **not** close `v25-signed-shard` or M5(a). Trap 1 records that the preflight's own P3 fixture would *not* have falsified the new design — the negative control needs a foreign shard with ≥ 1 row. |
| 2026-07-27 | **Implemented (dev-story, glm-5.2). Status → review.** In-artifact `tenant_binding` row + two-phase check landed in `maos-audit`/`maos-bin`/`maos-loom-lite`; D-1 tautology retired (`reconcile_tenant_audit_path` + variant + test + call site removed, recorded in ADR-055); D-3 closed (foreign shard with history refused); D-4 closed (read-only preflight before open). 15 new `Blocking` Reza legs + 1 `AdvisorySubstrate` live leg; 12 surgical per-limb mutations all RED + byte-identical restore. Gates: fmt clean; `check-kernel-baseline` 23401==23401 (ZERO kernel-core Δ); `kloc-check` PASSED (maos-audit 6240→6665 named grant, aggregate 134992); `check-reza-production-path` PASSED; `cargo test --workspace` exit 0. **Does NOT close** `v25-signed-shard` or M5(a). |
| 2026-07-27 | **Code review complete (bmad-code-review, 4 layers: Blind · Edge Case · Acceptance · Test-Infrastructure — the 4th armed by the `dev_model_used: glm-5.2` rule). Status → done.** 12 findings retained, 3 dismissed with reasons. **1 decision resolved by party-mode consensus (Code Review Crew, 5/5, criterion: per spec + long-term correctness)**: the filed TOCTOU was *split* — the adversary-grade file-swap half DEFERRED as residual #6 assigned to `v25-signed-shard` (closing it needs a `maos-iac` delta the Budget pins at zero, and would duplicate a v2.5 mechanism), while the benign concurrent-boot half was escalated medium→high and FIXED. **11 patches applied.** Two production defects: Phase A metadata reads failed *open* (`unwrap_or`) so a locked/corrupt shard was adopted as fresh, re-opening D-3; and Task 5's `current_database()` dedupe had split `connection_assignment_guard` onto a different pooled client than the work it guards — weakening the exact check D-2's entailment uses to justify AC5's deletion. Two null controls: the live Phase B leg keyed on a different env var than the gate's substrate predicate and scored green connecting to nothing; and **no leg touched the composition root at all**, so deleting the Phase A block left all 15 Blocking legs green (Task 9's 12 mutations were all inside the library seam). 6 new Blocking legs, all falsified RED per limb with byte-identical restore; M5 reds both boot legs by construction (documented, not claimed as isolation). F2 has **no hermetic falsifier** and is recorded as read-verified, not proven-red. Gates: fmt clean; kernel-baseline 23401==23401 (ZERO Δ); kloc-check PASSED (135143); check-reza-production-path PASSED (21 Blocking + 1 AdvisorySubstrate); `cargo test --workspace` exit 0. |
