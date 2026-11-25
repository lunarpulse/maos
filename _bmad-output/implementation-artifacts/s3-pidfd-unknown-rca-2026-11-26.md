# RCA: `s3_external_sigkill_requires_pidfd_fact` got `payload_exit == "unknown"` (PR #65, run 45614287117, job 139218571187)

Static analysis only (no cargo, no podman here). `[INFERENCE]` marks anything no log or code line proves.
Code read at `/home/lunarpulse/dev_ws-plants21` (current main).

## Verdict

- **Most likely a product defect** (a narrow, deterministic-in-shape race in `ContainerTerminal::facts_by`,
  `crates/maos-kernel-core/src/security/sandbox/t3/child.rs:253-254`), triggered by the known podman-4.9.3
  stalled-`inspect` family. Not a harness race. The podman exit-125 / stalled inspect is the trigger, not the cause.
- Confidence: about 70% on this mechanism. It is the only path in the code that yields `unknown` with the payload
  pidfd held and verified. The decisive datum, the exit row's `container` object, is not in the log (see §4).
- Real-world impact: cause classification does not change (a snapshot `Err` classifies `fault.truncated` either way,
  `crates/maos-domain/src/sandbox.rs:496-499`). The journaled `payload_exit` fact is lost (`unknown`/`none`)
  although the pidfd had proved `signaled(sig=9)`. A real Worker crash under a stalled runtime gets the same row.

## 1. What produces `"unknown"`

`payload_exit` is `ContainerTerminalFacts::payload.describe()` (`sandbox.rs:346,567`). `payload` is `Unknown` only
when `ContainerTerminalFacts::new(state, pidfd)` gets `pidfd == None` and the runtime code is absent/ambiguous
(`sandbox.rs:468-476`). So the question is: when is the pidfd argument `None`?

| # | Hypothesis | Code | Verdict |
|---|---|---|---|
| H1 | **Follower freezes expired facts with `pidfd = None`.** The snapshot owner (first of observer `on_child_exit` -> `terminal.facts()`, `supervision.rs:986`; bridge `wait_and_finalize` -> `facts_by`, `runtime.rs:1277`; `run_teardown` -> `facts_by`, `child.rs:459`) runs `observe(by)` to the 10 s `SNAPSHOT_BUDGET` while podman 4.9.3 `inspect` is stalled or non-terminal. A follower waits on the condvar until `deadline` (`child.rs:262-268`). At `now >= deadline` it does `get_or_insert(Arc::new(ContainerTerminalFacts::new(Err(DeadlineExpired), None)))` (`child.rs:253-255`) **before the owner publishes**. The owner's `pidfd_exit(by)` probe (`child.rs:208`) runs only after its inspect loop ends at `by`, so it loses the race and its `get_or_insert` (`child.rs:238`) returns the follower's pidfd-less value. Result: state `Err`, payload `Unknown`, source `None`, which is the failing row. | **Supported.** Log: elapsed 10.061 s = `SNAPSHOT_BUDGET` + 61 ms (same bimodal signature as the 6 earlier families, `flake-diagnosis.md`: failures sit exactly one budget higher). No `residue` line (`residue line []`) and no "CID kill failed" line in stderr means teardown returned `Ok`: K1w `snapshot_settled` (`child.rs:459-461,333-346`) waited for the owner and `rm` then succeeded. That is the post-20-7 shape of a stalled snapshot. The 20-7 stub vector (`t3_terminal_helpers_17_7.rs:525-595`) uses `adopt` (no pidfd), so it cannot see the loss. |
| H2 | Kill landed before the supervisor opened the pidfd | `spawn.rs:513-523` opens and arms the pidfd before `start --attach`; the test kills only after `cli_wrapper_loaded` publishes the CID (`worker_terminal_facts_17_7.rs:253-262`). | **Refuted.** |
| H3 | `inspect_live_payload` returned a pid that is not the pidfd's pid (conmon, pause, rootless-ns pid) | `inspect_live_payload` (`..._17_7.rs:183-198`) asserts `/proc/<pid>/cgroup` names the CID (`cgroup_names_container`), the same check that gates the pidfd (`spawn.rs:313-319`). The root also saw `process_crash`. | **Unlikely.** A pid mismatch would not crash the Worker. |
| H4 | pidfd opened after the process was reaped / `waitid` race | `open_payload_pidfd` returns `None` if the pidfd is already readable (`spawn.rs:319`); that is before `start`, so the payload has not run. No `waitid` in this path: the status comes from `PIDFD_GET_INFO` (`linux.rs:651-668`). | **Refuted** for the pre-start arm. |
| H5 | `open_payload_pidfd` returns `None` (cgroup not yet naming the CID at `/proc/<pid>/cgroup` read; rootless cgroup oddity) | `spawn.rs:313-319`. | **Possible but not excluded:** the same arm path passed in the same job's other T3 scenes and in PR #66. [INFERENCE] intermittent only if the cgroup move races `init`. Distinguishable only with the exit row source/pidfd flag. |
| H6 | Kernel < 6.15: `PIDFD_GET_INFO`/`PIDFD_INFO_EXIT` unsupported, so `pidfd_exit_status` is `Err`/`Ok(None)` and `pidfd_exit` returns `None` (`child.rs:178-185`) | `linux.rs:648-668`; `t3_terminal_helpers_17_7.rs:280-302` panics on CI kernels < 6.15. | **Would be deterministic** and fail every run; the test passes in other runs of the same image family (PR #66). Runner kernel is not in the log (image 20261116.133.1). Unverified, cheap to settle with `uname -r`. |
| H7 | Client exit 125 / conmon | 125 is the `podman start --attach` client status. It becomes `exit_cause`, not `payload_exit`; `payload_exit` ignores the client. | **Refuted** as a source of `unknown` by itself. 125 is the same symptom as the earlier family (podman 4.x start/attach exit-code race), not the cause. |

Why the race is real, not theoretical: at the deadline the owner still has to (a) let `invoke_bounded` kill/reap the
timed-out `inspect` client and drain pipes (`spawn.rs:138-226`, `DRAIN_GRACE` 200 ms), or (b) exit its non-terminal
`inspect` loop and then take the identity mutex and issue the `PIDFD_GET_INFO` ioctl. The follower only needs a futex
timeout wakeup at the same instant. In case (a) the follower wins nearly always. In case (b) it is a scheduling
coin-flip, which matches "10.06 s and flaky, passes elsewhere".

## 2. Classification

Product defect: the supervisor loses a fact it holds (`identity.pidfd`, no runtime lock needed per the comment at
`child.rs:446`) under a legitimate race, and the 20-7 test contract says "the pidfd/cleanup facts hold whether or not
the runtime CLI answered its snapshot" (`worker_terminal_facts_17_7.rs` comment before the facts assertions). The
product does not honour that contract in the follower-expiry path.

Missing evidence to decide among H1/H5/H6: the exit row `container` object (`runtime_exit_code`, `oom_marker`,
`payload_exit`, `payload_exit_source`, `removed`, `residue`) and `uname -r`. The log shows only the first 768 bytes
of the assertion message line and none of the row. With `runtime_exit_code: null, oom_marker: null, payload_exit:
"unknown", source: "none", removed: true` H1 (or H5/H6) is confirmed as a snapshot-`Err` plus pidfd-`None` row; H1 vs H5 needs
the ordering evidence below.

## 3. The ONE diagnostic to add (class b)

Print the whole exit-row `container` object and the kernel release in `s3_diagnostics`. The existing
`exit_row` panics before diagnostics could see it; read the row directly:

```diff
--- a/crates/maos-bin/tests/worker_terminal_facts_17_7.rs
+++ b/crates/maos-bin/tests/worker_terminal_facts_17_7.rs
@@ fn s3_diagnostics(run: &TerminalRun, elapsed: Duration, version: &str) -> String {
     let line = |needle: &str| {
         run.stderr
             .lines()
             .filter(|line| line.contains(needle))
             .collect::<Vec<_>>()
             .join(" | ")
     };
+    // The decisive fact for the `unknown` family: the journaled terminal facts
+    // and the kernel (PIDFD_INFO_EXIT needs Linux >= 6.15).
+    let exit_rows = run
+        .rows
+        .iter()
+        .filter(|(_, intent, _)| intent == "cli.subprocess.exit")
+        .map(|(_, _, payload)| payload.to_string())
+        .collect::<Vec<_>>()
+        .join(" | ");
+    let kernel = Command::new("uname")
+        .arg("-r")
+        .output()
+        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
+        .unwrap_or_else(|_| "unknown".into());
     format!(
-        "elapsed {elapsed:?}; runtime {version}; root status {:?}; client exit [{}]; \
-         residue line [{}]; stderr: {}",
-        run.status,
+        "elapsed {elapsed:?}; runtime {version}; kernel {kernel}; root status {:?}; \
+         client exit [{}]; residue line [{}]; exit rows [{exit_rows}]; stderr: {}",
+        run.status,
         line("exited ("),
         line("residue "),
         run.stderr
     )
```

Also put the diagnostics in front of the stderr dump (it is already last, so the 768-byte log truncation of the
message line cuts before it today: move `exit rows` and `kernel` ahead of the long `client exit` text, or print them
with a separate `eprintln!` before the first assertion so they survive log truncation).

## 4. The fix (class c) and the test that would catch it

`crates/maos-kernel-core/src/security/sandbox/t3/child.rs`, `facts_by` expiry branch: the follower must not discard
the pidfd. The probe is lock-free with respect to the runtime and non-blocking (`by = now`: one ioctl, returns at the
first `Ok(None)` before `pause`); the lock order is progress -> identity and nothing takes identity -> progress
(checked: `kill_payload`, `pidfd_ready`, `has_pidfd`, `started`, `arm`, `pidfd_exit` take identity only).

```diff
--- a/crates/maos-kernel-core/src/security/sandbox/t3/child.rs
+++ b/crates/maos-kernel-core/src/security/sandbox/t3/child.rs
@@ pub(crate) fn facts_by(
             if now >= deadline {
-                let expired =
-                    ContainerTerminalFacts::new(Err(TerminalUnavailable::DeadlineExpired), None);
+                // Typed unavailable runtime state, but the verified payload pidfd
+                // needs no runtime lock: one non-blocking probe keeps `signaled(9)`.
+                let expired = ContainerTerminalFacts::new(
+                    Err(TerminalUnavailable::DeadlineExpired),
+                    self.pidfd_exit(now),
+                );
                 let facts = Arc::clone(progress.facts.get_or_insert(Arc::new(expired)));
```

Same class, second site (optional in the same change): `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:1277-1283`
builds an unfrozen `DeadlineExpired` fallback with `None` when a capped follower gets `None`. Add a
`pub(crate) fn unavailable_facts(&self) -> Arc<ContainerTerminalFacts>` on `ContainerTerminal` that applies the same
probe and call it there. Not the cause of this failure (that follower path returns `None` only before the deadline).

Catching test (not run; kernel-core crate-internal because `arm`/`own` are `pub(crate)`; gated like
`require_pidfd_exit`, so it needs Linux >= 6.15 and skips with the existing helper otherwise): add in a `#[cfg(test)]
mod` in `child.rs` or expose a `#[doc(hidden)] pub fn adopt_with_pidfd`:

1. Stub runtime whose `inspect` is `exec sleep 30`.
2. External child killed with SIGKILL and reaped; `pidfd_open` taken before the kill; `terminal` armed with
   `(pid, Some(pidfd))` and `started = true`.
3. Owner thread: `terminal.facts_by(Instant::now() + 1s, false)` (snapshot deadline = cap, 1 s).
4. Main thread, after the stub `inspect` is entered: `terminal.facts_by(Instant::now() + 20s, true)`.
5. Assert both are `Arc::ptr_eq`, `state == Err(DeadlineExpired)`/`InspectTimedOut`, and
   `payload == PayloadExit::Signaled(9)`, `source == PayloadExitSource::Pidfd`. Loop 20 instances like
   `stalled_snapshot_instance` (`t3_terminal_helpers_17_7.rs:525`) to cover both orderings.

Red before the change (follower wins: `Unknown`/`None`), green after.

## 5. Recommended action for PR #65

PR #65 changes no runtime/sandbox code, so this is not a regression from it. Re-run the failed job once, as infra: the
failure is a known stalled-podman-4.9.3 family and the trigger is not reproducible on demand. Do NOT block #65 on it.
Open the fix + diagnostics as a separate change (main-line story or tracker row owned with the 20-7 family): ship the
diagnostic (§3) first or together, because it would convert the next hosted occurrence from 70% to a proof
(exit row with `payload_exit_source: none` and `kernel >= 6.15` => H1/H5, and the 20-instance pidfd test then decides H1).

## 6. Unverified

- The exit row contents and the runner kernel for this failing job (not in the log; 768-byte truncation of the
  assertion line).
- That podman 4.9.3 `inspect` stalled (rather than returned non-terminal) in this sample: only the 10.06 s duration and
  the clean `rm` support it.
- H5 (pidfd `None` at arm time) is not excluded; no log line distinguishes it from H1 without the diagnostic.
- The proposed diff and test were not compiled or run (no cargo, no podman on this host).
