The AC6.1 swap on the unmodified TS example is proven on a branch run, and so are the counter, AC6.2 and the four-image vectors. The 20-8 plant leg has the 101 panic gone but the session still dies, now on `OutOfFuel`.

Run `43217945522` on `ci/20-9-verify-2` is the proof run (commit `18af9303` = main `fe44017a` plus the workflow file only). Its overall conclusion is `failure`, as intended: only the AC6.2 plant job is red, and its judge step is green. The other 9 of 10 jobs are green. One earlier run, `43212140215`, failed on two recipe bugs, described below.

**Proven, with run `43217945522` job ids**

- **AC6.1, shipped example (job `131181865598`).**
  - The job checks the example is unmodified (`git diff --exit-code`, `cpu_max_pct = 100`). v+1 differs only in the version line.
  - Four attempts were refused `spawned_swap_not_ready` and retried. Attempt 5 returned exit 0, outcome `completed`, `latency_ns 29503939836` against the 90 s wall.
  - `pid_before=1` and `pid_after=1`. The job logged `one successor declaration row: ok` and `one lifecycle.hot_swap row: ok`.
- **Counter continues (job `131181865721`, recipe-local variant).**
  - The job builds the example with `console.log` in `idle`, `swapIn` and `consolidate`, and `consolidate` also does `idles += 1`.
  - The guest's stdout lands as `spirit.diagnostic` rows in the transparency log, and the job asserts on those.
  - The whole sequence is strictly 1,2,…,96, with `spirit_diagnostic_dropped rows: 0`:

    ```
    (13,'idle',2,16274) … (91,'consolidate',80,16274)
    TL row 94 spirit.upgrade completed; row 96 child 16274 exited(code=0)
    (97,'consolidate',81,16326) … (112,'consolidate',96,16326)
    last N = 80; first tick after the swap row = 81 (N+1); 16 ticks after
    ```
  - Same Spirit pid 1, but a new runner child process (16274 → 16326) holds the carried counter.
  - That job's swap came back as exit 75 (the door outlived maosctl's 30 s budget). The recipe read `operator.upgrade.op-3af1abcdecf42135198f01bf:completed` from the log, latency 30,060,406,892 ns. It also logged one successor declaration row and one HotSwap row.
- **AC6.2 plant (job `131181865685`).** The swap step is red as intended with `spawned_swap_unsupported: the running Spirit exports no lifecycle`, and the judge step is green.
- **Four images (jobs `131181865881` ubuntu-24.04, `131181866431` 26.04, `131181865938` 24.04-arm, `131181867170` 26.04-arm).**
  - Each ran 32/32 on `maos-wasm-host`, including `guest_stdio_t2_20_9` 3/3.
  - Each ran 132/132 passed (1 skipped) on `maos-bin`, including `spawned_door_20_9` 6/6 and the cold-swap door vector (red on both arm images in earlier runs, green here).
  - Both new witnesses passed and are asserted by name: `a_cold_swap_admission_waits_for_the_previous_session_to_release_its_mailbox` at about 1.2–1.4 s, and `a_cold_swap_admission_is_refused_typed_when_the_previous_session_never_ends` at about 15.5 s.
- **componentize-js.** Node v22.23.4 (job `131181865987`) and v24.22.2 (job `131181865280`) both passed vitest 14/14 plus build and componentize.

**20-8 plant leg (job `131181865883`, also run 1 job `131162174689`)**

- The guest's `console.log` after the denied `author()` now reaches `spirit.diagnostic` (`Skill authoring declined: capability-denied`).
- Nothing in the job's logs shows exit 101, a panic or an `unplanned_crash`, and the shell exits 0.
- The session still dies. The job logs `guest_export_failed … handle-frame`, then `maos-wasm-runner: OutOfFuel`, then `session child terminated: Exited { code: 4 }`.
- The cause is not determined. My guess, unconfirmed, is that the denied-author path uses more per-turn fuel than the accepted one. This is a product finding, not fixed and not asserted against.

**Product findings (none fixed)**

1. A denied `author()` in the TS example ends the session with `OutOfFuel` (exit 4), as above.
2. The successor's `swap-in` `console.log` never reaches `spirit.diagnostic`. `Prewarm::next_stdout` (`spawned_lifecycle.rs:160-162`) drains and drops the pre-warmed child's stderr. No `swap-in` row appeared in either run. The example README says `console.log` is routed to the host.
3. For a spawned Spirit under `maos run`, `idle` fires once per quiescence (`idle_watchdog.rs:93-95`) and nothing delivers a frame to re-arm it. The shipped example's `idles` is bumped once and then stops.
4. The shutdown panic is still present, in the logged leg's `daemon.err`: `panicked at crates/maos-bin/src/supervision.rs:2749:30: lifecycle_session_ended`, then `telemetry pump: on_consolidate panicked`. It is absent from the shipped leg's `daemon.err`.
5. Not seen: no swap exceeding the wall (29.5 s and 30.1 s), no registration race, and no `setTimeout` panic (not exercised).

**Recipe bugs fixed**

- Run `43212140215` (commit `a7ce0a40`): nextest's colour codes defeated my witness `grep`. All tests passed on the four images (maos-wasm-host 32/32, maos-bin 132/132); the recipe step failed afterwards. Fixed with `--color never`.
- Same run: the logged leg was designed to read `idle`/`swap-in` lines, which cannot work given findings 2 and 3. That is why the variant also ticks in `consolidate`.
- Run 1's first push (`14f835ff`) produced no run because the push filter named `ci/20-9-verify`. Fixed in `a7ce0a40`.

**Limits**

- "idle N+1 after the swap" is not shown for the shipped example, because it cannot occur there.
- The release-daemon swap legs ran on ubuntu-24.04 x86_64 only.
- They run in a branch-only job, not in main's `epic-20-exit`.

**Files, under `_bmad-output/implementation-artifacts/20-9-spawned-form-lifecycle-hooks-evidence/ci/`**

- `ci-20-9-verify.yml` is the updated recipe. Earlier run files still say it is byte-identical to the old recipe; that no longer holds, and the old version is in git history at `fe44017a`.
- `run-43212140215.md` and `run-43217945522.md`.
- `logs/run6-43212140215/` and `logs/run7-43217945522/`, about 260 KB each, with per-image nextest summaries, swap-leg artifacts, `ticks.tsv` and excerpts.

The temp clone and `/tmp` scratch are deleted. `ci/20-9-verify-2` remains on GitHub, and I did not touch git in the main checkout.