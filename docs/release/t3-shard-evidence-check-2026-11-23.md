# T3 shard evidence check (WP-16 premise, decision 9), 2026-11-23

## Finding: the precondition does NOT require serial links or corpus hand-off. WP-16 proceeds.

Evidence (worktree `ci-release-evidence-2026-11-23`, base `eba34d1f`):

- `xtask/src/check_release_precondition.rs:115-119`: `T3_TARGETS` (3), `T3_LINKS = 5`, `T3_SECONDS = 86_400`;
  a chain is "links 1..=5 of ONE workflow run (`run_id` = `<run>-<link>`)".
- `check_release_precondition.rs:159-177`: per target, records are filtered by `tier == "T3"` and
  `commit == sha`, grouped by run id prefix, keyed by link number 1..=5; the check is
  `links.len() == 5 && links.values().sum() >= 86_400`. No timestamp, ordering, `needs`, or corpus
  field is read; link N is never compared with link N-1.
- `scripts/run-fuzz-link.sh:31-33` and `scripts/record-fuzz.py` write each record independently
  (`--run-id "$GITHUB_RUN_ID-$link"`, `--commit "$GITHUB_SHA"`, measured libFuzzer seconds); a crash or
  worker failure exits 1 before any record exists (`run-fuzz-link.sh:27-30`).
- `fuzz-prerelease.yml` `t3-ledger-collect` (unchanged) asserts all five link jobs `success`, five records per
  target at `GITHUB_SHA`, `sum(cpu_seconds) >= 86400`. Order is not asserted.
- PRD `prd/non-functional-requirements.md:38` (NFR-Sec-5): "24h cargo-fuzz, zero crashes/OOMs/infinite loops";
  NFR-Sec-6 is the same for the wire protocol. Neither says serial or continuous corpus.
  The serial shape was only the workflow's own `needs: fuzz-link-(N-1)` plus an artifact hand-off.

Conclusion: the evidence form is five records of one run on the exact commit, >= 86,400 measured seconds per
target, zero crashes. Parallel shards satisfy it.

## What changed

- `.github/workflows/fuzz-prerelease.yml`: removed `needs:` of links 2-5 and their "Resume previous link corpus"
  downloads (all five links start from the checked-in seed corpus); added `corpus-merge` (6-target matrix,
  `needs` all five links, unions the five `t3-corpus-<target>-<n>-<run>` artifacts, minimizes with
  `scripts/minimize-fuzz-corpus.sh`, uploads `t3-corpus-merged-<target>-<run>` as the next chain's seed).
  `t3-ledger-collect` and `check_release_precondition.rs` untouched.
- Deviation from the plan text: the plan said `cargo fuzz cmin`; the repo's own minimizer header
  (`scripts/minimize-fuzz-corpus.sh`) explains why `cmin` is deliberately not used (unguarded swap, no time bound),
  so the guarded libFuzzer `-merge=1` script is used.
- `xtask/tests/ledger_collectors_20_3a.rs`: job count 6 -> 7 and overwrite-upload count 15 -> 17 (the two new
  `corpus-merge` uploads, both `overwrite: true`). Refusal vectors in `release_precondition_t3_20_5.rs` are untouched.
- `docs/release/tag-procedure.md` step 2: wall time about 6 h (INFERENCE, unmeasured), up to 30 concurrent runners.
- Not wired: consuming the previous chain's merged corpus as a seed (a cross-run download) is not automated;
  the artifact is kept 90 days for the operator to commit or fetch.

## Correction: corpus hand-off (run 45152159643, SHA e51b6877, 2026-11-23)

The first chain ran the fuzz steps with zero crashes, but `Save corpus for next link` (upload-artifact on a
directory of 597,314 `frame_deser` files) died with `Maximum call stack size exceeded`, and the record step
behind it was skipped, so the links had no ledger record. Fix (workflow only; the evidence rules and
`check_release_precondition.rs` are unchanged):

- Every link packs its corpus directory into ONE `t3-corpus-<target>-<n>.tar.zst` (`tar | zstd`, sorted, mtime 0,
  file count, byte size and sha256 in the step log) and uploads that single file (`compression-level: 0`).
  `corpus-merge` downloads the five archives, extracts and unions them, minimizes with the guarded
  `scripts/minimize-fuzz-corpus.sh` (cap 2400 s, step timeout 60 min) and uploads ONE
  `t3-corpus-merged-<target>-<run>` archive (`t3-corpus-merged-<target>.tar.zst`; unpack with
  `tar --zstd -xf <file> -C crates/<crate>/fuzz/corpus/<target>`). On a minimizer timeout the union is kept
  unminimized and is archived as is.
- Step order per link: fuzz, minimize, **record**, pack, upload corpus, logs. The record step has no `if`, so it
  runs exactly when the fuzz step and the minimizer (which fails on a confirmed crash) succeeded; a failed or
  crashed fuzz step still yields no record, and a pack/upload failure after it can no longer skip it. The job
  still ends red when the hand-off fails.
- `corpus-merge` was already NOT a `needs` of `t3-ledger-collect` (needs: the five links only) and stays so; its
  failure is red on its own job plus a warning and a step-summary line, and cannot void the evidence.
- Hosted smoke: `.github/workflows/t3-corpus-smoke-tmp.yml` (temporary, not merged) runs the same pack/upload and
  download/extract/merge step text against 600,000 synthetic files.
