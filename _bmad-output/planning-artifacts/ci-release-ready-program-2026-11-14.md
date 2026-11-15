# CI release-ready program (plan, 2026-11-14)

Authority: `decisions-2026-11-14-release-ci.md` (operator rulings of 2026-11-14). This is a plan, not a tracker; the owning tracker key is `ops-ci-release-ready` in `sprint-status.yaml`. Measurements: green main discipline run 43571926114 on `650efcff` (183 jobs, 45,244 job-seconds, 59 min wall). Classification was done by reading `discipline.yml`, `xtask/gate-registry.toml`, job bodies and release documents; about 30 jobs were read from headers only (marked low confidence in the working notes). Verify each item before changing it.

## 1. Definitions
R release-critical: red on the tagged SHA means shipping something wrong, unsafe, unbuildable, unverifiable or mis-claimed. S release-relevant but slow or sensitive. H merge hygiene (process/consistency; the merge path already guarantees it). A advisory/measurement. D proven duplicate of another job. U unknown.

## 2. Structure
| Layer | Content | When |
|---|---|---|
| local `xtask prepush` | H and deterministic gates | before every push (exit 0 required) |
| merge gate (`aggregate`, smaller) | fast deterministic H, workspace tests, builds | PR and main push |
| `release-ready` (new fan-in check) | R set and the trusted S set | main push; read by `check-release-precondition` |
| nightly / weekly | A set, S set until fixed, timing profile of tests | scheduled; results recorded, trend alarms only |

## 3. Classification summary (45,244 s)
R 35 jobs 9,484 s; S 13 jobs 9,842 s; H 52 jobs 8,294 s; D 40 (20 proven) jobs; A 37 jobs 8,704 s; U 6 jobs 1,682 s.
Proven duplicates (20 jobs, 2,758 s): per-Spirit default-feature test jobs (`hello-spirit-tests`, `butler-tests`, `researcher-tests`, `observer-tests`, `mira-tests`, `nash-tests`, `orchestrator-tests`, `worker-tests`, `architect-tests`, `reviewer-tests`), `fr4-1000-call-fixture`, `manifest-field-coverage`, `retract-corpus-tests`, `safety-critical-corpus-8-5`, `one-daemon-one-door`, `maosctl-load-over-the-door`, `j0-scene`, `audit-spine-smoke` (also invalid evidence), `v01-evaluator-path` (historical gate), `check-mock-not-in-release` (release.yml runs the same check on every shipped binary). Basis: all are workspace members and the nextest default filter excludes only `maos-kernel-core` lib. NOT duplicates: jobs that use non-default features (`spirit_test`, `fixture_replay`): `example-spirit-tests`, `spirit-test-tests`, `lcas-corpus-tests`, `isolation-framework-tests`, `fr26-schedule-firing-corpus`, `adr-034-consent-rupture-corpus`.
Smoke family (16 jobs, 5,239 s, each rebuilds `maos-bin`): consolidate into one shared-build job instead of deleting.

## 4. Judgments for the two delegated decisions
Registry gates (blocking at v1_0 in the registry):
- keep in `release-ready`: `check-stability-matrix`, `check-breaking-md`, `check-export-control`, `check-ko-coverage` (STABILITY.md, BREAKING.md, RELEASE-HOLDS and NFR-Doc-6 make claims that depend on them; cheap and deterministic).
- merge path only (H): `check-fuzz-targets`, `fuzz-build` (the T3 chain already builds and runs the targets at the exact SHA), `check-env-contract`, `check-decision-register`, `check-loom-substrate-drift`, `check-ship-gate-completeness` (extend it to validate the `release-ready` membership), `check-model-currency`, `check-test-cost`, `check-test-budget` (flaky; 10 red runs), `check-exit-commands`.
- advisory (A): `check-perf-counts`, `check-perf-runner-policy` (no perf runner; the release notes label perf ABSENT and the perf ledger is judged by `check-perf-verdict`), `check-cert-rotation-trigger` and `windows-check` (registry says advisory, `docs/release/windows-deferral.md` says Windows is not supported for release; both are removed from `v1-0-ship-gate.needs`; the registry is authoritative).
Slow/sensitive (S):
- keep in `release-ready`: `nfr-rel-1-crash-detection-2s`, `nfr-rel-2-hang-detection-60s` (delete their retry-once), `nfr-rel-9-revocation-5s-p99`, the x64/ubuntu-24.04 leg of `cgroup-ceiling-*` and `wasm-host-tests`, `ci-test-producer` x64.
- nightly until fixed, then back after 10 consecutive green runs: `nfr-rel-11-halt-receipt-999pct`, `expensive-tests` (5 s shutdown bound and the 20 s erasure oracle must become deterministic first).
- path filter plus nightly: `example-spirit-ts-tests`; merge path: `multi-provider-drift-tests`, `check-j1-two-host-signed-run` (fixture-based verifier; align the release wording).
Other jobs removed from `aggregate.needs`: `reproducible-build` (continue-on-error; hashes debug rlibs on one host; no public reproducibility claim; keep as nightly), `nfr-perf-1`, `nfr-perf-j4-latency`, `nfr-perf-8`, `determinism-tests`, `clippy-advisory`.

## 5. Defects found while classifying (fix in W1)
- `audit-spine-smoke` checks a python-fabricated sqlite row with keys the FR4 surface does not emit (invalid evidence): delete.
- `server-exit-drain` exits 0 without testing when `ANTHROPIC_API_KEY` is unset: fail or use a fixture_replay one-shot.
- `nfr-rel-1` and `nfr-rel-2` retry the corpus once ("retrying once"): contrary to the no-retry-to-green ruling; delete the retry and fix causes.
- `mira-nash-bench` runs a bench with `required-features = ["kernel_measurement"]` without the feature (verify).
- `xtask/gate-registry.toml:57-59` says advisory gates are absent from `aggregate.needs`; false.
- `check-cert-rotation-trigger` and `check-exit-commands` are `advisory` in the registry but block through `v1-0-ship-gate.needs`; `windows-check` likewise.
- `docs-site.yml` fails because `ubuntu-latest` became ubuntu 26.04 and Playwright does not support chromium there; pin runners (release, docs-site and every `ubuntu-latest` use) to an explicit image.
- `container.yml` fails on pre-release tags: `SHA256SUMS-packages` lists `maos-1.0.0~rc.2-1.*.rpm` / `maos_1.0.0~rc.2-1_*.deb` but GitHub stores the assets with `~` replaced by `.`, so name verification fails. Normalise package names for pre-releases; confirm GA (`1.0.0`, no tilde) is unaffected.
- Unretried network installs (`apt-get`, `pip`, `cargo install cargo-fuzz`/`cargo-public-api` from source) are infrastructure flake sources: use pinned prebuilt tools with caching.

## 6. Waves (execute after each previous wave is green; every wave runs local prepush first and a hosted discipline run)
W0 housekeeping: delete the seven merged branches (`epic-11`, `epic-12`, `epic-13`, `epic10`, `recovery-lane`, `story-8-16-epic-9-readiness-bridge`, `story/1a-5-migrate-abi-diff`; verified 0 branch-only commits), keep `ci/*` branches until archived by tags, never touch `fuzz-ledger`/`rto-ledger`; pin runner images; `docs-site` runner; minor fixes (unused `CARGO_NEXTEST_VERSION`, `--locked` on bench, input-injection via `env:`, CODEOWNERS format, missing `timeout-minutes`, `concurrency` groups, the two 03:00 crons).
W1 remove the 20 proven duplicates, fix the defects in section 5, drop retries, take `reproducible-build` and the advisory jobs out of `aggregate.needs`, align registry dispositions; update `aggregate.needs`, `v1-0-ship-gate.needs` and the xtask tests that pin job names (`ci_shape_18_0.rs` etc.) in the same change.
W2 consolidate the deterministic xtask gates (about 20 jobs, about 3,000 s) into one hosted job keeping per-gate ledger names; build `maos-bin` once for the smoke family.
W3 move the A set and the not-yet-trustworthy S jobs to nightly; split nextest profiles so timing-bound tests (`root_shutdown_unload_16_3`, `t3_terminal_helpers_17_7`, `erasure_million_20_11`) run in a timing profile; paired-delta evidence stays blocking, absolute p95 becomes advisory with recorded numbers.
W4 add the `release-ready` fan-in, switch `check-release-precondition` to it for rc (GA strict), rehearse with `workflow_dispatch dry_run=true` on the main tip.
Expected effect (estimate, not measured): release-ready about 12,000-15,000 job-seconds (27-33 percent of today); wall time bounded below by `workspace-test-suite` (about 32 min) or the producer-fed `epic-20-exit` chain (about 43 min); the larger gain is fewer ways to fail. Re-measure after W1.

## 7. Open verification (before W1)
Low-confidence jobs (`nfr-rel-3/4/9/10`, `upgrade-policy-corpus`, `nfr-aud-7`, `t3-smoke-busybox`), which hosts the release claims for the cgroup/wasm legs, whether the aggregate/ship-gate needs lists match this plan, and the GA impact of the package-name issue.

## 8. Execution record (2026-11-15, orchestrator)
Waves W0-W4 are landed on main (W0 `f9d5f3e3`, W1a `9d939d56`, W1b+docs `b51d0091`, W2+W3 `9b43ab1b`, W4 `6a4bda5c`), each with local `xtask prepush` exit 0 and a hosted discipline `aggregate` success on the same SHA.

**Corrections to this plan, found by review:** (1) section 4's statement that `windows-check` and `check-cert-rotation-trigger` leave `v1-0-ship-gate.needs` is WRONG: their registry `binding = "blocking"` (disposition and binding are different fields) and `CURRENT_PHASE` is `v1_5`, so they stay in the merge path; the merge/release split is `release-ready` (W4). (2) The plan expected `mira-nash-bench` to run a feature-gated bench without the feature; it runs `section_13_1`, byte-identical to `founder-loop-bench` and `researcher-bench`, so both duplicates were removed. (3) `example-spirit-ts-tests` stays in the merge path: `wasm-host-tests` needs its artifact (deferred-work row).

**Measured (hosted, same method: sum of job durations; matrix legs counted):**

| Run | Jobs | Job-seconds | Longest job |
|---|---:|---:|---|
| baseline 43571926114 (`650efcff`) | 199 | 45,244 | workspace-test-suite 31.7 min |
| W1a 43748593387 | 177 | 40,819 | workspace-test-suite 31.1 min |
| W2+W3 43755858681 | 129 | 29,349 | workspace-test-suite 30.9 min |
| W4 43763468882 (`6a4bda5c`) | 130 | 28,311 | workspace-test-suite 32.4 min |

Job-seconds fell 37 percent. The critical-path job did not move (about 31-32 min); end-to-end wall time (59-75 min) is dominated by hosted runner queueing (up to 39-52 min before a job starts), not by job length, so wall time was not improved by these waves.

**Rehearsal of W4:** `release.yml` `workflow_dispatch dry_run=true` on `6a4bda5c` (run 43769873728): builds, signing and deb/rpm steps green; classified `rc`; the only refused guard was the T3 fuzz chain for this SHA (none was dispatched, ~26 h), as designed. The `release-ready` leg raised no refusal, but the precondition did not reach a final PASS because of the T3 refusal. Nothing was published.

**Open:** `nfr-rel-11` and `expensive-tests` (nightly) must be re-promoted before GA; GA perf wording conflict (decisions-2026-11-14 vs B1) needs an operator reading; the shared-runner queueing; `ci-nightly.yml` has not yet run on hosted; the new `xtask-gates` and `smoke-suite` jobs have one green hosted run each.
