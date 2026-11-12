# 20-9 L2 landing plan (Dev209K) — STEP 0 prep, priced on live HEAD 998b36e6, /tmp only

Instrument: `git archive HEAD` copy (`/tmp/dev209k/base`, touched), rustfmt (edition 2021) on the touched files, prebuilt
`/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline`, `kloc-check`, `check-service-boundary`, plus
`check-serde-error-handling`, `check-pub-field-constructors`, `check-empty-kernel`, `invariant-lock --changed-files`), no cargo.
HEAD reproduces the pins exactly (kernel 27,044 phys / 21,202 tokei, 95 files, set hash `d7d3df11…`; iac 8771; audit 8308; aggregate 202,917, hardfail 202,918;
bin 38,563, xtask 45,355, domain 10,274, capability 1,454, cli 6,612). Diffs: `/tmp/dev209k/protos/*.diff` (apply from repo root with `patch -p1`).
Measurement tooling: `/tmp/dev209k/measure.sh <label>`; raw json `/tmp/dev209k/{ckb,kloc,sb}-<label>.json`.

## 1. Row R re-cut on the live base (measured)

R = D-20-9-13 A + D-20-9-14 A (the story's `r-kd07a-lean-plus-ko-poll-outcome-in-iac.diff`). The prototype's hunk 5 no longer applies (20-11 added the
`fence` guard around the cadence stamp); re-cut by hand: the Fired outcome is pushed after the fence-guarded stamp, the Suppressed arm stamps `last_fire_ns`
under the still-held `fence` (the candidate's `fence` guard is alive until `continue`).

**Re-cut finding (prototype gap, NOT in the FLAG record's pricing):** the FR4 inventory doorbell
(`crates/maos-audit/tests/fr4_classifier_16_2.rs` `scan_tree`, roots `maos-kernel-core/src`, `maos-bin/src`, `maos-iac/src`) keys writer sites by
`(file, enclosing fn, ordinal)` in `crates/maos-audit/src/fr4_classifier.rs` WRITER_SHAPES. R (a) renames the site
`("schedule_watchdog.rs","check_and_fire",0)` -> `poll_once`, and (b) adds a NEW tokenless kind-7 writer in maos-iac
(`("payload.rs","journal_schedule_suppressed",0)`), which has no disposition -> the doorbell reds and FR4 would project the row as an unmediated call.
The recut therefore adds one WriterShapeEntry (+ renames one string) in `maos-audit` and makes `ScheduleSuppressedRecord.refused_scope` an
`Option<String>` (Debug-formatted scope; an `Option<Scope>` JSON object needs a new `PayloadType` variant = more maos-audit lines). `kind 28` governance
rows are not an escape (they must decode as `GovernanceEventPayload`, maos-domain). maos-audit is NOT in the ruled `kloc_grant` ("every other crate 0"):
**needs an operator kloc grant for maos-audit (+12 tokei; x2 ceiling 8332)** in addition to the kernel row.

| Item | Kernel phys | Kernel tokei | Other crates | Surface (check-service-boundary) |
|---|---:|---:|---|---|
| **Row R (re-cut, with FR4 entry)** | **+16 -> 27,060** | **+21 -> 21,223** | iac 8771 -> 8810 (+39); **maos-audit 8308 -> 8320 (+12)**; aggregate 202,917 -> 202,989 (+72, hardfail 202,990) | passed, 0 violations, all 302 item hashes identical to HEAD (hash-neutral: `poll_once` is an inherent method; no named symbol moves) |
| bin 38,563 / xtask 45,355 / domain 10,274 / capability 1,454 / cli 6,612 | 0 | 0 | unchanged vs pins | — |

R against the story's creation figure: kernel +16/+21 exactly as priced; iac +39 as priced; maos-audit +12 is new.
Cumulative kernel tokei after R: 21,223 (STOP 22,000: 777 below). No surface hash or removal moves; nothing outside `schedule_watchdog.rs` in the kernel.
clippy preflight by reading: `Result<Option<TokenId>, (Option<String>, String)>` is 48 bytes (< 128, no result_large_err); `journal_schedule_suppressed` takes 6 args (< 8).

x2 ceilings re-based on the live pins (ceilings, not reservations; pins land exact, hardfail = measured + 1):
kernel <= 27,076 phys / 21,244 tokei; maos-iac <= 8,849; maos-audit <= 8,332 (needs the new grant); aggregate <= 203,061 (hardfail at landing = measured + 1).

## 2. Options priced separately (each on top of R; each needs its own measured operator grant)

| Opt | What (exact definition from the deferred row + review text) | Files | Kernel delta over R (phys / tokei) | Cumulative tokei with R | Surface | Other crates |
|---|---|---|---|---:|---|---|
| **(a) F2 coordinator `expect` window** (GLM r3 F2, deferred row `:1665`) | replace `map.get(&predecessor_pid).expect("successor SCB just inserted")` (`coordinator.rs:355-361`) with a `let Some(..) else { saga.compensate(DiscardSuccessor{..}).await; return Err(HotSwapError::NotLoaded{spirit_id}) }` (the row's "NotLoaded-style error with compensation"). Cheaper variant a-min: delete the lookup and use `&predecessor_scb` (the map holds the SAME Arc; step 8 replaced only its interior) = kernel **-5 / -7** but no typed refusal (swap-in runs on the unloaded SCB), so NOT recommended as the F2 closure. | `hot_swap/coordinator.rs` | **a1: +12 / +10** (a-min: -5 / -7) | 21,233 (a-min 21,216) | hash-neutral (passed) | 0. The row's second half (a swap-in-flight marker the session teardown honours; kill the `Handover` on teardown-before-adoption) is bin code (`spirit_session.rs`/`spawned_lifecycle.rs`), kernel-free but needs a bin kloc grant [INFERENCE +30..60] |
| **(b) F3 `archive.rs` atomic record** (GLM r3 F3, row `:1666`) | `SpiritArchive::write` uses `write_atomic` (tmp in same dir, `sync_all`, `rename`, fsync of the directory) for `manifest.toml` and `snapshot.cbor` instead of `fs::write`. Fixes the "death mid-write leaves a partial manifest.toml that write-if-absent never repairs" half only. The two-sided order (provisional record before `start`, or a journaled intent read by `refuse_without_migrator`) is an admission-gate rewrite in bin and stays deferred. | `hot_swap/archive.rs` | **+14 / +11** | 21,234 | hash-neutral (passed) | 0 (bin side of the two-sided order not priced) |
| **(c) idle-watchdog sequential fire** (GLM r4 adjacent note, row `:1654`) | `IdleWatchdog::check_and_fire` awaits `fire_on_idle` one Spirit at a time. Fix: a `tokio::task::JoinSet`, one task per due Spirit (each clones `Arc<HookDispatcher>`), tick awaits `join_next` until empty (no new field, no signature change; `futures` is not a kernel dep). Detached per-Spirit tasks are NOT used: `main.rs:6124-6129` relies on the watchdog task awaiting every in-flight `on_idle` before unload (halt receipts). Residual: the TICK still waits for the slowest Spirit, so a stalled Spirit delays the NEXT poll for all (bounded by its cap), but no longer starves the other Spirits inside the same poll. | `scheduler/idle_watchdog.rs` | **+9 / +5** | 21,228 | hash-neutral (passed) | 0 |
| (d, not asked, FYI) `authorize_principal_writes` release (row `:1677`) | `pub fn deauthorize_principal_writes(pid)` in `memory/mod.rs` (+ bin callers in rollback/teardown) | `memory/mod.rs` | +9 / +7 | 21,230 | hash-neutral (passed) | bin callers |
| **a1 + b + c1 together** | | | **+35 / +26** (measured jointly 27,095 / 21,249) | **21,249** (751 below STOP) | hash-neutral | — |

Schedule watchdog has the same await-one-hook-at-a-time shape (`fire_on_schedule` after the fence drop); the deferred row names only idle, so it is unpriced.

Risk if each option is left deferred (all confined to the target tenant or availability; the story's severity policy keeps Low/Info as rule-10 rows):
- (a): the `expect` panics the upgrade task when a clean guest exit unloads the pid in the (microsecond, on a multi-thread runtime) gap between `replace_runtime` and the lookup; if the unload lands later the swap reports `Completed` and the Spirit vanishes behind a monitor over a dead pid. Operator-visible integrity failure of one tenant's upgrade, MEDIUM in the review. a1 removes the panic and refuses typed; the "completed then vanishes" outcome needs the bin-side marker.
- (b): a daemon death inside a ~20 ms window leaves a started cross-major version unrecorded (fail-open downgrade) or a partial `manifest.toml` (permanent spurious `migrator_missing` for that spirit until manual repair). Needs a daemon death in the window. b closes the partial-file half only.
- (c): one spawned Spirit with a silent guest and a long `[budget].time_cap_seconds` delays every other Spirit's idle trigger for that cap (availability, same-root DoS by an admitted component). c1 fixes in-poll starvation, not next-poll delay.

Witness plan per option (failing-before = real red at HEAD with raw logs; plants restored by copy-back + cmp + touch):
- (a) `crates/maos-kernel-core/tests/hot_swap_cross_major_migration.rs` (new vector): a CrossMajor swap whose successor `on_migrate` hook removes the pid from the shared `scbs` map (the migrator runs between predecessor snapshot and step 8). HEAD: the upgrade future panics at `coordinator.rs:359` (observed red, task panic). After: `Err(HotSwapError::NotLoaded)`, a `DiscardSuccessor` saga compensation row in the journal/TL, no panic. Plant: put the `expect` back / drop the compensation call -> red.
- (b) unit vectors in `archive.rs` tests: (1) hard-link `manifest.toml`, overwrite via `write`: HEAD mutates the shared inode (old link sees the new bytes, in-place truncate+write), after the old link keeps the old bytes and the path has the new bytes; (2) a stale `manifest.tmp` left by a crash does not block a write. Plus `strace -f -e trace=openat,fsync,rename,renameat2` log of the vector on HEAD (no fsync/rename) vs after (fsync file, rename, fsync dir) as the fsync evidence. Plant: `write_atomic` writes the final path directly -> vector (1) reds.
- (c) `crates/maos-kernel-core/tests/on_idle_substrate.rs` (new vector): two Running Spirits, the lower pid's `on_idle` blocks until the higher pid's `on_idle` has fired (std channel, 5 s guard); HEAD: sequential, A waits out its guard before B fires -> assertion "B fired while A was held" reds. After green. Plant: await sequentially inside the JoinSet loop -> red.

## 3. Landing plan (after Main's ruling on options and `CARGO HANDOFF 209-L2`)

Order (one cargo per checkout, `env -u CI -u GITHUB_ACTIONS`, inherit `target`):
0. Re-measure on the live tree (Dev2014R2 may have moved the pins: its working tree currently touches `crates/maos-iac/src/adapter/transparency_log.rs`, `xtask/kloc.toml`, `operator_door.rs`, tests). Re-base ceilings and iac/aggregate on the landed pins; STOP + tell Main if any surface hash/removal moves or any figure passes its ceiling.
1. Red-before at HEAD with raw logs (`20-9-spawned-form-lifecycle-hooks-evidence/landing-2/`):
   - `schedule_17_9_exhausted_quota_records_fire_without_minting` inverted per AC8.2 (counter 0, no `schedule.fire:` row, one `schedule.suppressed:quota-exhausted` row naming the refused scope, quota evidence row kept) -> real assertion red against the HEAD kernel (spawn API, no new symbol needed).
   - new partial-mint vector (two scopes, second refused by policy): one `Revoke` cap-audit event for the first token, no live token for the pid, no fire row, one suppressed row -> red at HEAD (FW-10 fires with the first token live).
   - registry-absent-with-declared-scopes vector (fire suppressed, row `refused_scope: null`).
   - `poll_once` vectors (AC7.2): `[Fired]` then `[RateRefused]` on a `rate=1` entry, `[PrincipalRevoked]` with 0 fires, `[Suppressed]` for a refused mint, `[]` for not-due. At HEAD these are E0599 (no `poll_once`) -> raw compile-red log. NOTE (deviation to rule on): `monotonic_now_ns` is `Instant`-based and a fire stamps the cadence, so the second poll returns `[]` until the (fast-mode, >= 1 ms) cadence window elapses; the vector calls `poll_once(true)` in a bounded `yield_now` loop until a non-empty result and asserts only on the outcome sequence — no `sleep`, no TL-row wait, no timing assertion (AC7.2 says "no sleep").
   - FR4: `fr4_classifier_16_2.rs` doorbell red at HEAD+prototype without the entry (observed via the scratch step), vector row for `schedule.suppressed:<id>` added next to `schedule.fire:daily-9`.
   - iac unit round-trip for `ScheduleSuppressedRecord` (+ `PollOutcome` derives).
2. Implement R (`schedule_watchdog.rs`, `maos-iac/src/adapter/payload.rs`, `maos-audit/src/fr4_classifier.rs`) + granted options. No unwrap/expect after serde (the journal fn uses `if let Ok`; on serialize failure it should `eprintln!` rather than drop silently — +1 tokei, re-measure); hash-ish text in TL-redacted fields stays colon-delimited hex (the suppressed `reason` is the registry error text; assert no 64-hex run in the vector).
3. Plants (observed red, restored by copy-back + cmp + touch): AC7 (RateRefused arm pushes nothing), AC8 (fall through to the fire), partial-mint (skip the revoke), FR4 entry removed (doorbell red), plus one per granted option (section 2).
4. `cargo fmt --all`, then exact re-pin (every number measured, never forecast):
   - `xtask/kernel-core-baseline.toml`: `src_lines` (line 505) 27044 -> measured with a new HISTORY block citing the live story key with `->` (`20-9-spawned-form-lifecycle-hooks -> landing 2 …`, D11 A); `[kernel_src]` set_hash (:1089) and the per-file digest(s) (`scheduler/schedule_watchdog.rs` :1155, plus `hot_swap/coordinator.rs`, `hot_swap/archive.rs`, `scheduler/idle_watchdog.rs` only if granted), all from `check-kernel-baseline --emit-pin`.
   - `xtask/kloc.toml`: `maos-kernel-core` (:264), `maos-iac` (:406), `maos-audit` (:409, if granted), `_aggregate_hardfail` (:718) = measured aggregate + 1, each with a HISTORY segment whose first segment cites a live story key with `->`.
   - `xtask/tests/kernel_pin_content_hash_16_0.rs` (`27044` at :820, :838, :847, :858), `xtask/tests/recovery_lane_ceiling_rule.rs` (`RATIFIED_AT_EASING` 21_202 -> measured + comment block).
   - Never edit `docs/invariants/*.md`.
5. Gates (`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=$PWD/xtask /mnt/build/cargo-maos/debug/xtask <gate> --json`; JSON + .err kept under `landing-2/gates/`): check-kernel-baseline, kloc-check, check-service-boundary (hash-neutral expected; any move -> STOP), check-serde-error-handling, check-pub-field-constructors, check-empty-kernel, check-fkcs, check-dependency-closure, check-composition-root-completeness, check-abi-ratification, check-env-contract, check-decision-register, check-exit-commands, check-epic-close-coherence, check-dev-record-completeness, check-dev-model-tier, `invariant-lock --changed-files` for every touched kernel file, gen-abi-docs --check, stability-matrix.
6. Focused suites (accepted process, NO full workspace nextest): `cargo nextest run --locked -p maos-kernel-core --test schedule_watchdog_fr26 --test schedule_forget_spirit_20_11 --test on_idle_substrate --test halt_carry_20_3c --test hot_swap_cross_major_migration --test lifecycle_all_hooks_hot_swap_17_4 --test routed_halt_swap_20_3b` (+ `--features fixture_replay` run of `schedule_watchdog_fr26`, the hosted job's shape), `cargo test --locked -p maos-kernel-core --lib`, `-p maos-iac`, `-p maos-audit --test fr4_classifier_16_2`, `-p maos-bin --features maos-bin/wasm-host --test spawned_lifecycle_20_9 --test spawned_swap_20_9 --test spawned_door_20_9` (AC3 schedule vectors are scope-free so unaffected; no bin test relies on the untokened fire: grep 0), `-p xtask` tests `kernel_pin_content_hash_16_0` and `recovery_lane_ceiling_rule`. Clippy (hosted lints: result_large_err, collapsible_if, too_many_arguments, unused_mut, trim_split_whitespace) + ratchet over the touched crates.
7. Records (AC9): `20-9-flag-winston-decision.md` landed figures (+ the maos-audit/FR4 finding and any option grants); story Dev Agent Record + status; `tests/coverage-matrix.yaml` FR26 (:574) names AC3/AC7/AC8, FR55 (:879) AC1/AC2, FR10 (:357, `gates: []`) AC4; AC9.1 asks `docs/invariants/I14.md` "Routed FR53 witness" (:34) to name the spawned swap vector, but the standing rule is never to edit `docs/invariants/*.md`: I will not touch it and will ask Main/operator (claim stays `asserted`); docs: ADR-013 (~:98-100), ADR-030 (:50), `docs-site/docs/manifest/v5.md:97` (+ ko), cookbook `lifecycle-hooks.md:142` interim sentence (+ ko) -> say what D07 A does; `gen-abi-docs --check`; deferred-work: close `:1355` (PF-04), `:1371` (untokened fire), `:1499`/AC3 reference, `:1638` (interim) with `CLOSED <date> by 20-9 @<commit>` + witness, and per option `:1654`/`:1665`/`:1666` (closed or narrowed); AC6/`:1633`/`:1636` stay hosted; the remaining 20-9-owned rows (`:1623`/`:1634` promotion-set swap kernel side, `:1635`, `:1637`, `:1643`-`:1646`, `:1648`, `:1655`-`:1658`, `:1667`-`:1669`, `:1676`-`:1681`, `:1709`, `:1712`) stay open with the owner line moved to an operator re-owner or dispositioned (Low/Info -> rule-10 deferred rows), none of them kernel in this landing except the optional (a)/(b)/(c)/(d).
8. Cleanup: delete `/tmp/dev209k` scratch (keep nothing the repo needs), granular incremental prune of `/mnt/build/cargo-maos` (not newest per crate, not touched within 3 h, non-blocking lock probe, never deps/build/fingerprint), report freed bytes.

## 4. Open decisions for Main/operator

1. maos-audit kloc grant (+12; x2 8332) for the FR4 WriterShapeEntry R needs (prototype gap), or rule `Option<Scope>`/other shapes (more lines).
2. Options (a)/(b)/(c) (and (d)): each a separate measured grant; joint a1+b+c1 = +26 tokei / +35 phys -> 21,249 (751 under STOP 22,000). a-min is the -7 alternative to a1.
3. A bin kloc grant if the bin-side halves of (a)/(b) (swap-in-flight marker, two-sided archive order) are to land; they are kernel-free.
4. AC7.2 "no sleep" wording vs the bounded `yield_now` retry loop (cadence window >= 1 ms).
