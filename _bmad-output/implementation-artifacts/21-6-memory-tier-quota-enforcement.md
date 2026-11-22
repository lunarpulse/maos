---
story_id: '21.6'
story_key: 21-6-memory-tier-quota-enforcement
created: 2026-11-20
status: in-progress
context_commit: ea8c51b6a943f536179fe2238bcf050353580d35
baseline_commit: ea8c51b6
security_sensitive: true
kernel_grant: "Forecast only; no prototype or grant requested. Kernel +185…+440 tokei under the approved D-row options (method and per-option deltas below). ADR-069 kfree applies inside the envelope: measure at landing, re-pin the kernel exact pin with scripts/repin.py and write its HISTORY row in the same commit. No forecast reaches the 22,000 alarm."
kloc_grant: "Read-the-files forecast under the approved D-rows, not measured growth or permission: maos-kernel-core +185…+440; maos-domain +20…+60; maos-bin +15…+60; maos-manifest 0; maos-audit 0…+10. Re-measure against the actual landing base."
review: "DIFFICULT: kernel growth, cross-Spirit isolation (shared tier attribution) and concurrency (check-and-charge atomicity). Follow docs/testing/proof-contract.md. Story creation anthropic/claude-opus-5-5 (DraftMemoryQuota). No independent review or execution claimed."
---

# Story 21.6: Memory-tier quota enforcement (FR28)

Status: in-progress

**Readiness:** `backlog`. The operator approved D-21-6-01…06 as **B / A / A / A / A / B** on 2026-11-21 UTC. No quota-policy decision remains open. The rulings do not advance status, change sequencing after 21-9, or authorize landing or shipping.

**GA-required, size M.** Authority: `planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md:139-152`, especially the 2026-11-13 note at `:152`; `planning-artifacts/sprint-change-proposal-2026-11-13.md:50` (and `:32` for the GA reason); `planning-artifacts/prd/project-scoping-phased-development.md:421`; FR28 `planning-artifacts/prd/functional-requirements.md:74`; routing row `planning-artifacts/epics/requirements-inventory.md:556`; tracker `implementation-artifacts/sprint-status.yaml:418` (key) and `:508` (`v25-reza-postgres-legs`, which holds the collective leg). Paths under `planning-artifacts/` and `implementation-artifacts/` are relative to `_bmad-output/`.

## Story

As **an operator running always-on Spirits on one host**,
I want **the kernel to refuse, with a typed error, any private-tier or shared-tier memory write that would take a Spirit past its quota**,
so that **no Spirit fills the host's disk, while compaction stays the Spirit's own job (FR28: "kernel provides persistence and quota enforcement only")**.

**Exact scope:** AC1–AC4 on the **private and shared tiers only**, proven without Postgres. In scope: the AC1 typed refusal and the AC4 planted red on both tiers, one named quota source per tier (AC2, GA part), and correct accounting for every write, update, failed write, erase and (per D-21-6-04) delete.
Out of scope:
- The **collective tier.** AC2's clause "the collective tier on the live substrate obeys the same rule" is parked under `v25-reza-postgres-legs` (`sprint-status.yaml:508`). Collective writes are not charged, and no collective or Postgres quota claim is made.
- **Any dependency on 21-2.** 21-2 is dropped (`epic-21…:10`).
- A new WIT/ABI memory call. Spawned and WASM Spirits have no memory wire at base (below).
- Kernel-side compaction, eviction or truncation.
- Process RSS limits. `[resources] memory_max_mb` is a different unit.
- The unit of 17-9 (capability issuance window) and the unit of 18-5 (context tokens).

## Inventory at base — source measurements, not execution

All source observations below were read at `ea8c51b6` (`/home/lunarpulse/dev_ws-ga-wasm`). The memory, domain and manifest files are byte-identical to the `dev_ws-course28` HEAD `de16fa0a`; the only difference between the two bases in the cited set is `xtask/kloc.toml` (the xtask and maos-bin rows plus the new maos-topology row). Planning cites were read in `dev_ws-course28`. No Cargo, build, test, gate, formatter, tokei or prototype was run.

| Surface | State at base | Preserve / required change |
|---|---|---|
| Quota today | None. Under `crates/maos-kernel-core/src/memory/`, the `limit` parameters are scan page sizes (`mod.rs:367,853`; `private.rs:1057`; `shared.rs:196`). The `i9_exempt` reasons nonetheless say "bounded by … per-Spirit memory budget" (`mod.rs:74-76`, `private.rs:58-60`): that budget does not exist. | Make the claim true or reword it. No scan-limit semantics change. |
| Existing per-write limits and errors | `MAX_KEY_LEN = 1024` (`private.rs:39`), enforced by `sanitize_key` → `MemoryError::KeyTooLong`/`InvalidKey` (`private.rs:123-146`), **private tier only**: the shared tier has no key check (`shared.rs:123-149`). `MemoryError::ValueTooLarge { len, max }` is declared (`maos-domain/src/memory.rs:304-305`) but **no code constructs it**. Other refusals the trait write path already returns, all before any store mutation: region (`mod.rs:720-724`, mapped to `Storage`), `PrincipalWriteUnauthorized` (`:728-737`, error `memory.rs:306-307`), `NamespaceViolation` for Principal outside private (`mod.rs:192-206,739`). | Run the quota check after these refusals, so a refused write never charges. Keep every existing error and add the typed `QuotaExceeded` variant (D-21-6-05 A). |
| Size measure | `MemoryValue::approximate_len` (`memory.rs:207-219`): serialized JSON length or the raw string/blob length. Its only caller today is the spill-threshold test (`private.rs:902-904`). Shared stores those same bytes (`shared.rs:76-85`). | Reuse it as the value-size function. Do not add a second size convention. |
| Port / API | `MemoryManagerPort` (`maos-domain/src/ports/memory.rs:18-102`) has `write/read/scan/subject_access/forget/export_redactable` and **no delete**. Methods carry `/// Class:` docs (`xtask/kernel-api-classes.toml:83`). `SpiritMemoryView::write` (`memory/for_spirit.rs:31-40`) binds the pid into each call. | Add the owned delete per D-21-6-04 A, with a class doc and as an additive change under ADR-010. |
| Private tier | `PrivateMemoryStore` (`private.rs:61-67`): in-memory map keyed by `(pid, namespace, key)`, plus durable spills under the stable owner directory (`owner_dirname`, `:110-117`; legacy pid directories for unbound pids, `:9`). `write` (`:890-942`) holds `io_lock`. Principal, Markdown and over-threshold values spill (`:902-904`, commit `:917` via `replace_spills` with rollback `:786-884`). Writing a value under the threshold deletes any spill for that key (`:918-919`). Markdown is removed from the map and served from disk (`:936-940`). Small non-Markdown, non-principal values live **only in memory** and do not survive a restart. Threshold is 4 KiB in production (`maos-bin/src/main.rs:2130-2133`). | Charge each logical key once, wherever it is stored. On replacement, charge only the net delta. Charge only after `replace_spills` commits and under the same `io_lock`. Never touch the durability/rollback sequence. |
| Principal writes | `MemoryManagerAdapter::write` → `principal_index.record_write(…, spill)` (`mod.rs:741-757`; `principal.rs:105-140`). The index records intent first, then calls the spill closure and repairs the index if the spill fails. It already keys rows by stable `writer_owner`. | A charge happens only when the closure succeeds. `writer_owner` is the precedent for D-21-6-03 A. |
| Shared tier | Host-wide SQLite table `shared_memory` with `PRIMARY KEY (writer_spirit_pid, namespace, key)` (`shared.rs:17-28`). `write` is `INSERT OR REPLACE` under the connection mutex (`:123-149`). Rows are keyed by **pid**, so after a daemon restart the same Spirit under a new pid **adds** rows instead of replacing them, and any per-pid figure resets. No delete path. | Attribution and restart behavior are D-21-6-03. Charge under the existing connection mutex. External readers must keep working: `maos-audit/src/lib.rs:2079-2121`; raw-SQL fixture insert with an explicit column list `maos-bin/tests/erasure_uninstall_13_5b.rs:236-242`; `maos-audit/tests/multi_backend_erasure_test.rs:79,107`. Any new column must be nullable or defaulted. |
| Erase | `forget_with_reason` (`mod.rs:448-617`) is serialized by `forget_serialization`. It erases private principal entries through `private.forget_principal` (`:569`, returns a count, not bytes; `private.rs:1131`) and index rows (`:570`). Legal-hold suspension (`:469-551`) erases nothing. | Credit the freed bytes/entries to **each affected owner**: a principal can span several writers. A suspended forget credits nothing. The receipt and journal stay unchanged. |
| Spirit-facing writers | No WIT/ABI memory call exists (`wit/spirit.wit` mentions memory only at `:163`, digest refs). Production writers found: butler principal port, private Markdown (`maos-bin/src/butler_root.rs:86-101`); in-process `KernelCtx::memory()` (`scheduler/kernel_ctx.rs:122-124`); the kernel-authored halt-context write into the Spirit's private tier (`halt/resolver.rs:168-181`). No production shared-tier writer was found. The shared tier is reached through the port: `memory_three_tier_smoke.rs:47`. | AC3's "refusal tells the Spirit" means the typed `MemoryError` these callers receive. The butler maps it to a `String` that includes the error text. Kernel-authored writes are D-21-6-06. |
| Identity binding | Admission binds a pid to its stable storage identity before memory access (`maos-bin/src/admission.rs:1455-1458` → `mod.rs:416-418`) and authorizes principal writes (`admission.rs:1593`). | The quota owner key follows this binding (D-21-6-03). No new admission gate is needed under D-21-6-02 A. |
| Quota-source precedents | Operator config sections are resolved env → `~/.config/maos/operator.toml` → defaults (`security/operator_config.rs:1-5`; sections `:13,:167,:224`). Manifest `[resources]` is `deny_unknown_fields` (`maos-manifest/src/manifest.rs:164-171`), resolved strictest-of with an operator floor (`:130-137`; `security/mod.rs:396-403`). That floor, `OperatorPolicyConfig::resource_cap_floor` (`capability/cap_policy/mod.rs:30`), is only set by a test (`maos-bin/src/worker_spawn.rs:1420`). Manifest schema version is 7 (`maos-spirit-abi/src/lib.rs:126`). | Use operator configuration alone (D-21-6-02 A); no manifest schema or ABI change for quota configuration. |
| Serde allowlist | `xtask/serde-error-allowlist.toml:418-424` pins `crates/maos-domain/src/memory.rs:407,408,545`. | A new `MemoryError` variant (above line 407) shifts these lines. Update the allowlist in the same change. |

Unqualified kernel file names are under `crates/maos-kernel-core/src/`.

## Decisions — existing authority and operator rulings

### Already settled; do not ask again

| Decision | Binding disposition |
|---|---|
| GA scope (SCP 2026-11-13 §2, `:50`; epic `:152`) | Private and shared tiers proven without Postgres. The collective leg goes to `v25-reza-postgres-legs` and is not a GA condition. |
| `ga3` (`decisions-2026-11-12-dedrift.md:28`) | 21-6 is a `v1.0.0` GA precondition. |
| `kfree` (ADR-069 amendment; `decisions-2026-11-12-dedrift.md`; `epic-21-preflight-2026-11-12.md:38`) | The epic's "measured by prototype (E16-A7)" (`:72,:141`) and the "◆ FLAG-Winston" marker keep their history but carry no ask. Growth inside the envelope is self-service, measured at landing. No prototype, grant or STOP exception. |
| D-17-9-PF-R3 A (2026-10-22) | FR28's quota clause belongs to this key, not to 21-4. |
| FR28 | Compaction is Spirit-authored. The kernel only refuses. |

### D-rows — operator rulings (2026-11-21 UTC preflight)

| ID | Material choice and bounded options | Recommendation and evidence | State |
|---|---|---|---|
| **D-21-6-01** | Unit and defaults, per Spirit identity, per tier. **A** bytes only (key bytes + `approximate_len`). Proposed defaults: private 64 MiB, shared 16 MiB. **B** bytes **and** entries. Approved defaults: private 64 MiB / 16,384 entries, shared 16 MiB / 4,096 entries. **C** entries only. All limits inclusive; overflow-checked arithmetic. | **B.** Each private spill is a file inside per-namespace directories (`private.rs:902-919`), so many tiny keys consume inodes and blocks that a logical byte count does not see. An entry cap bounds that. Neither PRD nor architecture gives a number: the limits were proposed and are now approved as operator policy defaults, **not workload measurements**. Existing related constants: 4 KiB inline threshold (`main.rs:2132`), 1,024-byte private key cap (`private.rs:39`). | Approved **B** (2026-11-21 UTC) |
| **D-21-6-02** | AC2 quota source. **A** operator policy only: a new `[memory_quota]` section resolved like the sections at `operator_config.rs:1-5` (env → `operator.toml` → built-in defaults), with one limit per tier applied to every Spirit. No manifest change. **B** a manifest `[resources]` field per tier, strictest-of with an operator floor (`manifest.rs:130-137`). Needs `MANIFEST_SCHEMA_VERSION` 7→8 because of `deny_unknown_fields` (`manifest.rs:164-171`), plus SDK/template/corpus migration. **C** kernel constants only. | **A.** The kernel sets the quota and a Spirit cannot raise it. This is the smallest surface for GA and adds no schema or ABI change for configuration. B's operator floor is not populated in production today (`worker_spawn.rs:1420` is the only setter), so B also needs A's operator plumbing. C gives operators no control. Any new `MAOS_*` variable is documented with the section (no env-registry file exists at base). | Approved **A** (2026-11-21 UTC) |
| **D-21-6-03** | Who usage is charged to, and how it survives a restart. **A** the stable storage identity on both tiers (`owner_dirname`, `private.rs:110-117`). Shared gains a nullable `writer_owner` column through an idempotent migration (same convention as `principal.rs:105-125`), and writes replace by `(writer_owner, namespace, key)`. Usage is rebuilt when an identity is bound: a private disk walk using the existing spill-name helpers (`private.rs:167-186`), plus a SQL sum over that owner's shared rows; in-memory-only private entries are counted for the life of the process. Legacy shared rows with no owner stay readable, are not counted, and are disclosed. **B** pid-scoped counters for each boot, with no schema change. A restarted Spirit gets a fresh quota and duplicates its shared rows (`shared.rs:25,136`). **C** a persisted counter table updated in the same transaction as each write. | **A.** SCP 2026-11-13 `:50` gives the GA reason as "an always-on Spirit with no quota fills the disk". B leaves that open across restarts. C adds a second source of truth that can drift from the stores. A reuses the identity binding admission already performs (`admission.rs:1455-1458`). Unbound pids are charged to their legacy pid-directory owner (`private.rs:116`). | Approved **A** (2026-11-21 UTC) |
| **D-21-6-04** | How a full Spirit reclaims space (epic AC1 "deletes … still succeed"; AC3 Spirit-authored compaction). **A** add a `delete(spirit_pid, tier, namespace, key)` port method (additive, `/// Class: data-movement`) and a `SpiritMemoryView::delete`. It credits usage. On the shared tier a Spirit can delete only its own rows. The collective arm returns the existing typed unavailable/denied errors. **B** no new API: space is reclaimed only by overwriting with a smaller value, and AC1's "deletes" is proven through the forget cascade. | **A.** The port has no delete (`ports/memory.rs:18-102`), and the only erase is the operator/subject forget cascade (`mod.rs:448-617`), which is not compaction. Under B a Spirit at an entry cap (D-01 B or C) can never free an entry, so B is consistent only with D-01 A. | Approved **A** (2026-11-21 UTC) |
| **D-21-6-05** | Typed refusal. **A** a new `MemoryError::QuotaExceeded { tier, resource: Bytes \| Entries, used, requested, limit }` in `maos-domain/src/memory.rs`, with the serde allowlist shift (`serde-error-allowlist.toml:418-424`). **B** reuse the unused `ValueTooLarge { len, max }` (`memory.rs:304-305`). | **A.** AC3 requires the refusal to name the full tier, and `ValueTooLarge` carries no tier or resource. No out-of-crate exhaustive `match` on `MemoryError` was found (the only out-of-crate construction is `maos-loom-lite/src/replication/router.rs:156`, a test). | Approved **A** (2026-11-21 UTC) |
| **D-21-6-06** | Writes the kernel makes into a Spirit's tier (the operator-provided halt context, `halt/resolver.rs:168-181`). **A** charge and refuse: resolving the halt fails with `ResolveError::Internal` when the Spirit is full. **B** charge but never refuse on quota: usage may exceed the limit, and the Spirit's next own write that grows usage is refused. **C** exempt: not charged. | **B.** Under A the operator's halt resolution depends on the Spirit's compaction. Under C the operator path becomes an untracked way to grow usage. B keeps the halt path unblocked while keeping the accounting true. | Approved **B** (2026-11-21 UTC) |

No kernel permission, surface-hash permission or STOP exception is requested. A final forecast above 22,000 would need an **ENVELOPE ALARM** D-row, and one above 23,000 a new operator decision (ADR-069). Neither is forecast.

## Acceptance Criteria

Numbering follows the epic. Each "per D-21-6-0x" binds to the operator's ruling on that row.

1. **AC1 (command) — a Spirit that writes past its quota is refused, on the private and the shared tier.** The kernel test target `crates/maos-kernel-core/tests/memory_tier_quota_21_6.rs` (named here) drives `MemoryManagerAdapter` through `for_spirit(pid)`.
   - A write that would take one Spirit identity's usage past the ruled limit (D-21-6-01) receives the typed refusal (D-21-6-05).
   - After that refusal, the same Spirit's reads, scans, deletes (D-21-6-04) and writes to the **other** tier still succeed, and another Spirit's writes to the same tier still succeed.
   - Accounting is part of AC1:
     - (a) A new key is charged its full size (and one entry).
     - (b) An update is charged only `new − old`. A write that does not increase usage succeeds even when the Spirit is at or over its limit.
     - (c) A failed or refused write charges nothing. This covers every earlier refusal (region, unauthorized principal, namespace, key), a spill failure with rollback (the existing `spill_test_faults` failure points, `memory/spill_test_faults.rs:23-31,61-74`), a principal-index spill-closure failure, and a shared SQL error.
     - (d) A forget cascade that erases credits each affected owner. A forget suspended by a legal hold credits nothing.
     - (e) A delete credits usage (D-21-6-04 A).
     - (f) The usage figure after a daemon restart follows D-21-6-03.
   - The check and the commit are atomic under the store's existing lock (`private.rs:66` `io_lock`; `shared.rs:35` connection mutex). Two concurrent writers cannot both pass a check that only one of them fits under.
   - Every boundary test is deterministic: exact `limit`, `limit + 1` and update-at-limit cases. No timing or throughput claims.
2. **AC2 — one named quota source per tier**, as ruled in D-21-6-02. It is documented where operators configure the host, and it is effective for every admitted Spirit from the moment its identity is bound. A Spirit cannot raise its own quota. *Collective-on-Postgres clause: parked (`v25-reza-postgres-legs`), not tested, not claimed.*
3. **AC3 — compaction stays Spirit-authored (FR28).**
   - After a refusal, every prior entry of that Spirit stays readable and byte-identical: the kernel never compacts, evicts or truncates.
   - The refusal names the tier and the exhausted resource (bytes or entries).
   - Charge the kernel-authored halt context to the Spirit without refusing it on quota grounds; the Spirit's next usage-growing write is refused while it would exceed a limit (D-21-6-06 B).
   - The `i9_exempt` reasons at `mod.rs:74-76` and `private.rs:58-60` describe the enforced quota truthfully.
4. **AC4 — proven red.** One plant on one image (`ubuntu-24.04`), in place, per `docs/testing/proof-contract.md:10-31`: removing the quota check reds AC1 on both tiers. The plant, its wall-clock and its restore are recorded under `_bmad-output/implementation-artifacts/21-6-memory-tier-quota-enforcement-evidence/`. A test that greens with the check deleted does not satisfy AC1.

## Tasks / Subtasks

- [ ] **T0 — apply rulings.** D-21-6-01…06 are recorded above. The operator approved all recommended options, so no option-driven forecast change is needed. Keep the existing sequencing after 21-9.
- [ ] **T1 — quota type and source (AC2).** Add the ruled `[memory_quota]` operator configuration as a typed section in `security/operator_config.rs` (D-02 A), with no manifest schema or ABI change for configuration. Pass it in at the composition root (`maos-bin/src/main.rs:2130-2137`, where the stores are assembled). Keep one source per tier, with no second default elsewhere.
- [ ] **T2 — ledger and check (AC1/AC3).**
  - Implement the usage ledger keyed by the D-03 owner, with a single check-and-charge used by `MemoryManagerAdapter::write` on private and shared after the existing refusals (`mod.rs:720-739`).
  - Charge only after commit: private after `replace_spills` or the map update under `io_lock`, inside the principal `record_write` closure; shared in the same connection lock as the insert.
  - Use checked arithmetic. Leave the collective arm uncharged.
- [ ] **T3 — reclaim and erase (AC1 d/e).**
  - Credit from the forget cascade for each affected owner. Return freed sizes from `forget_principal` rather than rescanning the whole tree.
  - Implement the ruled delete (D-04 A) on the port, the view, private (both the map and the spills, reusing the existing rollback discipline) and shared (own rows only), with `/// Class:` docs.
- [ ] **T4 — restart and attribution (AC1 f).** Under D-03 A: add the nullable `writer_owner` column and an idempotent migration in `SharedMemoryStore::open` (`shared.rs:41-50`), change shared writes to replace by owner, and rebuild usage at bind (`mod.rs:416-418`). Keep `maos-audit` readers and raw-SQL fixtures working (inventory row "Shared tier").
- [ ] **T5 — typed refusal (AC3).** Add the D-05 error, update `serde-error-allowlist.toml:418-424`, and check that the butler and resolver callers carry the typed text. Apply the D-06 behavior to `halt/resolver.rs:168-181`. Reword the two `i9_exempt` reasons.
- [ ] **T6 — proof (AC1/AC4).** Write the named target and tests below, run the one plant, and record it.
- [ ] **T7 — landing, orchestrator-owned.**
  - Measure with `check-kernel-baseline --json` and `kloc-check --json`.
  - Re-pin the kernel exact bytes, file set and hashes with `scripts/repin.py`, and write the HISTORY row (driver, files, before → after) **in the same commit**. Refresh any ADR-066 surface hash this change causes, under `kfree`.
  - Non-kernel ceilings are retrospective rows, not exact re-pins for each landing.
  - Run difficult-tier review (Blind/Edge/Acceptance on opus, Test Infrastructure on glm, `proof-contract.md:64-76`).
  - Only the orchestrator updates the tracker, the epic and deferred work.

## Proof plan — existing anchors and named new tests, no execution claimed

### Existing anchors to preserve (must stay green; not evidence for the new boundary)

| Boundary | Target / anchor |
|---|---|
| Tier mechanics | `crates/maos-kernel-core/tests/memory_three_tier_smoke.rs`: `write_read_private` (`:28`), `write_read_shared` (`:47`), `collective_returns_typed_error` (`:66`), `scan_private_returns_entries` (`:90`). |
| Cross-Spirit isolation | `memory_i5_isolation.rs`: `spirit_a_private_not_readable_by_spirit_b` (`:28`), `spirit_b_cannot_write_as_spirit_a` (`:51`). |
| Markdown spill | `memory_md_opaque_write.rs`: `markdown_spills_to_disk_and_reads_back_byte_identical` (`:28`). |
| Kernel-authored write | `halt_resolution_writes_memory.rs`: `provided_context_writes_memory_and_scalar_marker` (`:87`). |
| Erase counts and restart | `private_forget_restart_13_5i.rs`: `restart_forget_reports_distinct_persisted_entry_count` (`:168`), `forget_counts_cached_and_spilled_value_exactly_once` (`:198`), `forget_preserves_bystander_and_default_namespace_content` (`:222`). |
| Spill fault injection | `principal_identity_20_3b.rs:507,553` use `spill_test_faults::{arm_failure, arm_pause, disarm}`. |
| Shared-table readers | `maos-audit/tests/multi_backend_erasure_test.rs`, `maos-bin/tests/erasure_uninstall_13_5b.rs`. |

### New target `crates/maos-kernel-core/tests/memory_tier_quota_21_6.rs` (names fixed here; their bodies follow the rulings)

- `private_write_past_quota_is_refused_typed_and_other_ops_succeed` (AC1, AC3)
- `shared_write_past_quota_is_refused_typed_and_other_ops_succeed` (AC1, AC3)
- `update_charges_only_the_delta_and_shrinking_write_succeeds_when_full` (AC1 b)
- `refused_and_failed_writes_charge_nothing` (AC1 c; arms a spill failure point)
- `forget_credits_each_affected_owner_and_held_forget_credits_nothing` (AC1 d)
- `delete_credits_usage` (AC1 e; D-21-6-04 A)
- `usage_after_restart_follows_the_ruled_attribution` (AC1 f; D-21-6-03)
- `refusal_never_evicts_or_truncates_prior_entries` (AC3)
- `concurrent_writers_cannot_overshoot_the_limit` (AC1 atomicity; deterministic barrier, no sleeps)

Kernel unit tests, if any, run under libtest: `cargo test -p maos-kernel-core --lib` (`proof-contract.md:37-39`). The approved operator-config option adds no manifest change. The landing owes one final-tree matrix run and one planted-red run (`:12-20`). No fixture, cassette, Postgres service or hosted job is introduced or claimed.

## Kernel and kloc forecast — method, not a prototype

**Method:** read the routines this story touches (`memory/mod.rs` write/forget/bind, `private.rs` write/read/scan/forget and the spill helpers, `shared.rs` open/write, `halt/resolver.rs` context write, `security/operator_config.rs` section pattern) and price each addition. No source prototype, compiler output, gate output or tokei run was produced. All figures are `[INFERENCE]`; the forecast assumes the now-approved options.

Recorded base: kernel exact pin **26,061 physical** (`xtask/kernel-core-baseline.toml:505`), **20,428 tokei / 90 files** (`:507`). Kernel ceiling **20,429** (`xtask/kloc.toml:272`). These are recorded values, not a new measurement. Physical lines of `memory/` at `ea8c51b6`: 4,037 across 9 files (`wc -l`). That is not tokei; the preflight's 3,262 tokei (`epic-21-preflight-2026-11-12.md:236`) was not re-measured.

| Kernel area read | Net tokei | Basis / option dependency |
|---|---|---|
| Quota section in `security/operator_config.rs` (D-02 A) | +25…+50 | Follows the existing section pattern (`:13,:167,:224`). B moves this to maos-manifest and the ABI; C ≈ +5. |
| Ledger plus check/charge/credit (`memory/mod.rs` or a new `memory/quota.rs`) | +45…+90 | One owner-keyed table and one check-and-charge used by both tiers. A new file changes the kernel file set (91 files), which the re-pin captures. |
| `private.rs` prior-size lookup, post-commit charge, rebuild at bind | +45…+110 | The map or spill lookup reuses `select_disk_value` and the spill-name helpers. The rebuild walk is D-03 A. |
| `shared.rs` owner column, migration, delta and sum under the lock | +30…+80 | D-03 A. Under B ≈ +10…+25. |
| Forget credit | +10…+30 | `forget_principal` returns sizes for each owner. |
| Delete on private and shared plus the adapter arm (D-04 A) | +30…+70 | Under B: 0. |
| Halt-resolver charge without refusal (D-06 B) | +0…+10 | Under A or C: ±5. |
| **Total (approved options)** | **+185…+440** | Rounded scenario estimate, not a grant. |

Projected from 20,428: **20,613…20,868**. That is below the 22,000 alarm and the 23,000 envelope. 21-9, active on `ga-21-9-2026-11-20` from the same base, forecasts +87…+250 and also touches `memory/mod.rs`. Together the two reach at most ~21,118, still below the alarm. This is not a reservation: refresh against the actual landing base. The epic's +40…+120 (`:151`) priced only "one check per tier write" at `for_spirit.rs:31` and `write_entry_point.rs`. It did not include update/erase accounting, restart attribution, delete or the quota source, so copying it would understate the work.

Other production crates (tests and docs excluded): maos-domain +20…+60 (error variant, resource enum, port method docs; ceiling 10,483, `kloc.toml:436`); maos-bin +15…+60 (composition-root wiring, error text; ceiling 39,719, `:529`); maos-manifest 0 under D-02 A, +25…+70 under B (ceiling 4,889, `:426`); maos-audit 0…+10 if a reader needs the owner column (ceiling 8,487, `:420`).

## Cross-story dependencies and claim limits

- **No 21-2 dependency**, and no Postgres, nightly or hosted-services job is involved. The collective leg is parked under `v25-reza-postgres-legs` (`sprint-status.yaml:508`).
- **Sequencing unchanged:** 21-6 follows 21-9, which touches `memory/mod.rs` (promotion deauthorization next to `authorize_principal_writes`, `mod.rs:407-413`). The two stories own disjoint concerns. 21-10's invariant ledger may cite this quota. This story flips no invariant claim.
- **Not this story:** 17-9 (issuance window), 18-5 (context tokens), process RSS `memory_max_mb`, and a spawned/WASM memory wire (none exists at base; adding one would be new scope and a new D-row).
- **Claims:** no measured-workload basis for the default limits, no disk-usage guarantee beyond the logical bytes/entries ruled in D-21-6-01 (filesystem block and SQLite page overhead are not counted), and no collective or multi-host quota.

## Re-measured cite corrections — parent-owned source updates

Only this story file is written. These OLD → NEW corrections are a handoff to the owners of those files, not edits made here:

| Source premise | OLD → NEW at `ea8c51b6` |
|---|---|
| Epic §21-6 `:145` scan limits | `memory/mod.rs:363,894` → `:367,:853` (already noted at epic `:151`; still holds). Add `private.rs:1057`, `shared.rs:196`. |
| Epic §21-6 `:151` forecast basis | `memory/for_spirit.rs:31` holds (`write`). `write_entry_point.rs` is the region chokepoint (`enforce_region`, `:98`), not the quota seam. The real seams are `mod.rs:699-792`, `private.rs:890-942`, `shared.rs:123-149` and forget at `mod.rs:569`. Forecast +40…+120 → +185…+440 (above). |
| Epic §21-6 `:145` "no quota" premise | Holds, but the `i9_exempt` reasons at `mod.rs:74-76` and `private.rs:58-60` claim a "per-Spirit memory budget". Record that as a false bound this story corrects. |
| Epic §21-6 AC1 "deletes … still succeed" | No delete exists on `MemoryManagerPort` (`ports/memory.rs:18-102`). Now D-21-6-04. |
| Epic §21-6 `:72,:141` "measured by prototype (E16-A7)", ◆ FLAG-Winston | Superseded by `kfree` (epic `:151`; preflight `:38`). No prototype or ask. |
| SCP 2026-11-13 `:32` | `crates/maos-bin/src/escape_detector_consumer.rs:18-19` holds. |
| PRD / routing | `functional-requirements.md:74` and `requirements-inventory.md:556` hold. |
| Tracker | `sprint-status.yaml:418` (21-6) and `:508` (`v25-reza-postgres-legs`) hold, both in `dev_ws-course28` and at `ea8c51b6`. |

## Dev Agent Record

### Agent Model Used

Creation: **anthropic/claude-opus-5-5** (`DraftMemoryQuota`). No implementer or independent-review attribution is claimed.

### Completion Notes List

- Created this key's story file in `/home/lunarpulse/dev_ws-course28`, with source anchors read at `ea8c51b6` through read-only `git show`/`git grep`. Only this file was written. No epic, tracker, deferred-work or source edits, and no staging, commit or push.
- Read `bmad-create-story`, the 21-9 exemplar, epic §21-6 with its 2026-11-13 note, SCP 2026-11-13, PRD FR28, the routing row, the `kfree` ruling and the preflight, `docs/testing/proof-contract.md`, the memory module, the domain port and errors, the composition, admission and resolver seams, the manifest and operator-config precedents, and the checked-in kernel and kloc rows. `_bmad/scripts/resolve_customization.py`, `_bmad/bmm/config.yaml` and `_bmad/custom/` are absent in this checkout, so skill defaults were applied directly. The sprint-status update step is not performed: the tracker is parent-owned.
- No builds, Cargo, tests, gates, formatters, tokei, remote jobs or prototypes were run. No green, red-before, kernel exact delta or measured default is claimed. `intent-lineage-coverage-report.md` was not touched.
- At creation, the story stayed `backlog` with D-21-6-01…06 open. The operator rulings are now recorded above; status remains `backlog`.

### File List

- `_bmad-output/implementation-artifacts/21-6-memory-tier-quota-enforcement.md` — new, measured story only.

## §A6 independent review artifact — 2026-11-21 (GLM-5.3-flash, static)

**Review scope.** GLM-5.3-flash is from a non-Claude model family and was independent of the author of the Claude-authored implementation. The review was read-only and static; the reviewer ran no builds or tests. The orchestrator ran the suites listed below locally afterward. There are no hosted receipts. D-E20-6 B security review is satisfied for this static pass only. Under the standing no-more-review-rounds policy, the unresolved items below are recorded deferrals, not closed findings.

**Implementation on branch.** The private and shared tiers enforce separate per-owner byte and entry quotas via operator `[memory_quota]` configuration and `MAOS_MEMORY_QUOTA_*` environment overrides. Refusals use `MemoryError::QuotaExceeded`. Delete is available on the memory port; Spirit-facing delete refuses Principal namespaces because the forget cascade owns them. Forget credits freed bytes to the affected owners. Kernel-authored halt writes charge usage but are never refused on quota grounds (D-21-6-06 B). The quota block lives in `memory/mod.rs`; no new kernel file was added.

| Finding | Fixed after review |
|---|---|
| H1 | Shared rows are replaced by the unique `(owner, namespace, key)` identity, preserving legacy ownerless rows. A reused pid can no longer erase another Spirit's row. |
| H2 | The private size cache is keyed by stable owner, so two pids bound to the same owner share one size source. |
| H3 | Partial forget retains inline charges and credits only spills actually removed. |
| H4 | Injected commit and rename faults roll back both the value and its charge. |
| M1 | A principal index refresh failure no longer turns a committed write into an error. |
| M2 | Restart rebuild uses the same newest-version rule as reads. The dedicated test remains open. |
| M3 | Usage growth is refused while any total is over its limit. |
| M4 | Malformed `operator.toml` and non-Unicode environment values now warn instead of silently defaulting. |
| L1 | Usage totals use checked arithmetic. |

**Recorded deferrals and open evidence.**

- Pre-existing finding, not introduced by 21-6: production halt journaling persists the full `ProvidedContext` text in approval-log `reasoning` (`operator_door.rs`, `transparency_log.rs`, `halt/mod.rs`). It remains open.
- M2 has no dedicated test. The shared usage query has no index on `writer_owner` and scans on each shared write.
- The AC4 planted-red run was not performed, and its evidence directory has no AC4 execution evidence. The collective-on-Postgres leg stays parked under `v25-reza-postgres-legs`.

**Orchestrator-run local results.** `memory_tier_quota_21_6` all green; `maos-kernel-core --lib` green; `forget_cascade_18_6` 22/22; `maos-domain` green.

**Measured size and baselines.** After formatting, kernel production lines measured **20,949**; the orchestrator raised the ceiling from **20,429** by adjudication. After the review fixes, growth exceeded the story's **+185…+440** forecast by about **80 lines**. `maos-bin` measured **39,904 ≤ 39,909**. The kernel pin was refreshed via `scripts/repin.py`. Kernel-surface baseline hashes for `MemoryManagerAdapter`, `PrivateMemoryStore`, and `SharedMemoryStore` were refreshed; the ADR-066 surface refresh is not ratified.

**Merge held — operator ruling required.** Combined with 21-9 (**21,631**) and 21-12 (**20,517** on its own base), the kernel is projected to reach about **22,239**. That crosses the ADR-069 **22,000** alarm but not the **23,000** envelope. The 21-6 merge is **held** pending the operator's choice: **A** accept within the envelope, **B** trim, or **C** defer.

## Change Log

- 2026-11-20: created at `ea8c51b6`. Recorded the exact private/shared GA scope (collective parked, no 21-2 dependency), base inventory of the limits, errors, stores and accounting seams, the testable AC1–AC4 contract with named tests, six open D-rows, the kernel and kloc forecasts with their method, and the parent-owned cite corrections. Status: backlog pending rulings.
- 2026-11-21: recorded preflight operator approvals D-21-6-01…06 (**B / A / A / A / A / B**, UTC). Approved the proposed defaults of private 64 MiB / 16,384 entries and shared 16 MiB / 4,096 entries as policy, not workload measurements. Confirmed operator-only configuration without a manifest schema or ABI change, stable-owner attribution with the shared-store migration, owned delete, typed quota refusal, and charging kernel halt context without refusing it on quota grounds. Private/shared scope without Postgres, `backlog` status, unchecked tasks and sequencing after 21-9 are unchanged; no implementation, landing or shipping authorization is recorded.
- 2026-11-21: implementation landed on the branch with local evidence only; story status is `in-progress`. The merge is held pending the operator ruling on the ADR-069 alarm.
