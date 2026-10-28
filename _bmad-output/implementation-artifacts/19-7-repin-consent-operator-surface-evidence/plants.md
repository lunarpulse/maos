# 19-7 proven-red plants

## Corrected-tree applicability (2026-10-28)

No plant was applied and no cargo command was run during this correction. Before correction, `ac2-default-decision-plant.diff` failed `git apply --check`; AC1 and AC3 passed. After the re-cut, all three pass forward checks and all three fail reverse checks (`git apply --check -R`) on the unplanted tree.

| Diff | Current planted site / asserting control | sha256 | Corrected-tree red / green |
|---|---|---|---|
| `ac2-default-decision-plant.diff` (re-cut) | `crates/maos-a2a-core/src/tofu.rs:768-778`: remove the recorded-verdict prerequisite from atomic removal and substitute `Approve { approval_id: [0; 16] }` for a missing verdict. Same fail-open defect, with deadline/nonce and pin checks retained. Control: `crates/maos-a2a/tests/restart_invalidates_pin_nfr_rel_6.rs:222,246-250` | `46c885e08e54cd189fcf21011f2358a094a9776e85adc812cf5eaad13d01c1ad` | Red rc=100, wall 18 s: `cargo nextest run --locked --color never --no-fail-fast -p maos-a2a -E 'binary(restart_invalidates_pin_nfr_rel_6)'` — 22 run, 17 passed, 5 failed: `repin_19_7_detection_records_one_pending_request_and_no_decision_times_out`, `repin_19_7_newer_detection_during_a_decision_supersedes_it` (panicked `:507`, `left: AcceptedByOperator { approval_id: [0; 16] }`), `repin_19_7_nonce_mismatch_is_typed_and_changes_nothing` (`:380`, same left), `repin_19_7_accept_repins_the_current_fingerprint_with_the_observed_nonce`, `repin_19_7_withdraw_drops_only_the_decision`. Restored byte-identical: `crates/maos-a2a-core/src/tofu.rs` sha256 `7e3656ca04dd6650d0c9feb1095c93a8e19790f0e4d4a5f1574789002ce1ed08`; green in the combined re-run below |
| `ac3-nonce-binding-plant.diff` (unchanged) | `crates/maos-a2a-core/src/tofu.rs:294`; controls `crates/maos-bin/tests/repin_door_19_7.rs:148,156` and `crates/maos-a2a/tests/restart_invalidates_pin_nfr_rel_6.rs:366` | `1834ffda4531cd9b0093e882ca064b5adce5fc68f57157989bf691a93b1209c0` | Red rc=100, wall 53 s (re-run on the corrected `tofu.rs`): `cargo nextest run --locked --color never --no-fail-fast -p maos-bin -p maos-a2a -E 'binary(repin_door_19_7) \| binary(restart_invalidates_pin_nfr_rel_6)'` — 32 run, 29 passed, 3 failed: `maos-bin::repin_door_19_7 a_wrong_nonce_is_a_conflict_and_the_pin_stays_invalidated`, `maos-a2a::restart_invalidates_pin_nfr_rel_6 repin_19_7_nonce_mismatch_is_typed_and_changes_nothing`, `repin_19_7_newer_detection_during_a_decision_supersedes_it`. Restored byte-identical: same `tofu.rs` sha256 as the AC2 row above |
| `ac1-f4-consent-removed-plant.diff` (unchanged) | `crates/maos-bin/tests/f4_pairing.rs:1343-1397`: remove the whole `(e) THE CONSENT` block, preserving hosted V4 semantics. Control starts at `:1174`; the following `(f)` block asserts successful resend | `4f73e4cbc4186aad533952dee378e459741708dbee3e93f4d379226d2d83b6f3` | Hosted only — proven red in run 40671241869 (below); not run locally. On the corrected tree V4 approves only after the `re-send 2 … still holding:` line and the sender backs off 1 s → 16 s capped at the 300 s request lifetime, so the planted red now ends at the hold's `min(REPIN_HOLD_SECS=20 s, 300 s)` expiry, inside the test's `REPIN_HOLD_SECS + 15` exit wait |

The re-cut LOCAL AC2 needs planted red and byte-identically restored green with exactly:

`env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo cargo nextest run --locked --no-fail-fast -p maos-a2a --test restart_invalidates_pin_nfr_rel_6`

## Corrected-tree plant pass 2026-10-28

The orchestrator re-ran the local plants (AC2 re-cut, AC3 unchanged) on the corrected tree — uncommitted corrections on top of `6f872383`, model `anthropic/claude-opus-5-5`, one cargo process, every run under `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. Each plant was applied in place: copy aside → `git apply <diff>` → run the target (red) → copy back → `cmp` → `touch` → green re-run; restore was proven by sha256 equality of `crates/maos-a2a-core/src/tofu.rs` before planting and after restore (`7e3656ca04dd6650d0c9feb1095c93a8e19790f0e4d4a5f1574789002ce1ed08`), not `git diff --quiet 6f872383`. Wall is plant applied → red observed, including the incremental compile. Green re-run, shared with 19-1 (command in `19-1-…-evidence/plants.md`, covering `binary(repin_door_19_7) | binary(restart_invalidates_pin_nfr_rel_6)`): 56 run, 56 passed (the T3 vectors return early locally: no rootless podman).

## Historical observations @6f872383


Plant pass 2026-10-28, `anthropic/claude-opus-5-5`, on candidate `6f872383`. The tree was clean before and after. One
cargo process ran throughout, under `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`.

Every plant was run in place with the same steps as 19-1's (`19-1-…-evidence/plants.md`):

1. Copy the file aside.
2. `git apply <row>-plant.diff` (the diff is in this directory).
3. Run the targeted test, which must go red.
4. Copy the file back, check it with `cmp`, and check `git diff --quiet 6f872383 -- <file>`.
5. `touch` the file.
6. Run the same test again, which must go green.

Wall-clock covers two spans: from the plant to the observed red, and from the restore to the observed green. Both
include the incremental rebuild.

Two plants ran locally, AC3 and AC2. The AC1 plant needs the T3 two-process substrate. This host's rootless podman
fails with `cannot clone`, so `require_t3` skips `restart_refusal_is_repinned_by_operator_consent_then_delivered` here.
That plant is recorded as a hosted plant below.

| AC | File · change (at `6f872383`) | Command | Red observed | Restore | Wall |
|---|---|---|---|---|---|
| AC3 nonce binding (route test: echo N+1) | `crates/maos-a2a-core/src/tofu.rs:294`. `record_repin_decision`'s nonce comparison is disabled (`if false && entry.request.observed_boot_nonce != boot_nonce`), so an echoed N+1 records the verdict (`ac3-nonce-binding-plant.diff`) | `cargo nextest run --locked --no-fail-fast -p maos-bin --test repin_door_19_7`, then `… -p maos-a2a --test restart_invalidates_pin_nfr_rel_6` | `a_wrong_nonce_is_a_conflict_and_the_pin_stays_invalidated` FAIL at `repin_door_19_7.rs:154`: `left: ("completed", "")` / `right: ("conflict", "repin_nonce_mismatch")`, so the door consumed the verdict against the request's own nonce and re-pinned. 8 run, 7 passed, 1 failed. Store vectors: `repin_19_7_nonce_mismatch_is_typed_and_changes_nothing` (`:356`) and `repin_19_7_newer_detection_during_a_decision_supersedes_it` (`:496`) FAIL (`left: Ok(RepinRequest …)` / `right: Err(NonceMismatch { observed: 7, supplied: 8 })`). 18 run, 16 passed, 2 failed | `cmp` OK; `git diff` clean; `touch`; green 8/8 and 18/18 | red 10.4 s + 28.8 s; green 10.5 s + 4.0 s |
| AC2 default stays fail-closed (no decision ⇒ `TimedOut`) | `crates/maos-a2a-core/src/tofu.rs:740-742`. `await_repin_consent` now treats a missing verdict as `Approve { approval_id: [0; 16] }` instead of returning `TimedOut`, which is a fail-open default (`ac2-default-decision-plant.diff`) | `cargo nextest run --locked --no-fail-fast -p maos-a2a --test restart_invalidates_pin_nfr_rel_6` | `repin_19_7_detection_records_one_pending_request_and_no_decision_times_out` FAIL at `:239` (the CR-1 "no recorded decision ⇒ TimedOut" assertion). `repin_19_7_accept_repins_the_current_fingerprint_with_the_observed_nonce` FAIL at `:300` (`left: AcceptedByOperator { approval_id: [0, …] }` / `right: TimedOut`). `…_nonce_mismatch_is_typed_and_changes_nothing` (`:365`), `…_newer_detection_during_a_decision_supersedes_it` (`:492`) and `…_withdraw_drops_only_the_decision` (`:551`) also FAIL. 18 run, 13 passed, 5 failed | `cmp` OK; `git diff` clean; `touch`; green 18/18 | red 3.1 s; green 2.9 s |

The story's control table cited `restart_invalidates_pin_nfr_rel_6.rs:128-145` for AC2 (stale: corrected 2026-10-28). That
range was `scenario_4_5_operator_timeout` (now `:181`). It stays green under this plant, because it has no pending
request and returns `TimedOut` on the earlier no-entry branch. The AC2 default control is therefore
`repin_19_7_detection_records_one_pending_request_and_no_decision_times_out` (`restart_invalidates_pin_nfr_rel_6.rs:222`).

The first attempt of the AC2 plant run failed to compile, and that failure is not a red. Between the AC3 green run and
the AC2 red run, another process emptied part of the shared target dir `/mnt/build/cargo`. The compile failed with
`failed to open …/libhashbrown-b23a46ef6d6a84e2.rmeta: No such file`, `E0463 can't find crate for maos_a2a_core`, on
both the planted and the restored tree. The target was rebuilt on the restored tree: 18/18 green. The plant was then
re-run from scratch, and the numbers in the row above are from that re-run. No plant touched
`crates/maos-kernel-core`.

## Hosted plant (T3; `ci/19-7-verify`, not run here)

| AC | Diff (sha256) | File · change | Command / job | Expected red |
|---|---|---|---|---|
| AC1.3 vector (Murat, 2026-09-21) | `ac1-f4-consent-removed-plant.diff` (`4f73e4cbc4186aad533952dee378e459741708dbee3e93f4d379226d2d83b6f3`) | `crates/maos-bin/tests/f4_pairing.rs:1343-1397`. The whole `(e) THE CONSENT` block (the `maosctl a2a repin approve` call and its reply and I4 assertions) is removed. Every helper it used (`maosctl_on_door`, `output_text`, `repin_approval_rows`, `REPIN_ACTOR`) is still referenced elsewhere in the file, so the plant compiles without new dead-code warnings | `ci/ci-19-7-verify.yml` on branch `ci/19-7-verify` — own-build in-job (not the 20-15 archive), T3 provisioned, control first: V4 `restart_refusal_is_repinned_by_operator_consent_then_delivered` (`f4_pairing.rs:1174`) must pass `1 passed` with no `SKIP: T3 ` line | `1 failed`: assertion `after consent the held frame must ACK and host A exit 0:` (`f4_pairing.rs:1412`), and the sender's `delegation re-pin hold expired after` (`crates/maos-bin/src/delegation.rs:476`) |

The run happens via the branch-only workflow `ci/ci-19-7-verify.yml` on branch `ci/19-7-verify`. The diff passes
`git apply --check` against `6f872383`. **Run 40671241869** (`ci/19-7-verify` @ `f2649164` = `e49b83c4` + branch-only workflow), job `proven-red` success 10:19:49→10:25:34Z (observed 2026-10-28 via the GitHub API): built the V4 processes/fixtures; the unplanted V4 control passed with no T3 SKIP — the first real-T3 execution of f4 V4; the consent-removal plant was applied and the V4 expiry oracle observed red; restored with a clean tracked diff; announce; upload.
