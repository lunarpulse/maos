---
story_id: '17.7'
story_key: 17-7-t3-worker-retained-lifecycle-and-terminal-facts
created: 2026-10-13
context_commit: 37c34be4
baseline_commit: 37c34be44d0d7e5496725bd305a45e199411e879
security_sensitive: true
---

# Story 17.7: T3 Worker retained lifecycle and terminal facts

Status: done

## Story

As an operator,
I want a T3 Worker's container to stay owned by MAOS from creation to removal, with its real terminal facts read before anything is classified,
so that a short task completes instead of being refused, an OOM kill, an external SIGKILL and an intentional `exit(137)` are journaled as what they are, and no failed stop leaves an unreported container behind.

**Contract:** epic-17 §17-7 (AC1–AC5) as amended by §R12, `deferred-work.md:1121`, `:1122`, `:1133` (original rows `:1119`, `:1120`, `:1126`). ◆ FLAG-Winston and security-sensitive: the non-author §A6 review net (Blind + Edge + Acceptance + Test-Infra + runtime execution) is non-degradable. The E16-A7 measured prototype is done (§Measured prototype). On 2026-10-13 the operator **ratified** `17-7-flag-winston-decision.md` (D-17-7-FW-01…09): **variant A** (payload pidfd + `PIDFD_INFO_EXIT`) over seven kernel files; kernel ≤ 25574 physical / ≤ 19946 tokei with the exact lower figure at landing; out-of-kernel rows exact at landing as measured requests; four `unsafe` sites in the sandbox zone with I9 exemptions; no boot-time sweep; the `ci/17-7-<purpose>` CI route; and the advisory routing. `ready-for-dev` means the guide is prepared and the charter ruled, not that any figure above a current ceiling is approved. The later D-17-7-FW-10 founder decision waives only this story's second-maintainer signature; runtime, review and measured-authorization requirements remain binding.

## Current-tree corrections — read before implementing

Every epic cite was re-measured at `37c34be4` by five read-only scouts and checked again by three non-author validators; the OLD→NEW rows that reshape the epic are **R17-117…R17-132** in epic §R12.

| Epic premise | Current fact and consequence |
|---|---|
| Three controls (`exit(137)`, external SIGKILL, OOM) are distinguishable from runtime evidence | **Not from Podman.** conmon collapses a signal death to `128+N` (`ctr_exit.c:100-107` in shipped 2.1.10/2.1.13); `InspectContainerState` has no signal field (podman `container_inspect.go` v4.9.3 `:208-232`, v5.7.0 `:298-322`); the `died` event carries only the integer. The only authoritative source is the payload's own wait status, readable for a **non-child** through a pidfd and `PIDFD_GET_INFO`/`PIDFD_INFO_EXIT` (Linux ≥ 6.15; runner kernels 6.17 and 7.0, R17-104). The operator ruled that route (variant A, D-17-7-FW-01); runtime-only evidence remains the per-spawn fallback (§AC1 fallback rows). |
| Removing `--rm` and polling `State.Pid` fixes the short-task refusal | An exited container reports `State.Pid = 0` (podman `container_internal.go` v4.9.3 `:2448-2476`). Startup identity is the **runtime-written pidfile**: `--pidfile` is a `create` option, written by the OCI runtime during `podman init` before the workload exec gate opens, and never removed at exit. `podman create` prints the full CID synchronously before start. The lifecycle is `create` → `init` → read pidfile → `start --attach`; no startup polling remains. |
| The container guard is dropped before journaling at `runtime.rs:503-510` | That range is now the T2 cgroup accessor. The drop is `self.child = None` at `runtime.rs:948`, before the exit row at `:974-989`. |
| `FaultCause::OomKilled` sits at `maos-domain/src/supervision.rs:63-85` | `:62-85` are the accessors; the enum is `:92-114`, `OomKilled` at `:109`. No production path constructs it; T2's `oom_kill` count writes only a `SandboxBlock` audit row (`runtime.rs:965-971`). |
| Named kills are unbounded | Only two are: `BridgeChild::kill` (`runtime.rs:481-490`, `.status()` at `:485`) and `stop_named_container` (`maos-bin/src/supervision.rs:906-941`, `.status()` at `:923`). The guard's `Drop` already uses `runtime_call_bounded` (`t3/child.rs:88-97`, `t3/spawn.rs:67-92`) but by **requested name**, ignoring results, without `--volumes`. That helper is also not truly bounded: it drains stdout only after exit and `wait_with_output` can outlive the deadline (`t3/spawn.rs:72-89`). |
| The lease cite `worker_spawn.rs:770-798` is the lease end | That is TTL/credential setup. Acquisition is `worker_spawn.rs:803-812`; release is `WorkerBinding::finish` (`supervision.rs:1113-1120`) or `WorkerEgress::drop` (`egress.rs:181-187`), the latter when the binding drops — **before** the bridge field drops (`LiveWorker`, `supervision.rs:1159-1162`). `WorkerEgress::finish` (`egress.rs:160-178`) has no "container termination unconfirmed" state: a successful proxy stop deletes the lease from the root registry even if the container still runs. |
| The 30 s hook budget bounds the `on_unload` container kill | The budget times out the await on `spawn_blocking` (`hook_dispatch.rs:572-589,665-687`); it never cancels the blocked runtime command. Mechanism (x) already releases SCB/receipt/capabilities before the hook (`scheduler_loop.rs:527-583`); the comment at `supervision.rs:816-821` that says otherwise is stale. The door unload (`operator_door.rs:524`) reaches the container **only** through the hook, so a hook that panics before its stop leaves the Worker running until root stop. |
| The startup report is journaled | `record_sandbox_application` promises "the exact report served by operator HTTP" in the lifecycle journal (`scheduler_loop.rs:232-253`), but maos-bin passes the scheduler adapter as the journal (`supervision.rs:1681-1687`), whose `journal_lifecycle` drops the payload (`scheduler_loop.rs:735-754`). Fixed at origin in maos-bin by AC2.4 (rule 10(a)). |
| Only two fixtures hold for 3 s; only two helpers scrape argv | Five T3 inspection holds exist (`worker-cli-fixture.rs:44`, `worker-probe-fixture.rs:81-88`, `worker_completion_2a.rs:837`, `sandbox_t3_spawn_linux.rs:47`, `t3-network-escape-probe/src/main.rs:27-28,43`). Six simulated-runtime tests pin `kill <name>` (`worker_supervision_16_3.rs:815-855,889-907,912-1152`). The resource probe's 5 s burst + 2 s retention (`worker-cli-fixture.rs:260-276`) is 17-2's live-cgroup evidence, **not** an inspection hold: keep it. |
| Only `worker_spawn.rs` builds a T3 launch | Five constructors: `runtime.rs:568` (`T3SpawnContext`), `worker_spawn.rs:927` (`T3Launch`), `main.rs:7084-7093` (direct smoke; reads `child.host_pid` at `:7106-7108`), `sandbox_t3_spawn_linux.rs:63`, `t3_network_escape_executor.rs:93-98`. All migrate. |
| Rewrite cookbook EN `:71` / KO `:72` | Also EN `:77` / KO `:78` (added by 17-2), `4-kernel-design.md:337` and `5-spirit-abi.md:130-131`. |
| `--pidfile`/`--cidfile` refuse a stale file | They do not: crun truncates (`container.c` 1.14.1 `:1939-1946`), runc renames over, podman's `CreateIDFile` truncates (`pkg/util/utils.go` v4.9.3 `:1104-1114`). Freshness is MAOS's job: a new private per-spawn directory. |
| OOMKilled proves the cgroup killed init | A conmon marker set on `memory.events` `oom` **or** `oom_kill` (shipped `cgroup.c` 2.1.10 `:268-314`), hierarchical and victim-less. Bounded evidence only — §OOM authority contract. |

## Acceptance Criteria

### AC1 — command: four terminal controls on both x64 images

**Given** the `epic-17-exit` job (`.github/workflows/discipline.yml:1806-1899`, matrix `ubuntu-24.04` + `ubuntu-26.04` x64, `:1807-1812`) with the T3 substrate it already provisions (`:1827`), running the new line **inside** 17-2's delegated rootless envelope (runner `scripts/run_cgroup_ceiling_17_2.py` **`run` action**, with `--mode delegated --unit maos-17-7-terminal --timeout 1800 --rootless-canary`; suite command follows `--`, as `cgroup-ceiling-delegated` does at `:3246-3253`) because the OOM scene needs a delegated memory controller,
**when** exit line 4 runs:

```
cargo test --locked -p maos-bin --features network --test worker_terminal_facts_17_7 -- --nocapture --test-threads=1
```

**then** it runs every scene of the suite — S1–S5 below plus the AC2/AC4 scenes named in T4 — refuses `SKIP` in CI, asserts the executed count equals the declared count, and each S-scene journals exactly these rows for its Worker's real `spirit_pid` (read back from the Transparency Log; stdout echoes are never evidence):

| Scene | Fixture | `cli.subprocess.exit` (kind 7) | `task.orphaned` (kind 1) | Verdict / other |
|---|---|---|---|---|
| S1 immediate exit | default mode, hold deleted | `exit_cause:"exited(code=0)"`, `is_crash:false`, `container:{runtime_exit_code:0, oom_marker:false, payload_exit:"exited(code=0)", payload_exit_source:"pidfd", removed:true, residue:null}` | **none** (no `lifecycle.crash`, no `unplanned_crash`) | `worker.completion-verdict` `completed:true`; run exits 0 |
| S2 intentional `exit(137)` | new mode `exit-137` | `exit_cause:"exited(code=137)"`, `is_crash:true`, `container:{runtime_exit_code:137, oom_marker:false, payload_exit:"exited(code=137)", payload_exit_source:"pidfd", removed:true, residue:null}` | `cause:"fault.non-zero-exit"`, `exit_code:137`, `exit_signal:null` | one `unplanned_crash`; `not_completed:process_crash` |
| S3 external SIGKILL | `line-then-hang`; the test resolves the live payload host PID from the CID (`inspect --format {{.State.Pid}} <cid>`), verifies `/proc/<pid>/cgroup` names the CID, then `kill -KILL` | `exit_cause:"exited(code=137)"`, `container:{runtime_exit_code:137, oom_marker:false, payload_exit:"signaled(sig=9)", payload_exit_source:"pidfd", removed:true, residue:null}` | `cause:"fault.signaled"`, `exit_signal:9`, `exit_code:null` | one `unplanned_crash`; detection (observer `on_child_exit`) and the `task.orphaned` row are each timed from the kill and reported; the row is ≤ 5 s (FR12) |
| S4 `memory.max` OOM | new mode `oom` under `[resources] memory_max_mb = 64`: prints its own `/sys/fs/cgroup/memory.max` and `memory.swap.max`, then retains and writes 1 MiB pages up to a fixed 1 GiB cap, printing a running total | `container:{runtime_exit_code:137, oom_marker:true, payload_exit:"signaled(sig=9)", payload_exit_source:"pidfd", removed:true, residue:null}` | `cause:"fault.oom_killed"`, `exit_signal:9`, `exit_code:null` | first persisted stderr line is `67108864`; the fixture never reaches its 1 GiB cap |
| S5 failing `on_unload` | `hang`; door-style unload with `on_unload` injected to panic **before** its stop, then (second run) to sleep past a seam-shortened hook budget | if written, `container.removed:true` or one `residue` | **no** `fault.*` row (a planned stop is not a crash); the `on_unload_hook_failure` writer only if ledger entries remain (`scheduler_loop.rs:689-723`) | the existing hook panic / `hook.budget.on_unload` row (`hook_dispatch.rs:647-687`); exactly one `planned_unload` receipt; the CID not running within 5 s of `unload` returning, then removed or named by exactly one residue report (AC4.6) |

The CI job fails on any `SKIP`, zero executed tests, an S4 run whose first stderr line is not `67108864`, or any S1–S4 scene whose `payload_exit_source` is not `pidfd` (rule 11(b): both runner kernels support `PIDFD_INFO_EXIT`, so missing evidence there is a failure, not a fallback). The aggregate gains an explicit `needs['epic-17-exit'].result != 'success'` predicate (none exists today; `discipline.yml:5176-5181,5342-5346` is the cgroup-job pattern) with a behavioral workflow test (precedent `xtask/tests/cgroup_ceiling_workflow_17_2.rs:103-161`). Every control in §Proven-red runs red once in CI with its run id recorded (rule 11(c)).

**Fallback rows (D-17-7-FW-01, variant A ruled).** When a spawn's pidfd evidence is unavailable (kernel < 6.15, `ESRCH`, identity not verifiable), exactly these cells change: S2 and S3 become identical — `payload_exit:"unknown"`, `payload_exit_source:"none"`, `task.orphaned` `cause:"fault.non-zero-exit"`, `exit_code:137`, `exit_signal:null`; S4 becomes `payload_exit:"unknown"`, `payload_exit_source:"none"`, `cause:"fault.oom_killed"`, `exit_signal:null`, `exit_code:null` (runtime code 137 and `oom_marker:true` unchanged); S1 keeps `payload_exit:"exited(code=0)"` with `payload_exit_source:"runtime"`. A dedicated scene forces this fallback through a test seam and asserts these rows.

**Decoding rule.** `payload_exit_source:"pidfd"` → a returned `PIDFD_INFO_EXIT` bit and a terminal raw wait status (`WIFEXITED`/`WIFSIGNALED`), never a zero-initialized field without the returned bit. Otherwise the runtime `ExitCode` of a **terminal** state is decoded only where unambiguous: `0…=128` and `193…=255` → `exited(code=N)` with source `runtime`; `129…=192` (`128+signal`) → `unknown` with source `none`. Missing, non-integral, negative (including podman's missing-exit-file `-1`) or >255 codes provide no valid runtime exit evidence: without a valid pidfd status the classification is `Truncated{reason}`, never `NonZeroExit{-1}` or an invented payload code. A signal is **never** inferred from `128+N`. `task.orphaned.exit_signal` is non-null only with source `pidfd`.

[Sources: epic §17-7 AC1; today's T3 crash scene `worker_supervision_16_3.rs:1614-1817` pins the old `exited(code=137)`/`fault.non-zero-exit` at `:1751-1769`; `worker_completion_2a.rs:226-238`; FR12 `functional-requirements.md:41`; NFR-Rel-1 `non-functional-requirements.md:20`.]

### AC2 — runtime-owned startup identity

1. A per-spawn private state directory (mode `0700`, owned by the daemon's euid, created fresh, empty) holds the runtime-written pidfile and CID receipt (AC4.1). Validate it **before runtime detection/probing or any runtime call**. Refuse with typed `T3Error` before `create` if it is not fresh, private, euid-owned and empty. After `init`, refuse, naming the owned CID and removing **only that CID**, if the pidfile is missing, a symlink, a FIFO or other non-regular file, oversized or invalid. The reader never blocks on the path: hold the inode with `O_PATH | O_NOFOLLOW`, check regular-file type, then reopen the held inode through `/proc/self/fd` non-blocking and re-check identity (`maos-exec-deps/src/lib.rs:215-241`). Read at most 11 bytes; accept ASCII decimal digits and at most one final newline, with a parsed PID in `1…=i32::MAX`; reject trailing data, signs, zero and overflow before any `pid_t` conversion. `inspect_container_host_pid` (`t3/spawn.rs:225-258`) and every startup `State.Pid` poll are deleted.
2. `SandboxInspectReport.pid: u32` (`maos-domain/src/sandbox.rs:297-305`) becomes `payload_pid` of a typed observed-PID whose T3 producer emits the **historical** startup observation (`{"historical":N}` on the wire), plus `container_id` (64 lowercase hex). Nothing in the report claims liveness. No production path takes a PID from the report as a kill target: the kill authorities are the owned CID and, under variant A, the payload pidfd. Migrate `crates/maos-control/tests/post_surface_16_1.rs:38-57,930-934` and the domain round-trip (`sandbox.rs:488-509`).
3. `cli_wrapper_loaded` (stdout, `worker_spawn.rs:968-979`) gains `container_id`; `child_pid` stays the runtime **client** PID (now the `start --attach` client). Tests find containers by this CID, never by argv scraping. The direct smoke (`main.rs:7084-7108`) prints the CID and the historical PID instead of `host_pid`.
4. **Rule 10(a), fixed at origin:** `commit_container_spawn` (`supervision.rs:1675-1689`) journals the report through the shared lifecycle `Arc<JournalAdapter>` (`main.rs:2752`), not the scheduler adapter, so the `SandboxApplied` entry keeps its payload. A test reopens the journal and deserializes the Worker's `SandboxApplied` payload to the report with its `container_id`. `SandboxApplied` is informational; the raw last-event contract of 17-2 (R17-110) is unchanged.

### AC3 — one terminal snapshot before either classifier

1. One typed `ContainerTerminalFacts` is read **once** by owned CID while retained. Required `.State` fields are `Status` (string), `ExitCode` (integer), `OOMKilled` (boolean), `Error` (string), `StartedAt` and `FinishedAt` (strings); absent/null/wrong-type fields yield typed unavailable, never default exit 0/marker false. Empty Error and legitimate zero-time strings are valid. Convert Error to a non-secret category and discard raw text before caching/debugging. Add the verified payload wait status when available; a valid pidfd status stays authoritative even if the raw runtime code is a sentinel/out of payload range. Whichever feed enters first owns initialization; all feeds get the same immutable result and **one absolute deadline of 2 s from first entry**, shared by terminality wait, inspect and pidfd read. Followers use that deadline, not another timeout; expiration freezes typed *unavailable* evidence. Only terminal state (`exited`/`stopped`) is decoded; `created`, `running`, `paused` or `stopping` is never cached as an exit. Unsupported/missing pidfd evidence takes the declared fallback; it does not invalidate a valid terminal runtime observation.
2. **T3 classification** (§Classification): unavailable → `FaultCause::Truncated { reason }` with a bounded, non-secret reason — **even when the runtime client exited 0**; a genuinely intentional `PlannedStop` still wins. Unexpected client death while the payload runs triggers **fault cleanup**, which never creates a `PlannedStop` latch and never rewrites the original unavailable/fault evidence after its kill succeeds.
3. The snapshot reaches **both** the pidfd observer (`BindingCore::on_child_exit`, `supervision.rs:734-747`, via `observe_exit` `:2600-2634`) and bridge finalization (`wait_and_finalize`, `runtime.rs:922-996`, then `crash_cause_from_exit` `supervision.rs:1176-1204`) **before** `classify_exit` (`:1820-1845`). The shared handle is held by `BindingCore` independently of the one-shot stop action (today `container_stop` is `take()`n at `:888,944` and cleared at `:675-676,806`), so both feeds keep it after every stop attempt. Removal happens only after the snapshot exists.
4. Preserved exactly: the observer publishes nothing on `ECHILD` (`:2622-2629`) and finish is then the sole classification; the intentional `PlannedStop` latch (`:370-382`) wins over any fact (an intentional MAOS stop is never a crash; fault cleanup creates no latch); `is_typed_refusal` (`:713-719`); one crash handler per binding. No binding, bridge, runtime or proxy lock is held while waiting on the snapshot or runtime I/O, and snapshot initialization never calls into a kill/remove that waits for the same initialization.
5. The runtime-client status stays separate: `exit_cause`/`is_crash` in `cli.subprocess.exit` describe the client, and the new `container` object describes the container. **T0/T2 exit rows stay byte-identical** (four keys, no `container:null`); native SIGKILL stays `fault.signaled` (`worker_supervision_16_3.rs:561-566`).
6. **Completion follows the same evidence:** for T3, the completion oracle's `WorkerExit` (`worker_spawn.rs:1044-1047`, today from `exit.cause.exit_code()`) comes from the verified payload exit, not `CrashCause::exit_code()` (which returns `None` for `Voluntary`, `maos-domain/src/supervision.rs:70-74`). Verified `exited(0)` supplies `WorkerExit::Exited(0)` to the existing journal-output oracle; signaled, unknown, unavailable or intentionally interrupted runs cannot complete, even with a persisted marker and client code 0. A cleanup verdict never upgrades completion. Native completion is unchanged.
7. `OOMKilled` follows the §OOM authority contract; every row of §Classification is a unit test.

### AC4 — ownership and cleanup

1. The CID is owned **before `init` or start**. `create` writes `--cidfile=<state_dir>/cid` into an initially absent path and prints the CID. Read this per-spawn receipt with AC2.1's held-inode, non-blocking discipline (≤65 bytes; exactly 64 lowercase hex plus optional final newline). The **validated private receipt is the sole cleanup authority**; stdout is only a corroborating channel, and normal success requires agreement. Preserve receipt identity even if the create client fails or times out; arm its CID guard immediately and refuse startup. On disagreement, clean up only the receipt CID; the stdout-named second container is untouched. Missing/invalid receipt means typed **creation-ownership-unknown** residue even with a syntactically valid stdout CID: refuse startup, report once with phase and non-secret per-spawn locator, retain the state directory as evidence, and make no name- or stdout-authorized destructive call. `ContainerCreate` precedes receipt writing and stdout ([create.go v4.9.3:181-195](https://github.com/containers/podman/blob/v4.9.3/cmd/podman/containers/create.go#L181-L195)), so no receipt is not proof of no container. This supersedes R17-119's stdout-only detail; it does not authorize a recovery sweep. A name-collision refusal touches nothing; its planted existing container survives (R17-114). CID authority survives rename: create A, rename A externally, create test-owned B under A's original name, finalize A → A removed, B survives.
2. `--rm` is gone; `--restart=no` is explicit; PID 1 is `/maos/spirit`. Pin `--entrypoint=/maos/spirit` and pass `IMAGE -- <command…>` (remove the old post-image `/maos/spirit`, `argv.rs:100-103`, so it is not duplicated); never add `--init`. Explicit removal is bounded: `rm --force --volumes --time=0 <cid>` then `container exists <cid>` → exit 1. Timeout, non-zero, volume errors on stderr or failed existence confirmation are typed **residue**, never claimed removed. Finalization, stop and guard `Drop` share one idempotent verdict per CID: one removal attempt, no concurrent remove while snapshot initialization is in flight, no later retry or reinterpretation by `Drop`. Reporting happens once per sink (exit row if available; root stderr always for residue), using §Exit-row schema.
3. Replace the two unbounded named kills (`runtime.rs:481-490`, `supervision.rs:906-941`) with bounded CID kills. **Intentional** abandon, unload and root stop latch `PlannedStop` before signaling; cleanup of an already-observed unexpected failure does **not** latch it. The kernel guard performs mechanical cleanup and never invents supervising intent. Both paths use CID kill → termination confirmation (terminal state, or verified payload pidfd `POLLHUP`) → immutable snapshot if absent → removal. Killing the client alone is never confirmation (it is not the payload's parent; SIGKILL is not proxied). Failed/timed-out kills are reported, not counted in `signals_sent` (`:892-902`).
4. **One teardown budget (constants, asserted with runtime stubs):** from teardown entry, kill plus termination confirmation share ≤5 s; snapshot uses ≤2 s; removal plus existence check share ≤10 s; every stage also uses the remainder of the same ≤17 s absolute teardown deadline. Root stop latches all bindings and dispatches their kills concurrently before waiting for any one result; confirmation, snapshots and removal remain in flight concurrently under **one root deadline**, not N sequential windows (`stop_workers` is serial today, `supervision.rs:1537-1543`). Concurrent finalization and `Drop` join the already-started work with its remaining budget. The signal listener's five-second idle grace / forced exit (`:2315-2347`) must not bypass T3 cleanup and residue reporting: it waits for the bounded coordinator or reports every unresolved CID at its deadline before exiting. Native behavior is unchanged. Runtime stdout/stderr are drained concurrently with caps; deadline expiry closes/cancels drains and returns a typed result without blocking on a descendant-held pipe or synchronous reap. A runtime client whose termination/reap cannot be confirmed remains explicit runtime-client residue; never promise that SIGKILL guarantees immediate reap.
5. **The proxy lease ends at confirmed payload termination.** The immutable classification snapshot and the monotonic cleanup-confirmation state are separate: a cached unavailable snapshot stays unavailable for both feeds, but a subsequent CID/pidfd confirmation permits lease finalization. No fresh classification or completion follows cleanup. On the normal path `wait_and_finalize` confirms termination before `WorkerBinding::finish`; on abandon/unload/root stop the binding confirms before `WorkerEgress::finish`, keeping `LiveWorker`'s binding-before-bridge order. If termination remains unconfirmed, revoke the bearer immediately (fail closed) but keep the lease as root-owned residue across `finish`, binding `Drop` and root cancellation (`egress.rs:35-53,76-82`; `supervision.rs:2174-2200`). Report it before root exit; never silently erase it when the in-memory owner disappears.
6. **On_unload independence:** every maos-bin path that unloads a Worker's SCB — door `operator_door.rs:524,991`, `unload_all_loaded` `supervision.rs:139-156`, the executor `:1925-1930`, `main.rs:6478` — stops that binding through its registry after `scheduler.unload` returns, whatever the hook outcome; the hook's own stop stays the fast path and is idempotent with it. A real T3 Worker whose `WorkerSpirit::on_unload` (`supervision.rs:1985-1993`) panics before its stop, or exceeds its hook budget, therefore leaves no running container and no unreported residue (`deferred-work.md:1126`; S5). Mechanism (x) is unchanged: exactly one `planned_unload` receipt.
7. A stale container after a daemon crash is never restarted with its stored bearer: `--restart=no`; no MAOS path starts a CID it did not create in this process. Scene: SIGKILL the root mid-Worker; record the CID and `StartedAt`; stop that test-owned CID externally if it still runs and confirm it is retained but stopped; run a second root in the same home and assert the old CID stays stopped with unchanged `StartedAt` while the new root creates a different CID; then run `maos purge` (dry run) in the same runtime/store context and assert it reports the old container (`purge.rs:1776-1821` — if its conventional-storage guard hides the configured store, fixing that guard is in scope). No boot-time destructive sweep is added (D-17-7-FW-07). Teardown removes only test-owned CIDs.
8. Runtime diagnostics are never echoed raw: refusals, residue and the exit row carry typed, bounded, non-secret categories (phase, CID, status, reason code). The snapshot requests `{{json .State}}` only, never full inspect or `Config.Env`. A runtime stub that echoes the bearer in `create` stderr, `State.Error` and `rm` stderr proves the value reaches no refusal, exit row, stdout, stderr or Transparency Log row.

### AC5 — no workaround survives

1. Delete the five T3 inspection holds listed in §Current-tree corrections — for the escape probe both `INSPECTION_HOLD` and its `sleep` (`t3-network-escape-probe/src/main.rs:27-28,43`); keep its `IO_TIMEOUT` (`:26`), the probe fixture's `IO_TIMEOUT` (`worker-probe-fixture.rs:29`) and the resource probe's retention.
2. Migrate the client-argv/name helpers (`worker_supervision_16_3.rs:1819-1854`, callers `:1617,1670` and `:1869,1915`; `root_shutdown_unload_16_3.rs:222-468`, six callers, module doc `:35-41`) to the published CID. Migrate the six `ControlledRuntime` name-kill tests (`:815-855,889-907,912-1152`) to CID semantics, keeping their ordering and failed-stop assertions; `wait_container_stopped` (`root_shutdown_unload_16_3.rs:370-384`) must also require removal.
3. Rewrite the limitation text to shipped behavior: cookbook EN `cli-wrapper-spirit.md:71,77`, KO `:72,78`; `4-kernel-design.md:337`; `5-spirit-abi.md:130-131`; fixture comment `worker-cli-fixture.rs:24-26`; `t3/mod.rs` DR-5.5a-7 (PID identity) and the `t3/child.rs`/`t3/spawn.rs` doc comments. KO coverage and glossary lock stay green (`LOCALES.md:20-33,123-136`).
4. At the `done` flip, rewrite the three owned deferred-work rows in place as closure records with evidence — never erase the owner cue (`check_dev_record_completeness.rs:213-266`). Completed records are now `deferred-work.md:1121`, `:1122`, `:1133` (formerly `:1119`, `:1120`, `:1126`).

### Delivery gate — measured authorization and the re-pin set

Kernel edits stay inside the FW-01 locations; any other kernel file returns as a charter request. Measure the complete formatted implementation; a figure above the ruled FW-02/FW-03 bound returns to the operator **before** landing. Every new `Arc`/`OnceLock`/`Mutex`-bearing kernel holder (the shared terminal handle, the guard field holding it) carries `#[maos_attrs::i9_exempt(reason = …)]` and a matching entry in `docs/invariants/i9-exemptions.md`; the default ≥ 2 maintainer signature requirement is satisfied for this story by the explicit sole-founder exception D-17-7-FW-10, not by counting one person twice (`xtask/i9-denylist.toml:12-20`; `check_empty_kernel.rs:148-163,229-265`; precedent `T3Launch` at `i9-exemptions.md:538`). Never widen `i9-whitelist.toml`. Then, in the Grant-protocol order of `17-7-flag-winston-decision.md` (E16-A1, `kernel-core-baseline.toml:849-857`), update the **seven distinct re-pin files**:

1. `xtask/kernel-api-classes.toml` — classify every new public kernel seam first (the walker emits no impl methods, `:630-636`).
2. `docs/ci-baselines/kernel-surface-v0.1-beta.json` — regenerate; enumerate additions/removals (`SandboxedContainerChild` `:872`, `T3SpawnContext` `:897`, `build_runtime_argv` `:412`, `T3Launch` `:672`, `ExitCause` `:122`).
3. `xtask/kernel-core-baseline.toml` — `src_lines` stays on line 505 (today 25145), file digests and `set_hash` (`:729`), HISTORY.
4. `xtask/tests/kernel_pin_content_hash_16_0.rs` — the four `25145` literals (`:820,838,847,858`) and `PINNED_ENTRIES` (`:34`) if the file set changes.
5. `xtask/kloc.toml` — kernel `:262` (19492), domain `:414-415` (9349), bin `:501-503` (25396), audit `:404` (7341), egress `:651-656` (1769), eval `:660` (4831), xtask `:335-336` (44370), `_aggregate_hardfail` `:697-698` (173501, strict measured+1). Every one is an exact, zero-headroom pin.
6. `xtask/tests/recovery_lane_ceiling_rule.rs:268` — `RATIFIED_AT_EASING` (19_492).
7. `xtask/abi-ratifications.toml` — additive entries for the report wire change and the terminal-facts types (precedent `:128-158`).

Then run `check-kernel-baseline`, `check-service-boundary`, `check-empty-kernel` and `kloc-check --json` at HEAD. `check-unsafe` scans only its capability path (`xtask/src/main.rs:158-163`); a green run is not evidence for the sandbox-zone wrappers — the §A6 review is.

## Tasks / Subtasks

- [x] **T1 — Kernel: create/init/start with owned identity** (AC2, AC4, AC5)
  - [x] `t3/argv.rs`: `create` argv — drop `run`/`--rm`, add `--restart=no`, `--pidfile=<state_dir>/pid`, `--cidfile=<state_dir>/cid`, `--entrypoint=/maos/spirit` and the non-duplicating command tail (AC4.2); no `-i` or `--init`; preserve other isolation/resource flags and names-only `--env`. T3 keeps refusing stdin-keeping postures (R17-108); `start --attach` proxies signals by default, which is not a stop path. Migrate `t3_argv_custody_17_1.rs:47-71` and the direct FD control `worker_resource_limits_17_2.rs:589-608`.
  - [x] `t3/spawn.rs`: validate the state dir before runtime probes; replace `runtime_call_bounded` (`:67-92`) with one concurrent, size-capped, absolute-deadline helper with typed failed/timed-out/exited results; create carrying explicit env (`specgen.go` v4.9.3 `:442-478`); read/validate the private CID receipt on every outcome, enforce receipt/stdout agreement on success, arm CID guard immediately, report ownership-unknown without name cleanup (AC4.1); bounded `init`; held-inode pidfile read (AC2.1); supervised `start --attach <cid>` with no credential env. Delete `inspect_container_host_pid`. Custody (`sandbox_t3_spawn_linux.rs:72-102`) keeps argv names-only, verifies the long-lived `start` client's `/proc/<pid>/environ` has no bearer or ambient credential, and proves the payload receives the scoped value.
  - [x] `t3/child.rs`: guard owns CID, historical startup PID, state dir, client and shared terminal handle; shared cleanup-confirmation state and idempotent removal verdict; `Drop` joins existing teardown, or mechanically kills the client/CID, confirms termination, obtains the snapshot if absent and removes, all under the remaining deadline. Delete the state dir only after confirmed clean removal; preserve it for residue. Never create `PlannedStop` during fault cleanup. `inspect_report` emits CID and historical PID.
  - [x] `security/sandbox/linux.rs` gains SAFETY-commented pidfd wrappers (prototype: four `unsafe` sites; FW-04). Use locked libc's `pidfd_info`, `PIDFD_GET_INFO` and `PIDFD_INFO_EXIT`, not the prototype's hand-written struct/request; require the returned EXIT mask and a terminal status before decoding. Unsupported ioctl, missing bit, `EINTR` and deadline expiry never fabricate exit 0; retries use the remaining deadline. Open the pidfd just after `init` while gated; trust it only if `/proc/<pid>/cgroup` has an exact CID-bearing component (systemd `libpod-<cid>.scope`, cgroupfs `libpod-<cid>`, split `libpod-payload-<cid>`; [getOCICgroupPath v4.9.3](https://github.com/containers/podman/blob/v4.9.3/libpod/container_internal_linux.go#L383-L415)) and the pidfd remains live after that read. Unverifiable identity disables pidfd evidence and takes fallback, never refuses an otherwise accepted spawn.
  - [x] Every new kernel holder with `Arc`/`OnceLock`/`Mutex` gets its I9 exemption and register entry (Delivery gate).
- [x] **T2 — Kernel, domain, audit: snapshot before classification** (AC1, AC2, AC3, AC4)
  - [x] `runtime.rs`: `BridgeChild::kill` uses the bounded CID kill; `wait_and_finalize` takes the snapshot **before** `self.child = None` (`:948`), removes by CID, records the verdict, writes the exit row — the five-key T3 shape with the `container` object, the native four-key shape untouched; `BridgeExit` carries the facts; `SpawnedBridge::container_terminal()` hands the shared handle out; `T3Launch` gains the state dir. Re-export from `cli_wrapper/mod.rs:31-43`.
  - [x] `maos-domain`: `ContainerTerminalFacts` + payload-exit enum beside `SandboxInspectReport`; the observed-PID type; `OomKilled` gains an optional signal so `exit_signal()` (`supervision.rs:62-85`) reports it while `exit_code()` stays `None` and `as_str()` stays `fault.oom_killed` — `crash_detector.rs:151-169` then needs no edit. Public structs follow the `::new` constructor rule (`check_pub_field_constructors.rs:156-191`).
  - [x] `maos-audit`: keep exact recognition of the existing four-key `cli.subprocess.exit` shape (`fr4_classifier.rs:396-409`, matcher `:589-604`) for native and historical rows, and add a second exact five-key T3 alternative whose `container` must be an object of the declared schema. Null/scalar `container`, unknown nested keys or any other fifth key fail closed to `Call`. Keep the native fixture at `fr4_classifier_16_2.rs:271-277`, add a T3 fixture beside it, keep the census `:330-355` one-to-one with recognized shapes, and add native/T3/malformed tests in `fr4_worker_rows_16_3.rs` (its orphan tests `:21-25,76-96` stay as they are). The `task.orphaned` shapes (`:274-315`) are unchanged.
- [x] **T3 — maos-bin: one snapshot to both feeds, bounded stops, lease, unload, journal** (AC2, AC3, AC4)
  - [x] Replace `ContainerStopIdentity`/`attach_container_stop` (`supervision.rs:604-607,666-685`) with the shared terminal handle held for the binding's life (AC3.3). Preserve late attachment: stop requested before attachment is executed by owned CID on attach; attachment cannot lose the latch, snapshot custody or pending cleanup. A test gates attachment behind an already-issued unload/root stop and confirms exactly one CID teardown.
  - [x] The observer builds T3 `ExitFacts` from the shared snapshot after `waitid` returns (`:2610-2616`); `crash_cause_from_exit` uses `BridgeExit`'s facts; `classify_exit` implements §Classification, including unavailable → `Truncated`. ECHILD/intentional PlannedStop/typed-refusal unchanged.
  - [x] Completion: T3 `WorkerExit` from the verified payload exit, including explicit exit 0 for a voluntary result (AC3.6).
  - [x] `signal()`/`stop_attached_container`/`on_unload`: intentional-stop versus fault-cleanup semantics (AC4.3), bounded CID sequence, failure reporting, no false `signals_sent`; fix the stale comment `:816-821`.
  - [x] Post-unload registry stop on every Worker unload path (AC4.6); signal listener / forced-exit integration with the root teardown coordinator (AC4.4).
  - [x] Lease ordering and separate termination-confirmation state on finish, abandon (`Drop` `:1133-1149`), unload and root stop (AC4.5); concurrent N-worker coordinator with one absolute deadline (AC4.4). If revocation-only needs `maos-egress` (`proxy.rs:212-247` couples revoke, join and socket removal), return a measured FW-03 request.
  - [x] `commit_container_spawn` through the shared lifecycle `JournalAdapter` (AC2.4); thread it into `WorkerSupervisor` and every constructor/test constructor.
  - [x] `worker_spawn.rs`: per-spawn state dir beneath a per-root `0700` directory, reusing `egress.rs:192-220`; if adding a durable root beneath `MAOS_HOME`, extend ADR-067's typed enumeration and root test (`:78-81`). Publish CID, pass state dir in `T3Launch`, remove it only after clean removal; preserve and report residue. `main.rs:7084-7108`: same ownership on every return path.
- [x] **T4 — Fixtures and the terminal suite** (AC1, AC2, AC4, AC5)
  - [x] `worker-cli-fixture`: modes `exit-137` (flush, `exit(137)`) and `oom` (AC1 S4; retained, written pages that cannot be optimised away; 1 GiB cap; no sleep); delete the hold helper (`:44,92,121,130-134`) and fix the `:24-26` comment. `fixture_modes_16_3.rs:74-132` stays green.
  - [x] New `crates/maos-bin/tests/worker_terminal_facts_17_7.rs` (`#![cfg(all(feature = "network", target_os = "linux"))]`): S1–S5; the variant-A fallback scene (seam-forced pidfd unavailability); the externally-killed-client scene (client dies, payload lives → owned-CID cleanup, `Truncated`, no completion); the client-first stop scene (`PlannedStop`, no running-state snapshot cached); the name-collision and identity-rebinding scenes (AC4.1); the real launch refusals before `create` and after `init` (AC2.1); the root-crash no-restart scene (AC4.7); the bearer-echo stub (AC4.8); the hung-kill/hung-rm/hung-exists stubs asserting the AC4.4 deadlines; the lease scenes (termination before `finish`; unconfirmed termination → bearer revoked, lease retained and reported, including root cancellation). Use `tests/harness/t3_substrate.rs` (CI never skips); bound every runtime call (`timeout 8s` pattern, `worker_resource_limits_17_2.rs:101-116`).
  - [x] Permanent kernel unit/helper tests for the full AC2.1 matrix (wrong owner, wrong mode, non-empty dir, pre-existing pidfile; missing, dangling symlink, FIFO with no writer, directory, oversized, trailing data, non-numeric, zero, replaced path), the decoding rule, §Classification, the terminality gate and the bounded-invocation helper (both pipes filled; a descendant holding a pipe after the client exits).
  - [x] Preflight oracles (§Preflight): partial-create receipt/unknown/disagreement; forced-ECHILD and observer-first unexpected-client failure; immutable-unavailable facts followed by successful cleanup; concurrent snapshot/stop/finalize with one deadline/result/verdict; N≥3 delayed runtime stubs with root forced-exit integration; late stop attachment; decoding boundary and missing EXIT-mask cases; exact exit-row schema with native shapes unchanged.
  - [x] S5 wiring: in-process, real `run_cli_wrapper_manifest` with a seam-configured supervisor and the real T3 bridge. Extend `SupervisorSeams` (`supervision.rs:582-600`) with an `on_unload` fault injected **before** the hook's own stop (panic | finite sleep) and a hook-cap override routed through the Worker bundle or the dispatcher's `time_cap_seconds` (`hook_dispatch.rs:64-65`); the over-budget hook's blocking task is allowed to finish after the timeout so runtime shutdown cannot hang. No production env-var switch; no `hook_dispatch.rs` edit.
- [x] **T5 — Migrate pinned tests, constructors and holds** (AC5)
  - [x] `worker_supervision_16_3.rs`: both real-T3 scenes read the CID from `cli_wrapper_loaded` and resolve the live payload PID by CID + `/proc` cgroup check; the crash scene asserts the S3 rows instead of `exited(code=137)`/`fault.non-zero-exit` (`:1751-1769`); the FR4-feed scene (`:1869-1963`) keeps its omission assertions.
  - [x] Six `ControlledRuntime` tests → CID; `root_shutdown_unload_16_3.rs` helpers → CID, stop oracle requires removal.
  - [x] Constructors: `sandbox_t3_spawn_linux.rs:63` and `t3_network_escape_executor.rs:93-98` own a fresh state dir and supply the new fields.
  - [x] Delete the holds in `worker-probe-fixture.rs:81-88`, `worker_completion_2a.rs:837`, `sandbox_t3_spawn_linux.rs:47`, `t3-network-escape-probe/src/main.rs:27-28,43`; those suites must stay green without them (the regression for `:1120`).
  - [x] Keep `worker_resource_limits_17_2.rs:536-543` (production removal before teardown) green under explicit removal.
- [x] **T6 — CI** (AC1; post-review dual-image runtime acceptance and eight assertion-level red controls verified)
  - [x] `epic-17-exit`: exit line 4 on both x64 images using the delegated runner's `run` action (AC1), preserving its `--rootless-canary` prerequisite. Tee **only the terminal suite's output inside the unit** (the canary emits a separate one-test summary); propagate `PIPESTATUS` and call the existing `check-log` action on that suite log with its independently pinned expected libtest count. Pin/verify the suite's declared scene inventory too, so a passing subset cannot substitute for AC1's scenes. Reject mid-line SKIP and ignored/zero tests; upload full logs; add the aggregate predicate/workflow test per AC1.
  - [x] Log the runner's `podman version`, `conmon --version`, OCI runtime name/version, cgroup manager and `uname -r` (nothing pins or prints them today: `.github/actions/provision-t3-test-substrate/action.yml:17-22`).
  - [x] Final run 38708067558 is green at `f3fadea9`: both x64 images passed 31/31 terminal tests, all migrated suites and 2/2 resource tests; all eight independent mutants failed their named assertions after successful builds. SKIP=0, ignored=0. Historical 26-scene runs remain pre-review evidence only.
- [x] **T7 — Docs, records, measurement, review** (all ACs, delivery gate)
  - [x] EN/KO cookbook, architecture/ABI sentences, `BREAKING.md` entry for the sandbox-report wire change (`payload_pid`/`container_id`) and the T3 `cli.subprocess.exit` `container` object, with `**Migration:**`.
  - [x] Exemptions/register and all seven measured re-pin files are updated; exact review raises explicitly approved by Lunarpulse on 2026-10-15; baseline/governance gates pass. Lunarpulse, the sole maintainer/founder, explicitly approved the reviewed implementation and waived this story's second-signature requirement under D-17-7-FW-10.
  - [x] At dev start, record the implementation model as the first model token in `### Agent Model Used` (and `dev_model_used:` if used); creation attribution stays in §Story context creation.
  - [x] Non-author §A6 review, local verification and post-review provisioned runtime/proven-red evidence are recorded below. The founder exception satisfies the approval gate; the three owned deferred-work rows are closed in place with evidence and their original owner cues preserved.

### Review Findings

Full review 2026-10-15: Blind Hunter, Edge Case Hunter, Acceptance Auditor and Test Infrastructure Auditor completed. Triage: **17 patches (4 high, 13 medium), 0 defer, 1 dismissed**. Lunarpulse selected “Apply every patch.” All 17 patches below are applied. The duplicate I9-approval observation remained a landing prerequisite at triage; the subsequent explicit founder decision D-17-7-FW-10 satisfies it through a scoped second-signature waiver. The previously authorized runtime-referenced OOM oracle, 10 s snapshot / 25 s teardown budgets and S3 bound were preserved.

- [x] [Review][Patch] **R1 — high: Publish CID custody before blocking initialization** — `ContainerLaunchCustody` is registered before creation and publishes receipt ownership before `init`; root cancellation joins launch permits before cleanup. Declaration order releases the launch permit before guard Drop, avoiding a self-join. The real `RootSupervision` signal-listener subprocess scene proves SIGTERM during delayed init removes the receipt CID once and never starts it.
- [x] [Review][Patch] **R2 — high: Reject incomplete removal diagnostics** — `Invocation::Exited` records stream completeness; overflow and descendant-held stderr cannot certify volume cleanup. Standalone probes that formerly returned clean removal now return `VolumeCleanupUnconfirmed` and retain state.
- [x] [Review][Patch] **R3 — high: Cap snapshot followers by teardown remainder** — Cap-limited followers return without rewriting shared evidence; removal is refused while snapshot initialization remains active. A standalone probe returned at its 200 ms cap while the original 1.8 s observer completed normally, with no removal.
- [x] [Review][Patch] **R4 — high: Restore non-Unix smoke guards** — Unix-only state allocation is compile-time guarded in `main.rs`; other platforms retain a typed unavailable response. Linux smoke entrypoint exercised. Windows compiler verification remains blocked by the missing MinGW C compiler.
- [x] [Review][Patch] **R5 — medium: Share bridge stop/Drop teardown** — Container bridge kill, unload, finalization and Drop join the same coordinator/deadline instead of issuing a separate 10 s kill followed by a fresh teardown. Native handling is unchanged.
- [x] [Review][Patch] **R6 — medium: Namespace temporary state by effective UID** — Fallback parents use `maos-worker-state-<euid>`. An isolated subprocess proves an obstruction at the old shared parent cannot prevent private state allocation.
- [x] [Review][Patch] **R7 — medium: Retain failed-kill diagnostics and truthful signal counts** — Failed CID kill is latched/reported independently of eventual removal; container signal accounting requires actual CID-kill success.
- [x] [Review][Patch] **R8 — medium: Exercise production observer/finalizer/fallback feeds** — Real bridge and binding scenes cover observer-first and forced-ECHILD finalizer-first failures, one persisted truncated fault, immutable evidence, CID cleanup and rejection of a persisted completion marker. A clean marker control and binding-driven 137 fallback pass. The real OOM fallback scene now uses the production binding/writer path.
- [x] [Review][Patch] **R9 — medium: Prove lease decisions through binding teardown** — Both feed orders attach real leases and prove unavailable facts remain truncated after confirmed cleanup releases the lease. The never-confirmed counterpart proves a request succeeds before stop, the revoked bearer receives HTTP 403, the owner retains its lease, and binding Drop/root cancellation join the proxy and report residue.
- [x] [Review][Patch] **R10 — medium: Require explicit S5 absence** — Only runtime exit code 1 proves absence; 0 means present and every other status fails the scene. Real-runtime S5 remains capability-gated locally.
- [x] [Review][Patch] **R11 — medium: Time the unload itself** — S5 starts its budget clock before awaiting unload and preserves a separate return timestamp for post-return liveness.
- [x] [Review][Patch] **R12 — medium: Join final stdout capture** — Capture retains its reader JoinHandle; both final-output consumers join before verdict assertions.
- [x] [Review][Patch] **R13 — medium: Cover every bearer sink and refusal path** — Failed-create stdout/stderr diagnostics and successful bridge finalization are exercised in subprocesses; captured streams and all reopened journal rows are checked, including refusal formatting and typed cleanup reports.
- [x] [Review][Patch] **R14 — medium: Replace a receipt path after inode hold** — A deterministic callback replaces the pathname after opening the original inode; the reader must return the held inode's bytes, never the replacement.
- [x] [Review][Patch] **R15 — medium: Evaluate aggregate workflow behavior** — The regression executes the aggregate shell/status behavior for success, skipped, failure and cancelled dependencies, with predicate-removal negative control. The fake scene echo/source-count test is removed. CI instead pins all 31 actual libtest names, and `check-log --required-test` rejects same-count substitution.
- [x] [Review][Patch] **R16 — medium: Bound independent OOM reference commands** — The plain runtime reference retains independent marker authority, uses bounded run/inspect calls and owns a private CID receipt with bounded cleanup on every exit path.
- [x] [Review][Patch] **R17 — medium: Round-trip finalized rows through FR4** — Production bridge-written success, runtime-fallback, unavailable and residue rows are reopened through public audit query, passed through both renderers, and checked for exact NDJSON omission accounting on stderr. The duplicate fixture-only renderer test is removed.

**Local repair verification:** Cargo used `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/mnt/build/cargo-17-7-review-az5lixbr`; real-runtime-gated commands additionally used `env -u CI`.

- `cargo test --locked -p maos-bin --features network --test worker_terminal_facts_17_7 -- --nocapture --test-threads=1`: 31 reported passed in 68.63 s; **21 scenes exercised, 10 real-runtime scenes returned after explicit SKIP**. This is not a local real-container pass. The startup scene runs the actual root signal listener in a bounded subprocess.
- `maos-bin --test worker_supervision_16_3`: 23 reported passed, subject to its runtime capability gates; `--test egress_daemon_17_1`: 4 passed.
- Kernel `t3_terminal_helpers_17_7`: 7 passed; audit `fr4_worker_rows_16_3`: 11 passed; xtask `epic17_exit_workflow_17_7`: 1 passed; `kernel_pin_content_hash_16_0`: 24 passed; `recovery_lane_ceiling_rule`: 10 passed.
- `python3 -m unittest scripts/tests/test_cgroup_ceiling_17_2.py`: 8 passed. Actual `check-log` CLI smoke accepted a complete log and rejected a same-count substituted test. All 31 pinned CI names equal the compiled binary's `--list` inventory.
- Standalone Rust probes linked the repaired kernel/domain libraries: overflowed and descendant-held removal stderr both yield `VolumeCleanupUnconfirmed` with retained state; joining an in-flight 1.8 s snapshot returns at 200 ms without removal or evidence rewriting. Probe sources/binaries and temporary logs were removed after success.
- Actual `MAOS_ONE_SHOT=smoke-t3-sandbox-5 maos` reports `runtime_detect` unavailable: neither Podman nor Docker is installed.
- `cargo check --locked --workspace --all-targets`, `cargo check --locked -p maos-bin --all-targets --all-features`, and `cargo check --locked -p maos-bin --no-default-features` (library/binary): pass with existing warnings. The broader `--no-default-features --all-targets` attempt fails in network-dependent integration tests (`maos_a2a_tcp`/`rcgen` resolution and feature-specific trait implementations); those tests are not repaired by this review.
- PASS: `check-kernel-baseline`, `kloc-check`, `check-service-boundary`, `check-empty-kernel`, `check-abi-ratification` (no Spirit ABI changes), `check-unsafe`, `check-pub-field-constructors`. KO glossary lock passes; canonical KO coverage is 41/41, with the overall 45/51 report-only result retaining intentional supporting fallbacks.
- PASS after record synchronization: `check-review-findings-resolved` (169 stories), `check-bare-review-findings` (zero placeholders), `check-dev-record-completeness` (168 done-status stories; dispositioned ownerless rows remain reported), and `check-epic-close-coherence` (21 epics against pin 26311). The resolved code-review completion hook is empty.

**Measured authorization:** After formatted measurement, Lunarpulse explicitly selected “Approve exact re-pins” on 2026-10-15: kernel physical 25775 → 26311, kernel tokei 20050 → 20545, bin 25847 → 25939, aggregate measured 175372 with strict hard-fail pin 175373 (previous 174795). No spare headroom. `maos-attrs` shrank 20 → 11 lines while fixing `i9_exempt` to preserve generics/where clauses. Kernel changes stay in four authorized files; all 95 source files remain; `src_lines` stays on line 505.

**Surface delta:** Five added identities: `t3::child::ContainerLaunchCustody` and its `cli_wrapper` re-export, `cli_wrapper::runtime::bridge_container` and its re-export, and `t3::spawn::read_receipt_after_hold`. No symbol path removed. Twenty old signature identities are replaced: `Invocation`, `T3SpawnContext`, `T3Launch`, `ContainerTerminal`, `SandboxedContainerChild`; re-export hashes for `ContainerTerminal`, `TEARDOWN_BUDGET`, and runtime `{Backpressure, BridgeError, BridgeExit, BridgeSpawnSpec, ExitCause, PumpOutcome, RecoveryOutcome, SpawnedBridge, SubStream, T3Launch, argv_prefix_hash, execute_recovery, spawn_and_bridge}`. Grouped-use hashing accounts for the unchanged members' re-hashes. Classes precede the surface refresh; the physical/digest pin, four reader literals, tokei pins, recovery ceiling and additive ABI ledger follow in the ruled order.

**Completion state:** `done`, synchronized with sprint status. Post-review Linux runtime acceptance, all eight proven-red controls, exact measured authorization and the full non-author review are verified. Lunarpulse's genuine approval plus the explicit sole-founder exception D-17-7-FW-10 satisfies this story's approval gate. The three owned deferred-work rows are closed with evidence. No second signature, merge or release is fabricated.

**Operator execution directive (2026-10-15):** Execute the remaining acceptance/approval plan. Windows is not currently supported for release; defer Windows release until an explicit subsequent operator command. Windows compilation/packaging is not a completion gate for this Linux T3 story. `docs/release/windows-deferral.md` corrects the previous support claim, and the release workflow retains its existing three non-Windows targets. Linux runtime evidence and substantive I9 requirements remain binding; the later D-17-7-FW-10 decision waives only this story's second-maintainer signature.

**Execution candidate:** commit `f3fadea9c1ef0c283c836c7a988c5ff7b2a6dba5`, tree `e11d2ecf18856d5060be38ef137e7d05a7657bfa`; [run 38708067558](https://github.com/lunarpulse/maos/actions/runs/38708067558). This authorized CI-only snapshot is not a merge or landing commit. All 50 changed/new Rust source files were byte-compared with the original checkout before freezing it. The original main working tree/index were not committed; unrelated party-memory changes were excluded. Exact commands, seven revision diffs, run IDs, fault patches and assertion notices are archived in `17-7-evidence/review-ci.json`.

- **Final run 38708067558 completed successfully: all 10 jobs green at `f3fadea9`** — 31/31 terminal tests, all migrated suites and 2/2 resource tests on both Ubuntu 24.04 and 26.04; SKIP=0, ignored=0; all eight assertion-level red controls verified. Earlier run 38704276898 also passed both normal acceptance jobs; its unreliable C5 end-to-end oracle was superseded, not reported as a successful overall run.
- Local strengthening check: the binding/lease scene passed in the 9/10 initial feed run; the sole failure overconstrained the unavailable error subtype at a deadline boundary. That incidental subtype assertion was removed, retaining unavailable/no-completion/one-fault invariants. The strengthened observer-first test then passed as an exact single-test run. Both test source files are identical between the original tree and candidate.
- All eight independent controls are assertion-level red in run 38708067558, after successful builds. C1/C2/C3/C4/C6 retain S1/S3/domain OOM/CID ownership/bounded kill; C7 proves real lease retention after binding Drop; C8 uses the non-terminal-state production observer feed. C5 now isolates that observer feed: ordinary S3 can be rescued by bridge finalization and demonstrably exited 0 under the older observer-only mutant. The revised fault omits snapshot acquisition/classification only in the observer; the observer-first oracle withholds bridge finalization. Local unmutated code passed and the mutant failed the exact observer deadline assertion; the patch was then reversed.
- Actual CI failures were repaired at origin, without larger deadlines or weaker assertions: invoke the release crash corpus with `--ignored`; require real tests per migrated command while permitting legitimate empty sub-suites/doctests; wait for owned `State.Running` before resource PID inspection. The full audit's 241 passing tests plus empty doctest passed the revised guard; zero-only, failed, SKIP and nonzero-command inputs were rejected. YAML, extracted Bash and embedded Python parse checks passed.
- **I9 approval satisfied:** Lunarpulse approved reviewed revision `f3fadea9c1ef0c283c836c7a988c5ff7b2a6dba5` and explicitly instructed: “override that two signature rule for now. only one maintainer here. me founder. I approve both slots. lets move on.” D-17-7-FW-10 records one genuine founder approval plus a scoped waiver of the second signature for the eight listed exemption rationales. This covers bounded single-invocation state, permit-before-guard destruction, immutable evidence/deadlines, root-cancellation custody and fail-closed residue. The register and evidence JSON retain the actual signer count of one; other stories' default rule and Windows release hold are unchanged.


## Dev Notes

### Lifecycle after this story

```mermaid
sequenceDiagram
    participant W as worker_spawn (maos-bin)
    participant K as spawn_t3 (kernel)
    participant P as podman / conmon / OCI
    participant O as pidfd observer (maos-bin)
    W->>W: private 0700 state dir per spawn
    W->>K: spawn_and_bridge(T3Launch{state_dir,..})
    K->>P: create --restart=no --pidfile=<dir>/pid --cidfile=<dir>/cid … (env on this client)
    P-->>K: private CID receipt + stdout agree → CID guard armed
    K->>P: init <CID> (runtime writes pidfile; payload gated)
    K->>K: held-inode pidfile read → historical PID; [A] pidfd_open + cgroup check
    K->>P: start --attach <CID> (supervised client, no env)
    W->>O: watch(client pid)
    P-->>O: client exits (normally after the DB exit is recorded)
    O->>K: terminal.facts() (once, ≤2 s: wait terminal → inspect State → [A] PIDFD_INFO_EXIT)
    O->>W: on_child_exit(facts) → classify → crash handler
    W->>K: wait_and_finalize → facts() (cached) → rm --volumes by CID → exit row
    W->>W: binding.finish → lease ends after confirmed termination
```

### Classification (T3)

| Facts after snapshot | `CrashCause` | `task.orphaned` cause / signal / code |
|---|---|---|
| `PlannedStop` latched | none (planned) | — |
| snapshot unavailable or non-terminal (any client status, including 0) | `Truncated{reason}` | `fault.truncated` / null / null |
| payload `exited(0)` (any OOM marker) | Voluntary | — (OOM marker recorded in the exit row only) |
| payload `exited(N≠0)` | `NonZeroExit{N}` | `fault.non-zero-exit` / null / N |
| payload `signaled(9)` + OOM marker | `OomKilled{signal:Some(9)}` | `fault.oom_killed` / 9 / null |
| payload `signaled(S)` otherwise | `SignaledByKernel{S}` | `fault.signaled` / S / null |
| payload `unknown`, runtime 137 + OOM marker | `OomKilled{signal:None}` | `fault.oom_killed` / null / null |
| payload `unknown`, runtime C in `129…=192`, except the preceding OOM case | `NonZeroExit{C}` | `fault.non-zero-exit` / null / C |
| no valid pidfd status and runtime exit evidence missing, malformed, negative or >255 | `Truncated{reason}` | `fault.truncated` / null / null |

T0/T2 keep today's client mapping unchanged.

### OOM authority contract

`oom_marker` is conmon's observation that `memory.events` `oom` or `oom_kill` moved in the container's cgroup. It names no victim and no limit. MAOS labels a death `fault.oom_killed` only when the marker is set **and** the payload died by SIGKILL (variant A) or with runtime code 137 and unknown payload status. A marker with a clean or other exit is recorded but changes nothing. `--memory` without `--memory-swap` lets podman allow swap up to twice memory ([LimitToSwap v4.9.3](https://github.com/containers/podman/blob/v4.9.3/pkg/specgenutil/specgen.go#L139-L145)), so S4 asserts `memory.max` and the kill evidence, not an allocation total; it records `memory.swap.max` and caps its own allocation. Tests: S4 (positive), S1 with the marker absent, and the unit table above. The cookbook states this bound.

### Exit-row schema

Native and historical rows keep exactly `event`, `cli_child_pid`, `exit_cause`, `is_crash`. T3 adds exactly one fifth key, `container`, an object with **all six keys below always present**, no unknown keys and no raw diagnostics. These are the writer and FR4 consumer contract, not the sizing prototype's wire shape.

| Nested key | Type / values |
|---|---|
| `runtime_exit_code` | Signed i32 integer or null. Preserve a representable raw runtime value even if invalid as payload evidence; missing/malformed/out-of-i32 values are null. Never use an invalid value as an orphan exit code. |
| `oom_marker` | Boolean or null when unavailable; never default missing evidence to false. |
| `payload_exit` | Exactly `unknown`, `exited(code=N)` with N in `0…=255`, or `signaled(sig=S)` with S in `1…=64`. |
| `payload_exit_source` | Exactly `pidfd`, `runtime`, or `none`. `unknown` requires `none`, and `none` requires `unknown`; `pidfd` requires a validated `exited(code=N)` or `signaled(sig=S)`; `runtime` requires an unambiguous decoded exit and matching runtime code; `signaled` requires `pidfd`. |
| `removed` | Boolean: true only if removal and volume cleanup succeed and the existence check confirms absence. |
| `residue` | Null when `removed:true`; when false, exactly one typed category: `kill_failed`, `termination_unconfirmed`, `remove_failed`, `remove_timeout`, `exists_failed`, `still_exists`, `volume_cleanup_unconfirmed`, or `runtime_client_unconfirmed`. Pick the first unresolved teardown failure in stage order; keep additional typed diagnostics on root stderr, never raw runtime text. |

An unavailable snapshot without validated payload evidence uses `payload_exit:"unknown"` / source `none`; missing runtime code and marker are null. Successfully cleaning up such a container may set `removed:true` / `residue:null` but never changes its classification or completion. Creation-ownership-unknown has no trusted CID and may have no exit row; report it through the typed launch refusal and root stderr, with its retained per-spawn directory.

Both FR4 renderers must consume real finalized success, fallback, unavailable and residue rows without `Fr4SchemaViolation`; plain rendering includes them and NDJSON omission counts them. Null/scalar container, unknown/missing nested keys, incorrect types, inconsistent source/payload or removed/residue pairs and unapproved fifth outer keys remain `Call`. No native row gets `container:null`.

### File implementation map

| File | Current | Change / preserve |
|---|---|---|
| kernel `security/sandbox/t3/argv.rs` | `run --rm`, name-based (`:71-104`) | `create`, `--restart=no`, `--pidfile` + `--cidfile`, pinned entrypoint and non-duplicated command tail; preserve isolation flags, digest pin, names-only env, 17-2 resource flags. |
| `…/t3/spawn.rs` | guard before PID poll by name (`:184-205,229-258`); drain-after-exit helper (`:67-92`) | create→CID guard→init→pidfile→start; one truly bounded invocation helper; refusals remove by CID. Preserve rootless gate (`:97-109`), digest check (`:155-168`), scrubbed client env (`:47-61`). |
| `…/t3/child.rs` | name-based `Drop`, results ignored (`:76-98`) | CID ownership, terminal handle, terminality gate, idempotent removal verdict, typed residue; I9 exemptions. |
| `…/t3/mod.rs` | DR-5.5a-7 "PID identity = host PID" (`:42`) | CID identity and the historical PID. |
| `…/lifecycle/cli_wrapper/runtime.rs` | client-only exit, guard drop before row (`:922-996`), unbounded kill (`:481-490`) | snapshot→remove→row; dual exit shapes; bounded kill; handle accessor. `#![forbid(unsafe_code)]` stays. |
| `…/lifecycle/cli_wrapper/mod.rs` | re-exports `:31-43` | new public types. |
| `…/security/sandbox/linux.rs` | unsafe zone | pidfd wrappers with `// SAFETY:` (FW-04). |
| `crates/maos-domain/src/{sandbox.rs,supervision.rs}` | `pid: u32`; `OomKilled{stderr_tail}` | facts types, observed PID, `OomKilled` signal. |
| `crates/maos-bin/src/supervision.rs` | name stop identity, unbounded kill, client facts, scheduler-adapter journal | shared handle, snapshot facts, bounded CID stop sequence, post-unload stop, lease ordering, seams, lifecycle journal. |
| `crates/maos-bin/src/worker_spawn.rs` | name `maos-<id>-<ns>` (`:905-906`), attach name (`:952-954`), completion from client (`:1044-1047`) | state dir, `container_id`, authoritative completion. |
| `crates/maos-bin/src/{operator_door.rs,main.rs}` | door unload `:524,991`; smoke `:7084-7108`; unload `:6478` | post-unload registry stop; smoke state dir and historical PID. |
| `crates/maos-bin/src/egress.rs` (+ `maos-egress/src/proxy.rs` if needed) | lease finish/drop (`:160-187`), root drain (`:35-53`) | *termination unconfirmed* state: revoke now, retain and report. |
| `crates/maos-audit/src/fr4_classifier.rs` + tests | exact four-key exit shape (`:396-409`) | add the exact five-key T3 alternative. |
| `docs/invariants/i9-exemptions.md` | register | entries for the new kernel holders. |
| `crates/maos-control` tests | report fixture `pid:42` | migrate. |
| `spirits/worker/src/bin/*.rs`, eval probe, kernel/eval tests | 3 s holds, old constructors | delete holds; new modes; new fields. |
| `.github/workflows/discipline.yml`, `xtask/tests/` | epic-17-exit lines 1–3 | line 4 lane, version log, aggregate predicate + test. |
| docs (EN/KO), architecture, `BREAKING.md` | limitation text | shipped behavior. |

Kernel files outside this list are a charter request (FW-01). Not touched: `supervision/crash_detector.rs`, `scheduler/*`, `hook_dispatch.rs`, `t3/image_lock.rs` (the prototype touched it only for probe scaffolding).

### Guardrails

- **Three identities stay separate:** scheduler `spirit_pid`, runtime-client OS PID (`child_pid`, pidfd target), payload host PID (historical). Never kill a raw historical PID (`supervision.rs:2580-2586` explains why).
- **No startup polling, no sleeps:** startup identity comes from `init` + pidfile; a short task needs no hold. The bounded wait for a **terminal** state inside the snapshot deadline is not startup polling and is allowed.
- **Never block under a lock:** snapshot, kill and rm run after lock → decide → release (`supervision.rs:728-733,816-821`).
- **RAII order:** `LiveWorker` keeps binding before bridge (`:1151-1162`); confirmation happens in the binding, never by reordering fields. The guard `Drop` is the last resort, not the reporting path.
- **Secrets:** the bearer reaches the container only through `create`'s environment; argv stays names-only; diagnostics are typed categories (AC4.8).
- **Platform gates:** pidfd/pidfile code is `#[cfg(target_os = "linux")]`, never `unix` (macOS is a release target; `macos_compile_guard_20_6`). Non-Linux T3 still returns `SandboxUnavailable` (`t3/spawn.rs:129-138`).
- **Purge stays report-only** (ADR-067 `:87-90`); the live owner removes its own CID.
- **Upstream advisories:** CVE-2026-57231 (image `Env` key imports client env; fixed 5.8.4/6.0.0) and CVE-2026-55686 (WORKDIR traversal; fixed 5.7.1) fall in both runner baselines. Only operator-signed images run (`image_lock.rs` trust anchor), and the scrubbed `create` env limits what the first could import; neither is fixed by this story. Disposition: D-17-7-FW-09.

### Proven-red controls (rule 11(c))

Each runs red once in CI and green before and after (same source, sole patch archived):

| # | Planted fault | Oracle that turns red |
|---|---|---|
| 1 | re-add `--rm` | S1/S2 snapshot unavailable → `Truncated`; S1 not completed |
| 2 | classify T3 from client status only | S3 shows `fault.non-zero-exit`; S4 loses `fault.oom_killed` |
| 3 | treat `oom_marker` alone as sufficient | marker + clean-exit unit row fails |
| 4 | remove by requested name | identity-rebinding scene: B removed, A survives |
| 5 | observer skips the snapshot | S3 observer-first row is `fault.non-zero-exit` |
| 6 | unbounded kill in the stop sequence | hung-kill stub exceeds the 5 s confirmation deadline |
| 7 | lease `finish` before kill confirmation | lease-ordering scene fails |
| 8 | accept a non-terminal snapshot | client-first stop caches `running` → assertion fails |

### Tests and commands

Prerequisites (x64), as `epic-17-exit` builds them (`discipline.yml:1828-1832`): `cargo build --locked -p maos-bin --bin maos`; `cargo build --locked -p maos-cli --bin maosctl` (`root_shutdown_unload_16_3.rs:91-101` resolves it beside `maos`); `cargo build --locked -p worker --bin worker-cli-fixture`; the static probe `RUSTFLAGS='-C target-feature=+crt-static' cargo build --locked -p worker --bin worker-probe-fixture --target x86_64-unknown-linux-gnu`; the static escape probe `RUSTFLAGS='-C target-feature=+crt-static' cargo build --locked --release --target x86_64-unknown-linux-gnu --manifest-path crates/maos-eval/test-fixtures/t3-network-escape-probe/Cargo.toml` (`t3_network_escape_executor.rs:49-74`; reflect any `CARGO_TARGET_DIR` in `MAOS_T3_NETWORK_ESCAPE_PROBE`); the provisioned T3 substrate; the delegated rootless envelope for the resource/OOM suites.

```sh
cargo test --locked -p maos-kernel-core --test t3_argv_custody_17_1 -- --nocapture
cargo test --locked -p maos-kernel-core --test sandbox_t3_spawn_linux -- --nocapture
cargo test --locked -p maos-bin --features network --test worker_terminal_facts_17_7 -- --nocapture --test-threads=1
cargo test --locked -p maos-bin --features network --test worker_supervision_16_3 -- --nocapture
cargo test --locked -p maos-bin --features network --test root_shutdown_unload_16_3 -- --nocapture
cargo test --locked -p maos-bin --features network,wasm-host --test worker_resource_limits_17_2 -- --nocapture --test-threads=1
cargo test --locked -p maos-bin --features network --test worker_completion_2a
cargo test --locked -p maos-eval --test t3_network_escape_executor -- --nocapture
cargo test --locked -p maos-audit
cargo test --locked -p maos-control --test post_surface_16_1
cargo run --locked -p xtask -- check-kernel-baseline
cargo run --locked -p xtask -- check-service-boundary
cargo run --locked -p xtask -- check-empty-kernel
cargo run --locked -p xtask -- kloc-check --json
```

These are plans, not results. This host has no container runtime, so T3 evidence is CI-only. T0/T2 suites (`worker_supervision_16_3` native scenes, ECHILD gate `:1298-1370`, crash/hang corpora) must stay green unchanged.

### Previous-story intelligence (17-2, done 2026-10-12)

- 17-2 kept the T3 lifecycle untouched and named these handoffs; its T3 resource suite inspects the **running** container and asserts production removal (`worker_resource_limits_17_2.rs:428-543`). Keep both under explicit removal.
- Its delegated rootless envelope (`scripts/run_cgroup_ceiling_17_2.py`) is where rootless `memory.max` was proven (`67108864`); S4 needs it. Transport `PATH`, `HOME`, Cargo settings, the three `MAOS_T3_*` exports, `XDG_RUNTIME_DIR`, `DBUS_SESSION_BUS_ADDRESS` (17-2 AC3.3).
- Lessons that cost 17-2 a HALT: libtest prints `test … ... ` before a test body, so a `SKIP` guard must match mid-line; runtime calls in test teardown need their own `timeout`; a fixture path in a `static` never drops; the public annotation limit truncates causes — archive full logs.
- Ceilings: domain 9349, bin 25396, audit 7341, egress 1769 sit exactly at their ceilings; aggregate 173500 vs strict 173501. Any growth is a measured request before landing (FW-03).
- Git: `37c34be4` (17-2), `b19556ab` (17-1), `8f6869b8` (17-1 re-verification), `f88a1968`/`d371b5a4` (20-6). 17-1 built the T3 route, egress lease and bounded helper; 17-2 added resource flags and the delegated CI substrate.

### Measured prototype — E16-A7 creation precondition

Creation used an isolated copy (`git archive 37c34be4` into a throwaway repo, its own `CARGO_TARGET_DIR`), never the production tree. Evidence: `17-7-evidence/measurements.json` (with base/B commit ids and diff hashes), `prototype-variant-b.diff`, `prototype-variant-a-increment.diff`, `prototype-variant-a.diff`, `smoke.log`. Both variants were `cargo fmt`ed and built (`cargo build --offline --locked -p maos-kernel-core --lib`, 42 pre-existing warnings, 0 new). Instruments: physical = `lines().count()` over kernel `src/**/*.rs` (`check_kernel_baseline.rs:472-484`); tokei 14.0.0 with the exact `kloc_check.rs:302-325` exclusions. The base reproduced the pins exactly. The first archived B diff had been generated from the A tree; validation caught it, the diffs were regenerated from the recorded commits, and all three revisions were re-measured from separate worktrees with identical results.

| Instrument | Base (`37c34be4`) | Variant B | Variant A | Δ B | Δ A |
|---|---:|---:|---:|---:|---:|
| Kernel physical `.rs` lines | 25145 | 25436 | 25574 | +291 | +429 |
| Kernel files | 95 | 95 | 95 | 0 | 0 |
| Kernel tokei | 19492 | 19818 | 19946 | +326 | +454 |
| `maos-domain` tokei | 9349 | 9396 | 9396 | +47 | +47 |

Per-file physical deltas — variant B: `t3/child.rs` +156, `t3/spawn.rs` +95, `cli_wrapper/runtime.rs` +22, `t3/image_lock.rs` +15 (probe scaffolding), `t3/argv.rs` +2, `cli_wrapper/mod.rs` +1 (domain: `sandbox.rs` +51, `supervision.rs` +7). Variant A adds `security/sandbox/linux.rs` +96, `t3/child.rs` +26, `t3/spawn.rs` +16 (`runtime.rs` net 0: it also fixes a B-prototype bug that read the CID after `self.child = None`). `unsafe`: variant B 0 new; variant A 4 new sites, all in `linux.rs` (`pidfd_open` syscall, `OwnedFd::from_raw_fd`, `poll`, `ioctl(PIDFD_GET_INFO)`).

**Read the figures honestly.** They include 79 physical lines of probe-only scaffolding (`VerifiedImageAttestation::for_probe`, 15; a duplicate `parse_container_state`, 64), and the prototype deleted existing doc comments in `child.rs`. It omits: the CID guard between `create` and `start`, the terminality gate, the truly bounded helper, the held-inode pidfile reader (its reader would block on a FIFO), typed residue and the idempotent removal verdict, the `container exists` post-check, new docs, and all maos-bin/maos-audit/maos-egress/test/CI/doc changes. No adjusted estimate is claimed: the measured figures are the E16-A7 numbers, and the complete implementation may land below or above them.

**Smoke (throwaway probe, real output in `smoke.log`):** the create argv has no `run`/`--rm`, has `--restart=no` and `--pidfile`, keeps names-only env; CID validation; pidfile refusals (missing, symlink, directory, non-numeric, zero, stale pre-existing, wrong directory mode); inspect-JSON parsing of 4.9- and 5.7-shaped `.State` objects (exit 0, 137, 137+OOMKilled, malformed → unavailable). On this host (kernel 7.2.9) the variant-A helper read the wait status of **non-child** processes reaped by another parent: `exit 137` → `Exited(137)`, an external `kill -KILL` → `Signaled(9)`, and a pidfd opened after the reap → `ESRCH` → `Unknown`. That is the mechanism AC1's S2/S3 rely on.

**Not proven here (no container runtime on this host):** real `create`/`init`/`start --attach` on podman 4.9.3 and 5.7.0, pidfile timing for an immediate exit, cgroup-manager identity paths, the conmon OOM marker on the runners, `rm --volumes` residue behavior, and all maos-bin wiring. The scout estimate +120…+360 is superseded: variant B measured inside it, variant A above it.

### Latest technical information

- Runner baselines (packages.ubuntu.com): noble podman `4.9.3+ds1-1ubuntu0.2` (noble-updates), conmon `2.1.10`, crun `1.14.1`; resolute podman `5.7.0+ds2-3build1`, conmon `2.1.13`, crun `1.21`. The active OCI runtime is not established by the package pages — log it (T6). Upstream latest is podman v6.1.4 (2026-10-12); do not assume it.
- `podman create` prints the CID before start ([create.go v4.9.3 :181-195](https://github.com/containers/podman/blob/v4.9.3/cmd/podman/containers/create.go#L181-L195)); `--pidfile` is a create option ([manual](https://docs.podman.io/en/v4.9.3/markdown/podman-create.1.html#pidfile-path)), written via conmon/OCI create ([conmon runtime_args.c :34-40](https://github.com/containers/conmon/blob/v2.1.10/src/runtime_args.c#L34-L40)) and not removed at exit ([run_test.go :2067-2081](https://github.com/containers/podman/blob/v4.9.3/test/e2e/run_test.go#L2067-L2081)).
- `start --attach` returns the payload exit code after the DB exit is recorded ([containers.go v4.9.3 :946-1009](https://github.com/containers/podman/blob/v4.9.3/pkg/domain/infra/abi/containers.go#L946-L1009)); attach precedes start, so immediate output is not lost ([container_api.go :131-136](https://github.com/containers/podman/blob/v4.9.3/libpod/container_api.go#L131-L136)). Killing only the client does not kill the payload, and a killed client skips that exit wait.
- Image `Config.Entrypoint` is prepended to the command, and `--init` inserts `/run/podman-init` as PID 1 ([oci.go v4.9.3 :25-56](https://github.com/containers/podman/blob/v4.9.3/pkg/specgen/generate/oci.go#L25-L56)) — hence AC4.2's PID 1 rule.
- Signal loss: [conmon ctr_exit.c v2.1.10 :100-107](https://github.com/containers/conmon/blob/v2.1.10/src/ctr_exit.c#L100-L107). OOM marker: [cgroup.c v2.1.10 :268-314](https://github.com/containers/conmon/blob/v2.1.10/src/cgroup.c#L268-L314). `StoppedByUser` is set by any successful podman kill/stop and is **not** death provenance ([container_api.go :284-301](https://github.com/containers/podman/blob/v4.9.3/libpod/container_api.go#L284-L301)).
- `rm --volumes` removes anonymous volumes but logs volume errors while exiting 0; `--time` bounds only the graceful stop ([runtime_ctr.go v4.9.3 :920-1044](https://github.com/containers/podman/blob/v4.9.3/libpod/runtime_ctr.go#L920-L1044)); `rm` exit 1 = no such container ([manual](https://docs.podman.io/en/v4.9.3/markdown/podman-rm.1.html#exit-status)).
- `--env=NAME` resolves from the create client and is stored in `Config.Env` ([specgen.go :442-478](https://github.com/containers/podman/blob/v4.9.3/pkg/specgenutil/specgen.go#L442-L478)); `podman-restart.service` restarts only `always` ([service :12-13](https://github.com/containers/podman/blob/v4.9.3/contrib/systemd/system/podman-restart.service.in#L12-L13)).
- `PIDFD_INFO_EXIT` (Linux 6.15): exit information is available once the task is reaped (`EPOLLHUP`), to any holder of a pidfd opened earlier ([vfs-6.15 pidfs pull](https://lkml.iu.edu/hypermail/linux/kernel/2503.2/09488.html); [uapi pidfd.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/pidfd.h)). The prototype's 64-byte v0 struct and request `0xc040ff0b` match the uapi. The locked `libc 0.2.186` carries `PIDFD_GET_INFO`/`pidfd_info`; `rustix 1.1.4` has no safe wrapper, so the call is `unsafe` and belongs in the sandbox zone (maos-bin stays `#![forbid(unsafe_code)]`).
- No new crate or dependency version is needed.

### Non-author validation — 2026-10-13

Three fresh read-only validators (cite consistency, design soundness, governance) re-read the draft against `37c34be4`. They found 15 blockers, 22 should-fix items and 3 nits, several overlapping; every finding is applied above. The ones that changed the design:

| Finding | Disposition |
|---|---|
| The archived variant-B diff was the A tree | Regenerated from the recorded commits; all three revisions re-measured; provenance in `measurements.json`. |
| A killed client could cache a live container's `ExitCode 0` | Terminality gate, stop sequence and `Truncated` for unavailable/non-terminal facts (AC3.1–2, AC4.3). |
| Unavailable snapshot vs client 0 was contradictory | `Truncated`, never clean; completion follows the same evidence (AC3.2, AC3.6). |
| A required `container` field would break native FR4 rows | Two exact shapes; native rows untouched (AC3.5, T2). |
| `egress.rs` cannot retain a lease when termination fails | *Termination unconfirmed* state (AC4.5). |
| The pidfile reader blocks on a FIFO; the helper is not bounded | Held-inode reader (AC2.1); truly bounded helper (AC4.4). |
| A hook that panics before its stop leaves the Worker running | Post-unload registry stop on every unload path (AC4.6). |
| `main.rs` smoke and eval executor construct T3 directly | Added to T3/T5. |
| New `Arc`/`OnceLock` kernel holders trip I9 | Exemptions + register (Delivery gate). |
| A runtime-only variant could not prove controls 2/5 | Proven-red oracles named per control; variant A ruled. |
| The startup report's journal entry is lossy | Fixed at origin in maos-bin (AC2.4). |
| PID 1 may not be the Worker; CID may be absent from the cgroup path | PID 1 rule (AC4.2); identity fallback, not refusal (T1). |

### Preflight — long-term correctness (2026-10-13)

Party-mode preflight builds on, rather than repeats, the creation prototype and three-validator review. Two independent read-only reviewers checked identity/audit and lifecycle/concurrency; the table records the resolved contract gaps and the integration proof required during development. These corrections stay inside the ruled locations; they grant no extra lines, unsafe sites, dependencies or CI permissions. Epic §R12 adds **R17-133…R17-140**.

| ID | Problem / disposition | Required development oracle |
|---|---|---|
| **PF-01** | A runtime can commit creation before stdout succeeds. AC4.1 makes a private CID receipt the sole cleanup authority and reports unknown ownership; never recover destructively by name or stdout alone. | Receipt written then stdout hangs → bounded refusal/receipt-CID cleanup; absent/malformed receipt, including each paired with a valid foreign stdout CID → one unknown report/retained state dir/no init or start/foreign CID untouched; conflicting stdout/receipt → no start and foreign stdout CID survives; name collision unchanged. |
| **PF-02** | “Every stop latches PlannedStop” suppressed unexpected-client faults on the ECHILD finalization path. AC3.2/AC4.3 separate intentional stop from fault cleanup. | Kill only the client; force ECHILD, then observer-first counterpart. Freeze unavailable facts while payload lives; cleanup succeeds; exactly one `fault.truncated` disposition, no completion, CID removed. |
| **PF-03** | Classification admitted `NonZeroExit{-1}` and unspecified >255 codes. The table and decoding rule reject missing/invalid exit evidence. | Without pidfd evidence, `-1`, another negative, `256`, out-of-i32 and malformed fields → `Truncated`, null orphan code/signal, no completion on both feed orders; preserve 137 and OOM fallback. |
| **PF-04** | “Exact container schema” was undeclared for unavailable/residue paths. §Exit-row schema fixes keys, nullability and invariants; native rows stay unchanged. | Reopen genuine finalized rows for all four states, exercise both FR4 renderers; malformed/type/key/cross-field variants, including `unknown` with source `pidfd`, fail closed. |
| **PF-05** | Immutable death evidence must not prevent later confirmed lease release or be rewritten by cleanup. AC4.5 defines independent monotonic confirmation. | Unavailable snapshot then confirmed CID death → classifiers still `Truncated`, lease may end; never-confirmed counterpart retains/reports residue through binding Drop and root cancellation. |
| **PF-06** | “Kill all first” alone does not bound serial N-worker teardown; forced exit can bypass reporting. AC4.4 makes every stage concurrent under one root deadline and integrates the signal listener. | N≥3 delayed/hung kill/snapshot/remove stubs → all eligible CIDs attempted, each verdict/remainder reported within one 17 s window, not N windows; include forced-exit and observer/finalizer/Drop race with one snapshot deadline and removal verdict. |
| **PF-07** | PID conversion, ENTRYPOINT composition and missing field validity could misidentify the payload or fabricate success. AC2.1/T1 define PID bounds, libc mask checks, required state types and one executable in OCI argv. | PID zero/overflow/trailing data refused; ioctl success with EXIT bit absent never decodes zero; delete/mistype each State field (empty Error/zero-time strings remain valid); distinctive real task acknowledgement names the task, not `/maos/spirit`; late stop attachment still tears down once. |
| **PF-08** | Voluntary `CrashCause::exit_code()` is None; helper drain/reap promises could exceed deadlines. AC3.6 uses the verified payload status; AC4.4 bounds return and reports unconfirmed client custody. | Real immediate exit still completes via the existing journal oracle; cached unknown/client 0 never does. A descendant-held pipe and unfinished reaper cannot postpone the helper result past its deadline. |

**Verification boundary:** this is a corrected development contract, not an implemented runtime. No production code or prototype evidence was changed/re-run. Real Podman startup, OOM marker, pidfd identity conventions, lease/unload wiring and proven-red CI runs remain mandatory development evidence on both x64 images, as already required by AC1/T4/T6/§A6.

**Executed preflight checks:** artifact smoke verified synchronized `ready-for-dev`, all seven task groups, one-to-one PF-01…08 / R17-133…140, six schema keys and balanced fences. `cargo run --locked -q -p xtask -- check-epic-close-coherence` passed (21 epics, pin 25145); `check-dev-record-completeness` passed (168 done stories); `check-review-findings-resolved` passed (169 stories); `check-bare-review-findings` exited 0. A throwaway invocation of the actual delegated runner's `check-log` CLI accepted a complete suite and rejected mid-line SKIP, zero scenes, ignored scenes and nonzero command status; its temporary logs were removed. This checks the existing guard, not execution of the future T3 suite.

**Existing project gate debt (not waived):** creation observed six `check-exit-commands` findings and one `check-dev-model-tier` finding outside 17-7. The exit gate reads only `target/debug/{maos,maosctl}` (`check_exit_commands.rs:1580-1586`); this host uses `CARGO_TARGET_DIR=/mnt/build/cargo`, so “not built locally” does not prove a missing source binary. The 17-2 record still has creation attribution and says implementation has not begun (`17-2-worker-cgroups-applied.md:483-485`); the model-tier failure is not evidence to relabel the author or widen the allowlist. Resolve that record from actual implementation provenance and run the exit gate with its documented local binaries before claiming project-wide green. Those unrelated records/gates are intentionally unchanged here.

### References

- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md` — §17-7, exit block, rules 9–11, §R11, §R12.
- `_bmad-output/implementation-artifacts/17-2-worker-cgroups-applied.md`, `17-2-flag-winston-decision.md` (FW-07 creates this story; FW-08 CI precedent); `17-1-flag-winston-decision.md` (FW-05).
- `_bmad-output/implementation-artifacts/epic-16-retro-2026-09-20.md:144,150` — E16-A1 both pins; E16-A7.
- `_bmad-output/planning-artifacts/prd/functional-requirements.md:27-31,41,44`; `non-functional-requirements.md:20,30`.
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:294,337,596-603`; `5-spirit-abi.md:130-131,145-147`.
- `docs/adr/ADR-061-worker-egress-and-scoped-credential.md:21-38,139-161`; `ADR-062:161-167`; `ADR-067:78-90`; `docs/invariants/i9-exemptions.md`.
- No `project-context.md` matched the activation glob.

## Dev Agent Record

### Story context creation

Created 2026-10-13 by `anthropic/claude-opus-5-5` with five read-only scouts, one prototype agent and three non-author validators; charter ratified by the operator the same day (variant A). Creation evidence: `17-7-evidence/`. The prototype and its probe are throwaway sizing instruments, not shipped code or tests.

### Agent Model Used

anthropic/claude-opus-5-5 (bmad-dev-story, dev start 2026-10-14)

Review repairs: openai-codex/gpt-6.1-sol (bmad-code-review, 2026-10-15; original implementation attribution preserved).

### Debug Log References

- **Local** (devcontainer: no usable container runtime — user namespaces blocked by seccomp; `CI=true` is exported, so stub-backed runs use `env -u CI`):
  - `cd crates/maos-bin && env -u CI cargo test --locked --features network --test worker_terminal_facts_17_7 -- --test-threads=1` → 26 passed (real-runtime scenes report unavailable locally).
  - `cargo test --locked -p maos-bin --features network --test worker_supervision_16_3` → 23 passed (stub runtime).
  - `env -u CI cargo test --locked -p maos-domain -p maos-audit -p maos-kernel-core --no-fail-fast` → all pass except the pre-existing `cgroup_ceiling_smoke` (no delegated cgroup here); `maos-domain --test container_terminal_facts_17_7` 11, `maos-kernel-core --test t3_terminal_helpers_17_7` 4, `--test t3_argv_custody_17_1` 5, `maos-audit` 242.
  - `python3 -m unittest scripts/tests/test_cgroup_ceiling_17_2.py` 7 passed; `cargo test -p xtask --test epic17_exit_workflow_17_7`, `--test kernel_pin_content_hash_16_0` (24) and `-p maos-a2a-tcp --test t11_t12_chaos_absence` (3) pass.
  - `cargo check --locked --workspace --all-targets`, `-p maos-bin --all-targets --all-features`, `-p maos-bin --no-default-features`: clean.
  - Gates PASS: `check-kernel-baseline` (25775 lines, 95 files), `check-service-boundary`, `check-empty-kernel`, `kloc-check` (aggregate 174794), `check-abi-ratification`, `check-pub-field-constructors`, `check-unsafe`, `check-dev-record-completeness`, `check-epic-close-coherence` (pin 25775).
- **Initial implementation CI, before review repairs** (D-17-7-FW-08 route: throwaway branch `ci/17-7-acceptance`, workflow `ci-17-7`, branch-scoped push trigger, `contents: read`, no secrets, ubuntu-24.04 + ubuntu-26.04; evidence read from public check-run annotations):
  - 38557835145, 38561751109, 38564836448 — harness defects only (missing `cargo fetch` for the `--offline` delegated build; panic annotations added).
  - 38569363258 — first real-runtime pass: `spirit_session.rs`/`spawned_scope_17_3c.rs` still called `finish(&ExitCause)` under `wasm-host` (fixed); rootless podman's re-exec leaves the supervised client pid with an empty cmdline, so custody now reads the client's process tree.
  - 38573945942 — all eight proven-red controls red, each on its named oracle's assertion (24.04 acceptance shows the same oracles green unmutated).
  - 38577833777 — OOM probe in the delegated envelope: podman 4.9.3 / conmon 2.1.10 / crun 1.14.1 (24.04) never sets `OOMKilled` (plain `podman run --memory=64m` hog → `false`); podman 5.7.0 / conmon 2.1.13 / crun 1.21 (26.04) → `true`.
  - 38642562180 — 24.04 exit line 4 green (26/26); 26.04 kernel custody `InspectTimedOut` after a clean exit.
  - 38645560812 — podman 5.7's first post-exit `inspect` blocks on the container lock (1.1 s in a plain podman probe; 2 s + 5.8 s in the custody scene; ~30 ms afterwards).
  - 38654457883 — 26.04 fully green under the raised budgets; 24.04 root-crash external-stop race (test fix).
  - **38666336543** and **38668208182** (final implementation tree before review repairs) — both images green on exit line 4 (`worker-terminal-facts.log` 26 passed, inventory present, no SKIP), migrated T3 suites (16-3, 2a, 16-3 crash corpus, 17-2, kernel custody/argv/helpers, domain, audit, control, eval escape corpus) and 17-2 delegated resources (2 passed); all eight proven-red jobs red as required: 1 `s1_immediate_exit_terminal_facts` (S1 must complete), 2 and 5 `s3_external_sigkill_requires_pidfd_fact` (detection 6.8 s > 5 s; `fault.non-zero-exit`), 3 `payload_classification_obeys_oom_authority_and_fallback`, 4 `ownership_is_by_cid_never_by_requested_name`, 6 `deadlines_kill_is_bounded_and_verdict_is_shared` (30 s), 7 `leases_worker_egress_finishes_after_termination_decision`, 8 `client_death_freezes_unavailable_then_cleans_up_by_cid`.

### Completion Notes List

- Variant A delivered: `create --restart=no --pidfile --cidfile --entrypoint=/maos/spirit` → bounded `init` → held-inode pidfile + CID receipt → payload pidfd (`PIDFD_INFO_EXIT`, CID-bearing cgroup check) → supervised `start --attach <cid>`. One immutable terminal snapshot (`ContainerTerminal::facts`) feeds the observer, `wait_and_finalize`, the five-key T3 `cli.subprocess.exit` row and completion; teardown is CID-only (kill → confirm → snapshot → `rm --force --volumes --time=0` → `container exists`) with one shared verdict and typed residue.
- Pure parsers and classification live in `maos-domain` (`ContainerState`, `ContainerTerminalFacts::classify`, `ContainerExitRecord`) to keep kernel lines down; the kernel still exceeded the ratified ceiling, and the operator approved the exact measured re-pin (kernel 25775 physical / 20050 tokei; domain 9615, bin 25847, audit 7267, eval 3802, aggregate 174795 → measured 174794). Four new SAFETY-commented `unsafe` sites in `security/sandbox/linux.rs` (5 total, base 1).
- Operator rulings taken during development: (1) **S4 OOM oracle is runtime-referenced** — the suite runs a plain memory-capped hog container and asserts MAOS records that runtime's own marker: marker → `fault.oom_killed`, no marker → `fault.signaled` (pidfd) / `fault.non-zero-exit` (fallback); (2) **SNAPSHOT_BUDGET 2 s → 10 s, TEARDOWN_BUDGET 17 s → 25 s**, S3 detection bound 12 s on podman ≥ 5 (FR12's 5 s still asserted on podman 4). Both deviate from the ruled numbers in §Acceptance/§Dev Notes, which are left as written; the kernel pin/surface were re-pinned for the constant change only (line counts unchanged).
- Review repairs replace the kernel-only/manual fallback and client-death checks with production observer/finalizer/binding/writer scenes, real lease decisions, and finalized-row FR4 round trips. Added doc-hidden `bridge_container` and held-receipt seams use the production implementations; there is no second framing/receipt-reader convention. Revised controls 7/8 and the observer-specific control 5 are now verified by fresh assertion-level CI failures.
- Test fixes surfaced by real runtimes: concrete-payload receipt counting (16-3 precedent), the `hook.budget.on_unload` warning row (ratio < 1) beside the exceeded row, the ruled `on_unload_hook_failure` orphan row in S5, the 16-3 reap-race test now waits for the parked observer's ECHILD, the 17-2 resource test waits for `State.Pid` to exec past `crun create`, root-crash re-issues the external stop until the runtime publishes the exit.
- The review closes the reported FR4 integration/omission, lease-ordering and startup-root-join gaps. The initial implementation also disclosed an isolated other-UID owner-denial test and failure cleanup in some original real-runtime scenes as limitations; this review does not claim those broader proofs or a locally provisioned runtime.
- All Story 17-7 delivery gates are satisfied, including founder approval under D-17-7-FW-10. Linux runtime acceptance/proven-red evidence is complete; Windows release remains held until an explicit operator command. `deferred-work.md:1121`, `:1122`, `:1133` are closure records with evidence and preserved owner cues. Closing this story does not perform or authorize a main commit, merge or release.
- Host changes in this devcontainer: podman installed then purged (unusable: seccomp blocks user namespaces); `nodejs` apt-installed for the docs gates.
- Temporary branch triggers and all mutation patches are CI-only and must never merge into main. Their exact source diffs, commands and run IDs are archived in `17-7-evidence/review-ci.json`; temporary-branch cleanup is recorded there.

### File List

- `.github/workflows/discipline.yml`
- `.github/workflows/release.yml` (operator Windows hold; target matrix unchanged)
- `BREAKING.md`
- `_bmad-output/implementation-artifacts/17-7-t3-worker-retained-lifecycle-and-terminal-facts.md`
- `_bmad-output/implementation-artifacts/17-7-flag-winston-decision.md` (scoped sole-founder quorum exception D-17-7-FW-10)
- `_bmad-output/implementation-artifacts/17-7-evidence/review-ci.json` (exact CI diffs/commands, runtime results, red assertions and approval state)
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/deferred-work.md` (Windows false support statement closed; T3 owner rows remain until done)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md`
- `crates/maos-audit/src/fr4_classifier.rs`
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-audit/tests/fr4_worker_rows_16_3.rs`
- `crates/maos-attrs/src/lib.rs`
- `crates/maos-bin/src/egress.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/operator_door.rs`
- `crates/maos-bin/src/purge.rs`
- `crates/maos-bin/src/shell_host.rs`
- `crates/maos-bin/src/spirit_session.rs`
- `crates/maos-bin/src/supervision.rs`
- `crates/maos-bin/src/worker_spawn.rs`
- `crates/maos-bin/tests/egress_daemon_17_1.rs`
- `crates/maos-bin/tests/root_shutdown_unload_16_3.rs`
- `crates/maos-bin/tests/spawned_scope_17_3c.rs`
- `crates/maos-bin/tests/support/t0_worker.rs`
- `crates/maos-bin/tests/two_host_delegation_2b.rs`
- `crates/maos-bin/tests/worker_completion_2a.rs`
- `crates/maos-bin/tests/worker_crash_corpus_16_3.rs`
- `crates/maos-bin/tests/worker_hang_corpus_16_3.rs`
- `crates/maos-bin/tests/worker_resource_limits_17_2.rs`
- `crates/maos-bin/tests/worker_supervision_16_3.rs`
- `crates/maos-bin/tests/worker_terminal_facts_17_7.rs` (new)
- `crates/maos-bin/tests/worker_terminal_facts_17_7/{root_crash,runtime_scenes,startup_scenes,stub_scenes,supervisor_scenes,unload_scenes}.rs` (new)
- `crates/maos-control/tests/post_surface_16_1.rs`
- `crates/maos-domain/src/sandbox.rs`
- `crates/maos-domain/src/supervision.rs`
- `crates/maos-domain/tests/container_terminal_facts_17_7.rs` (new)
- `crates/maos-eval/test-fixtures/t3-network-escape-probe/src/main.rs`
- `crates/maos-eval/tests/t3_network_escape_executor.rs`
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/mod.rs`
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`
- `crates/maos-kernel-core/src/security/sandbox/linux.rs`
- `crates/maos-kernel-core/src/security/sandbox/t3/argv.rs`
- `crates/maos-kernel-core/src/security/sandbox/t3/child.rs`
- `crates/maos-kernel-core/src/security/sandbox/t3/mod.rs`
- `crates/maos-kernel-core/src/security/sandbox/t3/spawn.rs`
- `crates/maos-kernel-core/tests/sandbox_t3_spawn_linux.rs`
- `crates/maos-kernel-core/tests/t3_argv_custody_17_1.rs`
- `crates/maos-kernel-core/tests/t3_terminal_helpers_17_7.rs` (new)
- `docs-site/docs/cookbook/cli-wrapper-spirit.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/cli-wrapper-spirit.md`
- `docs/ci-baselines/kernel-surface-v0.1-beta.json`
- `docs/invariants/i9-exemptions.md`
- `docs/release/windows-deferral.md`
- `scripts/run_cgroup_ceiling_17_2.py`
- `scripts/tests/test_cgroup_ceiling_17_2.py`
- `spirits/worker/src/bin/worker-cli-fixture.rs`
- `spirits/worker/src/bin/worker-probe-fixture.rs`
- `xtask/abi-ratifications.toml`
- `xtask/kernel-api-classes.toml`
- `xtask/kernel-core-baseline.toml`
- `xtask/kloc.toml`
- `xtask/tests/epic17_exit_workflow_17_7.rs` (new)
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/tests/recovery_lane_ceiling_rule.rs`

### Change Log

- 2026-10-15 — Story 17-7 implemented (variant A retained T3 lifecycle and terminal facts); operator rulings: exact measured re-pins, runtime-referenced S4 OOM oracle, snapshot/teardown budgets 10 s/25 s with the S3 bound 12 s on podman ≥ 5. CI acceptance and all eight proven-red controls green/red as required on runs 38666336543 and 38668208182. Status → review.
- 2026-10-15 — Full adversarial code review: all 17 selected patches applied; local behavioral smoke and targeted suites verified; exact measured re-pins and source/surface ledger refresh explicitly approved by Lunarpulse. Status → in-progress pending post-review provisioned CI and maintainer I9 sign-offs.
- 2026-10-15 — Executed the post-review plan: Windows release explicitly held; dual-image real-runtime acceptance and all eight assertion-level red controls verified; repaired migrated-suite invocation/validation, resource readiness race and C5 feed-specific oracle. Exact evidence archived; status remains in-progress solely for ≥ 2 genuine I9 maintainer signatures.
- Operator follow-up — Lunarpulse explicitly approved the reviewed implementation and I9 exemption rationales in the conversation. Recorded 1/2 signatures; existing two-maintainer policy and Windows release hold remain unchanged. Runtime behavior is unchanged from the successful final CI candidate.
- Founder closure — Lunarpulse explicitly waived the second signature while the founder is the sole maintainer and approved the reviewed implementation. D-17-7-FW-10 records one signer, not two; T7 and the three owned deferred-work items are complete. Status → done. Windows release remains deferred; no main commit, merge or release performed.
