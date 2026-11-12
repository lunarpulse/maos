## Verdict

**Overall correctness: correct**  
**Confidence: 0.89**

No provable patch-introduced merge blocker found. The P1 fix addresses the reported mailbox-registration race, and the TS cap change remains valid and enforced on the supported spawned execution path.

**Counts:** P0: 0 · P1: 0 · P2: 0 · P3: 0. No correction round required.

## P1 assessment

- **Root cause confirmed.** `Recipient::drop` unregisters the mailbox at `spirit_session.rs:258–262`; registration occurs at `:405`. Unload and `stop_binding` signal termination without joining that blocking session.
- **Guard ordering is correct for mailbox and identity release.** `_ended` is bound before `_owner` and `recipient` (`spirit_session.rs:382–405`), so normal returns and unwinding release those resources before closing the watch channel. No sender cloning or forgetting was found.
- **Cancellation does not prematurely announce release.** A started `spawn_blocking` task cannot be aborted; it retains the sender while it retains the mailbox. Cancellation before execution cannot leave an acquired mailbox behind. Existing stuck blocking work could retain the sender, but admission stops waiting after 15 seconds.
- **No lock cycle found.** `SessionEnds::ended` releases its map mutex before awaiting (`spawned_lifecycle.rs:581–586`). Session teardown does not acquire the door’s per-Spirit or `upgrade_lock`.
- **Re-admission coverage is adequate.** Cold-swap successor and restore admissions share the wait. Hot swap, migration and auto-revert operate through the existing session rather than registering another mailbox. Boot and `maos run` establish initial sessions.
- **Rejected early-start race candidate:** a cold swap before `begin` cannot pass recovery preflight. `spawned_origin` requires a registered binding with `swap_context`; that context is installed after `begin`, adoption and mailbox registration. Thus scheduler PID visibility alone cannot bypass the wait.
- **Seam activation:** `before_session_release` is compiled into production libraries, not compile-time excluded. However, normal constructors supply `None`, and no production configuration, environment or command path enables it.

### Witnesses and stress evidence

The parked predecessor preserves the actual conflicting registration, making the wait-removal plant meaningful. Reviewed raw logs show:

- Initial witness fails with the hosted `already registered` signature.
- Removing the wait fails both new witnesses; the ordinary door vector still passes.
- Restoring the fix passes both witnesses and the door vector: **3/3**.
- Focused nextest record: **48/48 passed**.

The 750 ms pending assertion remains scheduler-dependent; it is not a mathematical guarantee against an arbitrarily starved plant. Its observed red/green results nevertheless exercise the intended ordering.

**The added wait contributes to route-budget pressure.** It serializes teardown before startup; calling every post-fix timeout unrelated would be inaccurate. Before: 19/32 completed, six registration races and seven route timeouts. After: 7/16 completed, nine route timeouts and no observed registration race. Those nine fail before reaching the refused-successor restore, limiting the comparison.

The safety trade-off is acceptable under the existing protocol: the 30-second client timeout reports `handler_still_running`, while the command remains committed and writes its completion row. The supplied stress logs do **not** establish eventual completion of each timed-out operation or prove hosted runners unaffected.

## F2 assessment

- `cpu_max_pct = 100` satisfies validation (`maos-manifest/src/manifest.rs:179–191`).
- Linux still requires delegated CPU control and writes finite `cpu.max = 100000 100000` (`sandbox/linux.rs:71–73,523–527`). This is one CPU, not unlimited CPU.
- Memory, fuel, T2 containment and fail-closed admission are unchanged.
- Example/template manifests, READMEs, proof assertion and discipline comment agree. No remaining live cap-10 assertion for the TS example was identified; remaining references describe historical measurements or other fixtures.
- Final run-5 artifacts support the stated comparison: cap 10 records **222 seconds** before the upgrade proceeds and subsequently `upgrade_failed`; cap 100 records completion, with **49.54 seconds Load→HotSwap** and a **29.0-second successful attempt**.
- Rejected Windows/macOS cap concerns: this spawned T2 bridge refuses non-Linux execution (`runtime.rs:555–585`). Their different backend semantics are not reachable through this example’s session path.
- Instrumented cap-100 failures are disclosed as the separately reviewed console issue in the final hosted record; they are not evidence that the plain cap-100 result failed.

## Deferred informational rows

| Observation | Evidence / disposition |
|---|---|
| `session_not_ended` is a detail token, not an `AdmissionRefusal::code()` value | Timeout returns `ClassBuild`, whose code is `class_build_failed`. The door preserves the token in `cold_swap_reverted`/`cold_swap_failed` detail; CLI prints it. The witness tests text, not a dedicated machine-code variant. |
| Registry retains closed entries | One entry per distinct session ID remains until root destruction; repeated same-ID swaps replace it. No production distinct-ID churn path establishing an unbounded-growth regression was found. |
| Timeout and exceptional-path coverage remain limited | Fifteen seconds is chosen, not slow-runner measured. Panic/abort ordering was assessed statically, not independently witnessed. |

## Accounting and verification limits

`repin-check.txt` records **38,628 → 38,677 (+49)**, within the pre-adjudicated +60. Pin application remains pending. The older `kloc-check.json` is red and records 38,701; it must not be cited as final-tree green evidence.

This was read-only source/log review: **no builds, tests, gates or mutations were run**. Still unverified: hosted P1 execution, delegated proof with the committed cap-100 manifests, slow-runner timeout sufficiency, and the final repinned tree.