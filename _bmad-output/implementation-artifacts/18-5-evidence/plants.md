# Story 18-5 proven-red controls

Finisher: `openai-codex/gpt-6.1-sol`. Every cargo command uses `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`; cargo execution is serialized. Plants use backups of the uncommitted implementation, never checkout of HEAD. Each restoration uses copy-back, `cmp`, and `touch` to force the restored source newer than planted artifacts.

## AC1 — once-per-crossing latch

- Mutation: in `crates/maos-capability/src/context_budget.rs`, crossing filter `previous < threshold && level >= threshold` becomes `level >= threshold`.
- Command: `cargo nextest run --locked -p maos-bin --test context_budget_18_5 four_day_process_reports_context_crossings_and_exact_journal_rows`.
- Proven red: exit **100**, wall **8.49 s**; journal count assertion found **5** context signals instead of **2**: pressure at 6600, pressure+limit at 7800, pressure+limit at 8300. The process witness detects the repeated upward signals.
- Restore: backup copied back, `cmp` exit 0, touched source.

## AC2 — door-only context binding

- Mutation: only `build_spirit_obj`'s Butler arm in `crates/maos-bin/src/main.rs` clears the cloned seam's context factory (`seam.context = None`). Boot binding is unchanged.
- Command: `cargo nextest run --locked -p maos-bin --test context_budget_18_5 door_loaded_butler_reports_for_its_pid_and_reload_starts_within`.
- Proven red: exit **100**, wall **13.06 s**; door-loaded context is null instead of `{used:6600,window:8192,level:"pressure",crossed:"pressure"}`.
- Restore: backup copied back, `cmp` exit 0, touched source.

Both restored process controls: `cargo nextest run --locked -p maos-bin --test context_budget_18_5` → **3 passed**, wall **8.89 s**, execution **1.899 s**. Full commands, outputs, mutations and restorations: `process-plants.log`.

## AC3 — real WASM guest forgery refusal

- The two new context witnesses use real admission/session dispatch and actual T2 seccomp/Landlock, with required `fd_max=64` resources only. They do not assert Story 17-2's unrelated cgroup envelope or declare a fake cgroup lane.
- Mutation: remove only `ContextPressure` and `ContextLimit` from `frame_refusal`'s `kernel_reserved_kind` match in `crates/maos-bin/src/spirit_session.rs`.
- Command: `cargo nextest run --locked -p maos-bin --features network,wasm-host --test recall_17_3d context_18_5_wasm_guest_cannot_forge_kernel_pressure_or_limit_to_peer`.
- Proven red: exit **100**, wall **6.76 s**, execution **0.535 s**; session counts were **(received=1, delivered=1, refused=0)** instead of **(1,0,1)**. The forged kernel signal reached the governed delivery path.
- Test-infrastructure correction: an initial invocation of this same mutation exited 100 after **5.43 s**, but I13 rejected the fixture's empty non-human lineage before delivery. That was **not counted as proof of the requested forgery control**. The seed now carries valid operator lineage, leaving the reserved-kind barrier as the sole mutated control; the corrected invocation above demonstrates actual delivery.
- Copy-back + `cmp` + `touch`: exit 0; temporary backup removed. Restored command selecting both `context_18_5_` tests: **2 passed**, execution **1.268 s**, wall **8.82 s**.
- Original valid planted delivery output is checked in as `ac3-original-red.log`; original restored-green output is `ac3-original-restored.log`. The earlier unrelated I13 diagnostic is historical only, not counted as control proof.

Six distinct control mutations cover AC1–AC6. AC3 repeats the same mutation once after the documented infrastructure correction; no unrelated mutation is represented as control proof.

## AC4 — threshold ordering

- Mutation: remove `RawBudget::validate`'s pressure ≥ limit rejection block in `crates/maos-manifest/src/manifest.rs`.
- Command: `cargo nextest run --locked -p maos-manifest --test budget_thresholds_18_5`.
- Proven red: exit **100**, wall **1.89 s**, **2 passed / 1 failed**; malformed pressure 950 / limit 800 was admitted.
- Copy-back + `cmp` + `touch`: exit 0. Restored: **3 passed**, wall **1.94 s**.

## AC5 — issuance gate and CI verification simulation

- Back up `crates/maos-kernel-core/src/capability/mod.rs`, verify `ci/plant.diff.sha256`, apply `ci/plant.diff` to remove the issuance gate.
- Separate compile: `cargo nextest run --locked -p maos-bin --test context_refusal_18_5 --no-run` → exit **0**; checksum/apply/build wall **5.81 s**.
- Proven-red command: `cargo nextest run --locked -p maos-bin --test context_refusal_18_5` → exit **100**; the exhausted pid incorrectly minted a token, defeating the typed refusal assertion. Execution **0.150 s**, wall **0.707400 s** (recovered from the original bash result's `details.wallTimeMs=707.4002900123596`, result `9a515e8c`; no repeated plant for timing).
- Copy-back + `cmp` + `touch`: exit 0. Restored: **1 passed**, wall **5.68 s**.
- The checksum, patch application, separate successful compilation, expected nextest failure code, and restored green simulate `ci/ci-18-5-verify.yml` locally. **No hosted Actions run is claimed.**

## AC6 — polled RateLimited delivery

- Mutation: in `crates/maos-kernel-core/src/inference/mod.rs`, replace the spawned delivery with the original unpolled `let _ = iac.deliver_typed(frame, 0, None);`.
- Command: `cargo nextest run --locked -p maos-kernel-core --test context_budget_gate_18_5 a_provider_429_journals_and_delivers_one_rate_limited_frame`.
- Proven red: exit **100**, wall **8.26 s**, execution **5.056 s**; the mailbox receive timed out (`the RateLimited frame is delivered, not dropped: Elapsed(())`).
- Copy-back + `cmp` + `touch`: exit 0. Restored: **1 passed**, wall **2.58 s**, execution **0.005 s**.

Native plant commands and observed outputs: `native-plants.log`.

## Review-patch replay and new controls — 2026-10-26

All commands and complete outputs, source mutations, backup/copy-back,
`cmp`, `touch` and backup removal are checked in as **`review-plants.log`**.
Every counted mutant below exited **100**; every identical command after
restoration exited **0**. Cargo remained serialized with the environment
above. Process witnesses used `--features network,wasm-host`; the existing
local resource contract was truthfully `MAOS_17_2_CGROUP_MODE=undelegated`
on the read-only cgroup2 mount, not a claim of delegated containment.

### Committed AC1/AC2 witnesses (AA-2)

- **AC1:** remove `previous < threshold` from the ledger crossing filter.
  `cargo nextest run --locked -p maos-bin --features network,wasm-host --test context_budget_18_5 four_day_process_reports_context_crossings_and_exact_journal_rows`
  failed at the committed `:172` assertion: **5 rows instead of 2**.
  Restored command: **1 passed**, execution 0.579 s.
- **AC2:** clear only the cloned Butler seam's context binder in
  `build_spirit_obj`; boot binding is unchanged.
  `cargo nextest run --locked -p maos-bin --features network,wasm-host --test context_budget_18_5 door_loaded_butler_reports_for_its_pid_and_reload_starts_within`
  failed at committed `:291`: **null context instead of pressure at 6600**.
  Restored command: **1 passed**, execution 1.636 s.
- AC1's source mutation invalidates the `maosctl` fixture's transitive
  source freshness. The log includes the real `build-test-fixtures --only
  maosctl` builds on both mutant and restored source, so neither result is
  a stale-fixture failure.
- **AC3:** the original actual-delivery red and two-test restored-green
  outputs are now repository files named above, rather than session links.

### New review controls

| Control / finding | In-place mutation | Focused nextest witness | Observed red |
|---|---|---|---|
| R1 / BH-2 / EC-1 | Restore slot deletion inside `revoke_all_for_pid` | kernel `--test on_revocation_three_actions` | DrainThenTerminate pid is no longer exhausted before unload (`:213`) |
| BH-1 authorization order | Insert context refusal ahead of verification | kernel `--test context_budget_gate_18_5 an_mcp_token_minted_before_the_crossing_is_refused_at_use` | Invalid token probes context state (`:238`) |
| R3 / BH-6 MCP audit | Remove `record_mcp_refusal` from the maos-iac typed refusal helper | same kernel MCP witness | **0 receipts instead of 1** (`:292`) |
| BH-4 / AA-5 / EC-6 rounded thresholds | Disable the zero-permille rejection | manifest `--test budget_thresholds_18_5 thresholds_rounding_to_zero_permille_are_rejected` | `0.0004` admitted as **0 permille** (`:84`) |
| EC-4 synchronous journal | Skip the report's synchronous journal call | IAC `--test context_signal_18_5 runtime_shutdown_cannot_erase_a_reported_crossing` | **No pressure row** before any executor poll (`:195`) |
| EC-3 ordered queue | Deliver the next queued frame before the current frame | IAC `--test context_signal_18_5 mixed_context_kinds_are_received_in_report_order` | **Limit received before Pressure** (`:247`) |
| BH-3 / AA-1 / EC-2 no duplicate fallback | On mailbox error, write an additional direct TL fallback row | IAC `--test context_signal_18_5 closed_registered_mailbox_does_not_duplicate_crossing_rows` | **[45,45] instead of [45]** (`:170`) |

One preliminary duplicate-fallback mutation retried the **same routed frame
id**, so the journal correctly deduplicated it and the witness stayed green.
That diagnostic is retained in the log but is **not counted as proven red**.
The counted mutation restores the actual old defect: a fresh-id direct TL
fallback after `ChannelClosed`. The witness then fails for duplicate rows.

The adapter suite also covers a dead bus, no runtime, synchronous persistence
through shutdown, and 64 queued reports against a 32-frame mailbox. Both
context kinds share one receive stream; `recv` and `try_recv` report the
actual frame kind and decrement that kind's metric.

### Branch CI plant

The existing `ci/plant.diff` still applies to the patched capability module:
`git apply --check` exited **0**, and `sha256sum -c ci/plant.diff.sha256`
(using the repository-relative checksum path) exited **0**. No regeneration
was necessary. The original AC5 compile/red/restore evidence remains above;
this review adds the real revocation and MCP controls rather than substituting
them for AC5.


**TI-2 disposition (operator/integrator, 2026-10-26): patch — hosted
proven-red run by the integrator at landing.** The integrator will commit,
then run `ci/18-5-verify` on **ubuntu-24.04**, as for prior stories. Local
simulation is **not accepted as a replacement** for that hosted proof.
The recipe now includes checksum, clean patch applicability, separate mutant
compilation, expected nextest failure100, and always-run copy-back/`cmp`/
`touch` restoration followed by green. No commit, push or hosted run was
performed by this patch worker.

