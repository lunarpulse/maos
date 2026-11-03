# 20-10 hosted incident findings

## Branch-only run 41584515474

All four scene legs failed at harness setup, before any scene test executed:

| Job | Saved log | Failure line |
|---|---|---:|
| Ubuntu 24.04 head | `job-125749866755.log` | 4058 |
| Ubuntu 26.04 head | `job-125749866883.log` | 4058 |
| Ubuntu 24.04 baseline | `job-125749866879.log` | 4061 |
| Ubuntu 24.04 plant | `job-125749866891.log` | 4059 |

Every line says: `cgroup-ceiling harness: unit must be a named maos-17-2-* or maos-17-7-* service without a unit-type suffix`. The cut supplied `--unit maos-20-10`; its restore command likewise supplied `maos-20-10-restored`. The existing allowlist in `scripts/run_cgroup_ceiling_17_2.py:385-388` is correct and remains unchanged. The cut now uses `maos-17-7-purge-20-10` and `maos-17-7-purge-20-10-restored`.

Neither baseline nor plant is an intended proven-red witness in this run. Their assertion-message guards correctly rejected the unrelated setup failure. No production removal, dry-run, custody or delegated cgroup assertion was reached, so this run cannot diagnose those behaviors.

## T0 results: no ruled-decision contradiction

Both T0 jobs passed. `t0-measurements.json` retains parsed actual outputs; the complete source logs retain argv/stdout/stderr.

| Observation | Ubuntu 24.04: `job-125746775660.log` | Ubuntu 26.04: `job-125746775593.log` |
|---|---|---|
| Podman version | 4.9.3, line 195 | 5.7.0, line 195 |
| Docker version | 28.0.4, line 196 | 29.4.2, line 196 |
| Docker `create --pidfile --cidfile` | rc 125, `unknown flag: --pidfile`, line 197 | Same, line 197 |
| Mount source `/workspace/./` inspection | normalized absolute source without `/./`, line 201 | Same, line 201 |
| Repeated spirit/nonce label filters | only the both-labelled CID, line 202 | only the both-labelled CID, line 202 |
| Forced removal incl. running CID | rc 0, lines 204/206/208 | rc 0, lines 204/206/208 |
| Post-removal `container exists` | rc 1, lines 205/207/209 | rc 1, lines 205/207/209 |
| Repeated force removal of absent CID | rc 0, empty stdout, lines 210-212 | Same, lines 210-212 |

These measurements support, rather than contradict, D-20-10-06 option A (no Docker cleanup path). No re-ruling or production runtime workaround is justified. Delegated creator-PID/custody behavior was not exercised by the failed scene setup.

The hosted-toolchain lint leg passed on Rust 1.99.1. `job-125746775456.log:2135` reports **1381 occurrences / 896 source-distinct**, beneath the unchanged 1383/897 baseline; line 2144 records the new-warning comparison result **`[]`**. This proves the submitted pre-incident tree, not later unexecuted revisions.

## Main run 41584453971

See `main-run-classification.md` and copied `main-{workspace,nfr-rel-11,loom,epic17-24,epic17-26,budget}.log` for the authoritative classification and line citations. Two new 20-10 regressions are distinct from known teardown flakes:

- The review P1 allocation helper unintentionally lengthened durable egress socket roots as well as volatile custody roots. Reconstructed hosted paths grew from 94-107 bytes to 108-121 bytes, exceeding Linux pathname AF_UNIX capacity. The failing Workers never reached their startup oracle; budget failures are consequences, not a reason to widen budgets.
- The terminal purge fixture set `HOME == MAOS_HOME`. Production correctly refused to purge HOME. The fixture must isolate user HOME from its purgeable child MAOS_HOME; the refusal must remain.

Both epic-17 images actually ran 34 tests (28 passed, six failed, zero skipped): the 34-count pin is correct and must not be lowered. The production fix is confined to choosing short nonce-only durable egress roots while preserving PID-bearing volatile custody roots; test diagnostics must capture root stdout/stderr instead of an empty context.
