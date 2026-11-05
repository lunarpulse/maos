# Landing 1 current-tree inventory and T0 continuation

Baseline: `76efc19d3ffd36a1b897724704d76f94fba3d9eb`, 2026-11-04, Dev2011b.
The five inherited tracked implementation hunks and untracked inventory test were
adopted only after preserving `inherited-wip.diff` and replacing the tracked
hunks with exact `git show HEAD:<path>` bytes for real baseline witnesses. No
kernel source or kernel baseline is changed. The old T0 files are preserved.

## Re-read baseline cites

| Area | Current HEAD location before L1 | Observation |
|---|---|---|
| Cascade callers | `crates/maos-bin/src/main.rs:7440`, `:10134-10149` | Offline and live door share the cascade. |
| Cascade | `main.rs:10495-10547`, `:10557-10989` | Terminal journal, principal discovery/forget, proof construction and signed outcome remain one path. |
| False NotFound | `main.rs:10762-10782` | Principal-only condition suppresses proofs over principal-free residue. |
| False lineage / gaps | `main.rs:10886-10901` | VerifiedEmpty constant; halt/schedule API-gap constants. |
| Door uninstall | `operator_door.rs:1609-1623` | Unload/Worker stop precede cascade; snapshot must be captured before unloading. |
| Door forget / release | `operator_door.rs:2046`, `:2090` | Blocking forget on worker; release bypasses Memory Manager serializer. |
| Late sweep | `hook_pump.rs:792-795` | Subject hold was not rechecked; new late frame was scrubbed under a newly placed hold. |
| Forget scans | `maos-iac/src/adapter/transparency_log.rs:1555-1633`, `:1636-1719` | Whole-scan lock holds, held-co-principal closure, malformed scan. |
| Registry | `maos-registry/src/storage.rs:136-165`, `:179-220` | Active installed inventory and generation directories; no uninstall operation at HEAD. |
| Proof | `maos-audit/src/erasure/proof.rs:179`, `:380-399`; `tools/verify-erasure/verify.py:273-312` | Frame-only P18 rejects non-frame removals; Python omitted principal-frame inclusions. |
| Other stores | `maos-skill/src/store.rs:20-24,41-51,137-146`; `main.rs:10463-10495`; `maos-bin/src/worker_workspace.rs:3-11,30-61,93-124` | Authored queue identity is class/boot/pid; Lifecycle Journal retained; Worker work product also persists beyond unload. |
| Admission schedules | `maos-bin/src/admission.rs:451-495`; `worker_spawn.rs:915-919` | Both constructors default SCB schedules; a declared input schedule does not fire. |
| Existing repair owner | `20-9-spawned-form-lifecycle-hooks.md:39,73,97,268-276` | D-20-9-08 A / AC3 already owns schedule carriage. L1 must not duplicate that work. |

## T0 outcomes

1. Actual affected binary/test builds compile the implemented IAC/proof/registry
   changes. The initial compilation mistakes (missing IAC hex dependency and
   guessed ManifestDocument schedule API) were fixed using existing formatting
   and SchedulesSection APIs, not by adding dependencies.
2. The observed unreceipted late halt and its old red remain exactly as recorded
   in `t0-stop.md` / `t0-unload-race-witness.rs.txt`. No L1 receipt count, fence,
   watchdog cleanup or kernel pin movement is claimed.
3. Original HEAD's 100k-row real-store forget measured wall **438.214496 ms**,
   maximum concurrent insert **200.031006 ms**, ratio **0.4564682543** (red), in
   `l1-before-scan.log`. That original vector stopped at 100k; it did not reach
   its 1M iteration. The improved real-door instrument runs both 100k and 1M
   before asserting and additionally instruments a constituent query. Its
   `i64::MAX` control is the recorded unbounded-window calibration at both
   sizes, not falsely labelled an untouched-HEAD 1M result.
4. The door returns spirit_not_loaded for a refused installed Spirit (there is
   no SCB to unload); the operator stops the serving root and uses offline
   uninstall under the existing exclusive store locks. The package-only test
   exercises offline deletion without an admitted incarnation or principals.
5. Scan results are explicitly sorted by `(timestamp_ns, frame_id)` as before.
   The existing 13.5b / 18-6 preservation suites are part of focused verification;
   no new rowid-order receipt contract is invented.
6. Python's old verifier omitted principal-frame inclusions, unlike Rust. V2
   checks both inclusion sets and category evidence. The missing-evidence
   vector re-signs the malformed envelope, so its rejection cannot be explained
   solely by a stale signature. Local Python initially lacked an Ed25519 backend;
   an isolated scratch venv supplies cryptography, with no system Python change.
7. Observed live researcher serving root accepted the schedule-bearing manifest,
   entered Running, spawned the watchdog, but never fired. Source reads above
   show why: schedules are not carried. AC6.3's allowed fallback therefore uses
   a canonical principal-free distillate seeded at the running Spirit's actual
   boot/pid, then the real live door cascade. The hosted Worker test uses the same
   oracle plus owned-CID teardown; local absence of the signed T3 substrate is
   explicitly skipped, not counted as a hosted green. The real scheduled-SCB
   door-boundary test separately proves snapshot enumeration before unload.

## Re-inventory disposition

The typed retained-store list includes FR11 manifest archive, authored skill
queue, Lifecycle Journal **and Worker output workspaces**. Workspace directory
names are matched by the existing `maos-<spirit-id>-<numeric nonce>` shape; only
metadata is inspected, never subuid-owned work product. These are counted and
retained with reasons, not silently erased. TL audit rows and IAC lineage arrays
remain append-only; that retention is explicitly named in signed gap reasons.
Cost numerical facts retain the existing structured-redaction behavior.
