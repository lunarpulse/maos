# Epic 16 — One Daemon, One Door (J0 for real) (W1)

**Status:** `backlog` — Round 2 correct-course 2026-09-04 (`sprint-change-proposal-2026-09-04-round2.md`, Lunarpulse-ratified; Fork C). Replaces the morning's Epic 16 file. `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 4–5 weeks (measured) · **Makes true:** v0.1 · J0 evaluator — the operator controls a running Spirit and can watch a halt resolve

**Closes:** S1 (control plane never reaches the daemon); FR2, FR9, FR12, FR13, FR15, FR16, FR20 (door), FR24, FR50, FR51, FR58; D3/D5/D7/D12/D18

**Hermetic exit command (CI, no secrets; the epic's first CI job, red until green):**

```
maos init && MAOS_INFERENCE_MODE=replay maos shell
@hello-spirit refactor src/main.rs to be more idiomatic        # [HALT task.acceptance_criterion.ambiguous]
maosctl halt list && maosctl halt resolve <id> --provided-context "idiomatic = clippy-clean, no unwrap"
maos run spirits/butler/manifest.toml &   ;   maosctl pause butler   ;   maosctl spirit inspect butler   # shows paused; daemon journal has the row
maos audit query --spirit hello-spirit && maos uninstall --keep-log
```

**Operator-lane legs (never in the exit):**
- live-key J0 leg (`MAOS_INFERENCE_MODE=live`) recorded in the runbook, never in CI

**Kernel-Δ:** ZERO kernel-Δ for 16-1..16-4 (the kernel APIs are already `pub`: `PolicyTable::shift_posture` `cap_policy/mod.rs:233`, `OrchestratorBuffer::enqueue` `buffer.rs:46`, `SpiritSchedulerAdapter::pause/resume` `scheduler_loop.rs:401/428`); **16-5 is FLAG-Winston** (+5–25 kernel lines, re-pin in its own commit). This epic holds the shared-write lock on `crates/maos-bin/src/main.rs` first.

**Confidence rules (ratified 2026-09-04 R2, binding):** 1 verb-first (exit verbs exist at HEAD or are AC1 in this epic; `check-exit-commands`) · 2 hermetic exit, live optional (paid/human/clock legs are operator-lane rows) · 3 foundations first · 4 cite-or-cut (every AC names file:line + kernel-Δ) · 5 spike before build · 6 size from measurement (+30%) · 7 one decision per fork · 8 inventory from the tree.

**Dev-gate:** external holds (NFR-Sec-7, NFR-Comp-1) = GA ledger only. **Model/review:** frontier allowlist + §A6 net binding; non-degradable on stories marked ◆.

---

## Stories

| Key | Story | Closes · Δ |
|---|---|---|
| `16-1-daemon-post-surface-and-verb-retarget` | Daemon POST surface and verb re-target | S1 · kernel-Δ 0 · maos-bin +300–500, maos-control +400–600, maos-cli +100–200 |
| `16-2-shell-halt-registry-and-j0-scene` | Shell halt registry and the J0 scene | J0 · FR15/FR58 · kernel-Δ 0 · maos-shell +30–60 |
| `16-3-subprocess-crash-to-handle-crash` | Subprocess crash reaches the crash handler | FR12/FR50 · kernel-Δ 0–10 (`CrashCause` variant) · maos-bin +40–80 |
| `16-4-maos-uninstall-and-keyring` | `maos uninstall` and the keyring backend | FR2 · kernel-Δ 0 · maos-secrets +150–300 |
| `16-5-audit-drop-legal-hold-and-a2a-deny-vocabulary` | Debt slot: audit drop, legal hold, A2A deny vocabulary ◆ FLAG-Winston | D3/D5/D7/D12/D18 · kernel-Δ +5–25 · maos-a2a-core +20–40 |

AC1 is always the command and what the operator observes; every AC cites the code it changes; `bmad-create-story` may tighten but not add uncited ACs.

### 16-1-daemon-post-surface-and-verb-retarget — Daemon POST surface and verb re-target

*Closes · Δ:* S1 · kernel-Δ 0 · maos-bin +300–500, maos-control +400–600, maos-cli +100–200

- **AC1 (command).** With `maos run spirits/butler/manifest.toml` running, `maosctl pause butler` makes `maosctl spirit inspect butler` report `paused` and the daemon journal carries the pause; `maosctl posture shift butler cautious` changes the next capability decision of the running Spirit.
- **AC2.** `maos-control` gains POST + a body parser + typed errors under the bearer token (today GET-only, 4 routes, `lib.rs:200-219`; design note `lib.rs:68-76` amended by ADR-062); `maos init` mints `MAOS_OPERATOR_BEARER_TOKEN` into `MAOS_HOME` config; the daemon binds the loopback surface by default.
- **AC3.** The **19** one-shot verbs (16 literals + `start/stop/unload`, `maos-cli/src/subcommands.rs:1461`) are re-targeted to the surface; one-shot survives for `init`, `backup`, `import`, `smoke-*`, `cohort-a2a-daemon`; the synthetic-halt seeding in `halt list/resolve` (`main.rs:5566-5580`) is deleted.
- **AC4.** `maosctl revoke-token` mutates the daemon-held `CapTokensShardRing`; `maosctl revocations import` reports `matched > 0` against a running Spirit; `maosctl orchestrator queue` enqueues into the daemon registry (`registry.rs:32`).
- **AC5.** Proven-red: with the daemon down every verb exits non-zero with a typed error, never journal-and-succeed.
- **AC6.** The 13 test sites that spawn `MAOS_ONE_SHOT` (hello-spirit 7, posture-shift 3, legal-hold 2, uninstall 1) are updated; `cohort_daemon_smoke_13_5c` `SCANNED_SOURCE_FILES` re-counted.

### 16-2-shell-halt-registry-and-j0-scene — Shell halt registry and the J0 scene

*Closes · Δ:* J0 · FR15/FR58 · kernel-Δ 0 · maos-shell +30–60

- **AC1 (command).** Lines 1–3 of the exit command run on a clean VM inside 5 minutes (NFR-Onb-2) and `maos audit query --spirit hello-spirit` shows the halt and its resolution rows.
- **AC2.** `maos_shell::run_shell` receives `Arc<HaltRegistry>` + the Transparency Log; hello-spirit's existing `Ambiguous` return (`maos-spirit-hello/src/lib.rs:127-148`) inserts a `PendingResolution` and emits an `EpistemicHalt` frame (today: a cap-audit row only, `maos-shell/src/lib.rs:271-282`); `maosctl halt resolve` (via 16-1) resolves that halt.
- **AC3.** `journey_j0` asserts halt, resolution and audit rows (today banner strings only, `journey_j0.rs:28,55`).
- **AC4.** `docs/maos.dev` run guide documents `MAOS_INFERENCE_MODE` and the API key.

### 16-3-subprocess-crash-to-handle-crash — Subprocess crash reaches the crash handler

*Closes · Δ:* FR12/FR50 · kernel-Δ 0–10 (`CrashCause` variant) · maos-bin +40–80

- **AC1 (command).** `kill -9` on a fixture Worker yields `task.orphaned` within 2 s; on a 100-SIGKILL corpus ≥99/100, asserted by `nfr-rel-1-crash-detection-2s` (today `crash_detector_in_process_panic` asserts ≥1 frame, `discipline.yml:1161-1185`).
- **AC2.** A 50-hang corpus yields `task.stalled` within 60 s for ≥48/50, asserted by `nfr-rel-2-hang-detection-60s`.
- **AC3.** `wait_and_finalize` (`cli_wrapper/runtime.rs:632`, caller `worker_spawn.rs:732`) feeds the `CrashDetector`; `handle_crash` (sole caller `scheduler_loop.rs:583`) gains the process-exit path; `on_crash.action` applied with reassign→escalate journaled.

### 16-4-maos-uninstall-and-keyring — `maos uninstall` and the keyring backend

*Closes · Δ:* FR2 · kernel-Δ 0 · maos-secrets +150–300

- **AC1 (command).** `maos uninstall` (new top-level verb; today `uninstall` is `maosctl uninstall <spirit>`, `main.rs:5152`) removes the binary, `MAOS_HOME` (`journal/lifecycle.ndjson`, archive dir, TL sqlite, shell config) and the XDG fallbacks; `--keep-log` retains the TL; `maos init` afterwards is clean.
- **AC2.** `maos-secrets` gains an OS-keyring backend (`keyring` crate; MIT/Apache within `deny.toml:36-49`); provider keys are read keyring-first, env fallback journaled as a downgrade; `FIXME(secrets)` at `main.rs:3079` retired.
- **AC3.** New env entries registered; `maosctl uninstall <spirit>` (FR65) unchanged.

### 16-5-audit-drop-legal-hold-and-a2a-deny-vocabulary — Debt slot: audit drop, legal hold, A2A deny vocabulary ◆ FLAG-Winston

*Closes · Δ:* D3/D5/D7/D12/D18 · kernel-Δ +5–25 · maos-a2a-core +20–40

- **AC1 (command).** A fault-injected audit sink makes `record_invocation` return `Err` and the invocation is refused (today `Ok(())` after `record_drop()` at `capability/mod.rs:243,349-350`); new `CapError` variant in `maos-domain`.
- **AC2.** The legal-hold check-then-act race in `forget_with_reason` (`memory/mod.rs:483`) is serialized; under hold the call returns a typed refusal.
- **AC3.** `map_a2a_error_to_iac_bus` (`router.rs:1933`) preserves the deny vocabulary through `IacBusError` variants (`maos-domain`; the refusal note at `main.rs:9812-9815` updated); `CrossWallRecallRefusal` variants surface typed (D7).
- **AC4.** The kernel re-pin (24472→N) and the measured grant land in the same commit.

## Dependencies

Epic 15 (15-2 allowances, 15-5 ADR-062, 15-6 replay seam). 16-1 before 16-2.
