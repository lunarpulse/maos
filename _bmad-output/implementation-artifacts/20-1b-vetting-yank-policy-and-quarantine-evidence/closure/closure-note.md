# 20-1b closure note (record-only), 2026-11-12

Story `20-1b-vetting-yank-policy-and-quarantine` closes under operator ruling `q20` (`_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 11), which supersedes the ruling-`q` clause "20-1b stays not `done`" for the rc. The quarantine gaps are carried by the new NON-`20-*` backlog key `later-registry-quarantine-closure` in `sprint-status.yaml`. No code, kernel file or `docs/invariants/*.md` was touched by this closure.

## Claim text (verbatim; release notes carry it)

> FR13 quarantine = capability-mediated fence only (issue/verify/recheck: MCP, inference, memory-by-token, IAC-by-token); private/shared memory writes, scalar mutation, DRR/native IAC/IO, hooks and telemetry are NOT fenced; per-pid in memory, shed by restart/unload/swap/reload; a consumed yank row is never re-applied (CRL path is durable); no operator release verb; `fenced` readback conflates pause and quarantine; not isolation (ruling `q`). I5 FR13 remains `asserted`.

PRODUCT FINDING added by the hosted AC11.5 leg: **a CRL cannot quarantine a Worker.** `worker_spawn.rs:919-922` binds a Worker with `..Default::default()`, so its SCB has no `[class]`; the CRL arm in `revocation/applier.rs` skips class-less SCBs (hosted record: `worker_scb_declares_class: false`, `crl_matched_count: 0`); the yank policy needs an installed package plus a class-bearing SCB (`yank_policy.rs` `reevaluate_at`). The AC11.5 test fences the Worker through `set_quarantined(pid, true)`, the primitive the CRL arm calls, and names that route in its record. "Quarantined by CRL" is NOT proven for a Worker.

## AC table

| AC | State | Evidence |
|---|---|---|
| AC6 vet binding + CLI | LANDED | `62faff63`, `c656a6e7`; `vetting_attestation_gate.rs:411,415`, `vet_lifecycle_20_1b.rs:8`; hosted 158/158 |
| AC7 signed yanks, restart-safe | LANDED | `yank_auth_20_1b.rs`, `yank_cadence_20_1b.rs` |
| AC8 yank policy | LANDED | policy `62faff63`; quarantine effect `608735d2`/`a2bbd687`; hosted T3 group 16/16 includes 12 `yank_policy_20_1b` tests |
| AC9 vetter lapse/revocation | LANDED | `yank_policy_20_1b.rs` |
| AC10 squatting | LANDED | `yank_auth_20_1b.rs:69`, `yank_policy_20_1b.rs:318` |
| AC11.1-11.4 kernel fence | LANDED, narrowed by `q` | kernel quarantine vectors 5/5 hosted (4x `quarantine_fr13_20_1` + `on_revocation_three_actions`) on both images |
| AC11.3 I5/coverage/arch text | PARTIAL / CLAIM-BOUNDARY | coverage matrix, ADR-056, SECURITY.md, `4-kernel-design.md` narrowed; I5 FR13 stays `asserted` (ADR-037), nothing to flip |
| AC11.5 hosted T3 leg | LANDED (CRL route NOT proven) | run 43184858544 jobs 131069396042, 131069396078: fenced live Worker, next MCP issue, inference issue and held-token verify each `Err(Revoked)`; `container_activity_continues: true` (4 heartbeats in 4 s, container still running, later `stop_workers` removed it); plant removing the fence fails both AC11.5 tests on both images |
| AC11.6 pins/decision record | LANDED | `20-1b-flag-winston-decision.md`, `608735d2`, `a2bbd687` |
| AC12 poller prints once | LANDED | `one_daemon_one_door_16_1.rs::ac12_20_1b_a_failing_revocation_poll_is_printed_once`; hosted green |
| AC13 ledger | LANDED | deferred-work rows stamped/re-owned/dispositioned: D:37 (now ~156), D:56 (FR59, ~175), D:1093 (~1213), D:1264 (~1389) CLOSED; D:1147 (FR13, ~1267) re-owned to `later-registry-quarantine-closure` (rule-10 re-own, not closed, by `q20`) |
| AC14 immutable name@version | LANDED | `spirit_install_20_1.rs:307` |
| AC15 first contact + transport | LANDED, hosted | TLS/dev/cohort teardown 158/158 + 54 required markers on ubuntu-24.04 (131069395919) and 26.04 (131069395836); public-WebPKI 131069395551; macOS native 131069395957 |
| AC16 yanked install refused | LANDED | `spirit_install_20_1.rs:404` |
| AC17 mutation auth | LANDED | `yank_auth_20_1b.rs:12` |

## Hosted proof

Branch-only run [43184858544](https://github.com/lunarpulse/maos/actions/runs/43184858544), `ci/20-1b-verify` @ `6c59aa46a29c47ce8585d1e33e5150b8e6cfc373`, 7/7 success (re-read with `gh run view` at closure; conclusion `success`, headSha matches).

| Job | id | Result |
|---|---|---|
| kernel-free-trust (ubuntu-24.04) | 131069395919 | success; 158/158, 54 markers |
| kernel-free-trust (ubuntu-26.04) | 131069395836 | success; 158/158, 54 markers |
| final-tree-vectors (ubuntu-24.04) | 131069396042 | success; kernel vectors 5/5, T3 group 16/16 |
| final-tree-vectors (ubuntu-26.04) | 131069396078 | success; kernel vectors 5/5, T3 group 16/16 |
| public-webpki | 131069395551 | success |
| registry-tls-macos | 131069395957 | success; aarch64-apple-darwin check, `registry_limits_20_1b` 4 tests, 3 TLS runtime cases |
| epic-20-exit | 131069396074 | success |

T3 group (16): 12x `yank_policy_20_1b`, AC12 one-door test, 20-11 Worker variant `ac6_running_t3_worker_uninstall_proves_live_inventory`, both AC11.5 tests. Run 1 (43180252358) failed only on recipe bugs (macos-14 refused -> macos-latest; test count 156 -> 158; plant step under `bash -e`), fixed in `6c59aa46`; evidence under `ci/`.

## Honest caveats

- `prepush` on the final tree was NOT run. The hosted discipline run 43175946640 (21-5 L2, a descendant of `608735d2`/`a2bbd687`) is green 199/199 except the advisory reproducible-build job; that is the discipline evidence cited, not a prepush.
- macOS: only the native check, `registry_limits_20_1b` and 3 TLS runtime cases ran; the whole workspace on macOS is discipline's `macos-check`.
- The live-container plant is hosted-only; the local T3 test SKIPs (no podman clone). Local container-less AC11.5 test and its plant are recorded under `ci/`.
- FR13 is a capability-mediated fence only. Container activity continues after a fence. No isolation is claimed. I5 FR13 remains `asserted`.
- The gaps (durable/load-time yank quarantine, memory/scalar fences with an UNMEASURED +30..+80 tokei kernel forecast, release verb + distinct readback, CRL-reaches-Worker, F-4 poller error print, KQ2, ADR-037 flip) are open on `later-registry-quarantine-closure` and are not delivered.
- Retained explicit limits (W1, W3, W5, W6, W7, D2/D3, F8/F10, search O(N x M), key-lifecycle rows) are ownerless-and-open rows dispositioned by `q20` and rule 10; no story schedules them.
- The three kernel counter figures quoted in the story history (26,062/20,429; 26,967/21,141; 27,009/21,178) were not reconciled by this record.
