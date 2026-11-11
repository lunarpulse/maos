> Independent review (round 2) by OpenAI gpt (agent `Review209L1r2`) of the 20-9 corrections, commit 8c669a21. Persisted by the correction-round dev (round 3) because the reviewer was read-only.

# Independent review R2 — `8c669a21`

**Verdict: incorrect. Five findings: 2 P1, 3 P2.** Static review only; no builds, tests, or mutations executed. No write tool is available, so this is the report for `landing-1/review/gpt-review-r2.md`.

## Findings

### 1. [P1] Tie snapshot fence ownership to the dispatcher’s cancellation
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/spirit_session.rs:665–673` — confidence 0.99**

`CallState` observes cancellation of the blocking `LifecycleLink::call`, not cancellation of `HookDispatcher::fire_snapshot`. The dispatcher starts its timeout before scheduling `spawn_blocking`, while the link starts another full-cap deadline inside that worker. If the dispatcher expires first, the coordinator can refuse the swap and drop the successor, clearing the predecessor’s fence; a subsequent reply can still win `state.answer()` and fence the predecessor permanently. The new test covers a reply substantially later than both deadlines, not this interval. **Fix:** connect the outer attempt’s deadline/cancellation to the session, and serialize cancellation/fence release with reply acceptance and fence acquisition. **Required failing-before:** delay the blocking hook’s start, deliver its snapshot reply after the outer timeout but before its internal timeout, and prove the refused predecessor continues processing frames. Original finding #5 is not fully closed.

### 2. [P1] Keep failed readiness attempts from poisoning cold-swap restoration
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/spawned_lifecycle.rs:580–581` — confidence 0.98**

The new readiness-failure rollback leaves the successor’s FR11 archive record behind: `load_admit_start` calls `archive_admitted` before this asynchronous readiness check. For a successor with a cross-major state schema and a valid `migrates_from` declaration that subsequently fails component startup, restoring the original manifest now compares it against that failed successor’s archive record. An original manifest without a reverse migrator is refused `MigratorMissing`, so the door returns `cold_swap_failed` and leaves the Spirit absent instead of restoring it. The new witnesses use same-major schemas and miss this case. **Fix:** make archive publication transactional with spawned readiness, preserving pre-existing records and committing the attempted version only after successful startup. **Required failing-before:** cold-swap to an existing invalid component with a cross-major schema and permitted forward migration; assert `cold_swap_reverted` and a serving predecessor.

### 3. [P2] Exclude session queue contention from telemetry quarantine
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/hook_pump.rs:505–514` — confidence 1.0**

The pump’s wall timeout can expire before the link’s non-striking queue deadline. With a 30-second manifest cap, an active turn occupying the session for six seconds, and a telemetry event arriving behind it, the pump synthesizes `BudgetExceeded` after five seconds and permanently quarantines the subscriber although its telemetry export never started; the link permits fifteen seconds of queue waiting. Dropping the dispatcher future also does not cancel its blocking caller, so that event can execute later despite already being accounted as failed. This contradicts D-20-9-18’s contention disposition. **Fix:** give spawned telemetry calls a cancellable queue deadline inside the pump’s wall, distinguish “not started” from export overruns, and cancel the queued call on pump expiry. **Required failing-before:** publish through the real pump while a six-second frame turn holds a 30-second component; assert no quarantine and no post-cancellation invocation.

### 4. [P2] Track consecutive telemetry warnings separately from consolidation
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/hook_pump.rs:399–406` — confidence 0.99**

The new `slow` counter is shared by telemetry and consolidation, and any `Fired` outcome resets it. `ConsolidationWatchdog` calls this same `account` function, so a subscriber whose telemetry consistently takes 80–95% of its budget can avoid quarantine indefinitely when quick consolidations finish between events. The warning witness creates no consolidation watchdog and therefore misses the production interleaving. **Fix:** maintain telemetry-specific consecutive-warning accounting, updating/resetting it only for telemetry outcomes; keep consolidation accounting separate. **Required failing-before:** interleave a successful consolidation between three near-budget telemetry deliveries and assert quarantine on the third telemetry event.

### 5. [P2] Withdraw hook registration when unwinding failed spawned admission
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/admission.rs:1431–1438` — confidence 1.0**

`unwind_admitted` removes scheduler/promotion/pid state but never calls `deps.hooks.unregister(pid, ...)`, although `load_admit_start` registered that pid before starting the session. Neither session finalization nor scheduler unload removes the pump registration. Without another telemetry delivery or consolidation scan, the failed pid retains its subscription and potentially its cached dispatch target; repeated failed admissions accumulate these entries. This is especially relevant to shell roots, which never enter the serving tail that starts consolidation. **Fix:** explicitly unregister the failed pid in both unwind branches before returning the refusal. **Required failing-before:** fail component startup with no watchdogs or further scalar publications, and assert immediate subscription revocation and removal of the failed registration.

## Closure assessment

| Earlier issue | Assessment |
|---|---|
| GPT #1 | Real `maos run` callers are now started before awaiting the session; normal success/error returns reach the shared teardown. |
| GPT #2 | Closed: installed and explicit-run roots both arm the shared handles. |
| GPT #3 | Invalid-component witness is fixed, but complete unwind/restoration remains incomplete: findings 2 and 5. |
| GPT #4 | Closed: the factory rejects disabled `on_swap_in` before coordinator mutation. |
| GPT #5 | Not fully closed: finding 1. |
| GPT #6 | Closed for the reported stderr/late-reply paths: every polling iteration checks remaining time. |
| GPT #7 | Normal handover gap is covered by the bounded queue; D-20-9-18 deliberately permits contention drops. |
| GLM F1 | Same-event fan-out is concurrent and event-to-event ordering remains serialized, but findings 3 and 4 remain. |
| GLM F2 | Static traversal/symlink containment is fixed. It does not eliminate concurrent filesystem replacement between canonicalization and the runner opening the pathname. |

`CallState` is monotonic and per-call, so no ABA reuse was found. The queue holds one call, not an unbounded event backlog. `JoinSet` drains each event before the next, preserving each subscriber’s event order; broadcast lag remains explicitly counted.

The teardown claim should remain narrower than “panic-safe and always joined”: `LifecycleCallers` has no unwind cleanup, its watchdog joins detach on their five-second timeout, and the AST guard recognizes a syntactic `callers.shutdown()` rather than proving awaited, path-complete ownership. I did not establish another concrete production failure from those limitations.

## Evidence and claims

- The retained red log records **13 selected tests, 13 failures** with corresponding reasons. All **13 plant logs** contain mutation details, expected failures, exit 100, and restoration markers. Their contents are internally consistent; this review did not independently execute or authenticate them.
- Recorded final focused run: **317 passed, 3 skipped**. The workspace log is **not green**: **5913 passed, 1 failed, 137 skipped**, with the disclosed million-row erasure failure.
- Baseline/final daemon logs show **0 → 379 recalls** and **0 → 60 schedule fires**. The corrected AC1 premise is honest. The baseline script does not fail when its Running wait exhausts, so its zero counts alone do not independently certify successful baseline startup.
- Recorded pins satisfy the ceilings: **37,633 ≤ 38,422**, **3,137 ≤ 3,348**, **201,197 ≤ 202,209**. The retained gate actually reports 201,324 including the disclosed foreign spike; the clean aggregate is a stated adjustment, not a separate green gate log.
- `MAOS_ARCHIVE_DIR` is registered. No new serde-followed `unwrap`/`expect` defect was found.
- Feature/Linux test guards are present; the retained no-default test records seven passes. macOS/Windows compilation, hosted legs, AC6, and AC7–AC9 remain explicitly unverified/open.

**Overall correctness: incorrect.** The corrections fix the demonstrated happy-path failures, but snapshot cancellation, failed-admission restoration/cleanup, and telemetry contention accounting still need changes.

**Overall confidence: 0.98.**

**Headlines: 2 P1 — snapshot re-fencing; failed-start archive poisoning. 3 P2 — false telemetry quarantine; consolidation resets warning streaks; failed hook registrations leak.**
