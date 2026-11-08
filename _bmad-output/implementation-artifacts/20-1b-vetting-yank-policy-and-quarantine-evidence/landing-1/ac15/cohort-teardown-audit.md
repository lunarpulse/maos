# Cohort yank-owner regression and complete composition-root teardown audit

Scope: `crates/maos-bin/src/main.rs` after `PollTask` creation at3246 through the final serving drain. Hosted finding42418118608 is a production regression, not a flake. `PollTask::start_local` legitimately retains the observer's scheduler/revocation/TL audit senders even without a remote registry. An audit-writer await cannot finish while that task owns them.

## Every explicit writer drain

All coordinates below are in the final formatted `main.rs`. Seven lexical sender-release sites cover eleven drains; five arms share the already-existing release macro.

| Arm | Poll-task join | Local sender release | Writer drain |
|---|---:|---:|---:|
| Shell, including captured shell/flush errors |4538|4566|4593|
| Topology `--once`, including interrupted/load/flush outcomes |5898|5938|5970|
| Standalone `cli_wrapper --once`, including captured completion failures |6120|6121|6137|
| Ordinary `run --once`, including interrupted/outcome failures |7013|7025|7060|
| `smoke-epic-4` | shared macro7104, invoked7650 | macro7105 |7651|
| `smoke-spirit-5` | shared macro7104, invoked7806 | macro7105 |7807|
| `smoke-supervision-5` | shared macro7104, invoked8021 | macro7105 |8022|
| `smoke-upgrade-revoke-5` | shared macro7104, invoked8318 | macro7105 |8319|
| `cohort-a2a-daemon`, success or daemon error | **9297, added** |9298|9327|
| `hello-spirit` one-shot tail | shared macro7104, invoked9571 | macro7105 |9572|
| Final serving SIGTERM/SIGINT tail |9708|9813|9845|

The structural witness `root_shutdown_unload_16_3.rs::every_audit_sender_release_joins_the_yank_owner`:67 parses the complete source with syn. It visits every named sender drop and writer await; a prior join is inherited only from the current lexical ancestry, not a sibling branch. It validates the opaque shared macro's join/drop contract and recognizes its invocations. Raw sender-release count and minimum existing drain count prevent a new opaque macro/release shape from disappearing silently. This is a structural contract for the named root owners, not a general Rust effect checker.

## Every early-return/panic family

- Ordinary Rust root returns and `?` propagation drop `PollTask`. Its existing `Drop` implementation (`crates/maos-bin/src/yank_policy.rs`:438-443) signals cancellation and aborts the task. This covers boot/provider/principal/readiness/manifest/admission refusals (main3930-6650), unknown and validation-only one-shot modes (7155-7453), smoke-unavailable/feature/refusal paths and helper-dispatch returns (7576-9427). No new Drop behavior is necessary. These paths do not await the root writer with a live poller.
- Returns inside successor-factory/Spirit callbacks and captured async bodies are not immediate root exits. The owning root remains live until its normal teardown above, or an outer propagated error drops the guard. Shell result/flush errors, topology load/interrupt outcomes, standalone completion failures, and ordinary once outcomes use their corresponding joined drains. The cohort daemon result is captured at9249-9273, then **both success and error** traverse the added join before its writer await; final unload-result returns are9338/9340.
- The supervision smoke's synthetic Spirit panic at7847 is handled by supervision and reaches the shared-macro drain. The unreachable erase-namespace branch at7250, fixture assertion panic at8694, and any other uncaught Rust unwinding panic drop the guard and abort its task. Abort-on-panic/process termination does not promise an audit flush.
- Explicit `std::process::exit` sites (operator/control/secret/provider setup3446-3896; eval4038; root-map4747; lifecycle7355; forget7381/7390; unavailable replay modes8496/8748/9015/9107) do not run Rust destructors. They terminate the process rather than awaiting the writer, so they cannot produce this retained-sender drain timeout. Their pre-existing fail-fast durability behavior is not claimed repaired. No broad root-lifecycle redesign is borrowed from AC15.

## Observed proof and applicability

The required original command `cargo test -p maos-bin --test root_shutdown_unload_16_3 -- --test-threads=1 cohort` exited0 with three results in0.00s. The explicit nocapture disposition records `SKIP: T3 cohort root shutdown Worker: signed test substrate not provisioned`. Earlier local workspace/prepush runs therefore did not exercise the hosted live-Worker case. This is not a locally observed live-Worker red.

The new **unconditional, non-T3** real cohort-daemon witness at2280 observed the exact10s writer timeout before the source fix (101, test10.34s). Removing the join as a plant produced the same timeout (101, test10.31s); the AST guard independently failed101 before and under the plant. Copy-back/cmp/touch all exited0. Restored complete test binary returned15/15 harness results in0.64s, with T3-dependent cases still explicitly skipped locally. No zero poll interval or timeout-message suppression is used.

The new declared-capability negative control at2314 runs the actual v5 test as a child with CI declared and the runtime absent. Its parent asserts explicit nonzero `declared-capable T3` refusal. The shared prerequisite already fails closed on capable Linux CI and was intentionally not changed. Real live-Worker repair remains hosted-unverified; the branch-only recipe uses the existing signed-T3 provisioning action and requires that case. Existing discipline coverage counts are updated to release10 + debug cohort5 =15. No per-binary test-budget exception was added.
