# 20-9 creation — cites re-measured at `8c809f9c` (2026-11-03)

Method: every cite was re-read in a `git archive 8c809f9c` copy (`/tmp/proto209/base`, deleted after use), because the
working tree carries Dev204L1's uncommitted 20-4 L1 edits (`admission.rs`, `main.rs`, …). Planning-record lines are the
working-tree lines at creation; the 20-9 rows in E20, PF, RA, RI, S and D are at the same lines as at HEAD. No cargo ran.
Abbreviations: E20 = `planning-artifacts/epics/epic-20-ship-it-w5.md`, PF = `epic-20-preflight-2026-10-30.md`,
RA = `routing-audit-2026-10-22.md`, RI = `epics/requirements-inventory.md`, S = `sprint-status.yaml`, D = `deferred-work.md`.

## C — charter cites (OLD → NEW)

| # | Where cited | OLD | Fact at `8c809f9c` | NEW |
|---|---|---|---|---|
| C1 | E20:268 (AC1), PF:244 | `crates/maos-bin/src/main.rs:3388-3402` "successor factory refuses spawned forms" | `:3381-3416` are the factory's comments and moved clones. The refusal is inside the closure `:3417-3461`, at `:3423-3432` (`SpawnedSurfaceUnsupported { surface: "hot-swap successor" }`) | `main.rs:3423-3432` |
| C2 | S:309 | `main.rs:3331-3340 @61af7461` | the same refusal, at a historical commit | `main.rs:3423-3432 @8c809f9c` |
| C3 | E20:269 (AC2) | `wit/spirit.wit:386-390` (handle-frame/on-start/on-shutdown) | accurate: exports `:386`, `:389`, `:390`; package `maos:spirit@2.2.0` `:30`; world `:378-391` | unchanged |
| C4 | E20:270, :367, :368 | `schedule_watchdog.rs:199-221,230,331-342`; `:199` revoked, `:212` bucket, `:216-221,331-342` | accurate: revoked skip `:199-201`, bucket refusal `:212-214`, mint comment `:216-221`, `side_effect_token_id` `:230`, refused-mint arm `:331-342`; `check_and_fire` private `:158-268` | unchanged |
| C5 | E20:270, :367 | `deferred-work.md:1193,1204` | `:1193` is a blank line in the 17-7 creation section; `:1204` is a 20-3a loom sub-bullet. The PF-04 row is `:1260`; the untokened-fire row is `:1275` (same at HEAD and in the working tree) | `deferred-work.md:1260,1275` |
| C6 | E20:367 | `schedule_watchdog_fr26.rs:505-542` | `:505-542` are the `Registry` helpers; the refused-fire vector `schedule_17_9_refused_fire_mints_nothing` is `:537-578`. The FW-10 vector D07 A inverts is `schedule_17_9_exhausted_quota_records_fire_without_minting` `:656-716` | `:537-578` (+ `:656-716`) |
| C7 | E20:369 | `first_id` `schedule_watchdog.rs:326-342`; fire row `:225-232` | `first_id` declared `:313`, set `:326-330`, returned `:345`; fire row `:225-232` accurate | `:313-345`; `:225-232` |
| C8 | RA:23, RA:182 | `17-4…md:51,135` "spawned forms receive neither new hook" | `:53` (inventory row: "Neither hook reaches spawned forms"), `:71` (non-goal), `:135` (docs), `:341` (closure limit) | `17-4-hooks-observer-and-activity-corpus.md:53,71,135,341` |
| C9 | PF:83 | `S:306; E20:271-276` | the 20-9 tracker row is `S:309`; the E20 §20-9 section is `:264-270` and its received items `:362-370` | `S:309; E20:264-270,362-370` |
| C10 | PF:127 | `S:306`; `19-5…md:183-188` | tracker `S:309`; 19-5 AC6 "spawned forms are labelled, not swapped" is `19-5-cross-form-upgrade-and-cold-swap.md:184-188` | `S:309`; `19-5…md:184-188` |
| C11 | PF:291 (D07) | `E20:276; watchdog :216-230,331-342; FR:65` | E20 D07 bullets are `:368-369`; watchdog anchors accurate; FR26 is `functional-requirements.md:65` (accurate) | `E20:368-369` |

## F — facts measured at creation (no prior cite)

| # | Fact | Evidence at `8c809f9c` | Consequence |
|---|---|---|---|
| F1 | Spawned SCBs already receive kernel hook calls; they are silent no-ops. `WorkerSpirit` implements only `on_unload` | `crates/maos-bin/src/supervision.rs:2612-2633`; `prepare_spawned` builds it `:1710-1730`; trait defaults `crates/maos-spirit-abi/src/lifecycle.rs:233-296` | The FR55 path for spawned forms is `WorkerSpirit` forwarding, kernel-free; the kernel's enabled-hooks gate and budget timeout stay as they are (`hook_dispatch.rs:174-184`, `:356-363`) |
| F2 | `IdleWatchdog` is form-agnostic | `crates/maos-kernel-core/src/scheduler/idle_watchdog.rs:65-90` (every Running SCB past `idle_window_ms`) | `on_idle` reaches a spawned pid once `WorkerSpirit::on_idle` forwards |
| F3 | The 17-4 callers exclude spawned forms at registration | `crates/maos-bin/src/admission.rs:960-968` (`require_in_process("hooks")`); Observer-class eligibility `hook_pump.rs:305`; ConsolidationWatchdog `:1018-1050` | consolidation and telemetry need a registration rule for spawned pids (D-20-9-07) |
| F4 | **`[[schedule]]` is never carried at admission, for any form.** `gate_manifest` never reads it; `GatedManifest::bundle()` leaves `schedules` at `Default` | `admission.rs:471-484` (bundle), `:590-745` (gate); the kernel's successor parser does carry it (`crates/maos-kernel-core/src/lifecycle/upgrade.rs:388-400`); no shipped manifest declares `[[schedule]]` (grep of `spirits/`, `examples/`, `templates/`, `crates/*/tests`, `tests/`: 0) | the production `ScheduleWatchdog` (`main.rs:9716-9727`) fires for nothing an operator admitted; it fires only after a hot swap. Rule 11: declared-not-carried is false today (D-20-9-08) |
| F5 | The WASM runner calls only `on-start`, `handle-frame`, `on-shutdown`; bindings are selected per import version | `crates/maos-wasm-host/src/runner.rs:172-332` (`run_guest!` `:257-320`, legacy selection `:321-331`), import major check `:359-373`, frame pump `:384-450`; `wit_guest.rs:1-20` | lifecycle exports need a binding selected by export presence (20-8 §Contract for 20-9 point 1) |
| F6 | Spawned per-turn leases are revoked at turn end; session finalize revokes every pid token | `crates/maos-bin/src/spirit_session.rs:445-447` (TurnComplete → `revoke`), `:523-527` (`revoke_all_for_pid`, `context_budget().remove`) | a spawned swap must retire the predecessor child without the finalize closure, and leaves turn leases to their own turn (D-20-9-10) |
| F7 | Hot swap keeps the pid and the SCB `Arc`; tokens and working memory are pid-keyed | `crates/maos-kernel-core/src/hot_swap/coordinator.rs:120-455` (steps 1-12; `replace_runtime` `:301-319`); the factory runs before the coordinator (`lifecycle/upgrade.rs:82-123`) | no kernel change is needed to keep tokens or working memory across a spawned swap |
| F8 | Post-swap auto-revert reinstalls the predecessor runtime object within a 30 s window (300 ms under `MAOS_AUTO_REVERT_FAST=1`) | `coordinator.rs:510-571`; `post_swap_monitor.rs:39-58`; no public window query | a retired predecessor child cannot be resumed by a revert without parking it (D-20-9-11) |
| F9 | Door cold swap of a spawned form refuses before unload | `prepare_spirit` → `require_in_process("operator door")` `main.rs:10198-10212`; `cold_swap` preflight `operator_door.rs:1415-1436` (`cold_swap_unrestorable`) | spawned cold swap needs the spawned admission path at the door (D-20-9-12) |
| F10 | `[cli_wrapper]` Worker refusal | `operator_door.rs:1190-1199` (`409 cli_wrapper_is_not_a_spirit_form`) | kept unchanged (epic AC1) |
| F11 | Kernel public surface is pinned | `docs/ci-baselines/kernel-surface-v0.1-beta.json` (401 items; `ScheduleWatchdog` struct signature hashed); `xtask/kernel-api-classes.toml` | measured: a new kernel `pub enum` needs a classes row; a new field on `ScheduleWatchdog` is a removed+new symbol (baseline re-issue) — `measurements.md` |
| F12 | 20-11's granted KS (`ScheduleWatchdog::forget_spirit`) edits the same file | `20-11-erasure-inventory-closure-evidence/creation/ks-schedule-watchdog-forget-spirit.diff` (hunk at `:130`) | distinct hunk; the later lander re-bases and re-cuts |
| F13 | I14 owner | `docs/invariants/I14.md:4-10,46-49` (owner `20-9-spawned-form-lifecycle-hooks`, claim `asserted`, "must inherit carry rather than re-add a drain") | the spawned swap witness carries a pending halt through the unchanged coordinator gate |
| F14 | Hosted jobs | `.github/workflows/discipline.yml` `epic-20-exit` `:2898`, `wasm-host-tests` `:4550`, `check-wasm-form-equiv` `:5236` | AC6 legs |
| F15 | Pins | `xtask/kloc.toml` kernel `:264` 21042, iac `:406` 8270, domain `:425` 10099, bin `:517` 34191, wasm-host `:703` 2412, xtask `:339` 44406, `_aggregate_hardfail` `:718` 192520; `xtask/kernel-core-baseline.toml` `src_lines` `:505`, set hash `:939`, `hot_swap/coordinator.rs` `:958`, `scheduler/schedule_watchdog.rs` `:1005` | landing pins |
