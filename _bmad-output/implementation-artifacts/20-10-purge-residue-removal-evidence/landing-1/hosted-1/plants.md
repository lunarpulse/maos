# 20-10 hosted incident — failing-before and byte-restored plants

Baseline: `16c35ef0d4b4de0f3969db15c9d7666fcdf9f2de`. These records are additive; earlier landing/review evidence remains unchanged.

## Branch-only workflow setup defect

`xtask/tests/epic17_exit_workflow_17_7.rs` now exercises every scene `run` unit against the real Python harness with timeout zero. The real unit-name validation precedes timeout validation, so the test requires the timeout rejection, not a copied allowlist or a running systemd service. Both initial and restored commands are checked.

- `workflow-contract-red.log`: baseline exit **100**, actual `maos-20-10` namespace rejection.
- `workflow-contract-green.log`: **4/4 passed**, exit **0**, after using the existing `maos-17-7-*` namespace; harness allowlist unchanged.
- `invalid-unit-plant.diff`: reintroduces the invalid initial unit. SHA-256 **61ac6a3d811f5106b4c193a1fb2dc033555b76e88bb98c9f8feee0ef7c1f03ef**.
- `workflow-contract-plant.log`: assertion-red, exit **100**, not a compile failure.
- `workflow-copy-back.log`, `workflow-cmp.log`, `workflow-touch.log`: all exit **0**; `workflow-contract-restored.log`: **4/4 passed**, exit **0**.

## Hosted main-run regressions F1 / F2

`hosted-regressions-red.log` runs two new executable preconditions against the baseline behaviors, exit **100**, **0 passed / 2 failed**:

1. `egress_daemon_17_1::hosted_length_home_keeps_egress_socket_below_sun_path_limit`: actual proxy bind fails with **`path must be shorter than SUN_LEN`**. The home is padded to a hosted-equivalent 70 bytes, a seven-digit Spirit PID is used, and the five-digit creator-PID regressed shape is explicitly above the Linux 108-byte capacity. This confirms the socket-length cause rather than only inferring it from startup timeouts. The witness is Linux-specific because socket capacities differ across Unix platforms.
2. Existing terminal scene `root_crash::purge_cgroup_native_binding_and_dead_creator_twins` now executes its real dry-purge fixture preflight before the substrate gate: **`refusing unsafe purge root .../home (equal to HOME or an ancestor)`**. No extra terminal test was added; suite pin remains **34**.

The fixed behaviors passed in `focused-prerequisite-stale.log` (egress test 26/164 and cgroup-scene preflight 109/164). That overall run was **not green**: 146 passed / 18 failed because registered fixture binaries predated `secret_store.rs` after the separately owned 20-4 source park. This is a concrete prerequisite failure, not a flake; fixtures must be rebuilt, not freshness checks suppressed.

| Plant | Mutation | Assertion-red result | SHA-256 |
|---|---|---|---|
| `F1-egress-pid-prefix.diff` | Put creator-PID naming back on durable egress roots | `plant-F1-egress.log`, exit **100**, real SUN_LEN bind error | `41fb842186f2faf0ffaf9b48545a15152c71198cf42d49ca259167fa271b5079` |
| `F2-home-equals-maos-home.diff` | Put user HOME back equal to the purgeable MAOS_HOME | `plant-F2-home.log`, exit **100**, real safety refusal | `8214228e81b3928886af269307b02c6b3cc441546aec56186ef7ca6227abea43` |

Each source plant was immediately copied back, compared byte-for-byte and touched; all six restore commands exited **0** (`F1-*` / `F2-*` logs). Neither plant remains. The custody witness also covers dead-PID roots (removed), live-PID roots (retained), and legacy nonce-only roots (retained and explicitly receipted/reported).

See `hosted-findings.md` for hosted log-line evidence. Neither failed baseline nor plant scene leg in run 41584515474 counts as an intended proven-red result: neither reached a scene assertion.

## Final corrected-source verification

Final anchors: `egress_daemon_17_1.rs:252`, terminal `root_crash.rs:268`,
custody `purge_residue_20_10.rs:202`, workflow contract
`epic17_exit_workflow_17_7.rs:418`. Final focused run: **164/164 passed**,
exit **0**. Final full serial: **5573/5573 passed**, 138 configured skips,
exit **0**; fresh serial budget **0**, 644.512 s sum, no findings.

Default concurrency: **5572/5573 passed**, exit **100**, with the known
snapshot-owner timing failure `instance 16: observer never entered inspect`.
It was not retried. Rule-10 owner/citations: `deferred-work.md:5`,
`workspace-default-final.log:7335-7377`, adjacent junit. All 20-10 witnesses
passed in that run. Prescribed prepush independently exited **0**; its
workspace/default-profile tests, kernel lib tests and doctests passed. That
does not erase the separately recorded default-concurrency failure.
Prepush budget was advisory **1** (hook-pump 30.317 s, one-daemon 34.159 s,
participant-recall 44.337 s), recorded under owner 20-7 without widening or
retry (`deferred-work.md:3`, `prepush-final.log:536-540`).

All-target build, requested structural gates, non-denying clippy and deny
exited **0**. Complete-stream ratchet: **1335 / 873**, baseline unchanged
**1383 / 897**; lint/file/message comparison against the prior local stream
has **no new warnings** (`clippy-changed-file-diff.json`).
Measured bin **34191 <= 34548**, aggregate **192519 <= 192876**,
hardfail **192520**; kernel **26841 / 21042** and xtask **44406** unchanged.

The first serial attempt's 19 failures were all stale registered release
fixtures (four equiv-native-twin, 15 maos-wasm-runner), not flakes or
production failures. All 16 registered fixture artefacts were then provisioned
before the final serial/default runs; no freshness guard was suppressed.
`serial-failure-classification.json` and the earlier junit/log preserve that
failure. An earlier combined eval timed out during compilation; its command
exit is unobserved and explicitly recorded as such in the ledger.

Artifact reads have line/byte pagination limits. The initial ratchet input
lacked `build-finished` due to capture truncation (exit 1); reconstructing
all pages of the **same successful clippy run**, without rerunning clippy,
produced the complete stream and ratchet exit 0. Both outcomes are retained.
No source plants or restore scaffolds remain.

Corrected hosted baseline/head/plant execution and post-fix hosted-toolchain
lint remain unverified; `ac-hosted-map.md` distinguishes each required leg
from local/self-skip evidence. The shared-host uptime readings do not certify
exclusive-idle timing. No commit, push, status flip or kernel edit occurred.
