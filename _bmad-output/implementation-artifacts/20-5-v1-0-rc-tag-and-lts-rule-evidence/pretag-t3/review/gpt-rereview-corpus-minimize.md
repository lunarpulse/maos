## Verdict: incorrect

The three original defects are addressed, but the corrected crash classifier can still fail a clean five-hour link for minimizer infrastructure failures. **Two new P2 findings block shipping.**

**Counts:** P0: 0 · P1: 0 · P2: 2 · P3: 0  
**Overall confidence:** 0.99

## Findings

### 1. [P2] Distinguish outer-process resource failures from input crashes

**File:** `/home/lunarpulse/dev_ws/scripts/minimize-fuzz-corpus.sh:163–167`  
**Confidence:** 1.00

When the merge’s **outer process** exceeds its RSS limit, it prints `ERROR: libFuzzer: out-of-memory`, which this regex treats as an input crash. The outer process starts the RSS-monitor thread before entering `Merge` (`FuzzerDriver.cpp:832,889`); its callback prints that diagnostic without requiring an executing target callback (`FuzzerLoop.cpp:311–323`). The manifest and ELF matrix entries retain the default **2048 MiB** limit (`FuzzerFlags.def:172`; workflow lines 26,32). Consequently, corpus/control-file memory growth alone can cause `minimize` to return 2 and `main` to exit 1 (`scripts/minimize-fuzz-corpus.sh:233–236,299`), invalidating the completed link without any input crashing. Distinguish outer-process resource failures from target failures and warn/retain the original for the former; do not simply discard all inner OOM evidence.

*Dependency references above are under the installed `libfuzzer-sys-0.4.13/libfuzzer/` directory.*

### 2. [P2] Skip minimization when crash-evidence scanning is inconclusive

**File:** `/home/lunarpulse/dev_ws/scripts/minimize-fuzz-corpus.sh:157–165`  
**Confidence:** 1.00

A failed artifact-directory scan or failed/timed-out log scan sets `found=0`, exactly like confirmed crash evidence. Thus a clean merge followed by a scan error—or merely a scan exceeding ten seconds—fails the link through `:233–236,299`. The supplied function vectors explicitly demonstrate this behavior for missing logs/directories. That contradicts the requirement to fail **only for a genuine crash**: uncertainty should prevent installing the candidate corpus, not invalidate the existing fuzz record. Return a distinct inconclusive result, retain the original corpus, warn and exit 0; confirmed crash evidence must continue to fail regardless of merge exit status.

## Original findings: closure

| Previous finding | Result and evidence |
|---|---|
| **P1-1: merge crash masked** | **Closed.** Evidence is inspected before merge-status handling (`script:233–239`), and crash status reaches exit 1 (`:297–299`). `correction-1/v1-planted-merge-crash.log:3–14` records merge exit 0, script exit 1 and an identical original corpus listing. |
| **P1-2: incomplete output installed** | **Closed.** Expected-count, regular-entry and content-address checks run before replacement (`script:116–146,251–258`). Missing/truncated-output rejection is recorded in `correction-1/v2-function-vectors.log:3–10` and the fake-cargo vectors. |
| **P2: unbounded statistics/cleanup** | **Closed for the reported operations.** Traversal, hashing, deletion and recovery now have explicit bounds (`script:44–56,71–79,132–138,174–180,190,269`). The supplied hanging-operation log records successful exits after bounded waits (`correction-1/v6-bounded-slow-ops.log:6,15,27`). This remains subject to normal `timeout`/uninterruptible-I/O limitations. |

## Remaining questions and rejected candidates

1. **Usage errors and missing corpus:** No additional finding. `ERROR: -merge=1 …` does **not** match `ERROR: libFuzzer`. The installed version’s insufficient-directory message is actually `INFO: Merge requires two or more corpus dirs` (`FuzzerDriver.cpp:517–522`). Missing-directory diagnostics also do not match (`:255–275`). Missing corpus is handled before invoking cargo (`script:187`).

2. **Timeout and partial log lines:** No independent false-crash path established from SIGTERM alone. LibFuzzer routes SIGTERM to its interrupt handler (`FuzzerUtilPosix.cpp:123–124`), which prints `run interrupted; exiting`, not a crash marker (`FuzzerLoop.cpp:268–274`). The supplied real timeout vector preserves the corpus and exits 0. A truncated matching outer-resource diagnostic remains finding 1.

3. **Stale crash artifacts:** A real local-rerun hazard, but not a separate hosted-chain finding. `mkdir -p` preserves existing artifacts (`script:188`), and any file triggers refusal (`:156–160`). These workflow jobs use fresh runners and do not download previous `out/` directories.

4. **`set -e` and sourcing:** No defect found. `minimize || rc=$?` intentionally disables automatic errexit inside minimization (`script:295–296`); significant operations explicitly check status. The sourcing guard does not invoke `main` (`:305`).

5. **Swap, `mv -T`, trap and interrupted rename:** No remaining silent-loss path established for the workflow’s same-filesystem layout. Validation precedes both renames (`script:251–258`); recovery moves `old` back only when the corpus path is absent (`:173–174`). Cleanup removes `merged` and `tmp`, **not `old`** (`:177–180`). `mv -T` avoids accidental nesting, and the second-rename timeout race is explicitly checked (`:258–265`). SIGKILL still cannot guarantee trap execution, but it already terminates the job.

6. **Validator format and naming:** Compatible with the installed pinned source. `CrashResistantMerge` emits one final count summary (`FuzzerMerge.cpp:534–537`), and the driver writes selected files then exits (`FuzzerDriver.cpp:535–543`). Output names always use `Hash(U)` (`FuzzerLoop.cpp:632–641`), whose encoding is lowercase SHA-1 (`FuzzerSHA1.cpp:211–225`). Neither dictionary use nor a merge-control-file option changes that naming path. An unfamiliar future summary safely rejects minimization rather than installing unchecked output.

7. **Zero selected files:** Nonempty corpus does **not universally imply** at least one selected file: zero-byte inputs are excluded (`FuzzerIO.cpp:123–128`), and selection requires a new feature (`FuzzerMerge.cpp:174–188`). Rejecting `N=0` is nevertheless safe here: it retains the original and exits 0 (`script:124–126,251–253`), rather than failing the link or replacing the corpus with nothing.

## Timing

- Declared bound sum: **1659 seconds = 27m39s** (`script:59–64`).
- Step limit: **1800 seconds**, leaving **141 seconds** nominal margin. All five steps consistently specify 30 minutes (`workflow:45,108,171,234,297`).
- Fuzzing plus minimization: **327m39s**, leaving **27m21s** of the 355-minute job for installation, builds, transfers and other overhead.
- This is budget arithmetic, **not a hosted-runtime guarantee**. Uninterruptible I/O or exhausted job headroom can still cause a workflow timeout.
- Failure propagation is confirmed: subsequent links require predecessor success, and the collector explicitly rejects unsuccessful links (`workflow:75,138,201,264,349`).

**Verification limits:** Read-only source/evidence review only. No builds, tests, fuzzing, fault injections, workflow dispatches or file writes were performed. Hosted timing, outer-RSS threshold occurrence, disk peak and signal-interrupted swaps remain unverified.