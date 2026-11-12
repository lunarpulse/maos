# 20-1b hosted legs: AC11.5 and AC15 (Hosted201b)

Branch `ci/20-1b-verify` (pushed to `lunarpulse/maos`, never merged). Tree: local HEAD `163ab9bd` plus ONE added test file
`crates/maos-bin/tests/worker_quarantine_20_1b.rs` (uncommitted in the main checkout) and the workflow. Final workflow:
`ci-20-1b-verify.yml` in this directory (byte-identical to `.github/workflows/ci-20-1b-verify.yml` at `6c59aa46`).

## Runs

| Run | Commit | Result | Why |
|---|---|---|---|
| [43180252358](https://github.com/lunarpulse/maos/actions/runs/43180252358) | `80a9f0a7` | cancelled/failed (diagnosed, three recipe bugs) | see below |
| [43184858544](https://github.com/lunarpulse/maos/actions/runs/43184858544) | `6c59aa46` | **success, 7/7 jobs** | final |

Run 1 recipe bugs, all fixed in `6c59aa46`, none a product failure:

1. `registry-tls-macos` used `runs-on: macos-14`; GitHub refused it (annotation on job 131054199803: "The configuration
   'macos-14-arm64-us-default' is not supported"). Now `macos-latest`, the label discipline's `macos-check` uses.
2. `kernel-free-trust` expected 156 tests, ran 158 (all 158 passed on both images, jobs 131054200051 and 131054200071). The recipe
   count predates `608735d2`/`a2bbd687`, which took `yank_policy_20_1b` from 10 to 12 tests. Count is now 158; the 54 required names
   were also re-checked offline against the downloaded run-1 log with `check-log --expected-count 158` (exit 0).
3. The plant step ran under GitHub's `bash -e`, so the expected red `nextest` (exit 100) aborted the step before the restore and
   the red-ness checks. Now `set +e` around the plant run and `set -e` after `rc` is captured. In run 1 the plant WAS red
   (both tests FAIL with `the Worker is fenced`; jobs 131054200012, 131054200005), only the step bookkeeping was wrong.

## Run 43184858544 (commit `6c59aa46`): jobs

| Job | id | Result |
|---|---|---|
| kernel-free-trust (ubuntu-24.04) | 131069395919 | success; 158 run, 158 passed; 54 required markers; kernel baseline passed |
| kernel-free-trust (ubuntu-26.04) | 131069395836 | success; 158/158; 54 markers; kernel baseline passed |
| final-tree-vectors (ubuntu-24.04) | 131069396042 | success |
| final-tree-vectors (ubuntu-26.04) | 131069396078 | success |
| public-webpki | 131069395551 | success |
| registry-tls-macos | 131069395957 | success (Apple silicon, Darwin 25.6.0 arm64, Xcode 26.6 SDK) |
| epic-20-exit | 131069396074 | success |

`final-tree-vectors` (each image): kernel quarantine vectors `5 tests run: 5 passed` (4x `quarantine_fr13_20_1` + `on_revocation_three_actions`);
T3 group `16 tests run: 16 passed` inside the delegated cgroup harness (`maos-17-2-20-1b-final`): 12x `yank_policy_20_1b`, the one-door
AC12 test `ac12_20_1b_a_failing_revocation_poll_is_printed_once`, the 20-11 Worker variant
`ac6_running_t3_worker_uninstall_proves_live_inventory`, and the two AC11.5 tests. `check-log` requires every named test as PASS,
an exact count, and no `SKIP` line.

## AC11.5: the recorded outcome (both images, identical)

Record (`artifacts/final-ubuntu-*/ac11-5-record.jsonl`):

* the quarantined live T3 Worker's next mediated calls are refused: MCP issuance, inference issuance and verification of a token
  held from before the fence all return `Err(Revoked)`; a control before the fence minted and verified Ok;
* **container activity continues**: `container_activity_continues: true`, 4 heartbeats journaled in the 4 s window after the fence,
  `container_state_after_window: "running"`; a later `stop_workers` still removed the container (`Ok(())` run result);
* this is a capability-mediated fence only. The container is neither frozen nor killed by quarantine. **No isolation is claimed.**

PRODUCT FINDING (not hidden): a CRL cannot quarantine a Worker. The record shows `worker_scb_declares_class: false`, `crl_matched_count: 0`
for a real `RevocationApplier::apply_crl` naming the Worker. Cause (source): `worker_spawn.rs` binds a Worker with
`SpiritManifestBundle { on_crash, supervision, ..Default::default() }` (no `[class]`), and the CRL arm skips an SCB with no class
(`revocation/applier.rs`, `let Some(class) = &runtime.manifest.class else { continue }`). The yank policy needs an installed package
plus a class-bearing SCB, so it does not reach a standalone Worker either. The test therefore fences the Worker with
`set_quarantined(pid, true)`, the primitive the CRL arm and the yank runtime call, and names that route in the record
(`quarantine_route`). The AC text says "quarantined by CRL"; that route is NOT proven for a Worker and is not available today.
If the product makes a Worker CRL-addressable the record changes (`crl_matched_count > 0`) and the test then requires the CRL itself to fence it.

## Plants

* `ac11-5-plant.diff`: delete `set_quarantined(pid, true)` in `fence_and_refuse`.
* Hosted, both images, run 43184858544 jobs 131069396042 and 131069396078: both AC11.5 tests FAIL (`FAIL ... a_bound_worker_scb_...`
  and `FAIL ... t3_worker_quarantine_refuses_...`, 2.6 s; message `the Worker is fenced`), nextest exit 100, file restored and `git diff --exit-code` clean.
* Local (no substrate): `ac11-5-plant-local.out`. The container-less test `a_bound_worker_scb_is_fenced_and_its_next_mediated_calls_are_refused_ac11_5`
  FAILS at `worker_quarantine_20_1b.rs:508` (`the Worker is fenced`), rc 101. The T3 test SKIPS locally (podman cannot clone here:
  "cannot re-exec process"), so the live-container plant is hosted-only. `ac11-5-local-green.out`: same two tests, rc 0, the T3 one printing
  `SKIP: T3 20-1b AC11.5 quarantined Worker: signed test substrate not provisioned ...`.

## AC15 legs, hosted status

* Hosted TLS leg (Linux): proven on ubuntu-24.04 and ubuntu-26.04, jobs 131069395919 and 131069395836. The 12 named `tls::*_ac15` cases, the
  P3 signed-body cases, `registry_limits_20_1b`, `registry_config_20_1b`, `yank_*`, dev flows and cohort teardown all in the 158.
* macOS native leg: job 131069395957. `cargo check` for `maos-registry` and `maos-spirit-cli` on `aarch64-apple-darwin`, `maos-bin --no-default-features`,
  `registry_limits_20_1b` (4 passed, native), and 3 TLS runtime cases (`configured_root_and_ca_authenticate_real_registry_ac15`,
  `tls_admission_refusal_is_encrypted_http_503_f1`, `incomplete_tls_handshake_cannot_enter_http_error_write_timeout_f1`) passed natively.
* Public-WebPKI leg: job 131069395551 (and 131054200103 in run 1): `private_ca_replaces_public_webpki_for_real_public_peer_d4_f7` ok against `https://github.com/mcp`.
* epic-20 exit lane (P1 after-state): job 131069396074 success.

## Not claimed

* Windows/other Apple legs, Apple runtime of anything beyond the four-plus-three cases above, and the whole workspace on macOS (discipline's `macos-check` owns the compile).
* A CRL-driven Worker quarantine (see finding). A durable/release/readback closure of FR13 (not an AC; ruling `q`).
* Kernel-free scope: the kernel is unchanged (`check-kernel-baseline` 26062 pinned = actual, hosted and local). No kloc re-pin: `tests/` is excluded from kloc.

## Local gates (this checkout, prebuilt xtask, rc 0; JSON in `local/`)

check-serde-error-handling, check-dev-model-tier, check-kernel-baseline, check-service-boundary, check-dev-record-completeness,
check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-test-cost, check-unsafe,
check-coverage-matrix-completeness, check-composition-root-completeness, `invariant-lock --changed-files crates/maos-bin/tests/worker_quarantine_20_1b.rs`;
`python3 scripts/repin.py --check`: PINS IN SYNC. `cargo clippy -p maos-bin --test worker_quarantine_20_1b`: no diagnostic in the new file
(the `-D warnings` form is red on pre-existing `maos-spirit-abi` lints, unrelated); `rustfmt --check` clean.

## Layout

`run-43180252358/`, `run-43184858544/`: `run.json`, `jobs*.jsonl`, per-job logs (`logs/`), uploaded evidence artifacts (`artifacts/`; the two large
cargo metadata files were dropped).
