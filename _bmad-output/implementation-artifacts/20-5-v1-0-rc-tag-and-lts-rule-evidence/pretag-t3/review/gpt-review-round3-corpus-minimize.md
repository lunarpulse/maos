## Verdict: incorrect — one P2 remains

**Counts:** P0: 0 · P1: 0 · P2: 1 · P3: 0  
**Confidence:** 0.96

Both round-2 findings are closed as specified. However, the argument that an inner-merge RSS failure necessarily implies a fuzz-worker failure does **not** hold.

### [P2] Separate merge-only RSS exhaustion from input failures

**Location:** `scripts/minimize-fuzz-corpus.sh:202`  
**Confidence:** 0.96

When the inner merger’s corpus metadata, accumulated coverage sets or allocator overhead push RSS past its limit, libFuzzer writes an `oom-` artifact even outside an input callback. This line consequently classifies minimizer resource exhaustion as CONFIRMED; `:284–287,354` then fails the completed five-hour link. Fuzzing and merging have different memory-retention paths, so successful workers under the same RSS limit do not exclude this condition. Disable the aggregate RSS limit specifically for minimization—append `-rss_limit_mb=0` after inherited flags at `:267–268`—or distinguish RSS-only OOM artifacts and handle them as INCONCLUSIVE, preserving refusal for independent crash evidence.

### Why the orchestrator’s argument fails

Dependency references below use **V** = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libfuzzer-sys-0.4.13/libfuzzer`.

- **Fuzz workers do not retain every disk-corpus input.** Startup reads inputs individually (`V/FuzzerLoop.cpp:831–839`); `RunOne` retains their contents only when they add features or retention is forced (`:535–541`). `keep_seed` defaults to zero (`V/FuzzerFlags.def:27`). Reloaded inputs occupy a temporary vector, not a permanently retained whole-corpus buffer (`V/FuzzerLoop.cpp:418–440`).
- **The inner merger retains its own whole-corpus metadata.** Parsing allocates one `MergeFileInfo` per file and retains every pathname (`V/FuzzerMerge.cpp:71–74,209`; `V/FuzzerMerge.h:53–56`). It also accumulates `AllFeatures` and `AllPCs` as node-based sets across units (`V/FuzzerMerge.cpp:221–225,247–249,263–265`). These are not the workers’ feature representation (`V/FuzzerCorpus.h:157,419–421`).
- **Important qualification:** the inner merger does **not** retain every input’s feature vector. It parses with `ParseCoverage=false`, and its local `Features` set is recreated each iteration (`V/FuzzerMerge.cpp:104–118,209,243`). Thus per-unit feature-vector accumulation is not the explanation.
- **Allocator handling also differs.** The normal mutation loop calls `PurgeAllocator` (`V/FuzzerLoop.cpp:912`), which can purge sanitizer caches/quarantines (`:780–792`). The inner merge loop has no corresponding call (`V/FuzzerMerge.cpp:226–271`). Malloc hooks themselves retain counters, not allocation histories; those counters reset around callbacks (`V/FuzzerLoop.cpp:49–71,615–623`).
- **An artifact does not establish input causation.** The RSS monitor checks process peak RSS independently of callback execution (`V/FuzzerDriver.cpp:314–327`). Its handler unconditionally dumps the current buffer (`V/FuzzerLoop.cpp:311–322`), whose size resets to zero after callbacks (`:627`). Hence the observed empty-input OOM artifacts are expected.

The supplied `correction-2/v4-real-outer-oom-attempts.log:87–103,184–200` demonstrates OOM artifacts—including the empty unit—causing exit 1 at reduced limits. **Occurrence at the workflow’s default 2048 MiB is unmeasured**, but worker success is not a valid upper bound on merger RSS.

`run-fuzz-link.sh:27–29` does refuse worker artifacts; that policy does not make the two execution modes’ memory requirements equivalent.

## Round-2 closure

| Finding | Result |
|---|---|
| **P2-1: outer-process OOM treated as input crash** | **Closed for marker-without-artifact failures.** Marker-only evidence becomes INCONCLUSIVE (`script:195–203`), and dispatch records it and returns success (`:288–290`). Fake outer-OOM exits 71 and 0 both preserve the original corpus and exit 0 (`correction-2/v2-fake-cargo-vectors.log:53–83`). |
| **P2-2: inconclusive scans fail the link** | **Closed.** Scan errors/timeouts set INCONCLUSIVE (`script:184–193,203`), rather than CONFIRMED. Supplied function vectors cover missing/unreadable paths and both scan timeouts (`correction-2/v1-function-vectors.log:38–43`); whole-script unreadable-path cases exit 0 with unchanged corpora (`v2-fake-cargo-vectors.log:85–112`). |

## Tri-state regression checks / rejected candidates

- **Exit-code collision:** none. Classifier status 2 is explicitly consumed as INCONCLUSIVE before returning minimization status 0; only confirmed evidence becomes minimization status 2 and script exit 1 (`script:282–290,351–357`).
- **`set -e` / pipefail:** no new defect identified. Status-bearing substitutions are guarded, and `minimize || rc=$?` disables implicit errexit within that call (`script:184–185,282,351`).
- **Marker persistence:** correct absolute path under `out/` (`script:347`), bounded best-effort write (`:209–218`), covered by all five always-run logs uploads (`workflow:67–72,130–135,193–198,256–261,319–324`).
- **Cargo-output regex false positives:** acceptable safe-direction behavior: they skip minimization without swapping or failing (`script:288–290`).
- **Actual crash refusal remains intact:** the planted crash records merge exit 0 but script exit 1, with unchanged original corpus (`correction-2/v5-planted-merge-crash.log:4–12`).
- **Late orphan artifacts:** the supplied 30-MiB run demonstrates the acknowledged classification race (`v4-real-outer-oom-attempts.log:66–86`). Not a separate tri-state regression finding.

## Timing

The declared bounded-wait sum remains **1659 seconds = 27m39s**, leaving **141 seconds** beneath all five 30-minute step limits (`script:68–88`; `workflow:45,108,171,234,297`). The INCONCLUSIVE branch adds bounded `tee` but skips validation and replacement, so it does not increase that maximum.

This is a **nominal bounded-wait budget**, not an unconditional runtime guarantee: uninterruptible I/O remains outside it, and an inner process orphaned by outer `_Exit` is no longer bounded by the departed `timeout` supervisor.

**Unverified:** hosted/default-limit RSS thresholds and wall time. Read-only source and supplied-evidence review only; no checks, builds, fuzzing, dispatches or writes performed.