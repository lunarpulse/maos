## Verdict: incorrect — **no-ship for T3**

**Counts:** P0: 0 · P1: 0 · P2: 1 · P3: 0  
**Overall confidence:** 0.94

The round-3 P2 is closed exactly as requested. However, removing both memory limits without containing the merge introduces a runner-exhaustion failure mode that the inner-SIGKILL sample does not cover.

### [P2] Contain merge memory before disabling its limits

**Location:** `/home/lunarpulse/dev_ws/scripts/minimize-fuzz-corpus.sh:275–277`  
**Confidence:** 0.94

When corpus metadata or target allocations exhaust runner memory, the appended zero disables both libFuzzer’s aggregate RSS monitor and its default single-allocation limit, without establishing a merge-specific memory boundary. Linux OOM selection is not restricted to the allocating inner process: it can select another eligible process, and a group OOM can terminate an entire shared cgroup. Killing cargo alone can reach the keep-original branch, but killing the supervising shell or runner can prevent that branch, cleanup and evidence uploads, losing the completed five-hour link. The 1200-second timeout and 30-minute step limit constrain time, not memory exhaustion before their deadlines. Run cargo and its merge descendants inside a separately capped memory cgroup, leaving the supervising script outside it, and handle exhaustion as an unsuccessful optimization.

Linux documents both [heuristic victim selection](https://docs.kernel.org/admin-guide/sysctl/vm.html#oom-kill-allocating-task) and [cgroup-local limits/group killing](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#memory-interface-files). **Actual hosted-runner OOM occurrence and victim selection remain unmeasured.**

## 1. Round-3 P2: closed

Let **V** denote the installed `libfuzzer-sys-0.4.13/libfuzzer/` directory.

| Boundary | Verified behavior |
|---|---|
| Script argument construction | `script:275–277` appends inherited flags first, then `-rss_limit_mb=0`. |
| Flag parsing | `V/FuzzerDriver.cpp:163–164` assigns each occurrence; `:211–218` processes arguments in order. Last occurrence wins. |
| RSS monitoring | `V/FuzzerDriver.cpp:323–325` returns without starting the RSS thread when the limit is zero. |
| Malloc defaulting | `V/FuzzerDriver.cpp:717–720` defaults an unspecified malloc limit to RSS. Both therefore become zero; `V/FuzzerLoop.cpp:125–127` disables that check. |
| Inner-process propagation | `V/FuzzerMerge.cpp:493–505` copies arguments and removes unrelated flags, preserving RSS occurrences and their order. `FuzzerCommand.h` confirms those copy/removal semantics. |

Supplied `correction-3/` evidence agrees:

- `v2-manifest-happy-and-rss60.log:21–38`: inherited 60 followed by zero; exit 0, no artifacts.
- `v3-manifest-tiny-rss-limits.log`: 1/30/100/200 all exit 0 with zero artifacts.
- `v4-frame_deser-minimize.log:15` and `v6-exec_deps-minimize.log:15`: real commands end their flags with zero for the other targets.

Thus aggregate merge RSS exhaustion no longer produces the misleading libFuzzer `oom-` artifacts behind round 3.

## 2. Memory containment versus existing deadlines

**The deadlines are insufficient for memory isolation.** The README’s `Correction 3` claim, “In no case … the link is not failed” (`README.md:356`), is broader than the evidence supports.

A concrete containment design is a dedicated cgroup for the complete cargo/merge subtree, with:

- `memory.max` below runner capacity, reserving supervisor/upload headroom;
- bounded swap usage;
- `memory.oom.group=1`, so exhaustion terminates the merge subtree rather than leaving a retrying outer process;
- the script and timeout supervisor outside that cgroup.

No numeric cap is justified by the supplied hosted measurements.

Neither suggested shortcut is equivalent:

- **`-malloc_limit_mb`** limits one allocation, not cumulative metadata/allocator growth (`V/FuzzerFlags.def:174–176`).
- **`ulimit -v`** limits virtual address space. ASan maps 16+ TB on 64-bit platforms, so ordinary RAM-sized limits can prevent startup rather than safely constrain RSS ([ASan limitations](https://clang.llvm.org/docs/AddressSanitizer.html#limitations)).

## 3. Inner-SIGKILL sample: no swap, exit 0

Confirmed for the supplied sample:

1. No artifact or classifier marker → CLEAN (`script:188–210`).
2. Outer merge exits 1 → keep-original return (`:302–309`).
3. Validation and both renames (`:314–321`) are never reached.
4. `main` exits 0 (`:360–366`).

`correction-3/v7-inner-sigkill.log:26–28` records the warning, script exit 0 and identical corpus listing, with no scratch remaining.

This was a deliberately targeted SIGKILL—not a real memory-pressure experiment.

## 4. Earlier corrections remain intact, with limits

- **Artifact-producing crash refusal:** intact. Classification precedes merge-status handling (`script:289–299`); CONFIRMED reaches exit 1 (`:363`). `v5-planted-merge-crash.log:4–11` shows merge exit 0, script exit 1, preserved crash artifact and unchanged original corpus.
- **Incomplete-output rejection:** intact. Count, regular-entry and SHA-1 validation (`:144–179`) precedes replacement. Correction-3 function vectors report all cases passing; correction-2 whole-script vectors preserve originals for truncated/missing output.
- **Inconclusive infrastructure evidence:** still warns, records the marker and preserves the original (`:297–299`).
- **Bounded operations:** unchanged nominal budget: **1659 seconds = 27m39s**, leaving 141 seconds beneath the step deadline. Existing hanging-operation evidence supports the reported bounds; this is not protection against global OOM or uninterruptible I/O.
- **Three-target/resume behavior:** supplied successful merges cover all targets; `v4-frame_deser-link2-inited.txt` shows all eight resumed workers reaching INITED.

One important qualification: **arbitrary SIGKILL is not guaranteed to preserve the original.** LibFuzzer can skip an interrupted input and later succeed (`V/FuzzerMerge.cpp:123–126,209–218,493–523`). Count/hash validation does not detect missing coverage from that skipped input. Consequently, `README.md:358–359` cannot promise that such an input survives for the next link.

**Final recommendation:** retain the round-3 fix, but add merge-specific memory containment before shipping the T3 chain.

**Verification:** read-only source and supplied-evidence review. No builds, tests, fuzzing, fault injections, workflow dispatches or writes performed.