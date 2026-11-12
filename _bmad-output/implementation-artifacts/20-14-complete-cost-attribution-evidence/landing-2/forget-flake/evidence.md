# journey_forget_18_6 hosted flake — root cause, fix, witnesses (DevForget, base HEAD `9996fe09`)

Deferred row: `deferred-work.md` "Forget journey pending outcome" (now ROOT-CAUSED AND FIXED; `CLOSED @commit` stamp Main-owned).

## Symptom (hosted)

`hosted/job-130902160663-lines-11499-11504.txt` (run 43133211598 @456793ca) and `hosted/job-129801925353-lines-11340-11352.txt` (run 42807966928 @a6b557c0; that run predates the
terminal-JSON diagnostic, so it only shows the failed assertion): `journey_forget_18_6.rs:217` `pending_count` 1 with
`redistill_outcomes[0].outcome = {status: pending, reason: {refused: "writer-mismatch"}}`, `writer_spirit_pid` 2, `redistilled_count` 0. The hosted log line is cut by the CI log
shipper at `"frame_sweep_completion":{"frame…`, so the rest of the payload (and the daemon diagnostics) is not available.

## Where `writer-mismatch` is produced (three sites)

1. `spirits/researcher/src/lib.rs:898` `Researcher::redistill` — route pid / request pid / boot nonce disagree (same predicate `consolidate_forget` already checks at :801-802, so unreachable unless the pid moves).
2. `crates/maos-bin/src/hook_pump.rs:741` `HookPump::check_redistilled` — the researcher reported `Redistilled { new_distillate_frame_id }`, and the row that id resolves to fails ANY of:
   `row.spirit_pid == request.writer_spirit_pid`, `row.boot_nonce == tl.boot_nonce()`, `row.kind == Distillate`, `row.timestamp_ns > receipt.timestamp_ns`, `frame_id != the redacted one`.
3. `crates/maos-bin/tests/hook_pump_17_4.rs:1632` / `crates/maos-domain/tests/forget_cascade_18_6.rs:60` — assertions only.

## Reproduction and diagnosis

Recipe: `cargo build --locked -p maos-bin -p maos-cli`, `PATH=$HOME/.venvs/maos-test/bin:$PATH`, the test binary run directly (`loops/loop.sh`).

| Loop | Runs | Fail | Raw |
|---|---:|---:|---|
| before, sequential, idle box | 12 | 0 | `loops/before-sequential-12-results.txt` |
| before, 8 parallel + 24 CPU hogs | 48 | 3 | `loops/before-stress-p8-48-results.txt`, `red/before-stress-p8-run-*.log` (same payload as hosted: pid 2, `writer-mismatch`) |
| before + temporary diagnostic in `check_redistilled`, same stress | 96 | 6 | `loops/before-diag-p8-96-results.txt`, `red/diag-p8-run-*.log`, `plants/diag-patch-hook_pump-check_redistilled.txt` |
| **after the fix**, same stress | **192** | **0** | `loops/after-stress-p8-192-results.txt` |

Failure rate before: 9 of 144 stress runs (6.25%); after: 0 of 192 (at 6.25% the chance of 0/192 by luck is ~6e-6).

The diagnostic (restored: copy-back, `cmp`, `touch`) printed, for every one of the 6 failures, the row the "new distillate id" resolves to:
`(spirit_pid 2, boot_nonce == tl.boot_nonce, kind CapabilityInvocation, intent "cap.verify.ok", timestamp_ns > since)` — pid, nonce, timestamp and `since` all pass; only
`kind == Distillate` fails. The id returned for the new distillate was a `cap.verify.ok` frame of the same Spirit, not the Distillate row.

## Root cause (product race, in `maos-iac`)

`DistillateWriter::write_distillate` (`crates/maos-iac/src/adapter/distillate.rs`, step 5/6) inserted the Distillate row with `frame_id: None` and then recovered the receipt id with
`TransparencyLogAdapter::last_frame_id()`. That is the log-wide "last inserted frame" under the log mutex, released between the two calls: any other thread's insert in that window
(here the same Spirit's concurrent `cap.verify.ok` capability-verification frame) is returned as the digest frame id. The Researcher reported it as `Redistilled`, the hook pump's
`check_redistilled` correctly rejected a non-Distillate row (`writer-mismatch`), the real distillate row existed but was never reported, `pending_count` was 1. Not a test race, not
pid reuse or a nonce change, not an ordering issue with the writer's registration (the notice reached the writer and redistilled).
`adapter.rs:843` already documented "never the shared `last_frame_id`" for the retract path.

## Fix

`distillate.rs`: `let digest_frame_id = self.transparency_log.mint_frame_id();` then `insert_distillate_frame(…, Some(digest_frame_id), …)`; the receipt is built from that id. The
`last_frame_id()` read is gone. Test seam: one `#[cfg(test)]` thread-local one-shot hook run right after the row commit (compiled out of production). Zero-pending assertion and all budgets
unchanged; kernel untouched.

## Witness table

| Witness | Failing before | Plant | After |
|---|---|---|---|
| `maos-iac adapter::distillate::tests::receipt_names_the_row_this_call_wrote_when_another_writer_inserts_after_it` (injects a pid-99 insert between the distillate commit and the receipt; asserts the receipt id is not the interloper's and resolves to a Distillate row of pid 1) | `red/witness-red-before-fix.log` (hook present, fix absent: `assertion left != right` at the receipt/interloper comparison) | `plants/revert-fix-id-from-last_frame_id.log`: fix reverted (`None` + `last_frame_id()`), same red; restored by copy-back, `cmp` clean, `touch` | green |
| journey `journey_forget_18_6::live_forget_redistills_and_never_resurrects_then_pending_is_typed` under 8-parallel CPU stress | 9/144 | n/a | 0/192 |

## Verification (observed)

- `cargo nextest run --locked -p maos-iac`: 198/198 (includes the new witness).
- `cargo nextest run --locked --no-fail-fast -p maos-bin --test hook_pump_17_4 --test forget_governance_20_14 --test erasure_inventory_20_11 --test erasure_live_20_11 --test recall_17_3d`: 63/64;
  the one failure `erasure_live_20_11::ac6_running_t3_worker_uninstall_proves_live_inventory` is `MAOS_T3_RUNTIME must select the provisioned runtime` (`tests/harness/t3_substrate.rs:27`, this box has no
  provisioned T3 substrate; unrelated to distillates) — `green-dependents-maos-bin.log`.
- `cargo nextest run --locked -p researcher -p maos-digest -p butler -p architect -p orchestrator`: 246/246 (3 skipped).
- Gates (`gates/*.json`, rc 0): check-abi-ratification, check-decision-register, check-dependency-closure, check-dev-model-tier, check-dev-record-completeness, check-env-contract,
  check-epic-close-coherence, check-exit-commands, check-fkcs, check-kernel-baseline, check-manifest-schema-version, check-pub-field-constructors, check-serde-error-handling,
  check-service-boundary, coverage-matrix, kloc-check, `invariant-lock --changed-files crates/maos-iac/src/adapter/distillate.rs`.
- `cargo clippy -p maos-iac --all-targets --locked`: 66 maos-iac warnings, 8 in `distillate.rs`, all on lines I did not touch (redundant closures at :183/:202/:211, indexed loop at :508); no warning on the new lines.
- `cargo fmt --all --check`: clean.

## Pins (exact, after `cargo fmt --all`, `kloc-check --json`)

| Pin | Before | After |
|---|---:|---:|
| maos-iac | 8773 | 8831 (+58 measured total, covering the fix, the cfg(test) hook and the witness test; disclosed for Main adjudication, zero headroom) |
| `_aggregate_hardfail` | 202921 | 202979 (measured 202978 + 1) |
| kernel tokei / physical (live) | 21202 / 27044 | 21202 / 27044 (unchanged; baseline pins untouched) |

## Not done / deferred

- `render_j3_digest_scene` (`maos-bin/src/main.rs`) has the same read-after-insert shape (Low), and every Distillate payload carries a zero `digest_frame_id` placeholder (Info): both are rule-10(b) rows in `deferred-work.md`.
- The hosted failure payload's remainder (cut by the log shipper) was not available; causality rests on the local reproduction (identical payload) plus the diagnostic and the 0/192 result, not on the hosted daemon diagnostics.
