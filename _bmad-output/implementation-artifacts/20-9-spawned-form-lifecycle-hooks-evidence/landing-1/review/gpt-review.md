> Independent review by OpenAI gpt (agent `Review209L1`), commit cdf3937d. Persisted verbatim by the orchestrator because the reviewer was read-only.

## Verdict: incorrect

**7 findings: 6 P1, 1 P2.** Static review only; no builds or tests executed.

### 1. [P1] Start lifecycle watchdogs before awaiting spawned sessions
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/main.rs:6825–6827` — confidence 1.0**

Registering the spawned pid with HookPump does not start the idle, schedule or consolidation callers. `maos run` awaits `run_admitted_session` at line 6865, then returns through the session teardown; those watchdogs are constructed only at lines 9761–9808. Consequently, standalone components never receive these three triggers. The new harness masks this by independently spawning the watchdogs. **Fix:** start and own them before entering the spawned session, and join them on every corresponding teardown. **Required red-before:** a real-daemon trigger/count test, not the in-process harness.

### 2. [P1] Install swap handles for installed-component roots
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/main.rs:4929–4930` — confidence 1.0**

The only `swap_slot.set` is inside the explicit `run_args` branch. Components booted by `InstalledSpirits::boot` under bare `maos` or shell mode use `installed_supervision`, but never receive these handles. The successor factory therefore identifies their predecessor as in-process and refuses a WASM successor with `swap_form_mismatch`; cold swap also cannot obtain their origin. **Fix:** wire the handles when installed supervision is armed, sharing the same root-owned setup. **Required red-before:** install and boot a component, then upgrade it through the door without `maos run`.

### 3. [P1] Await runner readiness before completing cold-swap admission
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/spawned_lifecycle.rs:442–450` — confidence 1.0**

`admit_spawned` returns success immediately after spawning the session task. An existing artifact containing invalid WASM—or a component whose `on-start` fails—therefore produces `cold_swap` outcome `completed`; the later session error only prints stderr, and the predecessor is never restored. The current refusal test covers only a missing artifact, which fails synchronously. **Fix:** await a bounded startup/readiness acknowledgement, unwind failed admission, and return the failure to the cold-swap restore branch. **Required red-before:** cold-swap to an existing invalid component and assert `cold_swap_reverted` plus a serving predecessor.

### 4. [P1] Reject successors whose manifest disables `on_swap_in`
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/spawned_lifecycle.rs:571–575` — confidence 1.0**

A lifecycle-exporting successor passes this factory even when its nonempty `enabled_hooks` omits `on_swap_in`. The coordinator accepts `SkippedManifest` as swap success, but this implementation transfers the child only from `LifecycleLink::call(SwapIn)`. The predecessor has already been fenced by snapshot, so neither child serves the replaced SCB. **Fix:** refuse this manifest before coordinator mutation unless `on_swap_in` is permitted by the existing invocation predicate. **Required red-before:** remove only that hook from the successor manifest; assert typed refusal and continued predecessor frame processing.

### 5. [P1] Preserve cancellation when processing a snapshot reply
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/spirit_session.rs:637–641` — confidence 0.99**

A snapshot queued behind an active turn can outlive the caller’s deadline while still completing within the session’s freshly started export deadline. The failed upgrade drops its successor and clears the fence; the later snapshot reply unconditionally sets it again, even though sending the answer fails. The predecessor then remains permanently fenced after a refused swap. **Fix:** carry the call’s cancellation/attempt state into `Pending` and synchronize fence ownership with cancellation and rollback. **Required red-before:** combine queue delay with a delayed snapshot so the caller expires first, then prove the predecessor still processes frames.

### 6. [P1] Enforce prewarm deadlines independently of stderr activity
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/spawned_lifecycle.rs:84–85` — confidence 1.0**

Both `exchange` and `await_ready` check expiration only after `BridgePoll::Idle`. A guest emitting continuous stderr can therefore keep either operation running beyond the declared cap; a reply arriving after expiration is also accepted. Factory prewarming has no surrounding dispatcher timeout. **Fix:** check the absolute deadline on every iteration, bound polling by remaining time, and reject late replies. **Required red-before:** a prewarm guest that continuously emits diagnostics beyond its cap; assert bounded refusal and child cleanup.

### 7. [P2] Queue hooks during the committed handover gap
**`/home/lunarpulse/dev_ws/crates/maos-bin/src/spawned_lifecycle.rs:191–195` — confidence 0.99**

Successful `swap-in` consumes the successor’s `prewarm`, but `attached` remains false until the predecessor session adopts it and retires the old child. Hooks dispatched to the already-replaced SCB during that interval return `Unsupported` without reaching the guest. Schedule, pause/resume and telemetry events can consequently be silently lost. **Fix:** distinguish a committed successor awaiting adoption from an unsupported/unstarted core and queue its hooks for the successor session. **Required red-before:** hold handover open, dispatch a hook, release handover, and assert exactly one guest invocation.

## Evidence and operator decisions

- All five recorded plants contain corresponding failures and restoration markers; no fabrication finding.
- The AC1 instantiation premise is demonstrably false: the baseline probe logs `ready:true`. Rewrite that AC; the implementation evidence correctly discloses the correction.
- Code-reading-only AC1–AC3 failing-before evidence needs explicit acceptance or executable baseline-compatible witnesses.
- `evidence.md` cites an absent `final-20-9-focused.log`. Correct the citation and obtain final-tree verification after fixes.
- Hosted/portability legs and AC7–AC9 remain explicitly unverified/open. Preserve those limitations; do not treat L1 as story completion.
- Recorded pins remain within the granted ceilings. Fixes require remeasurement.

**Artifact limitation:** `triage.md` and notes were not written: this reviewer has no file-write tool, and shell use is restricted to read-only Git commands. The report above is ready for the main agent to persist under `landing-1/review/`.
