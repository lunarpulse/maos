**Verdict: NO-GO as-is. The old failure is not a real defect, but a few late-failure risks are cheap to fix first.**

## (a) Old failure, run 38926626626
- **It was a deliberate plant, not a product, infra or script failure.**
  - Branch `ci/20-3a-plant-fuzz-crash`, commit title "20-3a T13 PLANT (never merge): AC3.5 planted crash input in manifest_parser; 30 s T3 links; ledger redirected".
  - The run used 30-second links (`run-fuzz-link.sh ... manifest_parser 30 T3 1`). The sha `c36857c8` is not in the local repo and sits on that branch only.
- **`fuzz-link-1 (manifest_parser)` failed as designed.**
  - Four workers hit `thread '<unnamed>' panicked at fuzz_targets/manifest_parser.rs:25:5`, then `ERROR: libFuzzer: deadly signal` and `Fuzz target exited with exit status: 77`.
  - The crash input was `crash-32fbc617…`, whose content is the text "20-3a-planted-crash".
  - The script then printed `##[error]T3 manifest_parser link 1 crashed or a worker failed; refusing evidence` (`run-fuzz-link.sh:27-29`).
- **Links 2–5 were skipped.** `t3-ledger-collect` then failed at `fatal: couldn't find remote ref refs/heads/fuzz-ledger-20-3a-proof-absent`, because the plant redirected the ledger branch. Neither failure applies to HEAD.
- **The other two targets succeeded.** `frame_deser` and `fuzz_exec_deps_parse_elf` link 1 finished and uploaded their `t3-record-*` artifacts, which the collector downloaded.
- **Fixed on HEAD?** Nothing to fix. The plant is absent from main, and the workflow no longer redirects the ledger branch (`scripts/collect-ledger.sh:5-10` hard-codes `fuzz-ledger`).
- **No full successful chain has ever run.** Only link 1 at 30 s has been exercised.

## (b) Late-failure risks, ranked

1. **Main moving during the ~26 h chain.**
   - Records carry `GITHUB_SHA` (`run-fuzz-link.sh:32`). The precondition requires `commit == aggregate head_sha`, and `tag-procedure.md:42-50, 204-205` requires the tag to sit on the `origin/main` tip.
   - Any push to main after dispatch makes the chain unusable for the new tip.
   - Fix: freeze main (or tag the dispatched SHA) before dispatching.
2. **Corpus growth on `frame_deser` (hand-off and disk) is untested.**
   - T2 does not upload corpora, so the corpus upload/download between links has never run on real data (workflow lines 44-51, 87-91 and equivalents).
   - T2 night run 43004514422 ended with `corpus_size` 409,464 files for `frame_deser`, about 83 bytes each; my local 60 s sample showed 10,172 files, 40,688 KB on disk against 846 KB apparent size.
   - Each file takes a 4 KB block, so 2 M files would use about 8 GB of disk. Zipping, uploading and extracting 0.5–2 M files per link may be slow.
   - [INFERENCE] I did not measure the T3 growth rate or runner free disk.
   - Fix: run `cargo fuzz cmin` (or a libFuzzer `-merge=1` pass) before the "Save corpus" step, and cap what is uploaded.
3. **A crash or failure in any one target blocks later links for all three targets.**
   - `needs: fuzz-link-N` waits on the whole matrix. `fail-fast: false` only stops matrix siblings from being cancelled.
   - A failed link costs about 5 h, not 26 h, because `gh run rerun <id> --failed` reuses the run id. Artifact names use `run_id` and `overwrite: true`. The collector dedupes identical records, and the precondition keeps the larger value per link.
   - The caveat is that link N must fail before its corpus step for a clean rerun.
4. **`manifest_parser` has the default 2048 MB RSS limit.**
   - Peak RSS in T2 was about 840 MB from an empty corpus. Loading a large carried-over corpus in each of 4 workers could raise it in links 4–5.
   - `frame_deser` uses `-rss_limit_mb=0`, so a runner out-of-memory kill is possible there instead.
   - Peaks there were about 715 MB from empty.
   - Fix: if watching, check the link-3 log for `peak_rss_mb`.
5. **Job timeout headroom is adequate but untested at scale.**
   - The fuzz run is 18,000 s (300 min) against a 355 min job timeout, leaving 55 min.
   - T2 setup (nightly install plus `cargo install cargo-fuzz --locked`) took 20–32 s, and the build 33 s to 1 min 13 s.
   - The risk is the corpus upload and download in item 2.
6. **Policy note on the 24 h rule.**
   - `cpu_seconds` is a sum over workers: 4 workers × about 18,001 s × 5 links ≈ 360,000 s.
   - The 86,400 s bar is therefore passed after roughly 2 links. The "five links" rule, not the seconds rule, is what makes the chain long. This is a policy point, not a bug.
7. **Fuzz lockfiles were already stale, not broken by the bump.**
   - `crates/maos-domain/Cargo.toml:26` has had `rustix` since commit `bb9f0657` (story 16-1), and the committed locks never included it.
   - The workflow never uses `--locked` for fuzz builds, and `cargo fuzz build` adds the missing packages. T2 nightly runs succeed on that basis.
   - The consequence is that `rustix`, `errno`, `bitflags`, `windows-sys` and `linux-raw-sys` resolve to the latest compatible version on each link, not pinned.

## (c) Collector ↔ precondition contract: matches

- **Record:** `record-fuzz.py` writes `target`, `cpu_seconds`, `corpus_size`, `timestamp`, `tier`, `commit`, `workers`, `run_id`, with `run_id` = `<run>-<link>`. The precondition (`xtask/src/check_release_precondition.rs:99-106`) needs `target`, `tier`, `commit`, `cpu_seconds`, `run_id`.
- **Rules agree:**
  - Targets and tier: both use the same three targets and tier `"T3"`.
  - Run and link: `rsplit_once('-')` yields run and link, with links 1–5.
  - Seconds: at least 86,400, summed within one run, using the max for a duplicate link.
  - SHA: the record SHA is `GITHUB_SHA`, the dispatch SHA when dispatched with `--ref main`. It is compared with the aggregate check-run `head_sha`.
- **Push path:**
  - It uses `secrets.FUZZ_LEDGER_WRITE_TOKEN || secrets.GITHUB_TOKEN` (workflow line 304). That secret is not in the repo's secrets (`CI_EVIDENCE_AUDIT_KEY`, `MAOS_RELEASE_PUBKEY`, `RELEASE_SIGNING_KEY` only), so `GITHUB_TOKEN` is used with workflow-level `contents: write`.
  - The `fuzz-ledger` branch has no protection and there are no rulesets, so the push is allowed.
  - Ledger tip `1fde6048` is a `github-actions[bot]` T2 append from 2026-11-11 (45 T2 records, 0 T3).
- **Not exercised:** the T3 push itself.

## (d) Lockfile and smoke

- **Lockfile check, in the main tree** (`cargo metadata --locked --offline --format-version 1 --manifest-path crates/<c>/fuzz/Cargo.toml`):
  - `maos-manifest`: exit 101, `cannot update the lock file … because --locked was passed`.
  - `maos-domain`: exit 101, same message.
  - `maos-exec-deps`: exit 0.
  - Without `--locked`, each failing lock gains 51 lines (the `rustix` dependency tree). The exit-101 result reproduces in a fresh clone. This matters only if someone adds `--locked` to the fuzz builds.
- **Local 60 s smoke,** run in a scratch clone at HEAD `23b2200b`.
  - Toolchain: `nightly-2026-09-25` is not installed, so I used installed `nightly` (rustc 1.100.0-nightly, 2026-09-24) and cargo-fuzz 0.13.2.
  - Environment: `taskset -c 0` (one worker) with the workflow's `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0`.
  - Command: `bash scripts/run-fuzz-link.sh <dir> <target> 60 T3 1 <rss_flag>`.
  - Results, all exit 0, all `"tier":"T3"`, `"commit":"23b2200b…"`, `"run_id":"999-1"`, `"workers":1`, `"cpu_seconds":61`:

| target | corpus_size |
|---|---|
| `manifest_parser` | 503 |
| `frame_deser` (`-rss_limit_mb=0`) | 10172 |
| `fuzz_exec_deps_parse_elf` | 180 |

  - The clone and `/tmp/t3audit-*` scratch are deleted; the main tree was not written.

## (e) Recommendation

- **NO-GO as-is: freeze main and measure corpus growth first (items 1–2).**
- **Before dispatching:**
  - Freeze main for about 26 h (item 1).
  - Add corpus minimization or a size cap before "Save corpus", or measure free disk and corpus growth in a 3-link dry run (item 2).
- **Verified:** the record contract, the push permissions, the T2 evidence (10 recent runs, 9 success; the 2026-11-12 10:46 run was still in progress), and the single-run smoke.
- **Unverified:**
  - Links 2–5 and the corpus hand-off.
  - The T3 ledger push by `GITHUB_TOKEN`.
  - The `t3-ledger-collect` assertion step end to end.
  - The real wall time. The estimate is about 25.5–26 h from 5 × (300 min fuzz + a few minutes setup + corpus transfer).
  - Exact toolchain parity (the local nightly is a substitute).