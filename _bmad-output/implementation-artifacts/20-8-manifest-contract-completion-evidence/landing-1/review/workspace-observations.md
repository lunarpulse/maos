# 20-8 L1 review-correction workspace observations

One prescribed default-concurrency workspace nextest run with metadata reuse, UUID `c97ac228-c921-4e4b-8607-8236b01ef968`, exited 100: 5,835 passed, 19 failed, 137 skipped; see `workspace-nextest.log` and the same-run `workspace-junit.xml`. No workspace retry.

- Two erasure verifier tests refused because this agent omitted the hash-locked Python `cryptography` dependency from PATH; subsequently `/tmp/maos-208-verifier` was provisioned with `python3 -m venv --system-site-packages` and `pip install --require-hashes -r tools/verify-erasure/requirements.txt` (exit 0) for the separately prescribed prepush, without erasing this red.
- The `maosctl` freshness failures resulted from this agent touching the CLI input after fixture provisioning. `xtask build-test-fixtures --only maosctl` subsequently exited 0; this is preparation correction, not a claim that the failed tests passed.
- The FKCS test detected the intended 31-line skill admission change. Independent `fkcs-skill-limits-review.md` approved re-pin to `3dc4d08e237fffb6338354d71fc504a7d4a20217230237471e13f7cbaec349f9`; registry admission unchanged. The catalog failures detected two missing skill error entries; registry, generated JSON, route manifest and error pages were added after this run.
- The existing million-row pending completion oracle failed in 41.748 s; functional owner 20-11, timing/I/O owner 20-7. Cause not proven; no retry or bound change.
- `xtask check-test-budget --junit target/nextest/ci/junit.xml --json` exited 1: five unchanged 30 s binary budgets exceeded: erasure-uninstall 30.437, hook-pump 32.368, one-door 36.515, participant-recall 41.060, xtask binary 30.424 s. Total 1104.886 s. See same-run JUnit; no exception or budget change.
