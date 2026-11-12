# Pre-tag T3: corpus hand-off minimization (2026-11-12, `FixT3Corpus`)

Fixes item 2 of `t3-chain-audit.md`: links 2-5 of `fuzz-prerelease.yml` carry the corpus by artifact; a 4 h T2 run
ended with 409,464 `frame_deser` files, one 4 KB disk block each. Nothing in the repo had ever exercised that
hand-off at scale.

## Change

- `scripts/minimize-fuzz-corpus.sh <fuzz_dir> <target> <rss_flag> [timeout_seconds]` (new). Logs files / `du -sh` /
  `df -h .` before and after, then runs `cargo fuzz run --fuzz-dir <dir> <target> <fresh> <corpus> -- -merge=1 [rss_flag]`
  under `timeout -k 30 <limit>` (default **1500 s**). The corpus is replaced by the merged directory only when the merge
  exits 0 and the new directory holds at least one file; every other outcome (timeout, merge failure, missing
  target/corpus dir, empty result, failed move) keeps the original, prints `::warning::` and exits 0. The merge output
  and control file live in `<fuzz_dir>/corpus-minimize-<target>/` (a sibling of `corpus/`, outside the uploaded path,
  removed by an exit trap). Its log goes to `out/minimize-<target>.log`, merge crashes (if any) to
  `out/minimize-crashes-<target>/`; nothing else under `out/` (records, `logs-*`, `crashes-*`) is read or written.
- `cargo fuzz cmin` was rejected after reading cargo-fuzz 0.13.2 `exec_cmin` (`src/project.rs`): it merges into a temp
  dir but then swaps with two unguarded `fs::rename`s (corpus -> `old`, `corpus` -> corpus), a kill between them
  leaves no corpus; it has no time bound; and it swaps even when the merge produced nothing.
- `.github/workflows/fuzz-prerelease.yml`: step `Minimize corpus before hand-off` (same `env` as the fuzz step) between
  the T3 fuzz step and `Save corpus for next link` in links 1-5 (`logs-corpus-minimize/workflow.diff`, +25 lines). No
  `if:`, so it is skipped when the fuzz step failed. The upload count is unchanged (15), triggers, permissions, link
  seconds, worker counts, artifact names, collector and assertion script untouched. No `df` logging was added to the
  fuzz step: the measured disk figures below do not call for it.

## Measurements (all from the logs in `logs-corpus-minimize/`)

Substitutions: the pinned `nightly-2026-09-25` is not installed locally; `RUSTUP_TOOLCHAIN=nightly`
(rustc 1.100.0-nightly 2026-09-24) and cargo-fuzz 0.13.2 were used. Local machine: 32 cores, so the "link" used 32
workers (a hosted runner has 4). `GITHUB_SHA`/`GITHUB_RUN_ID` were dummy values in a scratch clone of the repo.

| target | grow step | files before | du before | merge wall | files after | du after | logs |
|---|---|---|---|---|---|---|---|
| `frame_deser` | `run-fuzz-link.sh ... 600 T3 1 -rss_limit_mb=0`, 32 workers, `cpu_seconds` 19324 | 493,838 | 1.9 GB (79 MB apparent) | **690 s** | 22,879 (4.6 %) | 90 MB | `frame_deser-link1.log`, `frame_deser-pre-minimize.txt`, `frame_deser-minimize.log` |
| `manifest_parser` | 120 s, 32 workers, `cpu_seconds` 3872 | 13,719 | 54 MB | 17 s | 1,190 | 4.7 MB | `other-targets.log` |
| `fuzz_exec_deps_parse_elf` | 120 s, 32 workers, `cpu_seconds` 3872 | 18,361 | 72 MB | 1 s | 180 | 720 KB | `other-targets.log` |

- Whole-script wall for the `frame_deser` case was 11 m 42.7 s (shell `time` of the command; not in a log file), i.e. about
  25 s for deleting the 493,838 replaced files on top of the 690 s merge. The 600 s link itself took 10 m 37 s wall
  (shell `time`, including a 29 s build). Free disk on the local overlay was never a constraint (735 GB), so peak
  disk on a hosted runner is not measured.
- Merge speed: 690 s / 493,838 = 1.40 ms per file, single process (libFuzzer's merge is not parallel). I also watched
  the merge's exec/s fall from 8,192 to 942 as the size-sorted inputs got bigger; that progress trace was seen on the
  terminal only (`out/minimize-frame_deser.log` was overwritten by the later fault runs), it is not saved.
- Duplicate discovery dominates the corpus: a 60 s resume link with 4 workers on the minimized 22,879 files ended with
  39,256 files (`frame_deser-link2-resume.log`, `cpu_seconds` 244; each worker writes the same new inputs
  independently), so every link re-inflates the corpus and benefits from the merge.

### Extrapolation [INFERENCE, not measured on a runner]

Linear in file count at 1.40 ms/file on this machine (a fast desktop core); a hosted vCPU may be 1-2x slower
[INFERENCE: no hosted measurement].

| `frame_deser` corpus | this machine | 2x slower runner |
|---|---|---|
| 409,464 (T2 night) | ~570 s | ~1,150 s |
| 1,000,000 | ~1,400 s | ~2,800 s (over the limit) |
| 2,000,000 | ~2,800 s | ~5,600 s (over the limit) |

Disk at 4 KB per file: 2 M files is ~7.7 GB (`1.9 GB / 493,838` x 2 M), which is inside a ~14 GB runner but a
significant share. The minimized directory is ~5 % of that, so the merge peak is ~1.05x the original, not 2x.

### Chosen timeout: 1500 s

It is 2.2x the measured 690 s for a corpus larger than the T2 night's (493,838 vs 409,464 files), covers ~1.07 M
files at this machine's speed or ~540 k at half that speed, and still leaves ~27 of the job's 55 spare minutes
(355 - 300) for the corpus/record/log uploads, setup and the post-steps when it expires. A longer limit would spend the
headroom the audit identified as the real risk; a timeout simply keeps today's behaviour (the unminimized corpus is
uploaded), so the minimization can only help.

## Fault vectors (`logs-corpus-minimize/fault-vectors.log`, `final-script-check.log`)

Sorted sha256 listing of the corpus (`find . -type f | sort | xargs sha256sum | sha256sum`) before/after:

| vector | result |
|---|---|
| (a) `frame_deser`, timeout 1 s, 39,256 files | merge exit 124, `::warning::...merge exceeded 1s; keeping the unminimized corpus`, script exit 0, listing `6215c1dd...` before = after, no `corpus-minimize-*` left |
| (a') `manifest_parser`, timeout 1 s (final script) | exit 124, warning, script exit 0, listing `9d3b0ceb...` before = after, no leftover `/tmp/libFuzzerTemp.*` (the control file now lives in the scratch dir) |
| (b) nonexistent target `no_such_target` with a 3-file corpus dir | `cargo fuzz` exit 1 (`no bin target named`), warning, script exit 0, listing `6cc38a77...` before = after |
| (b2) nonexistent target without a corpus dir | warning "corpus dir ... missing", script exit 0 |
| (c) records | `sha256sum out/record-*.json` identical before/after in (a), (b); `other-targets.log` shows the same for `manifest_parser` and `fuzz_exec_deps_parse_elf` after a successful merge |
| (d) real target, empty corpus dir | merge exit 0 with an empty result -> warning "empty corpus; keeping", exit 0 |

Success path (final script, `final-script-check.log`): `manifest_parser` 1,178 -> 1,156 files, prints the libFuzzer
summary `MERGE-OUTER: 1156 new files ... 4507 new coverage edges`; the uploaded dir has no sub-directories and only
40-hex-named inputs. Re-minimizing an already minimized corpus still drops a few files (merge is not byte-deterministic).

## Resume check

Link 2 (60 s, `taskset -c 0-3`, 4 workers) from the minimized `frame_deser` corpus
(`frame_deser-link2-resume-inited.txt`): every worker prints `#22880 INITED cov: 31881 ft: 64532 corp: ~22,650`, loaded the
23.3k files with `Loaded N/233xx files` lines, link exit 0, no libFuzzer errors (the only `error` string matches are rustc
warning text). The 32 original workers ended at `cov: 31861-31881` on the full corpus
(`frame_deser-link1-worker-DONE.txt`); the resumed workers start at `31881`, i.e. no measurable coverage loss. `manifest_parser`: INITED `cov: 4389-4390 corp: ~1,148`; `fuzz_exec_deps_parse_elf`: INITED `cov: 316 ft: 955
corp: 175-176`, both from 20 s links started from the minimized corpus (`other-targets.log`).

## Checks

- `bash -n scripts/minimize-fuzz-corpus.sh`: exit 0. `shellcheck` and `actionlint` are not installed here (not run).
- `cargo nextest run -p xtask --locked --test ledger_collectors_20_3a --test release_precondition_t3_20_5`: 31 passed,
  exit 0 (`nextest-xtask.log`); the workflow-shape tests pass unchanged (job count 6, 15 overwrite uploads).

## Residual risks

- Not run on a hosted runner: merge speed, upload/download time of the artifact and runner disk are unmeasured there.
- Local toolchain is `nightly` 2026-09-24, not the pinned 2026-09-25; cargo-fuzz 0.13.2 is what the workflow installs.
- The merge outer process held 725 MB RSS for 494k files (from the first run; value seen on the terminal, not saved). For
  targets without `-rss_limit_mb=0` (`manifest_parser`, `fuzz_exec_deps_parse_elf`) a corpus of ~1 M+ files could hit
  the default 2048 MB limit, which only turns into the safe "keep the original" path.
- A merge keeps one coverage-equivalent set, not every input; the corpus carried to the next link is ~5-10 % of the files
  with the same measured edges/features. The T3 record's `corpus_size` still counts the pre-minimization corpus (the record
  is written by the fuzz step, before this script).
- If the fuzz step itself exhausts the runner disk (before this step) nothing here helps.

## Correction 1 (2026-11-12, `FixT3Corpus2`): review `review/gpt-review-corpus-minimize.md` (P1, P1, P2)

Supersedes the "keeps the original on every outcome / exits 0 always" wording and the 1500 s limit above. Logs:
`correction-1/` (`v1`..`v7`). Toolchain for every run below: installed `nightly` (rustc 1.100.0-nightly 2026-09-24) and
cargo-fuzz 0.13.2 substitute for the pinned `nightly-2026-09-25` (not installed); scratch clone `/tmp/t3fix2-clone`
(deleted), 32 cores, dummy `GITHUB_SHA`/`GITHUB_RUN_ID`. The final script is sha256 `8bed120a6af1...`; runs `v1`-`v3` and
`v4` (happy path) used copies that differ from it only in the wording of the control-file comment at line 224 (no code
difference; the later runs `v4` fault vectors, `v5`, `v6` used the final file).

| finding | fix (`scripts/minimize-fuzz-corpus.sh`) |
|---|---|
| P1-1 crash masked by a successful merge | merge gets `-artifact_prefix=$PWD/out/merge-crashes-<target>/` (l.218, 292); `merge_crash_evidence` (l.152) runs after the merge **whatever its exit status** (l.233): any file in that dir (every prefix incl. `slow-unit-`) or a log line matching `ERROR: (libFuzzer\|*Sanitizer)`, `deadly signal`, `panicked at`, or a scan that errors/times out (fail closed) -> `::error::T3 <target>: input crashed during corpus merge; refusing evidence` + artifact names, original corpus untouched, **exit 1** (l.234-236, `main` l.299). Same convention as `run-fuzz-link.sh:27-29`. Artifacts stay in `out/`, so the existing `if: always()` upload keeps them. `slow-unit-` counts although the merge cannot write one: it is emitted only by `Fuzzer::PrintPulseAndReportSlowInput` (`FuzzerLoop.cpp:445-457`), whose only callers are `RunOne` (l.534) and `MinimizeCrashLoop` (l.929), while the merge's `FuzzerMerge.cpp:239` calls `ExecuteCallback` directly; counting every file keeps the rule identical to `run-fuzz-link.sh` and cannot misfire. |
| P1-2 incomplete output installed | `validate_merged` (l.113) before the swap: exactly one `MERGE-OUTER: N new files` line (absent -> reject), N >= 1, no non-regular entry, regular-file count == N, and `sha1sum(content) == file name` for every file (libFuzzer names corpus files by SHA-1; truncated, zero-byte or missing files fail). `space_check` (l.88) before the merge: free KiB >= original `du -sk` x 1.1 + 262,144 and free inodes >= original files + 1024 (inode test skipped only when the fs reports 0 inodes). Rejection / low space -> `::warning::`, original kept, exit 0. Margin: the output is a subset of the input files with the same block rounding, so the worst case is one more full copy (1.0x); 10 % + 256 MiB cover the control file (measured 55,761,392 bytes for 392,647 inputs = 3.5 % of the corpus' `du`), the logs and crash artifacts. |
| P2 unbounded operations | every `find`/`wc`/`du`/`df`/`stat`/`grep`/`awk`/`sha1sum`/`mkdir`/`rm`/`mv`/`rmdir`, including the initial scratch removal, the replaced-corpus deletion and the EXIT trap, runs under `bounded` = `timeout -k 2 <bound>` (l.44-56, bounds l.45-49); failures only warn. Swap: same-filesystem check (`stat -c %d` on corpus, scratch, corpus parent; l.212), `mv -T corpus -> scratch/old` (l.257), `mv -T merged -> corpus` (l.258), then best-effort `rm -rf old`; `old` lives outside `corpus/<target>/` so a delete that never finishes is harmless. `restore` undoes a half swap, also after a timeout that raced a completed rename. Sourcing guard `BASH_SOURCE == $0` (l.305) makes the functions testable. |

### Timing: sum of bounds

`worst_case_seconds` (sourced; `correction-1/v2-function-vectors.log`, last rows) adds every bound: calls 27 x 2 s kill
grace + 2x mkdir 10 + rm scratch 15 + find/du 4 x 15 + df 5 x 5 + stat 5 + scans 4 x 10 + nonreg 20 + sha1 60 + awk 20 +
mv 2 x 10 + rm old 45 + trap (restore 2 x 10, rm 20, rmdir 5) + merge **1200 + 30 kill** = **1659 s = 27.65 min**. The merge
limit drops from 1500 to 1200 s (1.7x the 690 s measured for 493,838 files; 364 s for 392,647 files in this round) so the
sum fits ~28 min: 300 + 27.65 min leaves ~27 of the job's 55 spare minutes. `timeout-minutes: 30` was added to the five
minimize steps (nothing else in the workflow changed this round; `correction-1/workflow.diff` is the whole diff vs HEAD):
1800 s > 1659 s, so it fires only if a bounded call was stuck beyond its `timeout` (e.g. uninterruptible I/O).

### Vectors (all in the scratch clone; exit = the script's)

| # | vector | exit | result | log |
|---|---|---|---|---|
| 1 | `manifest_parser` target patched to panic on input starting `T3-MERGE-PLANT` (clone only, `v1-plant.diff`; its Cargo.lock hunk is cargo's lock refresh), 4,152-file corpus incl. the marker input | **1** | libFuzzer's own merge exited **0** (retry "successful in 2 attempt(s)", "719 new files") and would have installed a corpus without the crashing input; script printed `::error::T3 manifest_parser: input crashed during corpus merge; refusing evidence`, `merge artifact: crash-ac16d4ad...`, `deadly signal`/`panicked at` markers; artifact in `out/merge-crashes-manifest_parser/`; corpus sha256 listing identical (4,152 files, `v1-corpus-listing-before/after.sha`) | `v1-planted-merge-crash.log`, `v1-planted-merge-libfuzzer.log`, `v1-artifacts-listing.txt` |
| 2 | validator, sourced: correct dir | 0 | accepted | `v2-function-vectors.log` |
| 2 | truncated file / zero-byte file | 1 | `1 file(s) whose SHA-1 differs from the file name` | same |
| 2 | MERGE-OUTER 6 vs 5 files / 4 vs 5 files | 1 | `merged dir holds 5 regular files but MERGE-OUTER reported 6` / `... 4` | same |
| 2 | no MERGE-OUTER line / two lines / 0 new files / subdir | 1 | rejected with the matching reason | same |
| 2 | `space_check` at/below the requirement, inodes, no inode table, unmeasurable | 0/1 as expected (10 rows) | boundary values exact | same |
| 2 | `merge_crash_evidence`: artifact kinds, markers, unreadable log/dir | 0 = evidence, 1 = none (9 rows) | fail closed on unreadable | same |
| 3 | free-space guard on a **real** 64 MiB tmpfs (`/dev/shm`, 3,000 files, no override env at all) | 0 | `free space 53536 KiB < required 275344 KiB ...; skipping the merge`; listing identical; no scratch left | `v3-free-space-skip.log` |
| 3b | inode branch | - | only as `space_check` vectors: the local filesystem reports 0 inodes (inode test skipped, logged as `total_inodes=0`) and no unprivileged tiny-inode filesystem exists here (`unshare -Urm` is denied) | `v2`, `v3` |
| 4 | happy path, `frame_deser`, 300 s link x 32 workers -> 392,647 files / 1,570,588 KiB; then the script | 0 | merge exit 0, merge wall **364 s**, `MERGE-OUTER: 18356 new files`, validation (count == N, sha1 == name for all 18,356) < 1 s (`SECONDS` resolution), after: 18,356 files (4.7 %) / 73,424 KiB, whole script 6 m 12 s; independent re-check `find -exec sha1sum` over the output: 18,356 files, 0 mismatches, 0.23 s | `v4-frame_deser-grow-link1.log`, `-minimize.log`, `-sha1-throughput.log` |
| 4 | SHA-1 cost at scale: the output hard-linked 21x = 385,476 files hashed (hot page cache) | - | 5.7 s (~67k files/s), so the 60 s bound covers ~4 M hot files [INFERENCE for a hosted cold cache; a timeout only rejects and keeps the original] | `v4-sha1-throughput.log` |
| 4 | resume: 60 s link 2 on 4 cores from the minimized corpus | 0 | all four workers `INITED cov: 26041 ft: 51367-51371 corp: ~18,180`; the 32 link-1 workers ended at `cov` <= 26041 (13 distinct values, max 26041), i.e. no coverage lost; record written, 0 libFuzzer errors | `v4-resume-check.log` |
| 4 | records | - | `sha256sum out/record-*.json` identical before/after the merge and all fault runs | `v4-record-sha-*.txt`, `v4-fault-vectors-real.log` |
| 4a | `frame_deser`, 1 s limit, 47,866 files | 0 | merge exit 124, warning, listing `42d86526...` before = after, no scratch, no `libFuzzerTemp` in `/tmp` | `v4-fault-vectors-real.log` |
| 4b | nonexistent target + 3-file corpus; no corpus dir | 0 / 0 | `no bin target named` -> warning, listing identical; "corpus dir ... missing" | same |
| 5 | test double `cargo` (`fake-cargo.sh`) exiting 0 in every mode: good / truncated (last byte lost) / missing file / no MERGE-OUTER line / crash artifact + marker / marker only | 0 / 0 / 0 / 0 / **1** / **1** | good: swapped 20 -> 8 files; truncated, missing, no-line: rejected, listing identical; crash and marker: `::error::`, listing identical | `v5-fake-cargo-vectors.log` |
| 5 | real cargo, real target, **empty** corpus dir | 0 | no MERGE-OUTER line -> rejected, warning | same |
| 6 | a hanging `du` / `sha1sum` / `rm` (600 s sleep shims on PATH) | 0 / 0 / 0 | gave up after 15 s / 60 s / 80 s (initial rm 15 + old rm 45 + trap rm 20); original kept (du, sha1sum); for `rm` the merged corpus is in place and the replaced one is left in `corpus-minimize-<t>/old`, outside `corpus/<t>/` | `v6-bounded-slow-ops.log` |

### Checks

- `bash -n` on the script, `function-vectors.sh`, `fake-cargo.sh`: exit 0. `shellcheck` and `actionlint` are not installed (not run); no
  `yaml` module for a YAML parse (the xtask workflow tests parse the file, below). `v7-static-checks.log`.
- `cargo nextest run -p xtask --locked --test ledger_collectors_20_3a --test release_precondition_t3_20_5` in the main tree:
  31 passed, exit 0 (`v7-nextest-xtask.log`); the workflow-shape tests accept the extra `timeout-minutes` key.

### Residual risks (correction 1)

- A crash found by the merge now **fails the link** (the point of P1-1): the corpus upload is skipped, later links are skipped,
  the chain is not green. Recovery is `gh run rerun --failed` after the product fix; the crash input is in the `t3-logs-*` artifact.
- Fail-closed side effects: a stale file in `out/merge-crashes-<target>/` (never on a fresh runner) or a libFuzzer
  `ERROR:`/`panicked at` line from the merge's own process (e.g. the outer process hitting the default 2048 MB RSS limit on
  `manifest_parser`/`fuzz_exec_deps_parse_elf` with a very large corpus; their corpora were 180-4,152 files here) also refuse.
- `timeout` cannot stop a process in uninterruptible I/O; only the step's `timeout-minutes: 30` then ends it (and fails the
  link). The 1659 s sum is a bound on the script's own waits, not a hosted-runner measurement.
- Hosted speed, cold-cache SHA-1 time, runner disk and a real inode-limited filesystem are unmeasured; the inode comparison is
  verified on numbers only (`space_check` vectors), the local filesystem reports no inode table.
- Validation proves the output is what libFuzzer reported and that every file is intact; it cannot prove the merge kept
  all coverage (libFuzzer's own guarantee).

## Correction 2 (2026-11-12, `FixT3Corpus3`): review `review/gpt-rereview-corpus-minimize.md` (P2, P2)

Supersedes "a libFuzzer `ERROR:`/`panicked at` line or a scan that errors/times out also refuses" (Correction 1, row P1-1 and
residual risks). Logs: `correction-2/`. Toolchain: installed `nightly` (rustc 1.100.0-nightly 2026-09-24) and cargo-fuzz 0.13.2 substitute
for the pinned `nightly-2026-09-25` (not installed); scratch clone `/tmp/t3fix3-clone` (8 workers via `taskset`, dummy `GITHUB_SHA`/
`GITHUB_RUN_ID`), deleted at the end. The workflow was not touched (the script's CLI, exit codes 0/1 and artifact paths are unchanged;
the new marker file lives under `out/`, which the existing `if: always()` logs upload already covers).

### Change (`scripts/minimize-fuzz-corpus.sh`)

`merge_crash_evidence` (binary) became `classify_merge` (l.182-205; tri-state in the exit status `CM_CLEAN` 0 / `CM_CONFIRMED` 1 /
`CM_INCONCLUSIVE` 2, evidence on stdout), `record_inconclusive` (l.209-219) and the `case` in `minimize` (l.281-292):

| result | condition | action |
|---|---|---|
| CONFIRMED | at least one regular file in `out/merge-crashes-<target>/` (crash-/leak-/oom-/timeout-/slow-unit-), whatever the log or the scans say | original corpus kept, `::error::T3 <target>: input crashed during corpus merge; refusing evidence` + artifact names, `minimize` returns 2, **exit 1** (unchanged) |
| INCONCLUSIVE | no artifact file, and the merge log matches `ERROR: libFuzzer`, `ERROR: *Sanitizer`, `deadly signal`, `panicked at` or `out-of-memory`, **or** the artifact-dir `find` / log `grep` failed or timed out | original corpus kept, one `::warning::` line with <= 7 evidence lines of <= 200 chars, marker file `out/merge-inconclusive-<target>.txt` (target, merge exit, log and dir paths, reason, <= 5 marker lines), **exit 0** |
| CLEAN | neither | validation (`validate_merged`), space check and swap exactly as before |

Nothing else changed: space check, bounds, swap and `worst_case_seconds` (still 1659 s; the INCONCLUSIVE path adds one bounded `tee` but
returns before validation and the swap, so it is shorter than the 27-call path that the sum covers). `grep` got `-a` so a log with binary
bytes still yields its marker lines. A filename is printed only for a real regular file, so artifacts found before a scan timeout still
give CONFIRMED.

### Why an artifact is the line between CONFIRMED and INCONCLUSIVE (vendored `libfuzzer-sys-0.4.13/libfuzzer/`)

- **Inner process (`-merge_inner`) writes one.** `CrashResistantMerge` re-runs the command with `-merge_inner=1` (`FuzzerMerge.cpp`
  "Execute the inner process until it passes"; flags incl. `-artifact_prefix` are inherited from `Args`). The driver gives the inner
  process an input buffer: `if (Options.MaxLen == 0) F->SetMaxInputLen(kDefaultMaxMergeLen)` (`FuzzerDriver.cpp:892-894`) ->
  `AllocateCurrentUnitData` (`FuzzerLoop.cpp:163-166`). Its handlers then call `DumpCurrentUnit`: deadly signal / panic abort
  `CrashCallback` (`:226-237`, dump at :236), `AlarmCallback` timeout (`:301`), `RssLimitCallback` (`:311-323`, dump at :319),
  `HandleMalloc` (`:132`), `ExitCallback` (`:250`); `DumpCurrentUnit` writes `<artifact_prefix><kind>-<sha1>` via
  `WriteUnitToFileWithPrefix` (`:174-187`, `:644-655`).
- **Outer process does not.** The outer process builds its `Fuzzer` from `Options.MaxLen = Flags.max_len` (`FuzzerDriver.cpp:690`,
  default 0, `FuzzerFlags.def:16`; cargo-fuzz and the workflow pass no `-max_len`), so the constructor's `AllocateCurrentUnitData`
  returns early (`FuzzerLoop.cpp:158`, `:164`: `MaxInputLen == 0`) and `CurrentUnitData` stays null; the outer process never calls
  `SetMaxInputLen` (only the `-merge_inner` branch, `FuzzerDriver.cpp:892-894`, and `MinimizeCrashInput`, `:510`, do). Its RSS thread is
  started anyway (`StartRssThread` `FuzzerDriver.cpp:314-327`, called at `:832`, before `Merge` at `:889`), and `RssLimitCallback` prints
  `ERROR: libFuzzer: out-of-memory (used: ..)` (`FuzzerLoop.cpp:311-323`) and then `DumpCurrentUnit`, which returns at `:175`
  (`if (!CurrentUnitData) return;  // Happens when running individual inputs`) -> **no artifact**. This is the reviewer's P2 1: the outer
  process holds the control file and file lists and can pass the default 2048 MB with a very large corpus without any input being at fault.
- Consequence: "no artifact + marker" is an outer-process (or scan) condition, not evidence about an input. Keeping the original corpus is
  safe: a crashing input that is still in it is re-executed at the next link's `INITED` (libFuzzer loads the whole corpus) and fails that link
  properly. **Link 5 has no next link:** there the `::warning::` and `out/merge-inconclusive-<target>.txt` in the `t3-logs-*` artifact are the
  whole record, and the corpus uploaded by link 5 is the unminimized original.

### Vectors (exit = what the row names; all 45 rows `PASS`, `FAILS=0`)

`correction-2/v1-function-vectors.log` (sourced `function-vectors.sh`; the unchanged validator and `space_check` rows of Correction 1 are in the
same log) and `v2-fake-cargo-vectors.log` (whole script against `fake-cargo.sh`, which exits 0 except `outeroom` = 71):

| # | vector | result (rc: 0 CLEAN / 1 CONFIRMED / 2 INCONCLUSIVE; script exit in the fake-cargo rows) | log |
|---|---|---|---|
| 1 | artifact present (crash-, slow-unit-, oom-, timeout-, leak-), clean log | **1** CONFIRMED, name printed | v1 |
| 1 | artifact + deadly-signal log; artifact + OOM log; artifact + unreadable log | **1** (artifact decides) | v1 |
| 2 | no artifact, `ERROR: libFuzzer: out-of-memory ...` / `deadly signal` / `panicked at` / `ERROR: AddressSanitizer` / bare `SUMMARY: ... out-of-memory` | **2** INCONCLUSIVE | v1 |
| 3 | unreadable log (missing file, `chmod 000`); unreadable artifact dir (missing, `chmod 000`) | **2** | v1 |
| 3 | scan timeout: `find` shim sleeping 30 s, `b_scan=1` -> `exit 124`; `grep` shim likewise | **2** (shims scoped to the classifier call only) | v1 |
| 4 | clean log; log with binary junk but no marker; dir with only an empty subdirectory | **0** CLEAN | v1 |
| 5 | `record_inconclusive`: one `::warning::` line, marker file with reason + excerpt; unwritable marker path | warning only, no failure | v1 |
| 6 | fake cargo `crash` (artifact + marker) / `marker` (marker only, exit 0) | script exit **1**, corpus listing identical / script exit **0**, listing identical, marker file written | v2 |
| 7 | fake cargo `outeroom` (exit 71, `ERROR: libFuzzer: out-of-memory`, no artifact) / `outeroom0` (same marker, merge exit 0) | script exit **0**, original listing identical (`sha256` of the sorted listing), `out/merge-inconclusive-fake_*.txt` written, no scratch left | v2 |
| 8 | fake cargo `unreadablelog` (log `chmod 000` after the merge) / `unreadabledir` | script exit **0**, listing identical, marker file with `could not be scanned (exit 2 / 1)` | v2 |
| 9 | fake cargo `good` / `truncated` / `missing` / `nomerge` (validator rows from Correction 1, re-run) | exit 0: swapped 20 -> 8; three rejected, listing identical | v2 |

### Real runs (`manifest_parser`, scratch clone)

| run | result | log |
|---|---|---|
| grow: `run-fuzz-link.sh ... 120 T3 1 ''`, 8 workers | 10,369 files / 41,476 KiB, exit 0 | `v3-grow-manifest.log` |
| **happy path**: script, default flags | merge exit 0 (13 s), `MERGE-OUTER: 1257 new files`, validated, 10,369 -> 1,257 files (5,028 KiB), script exit 0, no artifact, no marker file, `out/record-*.json` sha256 identical, 0 non-40-hex names | `v3-happy-manifest_parser-minimize.log` |
| **planted merge crash**: clone-only `T3-MERGE-PLANT` panic (the `.rs` hunk of `correction-1/v1-plant.diff`), 10,369 + 1 planted input | libFuzzer's merge exited **0** ("successful in 2 attempt(s)"); script **exit 1**, `::error::`, artifact `crash-bd8c7d0a...` in `out/merge-crashes-manifest_parser/`, corpus listing identical | `v5-planted-merge-crash.log` |
| **real tiny `-rss_limit_mb`** through the script's `rss_flag` argument, same 10,369-file corpus restored before each run | see below | `v4-real-outer-oom-attempts.log` |

Real resource failures (`-rss_limit_mb=N` as the third argument):

- `N=1`: merge exit 1, repeated attempts each dying with `ERROR: libFuzzer: out-of-memory (malloc(1048576))` (no `MERGE-INNER` line; [INFERENCE] the inner
  process's 1 MiB `SetMaxInputLen` allocation hits the malloc limit before `CurrentUnitData` exists), **no artifact** -> INCONCLUSIVE, script exit 0, listing identical, marker file written.
- `N=30`: the outer process printed `out-of-memory (used: 53Mb; limit: 30Mb)` first (**no artifact from it**, as the source predicts) and
  the merge exited 1 -> the script classified INCONCLUSIVE (exit 0, listing identical, marker file) at a moment when `out/merge-crashes-*/` was empty;
  the **orphaned inner process** (the outer's `_Exit` does not stop it) then printed its own `used: 192Mb` OOM and wrote
  `oom-6079ee75...` after the classification. An isolated outer-only OOM was therefore *not* provoked: the inner process uses at least as much RSS
  as the outer, so no limit trips the outer alone; the outer-only path is shown by this ordering (outer died first, no artifact from it) and by the
  `outeroom`/`outeroom0` fake-cargo rows.
- `N=60`, `100`, `200`: the merge's **inner** process crossed the limit (`used:` 125-356 Mb in the log lines) in every attempt; libFuzzer restarted it and the merge
  exited **0**; the directory held 8 / 9 / 7 `oom-` artifacts, in each run including the empty unit `oom-da39a3ee5e6b4b0d3255bfef95601890afd80709`
  -> CONFIRMED, script **exit 1**, listing identical. See residual risk 1.

### Checks

- `bash -n` on the script and the three vector scripts: exit 0 (`v7-static-checks.log`); `shellcheck` not installed (not run).
- `cargo nextest run -p xtask --locked --test ledger_collectors_20_3a --test release_precondition_t3_20_5` in the main tree: 31 passed, exit 0
  (`v8-nextest-xtask.log`).
- Final script sha256 `6c669de1ee09...` (the vector logs `v1`/`v2` were produced with this file; `v3`-`v5` with a copy that differs only by the
  four-line "Known limit" comment at l.21-24).

### Residual risks (correction 2)

1. **An inner-process RSS trip is still CONFIRMED and fails the link.** The merge inner process runs the same RSS thread and, once its peak
   RSS passes `-rss_limit_mb`, writes an `oom-` artifact for whatever unit is current (even the empty one) on every restart. Its RSS grows with the
   corpus ([INFERENCE]; its peak was 125-356 Mb on this 10k-file corpus, growth with corpus size not measured), so a huge corpus under the default 2048 MB limit (`manifest_parser`,
   `fuzz_exec_deps_parse_elf`; `frame_deser` runs with `-rss_limit_mb=0`) could fail a link with an artifact that no input caused. Not
   changed because the assignment defines every artifact as CONFIRMED; possible follow-ups for the owner: pass `-rss_limit_mb=0` to the merge, or
   treat an `oom-` artifact that only appears with `out-of-memory (used: ...; limit: ...)` as INCONCLUSIVE. Link sizes at which this triggers are
   unmeasured on a hosted runner.
2. An artifact that libFuzzer could not write (full disk: `WriteToFile` ignores errors, `FuzzerIO.cpp`) leaves a genuine inner crash as INCONCLUSIVE
   (marker lines present, no file). The original corpus is kept; the next link's `INITED` re-executes it, link 5 has only the warning/marker.
3. After an outer `_Exit` the orphaned inner process keeps running briefly and may still write an artifact after classification (seen at `N=30`);
   it lands in `out/merge-crashes-<target>/` and is uploaded, but it cannot change the verdict of that run.
4. A marker-like string in cargo's own build output inside the merge log (the log holds cargo's stderr) classifies a clean merge as INCONCLUSIVE:
   safe direction only (the corpus is not minimized that link).
5. Hosted timing, real disk peaks and an outer-only OOM at the default limit remain unmeasured; the scan-timeout rows use `sleep` shims.

## Correction 3 (2026-11-12, `FixT3Corpus4`): review `review/gpt-review-round3-corpus-minimize.md` (P2)

> **Superseded in part by Correction 4 below:** the appended `-rss_limit_mb=0` was reverted and `oom-` artifacts are now INCONCLUSIVE; the residual-risk wording of this section (no promise for a SIGKILL-skipped input, no "never fails the link" beyond one targeted SIGKILL) is corrected there.

Finding: an inner-merge RSS trip writes `oom-` artifacts (even for the empty unit) that `classify_merge` counts CONFIRMED, failing a finished
5 h link, and merge RSS is not bounded by fuzz-worker success (the inner merger keeps a `MergeFileInfo` + path per file and the `AllFeatures`/`AllPCs`
sets, and never calls `PurgeAllocator`). This supersedes "residual risk 1" of Correction 2. Logs: `correction-3/`. Toolchain: installed
`nightly` + cargo-fuzz 0.13.2 substitute for the pinned `nightly-2026-09-25`; scratch clone `/tmp/t3fix4-clone` (8 cores via `taskset`, dummy
`GITHUB_SHA`/`GITHUB_RUN_ID`; script and workflow copied in), `/tmp/t3fix4-*` deleted afterwards. The workflow was not touched.

### Change (`scripts/minimize-fuzz-corpus.sh`, l.273-277 and the CONFIRMED comment l.21-30)

```
-  local flags=(-merge=1 "-artifact_prefix=$crashes/")
-  if [ -n "$rss_flag" ]; then flags+=("$rss_flag"); fi
+  # -rss_limit_mb=0 goes LAST so it overrides an inherited -rss_limit_mb=N: ...
+  local flags=(-merge=1 "-artifact_prefix=$crashes/")
+  if [ -n "$rss_flag" ]; then flags+=("$rss_flag"); fi
+  flags+=(-rss_limit_mb=0)
```

Vendored libFuzzer (`~/.cargo/registry/src/*/libfuzzer-sys-0.4.13/libfuzzer/`):

- last flag wins: `ParseOneFlag` assigns `*IntFlag = Val` on every occurrence (`FuzzerDriver.cpp:145-163`), the argument loop applies them in order (`:211-218`);
- 0 disables the RSS thread (`StartRssThread`: `if (!RssLimitMb) return;`, `FuzzerDriver.cpp:323-325`; started at `:832`);
- malloc limit follows: `-malloc_limit_mb` defaults to 0 = "same limit as rss_limit_mb" (`FuzzerFlags.def:172-176`); `Options.MallocLimitMb = Flags.malloc_limit_mb; if (!Options.MallocLimitMb) Options.MallocLimitMb = Options.RssLimitMb;`
  (`FuzzerDriver.cpp:717-720`); `HandleMalloc` returns at once when it is 0 (`FuzzerLoop.cpp:126`). Neither the workflow nor the script passes `-malloc_limit_mb`, so the one other `oom-` source is gone too;
- the inner inherits it: `CrashResistantMerge` copies `Args`, removes only `merge`, `set_cover_merge`, `fork`, `collect_data_flow` and adds `merge_control_file`/`merge_inner`
  (`FuzzerMerge.cpp:493-505`), so `-rss_limit_mb` (all occurrences, order kept) reaches every `-merge_inner` process.

`crash-`/`leak-`/`timeout-` artifacts stay CONFIRMED (unchanged classifier; `slow-unit-` cannot occur in the merge path). Space check, bounds,
`worst_case_seconds` (1659 s) and the swap are unchanged.

### Vectors (scratch clone; exit = the script's; `real-run.sh` restores the same grown corpus before each row)

| # | vector | result | log |
|---|---|---|---|
| 1 | real merge command line, matrix `rss_flag` values: `manifest_parser ''`, `frame_deser '-rss_limit_mb=0'`, `fuzz_exec_deps_parse_elf ''` | the flags libFuzzer got end in `... -artifact_prefix=<out>/merge-crashes-<t>/ -rss_limit_mb=0 <merged> <corpus>`; frame_deser `-rss_limit_mb=0 -rss_limit_mb=0`; with an inherited `-rss_limit_mb=60`: `-rss_limit_mb=60 -rss_limit_mb=0 <merged> <corpus>` | `v2`, `v4-frame_deser-minimize`, `v6` (`Running` line) |
| 2 | provoking vector of `correction-2/v4-real-outer-oom-attempts.log`: `manifest_parser`, 10,027-file corpus, `rss_flag` `-rss_limit_mb=60` (was: 8 `oom-` artifacts incl. empty unit, exit 1) | merge exit 0, **script exit 0**, 0 artifacts, 0 `oom-`, no marker file, validated (count == `MERGE-OUTER`, sha1 == name), swapped 10,027 -> 1,206 files | `v2-manifest-happy-and-rss60.log` |
| 2 | same corpus, `-rss_limit_mb=1` / `30` / `100` / `200` (before: INCONCLUSIVE, INCONCLUSIVE + late `oom-`, exit 1, exit 1) | **exit 0** each, 0 artifacts, validated, swapped to 1,207 / 1,209 / 1,213 / 1,206 files (counts differ with directory order, same input set) | `v3-manifest-tiny-rss-limits.log` |
| 3 | happy path, `rss_flag ''` (default 2048) | exit 0, 10,027 -> 1,206, no artifact, no marker | `v2` |
| 4 | planted clone-only `T3-MERGE-PLANT` panic (`v5-plant.diff`), corpus + 1 planted input (10,028 files), `rss_flag` `-rss_limit_mb=60` | libFuzzer merge exit 0; **script exit 1**, `::error::`, `crash-6ad1508c...` in `out/merge-crashes-manifest_parser/`, original listing IDENTICAL (`37d27ba7415c95f6`) | `v5-planted-merge-crash.log` |
| 5 | `frame_deser` short link (60 s, 8 workers, `-rss_limit_mb=0`) -> minimize -> link 2 (20 s) | 101,379 files / 405,516 KiB -> 13,644 / 54,576 KiB, merge 109 s, exit 0, validated, swapped; link-2 workers print `#13645 INITED cov: 21719 ft: 41578 corp: 13516/1181Kb` (13,644 + the empty unit) | `v4-frame_deser-*.log`, `v4-frame_deser-link2-inited.txt` |
| 6 | `fuzz_exec_deps_parse_elf` (60 s link, `''`) | 3,427 -> 180 files, exit 0, validated, swapped | `v6-*.log` |
| 7 | inner `-merge_inner` process SIGKILLed 3 s in (stand-in for a kernel OOM kill of the merge; one sample) | inner dies without artifact; outer attempt 2 (`caused a failure at the previous merge step`) reports `successful`, then `MERGE: failed to parse the control file (unexpected error)`, merge exit 1; classifier CLEAN (no artifact, no marker line) -> `merge failed with exit 1; keeping the unminimized corpus`, **script exit 0**, listing IDENTICAL | `v7-inner-sigkill.log` |
| 8 | sourced function vectors of Correction 2 (`correction-2/function-vectors.sh`, unchanged) | 45 PASS, `FAILS=0`, `worst_case_seconds` 1659 s | `v5-function-vectors.log` |

### Checks

- `bash -n` on the script and `real-run.sh`: exit 0 (`v9-static-checks.log`). Script sha256 `147d8dfdb37f...`.
- `cargo nextest run -p xtask --locked --test ledger_collectors_20_3a --test release_precondition_t3_20_5` in the main tree: 31 passed, exit 0 (`v8-nextest-xtask.log`).
- Process notes: the manifest grow, rows 1-3 (manifest) and row 7 built/ran with the session's shared `CARGO_TARGET_DIR` (`/mnt/build/cargo-maos`, same source as the main tree);
  rows 4-6 used the clone's own `fuzz/target` (`unset CARGO_TARGET_DIR`) so the planted panic never reached the shared dir. The planted run (row 4)
  started while another agent's `cargo check -p maos-bin` was running (my pre-check listed it and I did not wait; separate target dirs).

### Residual risks (correction 3)

1. **Merge memory is now unbounded by libFuzzer.** A merge whose RSS outgrows the runner is killed by the kernel. Measured (row 7, one SIGKILL of the inner): no
   artifact, no `oom-` -> not CONFIRMED; exit 1 from the outer, classifier CLEAN (the log has no marker line, the regex matches only libFuzzer/sanitizer/panic text),
   `merge failed with exit 1` warning, corpus kept, step exit 0. [INFERENCE from the code, not run] a different kill timing can let the outer finish with exit 0 (the
   crash-resistant merge skips the failed file; the output is still validated before the swap, so at worst one input's coverage is missing), and a SIGKILL of the outer
   `cargo` process gives exit 137/124 -> the `exceeded`/`failed` warning branch (`minimize` l.293-300). In no case is an artifact written, so the link is not failed.
   Growth of merge RSS with corpus size is unmeasured (`correction-2`: the 10k-file manifest inner peaked 125-356 Mb).
2. A genuinely memory-hungry *input* (single unit needing GBs) is no longer reported as `oom-` by the merge: it is OOM-killed there or passes. It is not lost: the
   input stays in the unminimized corpus (merge failed) or is skipped from the output, and the next link's workers (which keep their own `rss_flag`) re-execute the survivors. Link 5 has no next link.
3. `-rss_limit_mb=0` also removes libFuzzer's purge threshold (`PurgeAllocator` starts above 50 % of a limit, `FuzzerLoop.cpp:788-789`, `FuzzerFlags.def:166-169`); irrelevant here, the merge loop never calls it.
4. Residual risks 2-5 of Correction 2 stand (unwritable artifact on full disk, orphaned inner after an outer `_Exit`, cargo-output marker false positives, hosted timing unmeasured).

## Correction 4 (2026-11-12, `FixT3Corpus5`): review `review/gpt-review-round4-corpus-minimize.md` (P2), orchestrator design decision

Finding: Correction 3's appended `-rss_limit_mb=0` removes every memory bound from the merge (RSS monitor and, via the malloc-limit default, the per-allocation
limit) without establishing another one. A runner-wide OOM can then select the supervising shell or the runner instead of the inner process, and the completed 5 h
link is lost with its cleanup and evidence uploads. The same review shows that Correction 3's residual-risk text (`README.md:352-359`, Correction 3 above) claims more than
its evidence: (a) "In no case is an artifact written, so the link is not failed" is shown only for one targeted SIGKILL of the inner process, not for a real memory-pressure
event, a SIGKILL of the supervisor, or a runner-wide OOM; (b) "It is not lost ... the survivors are re-executed by the next link" is not guaranteed after an arbitrary
SIGKILL, because libFuzzer can skip an interrupted input and later succeed (`FuzzerMerge.cpp:123-126, 209-218, 493-523`), and count/hash validation of the output cannot
see that coverage from the skipped input is missing. Correction 3's Change section and vector table stand as history except where this section says otherwise.
Toolchain: installed `nightly` + cargo-fuzz 0.13.2 substitute for the pinned `nightly-2026-09-25`; scratch clone `/tmp/t3fix5-clone` (`git clone --no-local`; script and workflow copied
in; clone-local `fuzz/target`, `CARGO_TARGET_DIR` unset; 8 cores via `taskset`; dummy `GITHUB_SHA`/`GITHUB_RUN_ID`), deleted afterwards. The workflow was not touched. Logs: `correction-4/`.

### Design (as decided by the orchestrator; applied exactly)

1. **Revert the appended `-rss_limit_mb=0`** (`scripts/minimize-fuzz-corpus.sh`, the flags block before the merge). The merge inherits the matrix `rss_flag` exactly as the
   fuzz step received it: default libFuzzer 2048 MB for `manifest_parser` and `fuzz_exec_deps_parse_elf`, `-rss_limit_mb=0` for `frame_deser`, which is the exposure its fuzz
   step already has with its 8 unlimited workers (runner cores). The merge is therefore never less bounded than the fuzzing it follows. This restores the Correction-2 flags.
2. **`classify_merge`: an `oom-` artifact alone is INCONCLUSIVE.** A regular file whose *basename starts with `oom-`* is libFuzzer's RSS/malloc-limit dump
   (`DumpCurrentUnit` from `FuzzerLoop.cpp:311-323`); Correction 3's review showed it is written for whatever unit is current, even the empty one, and merge RSS is not
   bounded by fuzz-worker success. So when the artifact dir holds *only* `oom-` files the result is INCONCLUSIVE: original corpus kept, `::warning::`, marker file
   `out/merge-inconclusive-<target>.txt` with `reason: only oom- artifact(s)...` and one `merge artifact: oom-<sha>` line per file (<= 20), script exit 0. If *any other*
   artifact is present (`crash-`, `leak-`, `timeout-`, `slow-unit-`, an unknown prefix, `oomx`, `my-oom-1`) the result stays CONFIRMED, exit 1, with the non-`oom-` names listed first
   (the 20-line cap cannot hide them). All other classifier branches, bounds (`worst_case_seconds` still 1659 s), validation and swap are unchanged; the marker file's
   `result:` line now reads `INCONCLUSIVE (original corpus kept)` because case (a) also has artifacts.
3. **This README's earlier claims are corrected here, not rewritten there.**

### Limit of the decision (stated, not mitigated)

A genuine single-input OOM that libFuzzer reports as an `oom-` file during the merge no longer fails the link at that moment. The input stays in the kept original corpus
and is re-executed by the next link's workers under the same `rss_flag`, where `run-fuzz-link.sh` refuses evidence if it fails. **Link 5 has no next link**, so for
link 5 the only record is the `::warning::` and the uploaded `merge-inconclusive-<target>.txt`; a link-5 reviewer must read that marker file (the `if: always()` logs
upload carries it). Likewise an `oom-` caused by the merge's own metadata growth is indistinguishable from an input OOM; the classifier does not try.

### Vectors (scratch clone; exit = the script's; `real-run.sh` restores the same grown corpus before each row; `PLANT_CRASH=1` places a `crash-planted-by-vector` file in the merge artifact dir before the run)

| # | vector | result | log |
|---|---|---|---|
| a | real merge command lines (`Running` line), matrix `rss_flag` values `manifest_parser ''`, `frame_deser '-rss_limit_mb=0'`, `fuzz_exec_deps_parse_elf ''`; also `manifest_parser '-rss_limit_mb=60'` | flags end `... -artifact_prefix=<out>/merge-crashes-<t>/ [<rss_flag>] <merged> <corpus>`: nothing after `-artifact_prefix` for `''`; exactly one `-rss_limit_mb=0` for frame_deser (the inherited one); exactly one `-rss_limit_mb=60` for the 60 row; no appended zero anywhere | `v2`, `v3`, `v6`, `v8` |
| b | round-3 provoking vector: `manifest_parser`, 12,005-file corpus grown by a 120 s 8-worker link, `rss_flag` `-rss_limit_mb=60`, no planted artifact | libFuzzer merge exit 0, 14 `oom-` artifacts (incl. `oom-da39a3ee...`, the empty unit; 12-14 across repeated runs), `::warning::` ... only oom- artifact(s), **script exit 0**, no `::error::`, original listing IDENTICAL (`0afd2ab6c2850eae`, 12,005 files), marker file `target:/merge_exit: 0/result: INCONCLUSIVE (original corpus kept)/reason: only oom-.../merge artifact: oom-<sha>...` naming the oom- files, no scratch dir left | `v3` |
| c | same, plus a planted `crash-planted-by-vector` in `out/merge-crashes-manifest_parser/` before the run | `::error::T3 manifest_parser: input crashed during corpus merge; refusing evidence`, `merge artifact: crash-planted-by-vector` first, then the `oom-` names; **script exit 1**, listing IDENTICAL, no marker file | `v4` |
| d | planted clone-only `T3-MERGE-PLANT` panic (`v9-plant.diff`, the Correction-3 diff) + 1 planted input (12,006 files): default limit; and `-rss_limit_mb=60` | default: libFuzzer merge exit 0, a `crash-6311...` artifact only (0 `oom-`), `::error::`, **script exit 1**, listing IDENTICAL (`73ba2242ca6aeea6`); with 60: `crash-6311...` plus `oom-` files, **script exit 1** | `v10`, `v11` |
| e | happy paths: `manifest_parser ''` 12,005 -> 1,295 (1,297 on a second identical run; directory-order dependent) files; `frame_deser '-rss_limit_mb=0'` (60 s link, 99,125 files) -> 12,340; `fuzz_exec_deps_parse_elf ''` (60 s link) 3,262 -> 186 | each: merge exit 0, **script exit 0**, no artifact, no marker, validated (count == `MERGE-OUTER`, sha1 == name), swapped, 0 non-40-hex names after, no scratch left | `v2`, `v6`, `v8` (links: `v1`, `v5`, `v7`) |
| f | sourced function vectors (`correction-4/function-vectors.sh`, Correction-2 vectors updated): the three old "oom- counts as CONFIRMED" rows now expect INCONCLUSIVE; new rows: oom-+crash-/leak-/timeout-/unknown prefix -> CONFIRMED, `oomx`/`my-oom-1` -> CONFIRMED, 30 oom- + 1 timeout- -> CONFIRMED with `timeout-` shown and 20-line cap, 30 oom- only -> INCONCLUSIVE, nested oom- only -> INCONCLUSIVE, `record_inconclusive` marker names the oom- file | `FAILS=0`, all rows PASS, `worst_case_seconds` 1659 s | `v5-function-vectors.log` |

### Checks

- `bash -n` on the script, `real-run.sh`, `function-vectors.sh`: exit 0 each (`v12-static-checks.log`). Script sha256 `764ceeb3bbec...`.
- `cargo nextest run -p xtask --locked --test ledger_collectors_20_3a --test release_precondition_t3_20_5` in the main tree: 31 passed, exit 0 (`v13-nextest-xtask.log`).
- Process note: the `frame_deser`/`exec_deps` build+link started while another agent's `cargo check -p maos-bin` (shared `CARGO_TARGET_DIR`, separate from the clone-local target) was running, because
  my chained command did not wait on the `pgrep` listing it printed. The earlier manifest runs and the planted runs started with no cargo process listed.

### Residual risks (correction 4)

1. **Runner-wide memory exhaustion is only as bounded as the fuzzing it follows.** `frame_deser` runs the merge with no libFuzzer RSS bound (as its fuzz step does with 8 workers); on a
   hosted runner an OOM may select any process, including the supervising shell, and the link's evidence is then lost. No cgroup/`memory.max` containment is added (the review's
   suggestion; the orchestrator chose not to). Occurrence and victim selection on hosted runners remain unmeasured.
2. A SIGKILL (inner or outer) is not guaranteed to preserve the original: libFuzzer can skip the interrupted input and finish with exit 0, and the output validation cannot detect the missing
   coverage. It is not claimed that a skipped input survives for the next link.
3. The "limit of the decision" above: a genuine `oom-` is deferred to the next link, and link 5 relies on the marker file.
4. An `oom-` file written while an `oom-`-only result is INCONCLUSIVE skips minimization for that link: the next link carries the full unminimized corpus (more disk and merge time on the following hand-off).
5. Residual risks 2-5 of Correction 2 stand. The merge time of the real vectors is on an 8-core local machine, not a hosted runner.

## Correction 5 (orchestrator, 2026-11-12) — round-5 review P2

Review `review/gpt-review-round5-corpus-minimize.md` (GPT): the artifact-name `sort` in `classify_merge` ran outside
`bounded`, so a stalled `sort` could bypass the advertised 1659 s bound. Fix: the sort was only cosmetic ordering inside
each group (non-`oom-` names are still listed first), so it is removed; the loop now reads `printf '%s\n' "$names"`
(a bash builtin, no external process). `worst_case_seconds` is unchanged (the sort had no budget).

Proof: `correction-5/v1-function-vectors.log` (the correction-4 sourced vectors re-run against the edited script:
58 PASS, `FAILS=0`, exit 0); script sha256 in `correction-5/script.sha256`; `bash -n` exit 0. The review found the RSS
inheritance, prefix classification, evidence ordering, exit dispatch, marker path and validation-before-swap conformant
to Correction 4. The orchestrator closes the review rounds here: the accepted residual risks are those listed in
Correction 4.
