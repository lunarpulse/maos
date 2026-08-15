# Story J1-DEMO: One-Command Narrated Founder-Loop Scene (`xtask demo-j1`)

Status: ready-for-dev

<!-- Lane: POST-EPIC-13 BRIDGE (J1 cross-host developer-remote). Epic-external by operator-ratified
     decision (sprint-status.yaml:238-250, sprint-change-proposal-2026-07-16.md). Created 2026-08-14
     from the party-mode J1 assessment + the D0 rehearsal run. -->

## Story

As **Lunarpulse (founder-operator)**,
I want **one command that runs the J1 founder-loop on my machine and narrates every journey beat with its evidence state**,
so that **I can watch, show, and (optionally) sign what J1 actually proves today — and the same command lights up new beats as the crosshost rungs (1a → 1b → crosshost-2) land, until it becomes the J1 journey closer**.

## Context — why this story exists (2026-08-14 rehearsal findings)

A D0 rehearsal (`./target/debug/maos run spirits/topologies/j1-founder-loop.toml --once`, local build, 5.2s wall) proved the local leg live: 3 class spirits loaded, fixture worker admitted under T3 host grant, real subprocess (`child_pid`), `worker_completion completed=true` with `completion_tl_ref`, clean `--once` exit. It also surfaced three defects/warts this story must handle:

1. **Journal corruption noise** — the operator's shared `~/.local/share/maos` state carried 2 corrupted lifecycle-journal lines (`journal: WARNING — skipping corrupted line 115/116 … Consider rotating the journal`; emitter `crates/maos-kernel-core/src/journal/mod.rs:113-138`). Verdict: **PROVISION-AROUND** (isolated state home per demo run; no rotation CLI exists today).
2. **Cap-token mediation fallback** — `cli_wrapper cap-token mediation not granted … proceeding under AC5 host-grant authority (operator policy proc.exec grant is Epic 9 surface)`. The fallback is **by design** pending the Epic-9 operator policy surface: the wrapper path performs host-grant resolution but never registers the manifest with SecurityManager, and `PolicyTable.evaluate` denies unknown pid/scope — so even a Cedar permit via `MAOS_PDP_POLICY_FILE`/`_INLINE` cannot green this path by itself (`crates/maos-bin/src/main.rs:949-1020,1132-1166,199-242`; `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:127-149`; `crates/maos-kernel-core/src/security/mod.rs:338-360`; `deferred-work.md:39-40`). The `CapabilityInvocation` exit row IS journaled synchronously (`runtime.rs:635-684`). Verdict: **NARRATE** honestly.
3. **Audit-writer drain timeout (5.0s of the 5.2s wall)** — `maos run: audit writer topology drain timed out after 5s`. Root cause traced: the `--once` branch drops `audit_tx`/inference/capability/orchestrator/scheduler/lifecycle senders but **not** the memory adapter; `MemoryManagerAdapter` retains `Arc<CapabilityRegistryAdapter>` which owns an `audit_tx` clone, so the writer channel never closes and queued rows can be **lost at process exit**. A later `maosctl audit sealed-export` can therefore miss rows. Verdict: **FIX-IN-STORY** (evidence-integrity defect, proven-red).

## Acceptance Criteria

1. **AC1 — One-command narrated local scene (runs today).** `cargo run -p xtask -- demo-j1`:
   - Provisions an **isolated state home**: fresh temp dir exported as BOTH `XDG_DATA_HOME` and `MAOS_HOME` to the `maos` subprocess (exact pattern: `crates/maos-journey-test/tests/journey_j1.rs:38-86`); `--keep-home <dir>` optionally retains it for inspection. Zero `journal: WARNING` lines is an asserted beat (a fresh home makes corruption noise a real failure, not ambient noise).
   - Runs `maos run spirits/topologies/j1-founder-loop.toml --once` as a **captured** subprocess and parses the NDJSON event stream (`spirit_loaded` ×3 = exactly {orchestrator, architect, reviewer}, `topology_worker_admit`, `host_grant_disposition`, `cli_wrapper_loaded` with real `child_pid`, `cli_wrapper_exit`, `worker_completion`, `on_idle_fired` ×3, `drain`).
   - Renders a **narrated beat-by-beat scene** (narration conventions per `8-14a-j0-evaluator-surface-and-runtime-cli.md:129-151`) followed by a **claim table in execution order** (not BTreeMap key order — do not copy `demo_reza.rs` `summarize` ordering).
   - Exit 0 **iff** every executed beat's assertion holds: `worker_completion.completed == true` **parsed from the event** (a raw exit code is never completion — Tier-2 spec T2 rule), nonempty `completion_tl_ref`, drain observed **without timeout** (AC4), zero journal warnings. Any executed-beat failure → nonzero exit.
2. **AC2 — Honest beat ledger: future beats render ABSENT, never silently skipped.** The scene declares the FULL J1 beat set including beats whose substrate has not landed, each rendered with exact `EvidenceState` wire strings (`PROVEN_BLOCKING` / `PROVEN_LIVE_SIGNED` / `ABSENT` / `INDETERMINATE` — `xtask/src/gate_common.rs:120-160`; plain "PROVEN" is a product-claim string, not a leg state):
   - `delegation-frame-crosses-loopback` → ABSENT until **j1-crosshost-1a** lands (owner named in output).
   - `disallowed-intent-refused-blocking` → ABSENT until **j1-crosshost-1b** (refusal = -32001 `CODE_INTENT_DENIED`, distinct from -32009 `CODE_CONSENT_UNCLASSIFIED` — 1b's nonconflation rule).
   - `two-host-signed-run` → ABSENT until **j1-crosshost-2** (v1.0 rung).
   - `halt-resume-referential-identity` → ABSENT, owner **FOLLOWUP-J1-RESUME-SEAM** (`journey_j1.rs:208-214`; spec T4). The demo claims only "no in-flight preemption + safe shutdown" — never resume-citation identity.
   - ABSENT future beats do NOT fail the run (advisory placeholders; Family-B "skip → visible-RED never silent-green" discipline, Story 13.6e). Once a beat's owning gate publishes a ledger (`tests/reports/evidence-ledger-<gate>.json`), the demo consumes it via `load_published_ledgers` + `PublishedLedger::validate_against` (`xtask/src/evidence_ledger.rs:1213-1480`) — **never** raw serde-Value reads (demo-reza's `summarize` flaw) — and the beat flips to the ledger's state automatically. Loopback beats are labeled **"loopback rehearsal — v0.8 rung"**; the scene never renders the words "cross-host proven" while `two-host-signed-run` is not `PROVEN_LIVE_SIGNED`.
3. **AC3 — Optional signed live take (`--live-codex`).** With `--live-codex`, the scene drives the Tier-2 path end-to-end per `runbook-j1-tier-2-signed-live-run.md` Phases 3–5: env preflight (`MAOS_LIVE_AGENT=1`, `MAOS_HOST_GRANTS`, `CODEX_API_KEY` — **not** `OPENAI_API_KEY`), **refuses to start** when `~/.codex/auth.json` exists (clean-home invariant) or the grants file is absent (fail-closed, narrated), runs the codex topology `--live` from a disposable demo dir, writes the capture JSON (exact non-secret fields per spec: `signer`, `live_agent_identity`, `command_metadata` redacted, `host_grant_disposition`, `audit_refs`, `egress: declared-not-enforced` + `egress_followup: FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT`, `redaction_result: verified`, `outcome`), then `maosctl audit record-capture` (host-level, no `--spirit`) → `sealed-export --range` (never `--spirit`) → `verify-bundle`, and renders "verify OK (<N> entries)" in the claim table. Fixture runs NEVER claim the Tier-2 beat (fixture never closes Tier-2 — spec). CI never exercises this flag (CI must not set `MAOS_LIVE_AGENT`).
4. **AC4 — Drain integrity fixed, proven-red.** The `--once` topology path completes the audit-writer drain without timeout: release the remaining `audit_tx` owners (the `MemoryManagerAdapter` retention chain) before awaiting writer completion in the `--once` drain branch (`crates/maos-bin/src/main.rs:4164-4181`; retention: `crates/maos-kernel-core/src/memory/mod.rs:103-105,164-170` holds `Arc<CapabilityRegistryAdapter>`, which owns the clone taken at `main.rs:2607-2613`; the writer loop exits only on channel close, `writer_task.rs:22-31`). Required evidence:
   - A test that is **RED at HEAD** (drain times out / queued capability rows lost) and **GREEN after** the fix, following the lane's proven-red convention.
   - The demo asserts and narrates "audit drain clean in <duration>" as an executed beat; a drain timeout fails the scene.
   - **ZERO kernel-Δ intent**: the fix is expected to live entirely in `maos-bin` (composition-root drop ordering). Any kernel-core line pressure → STOP and reopen preflight via FLAG-Winston (lane rule, sprint-status.yaml:245); never re-pin unilaterally.
5. **AC5 — Truthful narration of known limits + no workstation paths.** The scene explicitly narrates: (a) the cap-token mediation fallback under AC5 host-grant authority, including that the granted path is an Epic-9 operator-policy surface and that a Cedar permit alone cannot green it (citations in Context §2), with the journaled `CapabilityInvocation` exit row referenced; (b) egress `declared-not-enforced` + followup ID; (c) the D4 resume-seam deferral; (d) rung labeling per AC2. No absolute workstation paths appear in scene output or written artifacts — use the `*_SOURCE` (absolute, env-side) / `*_DISPLAY` (repo-relative, rendered) masking convention (`xtask/src/check_multi_tenant_loom.rs:50-56,110-112`); do NOT copy `demo_reza.rs:187-207` (prints an absolute operator-key path).

## Tasks / Subtasks

- [ ] **T0 — BOUNDARY COMMIT — the story's ONLY writes to shared files; lands AFTER 1a commits** (AC4, AC1): one small commit containing (a) the `--once` drain drop-order fix in `crates/maos-bin/src/main.rs` plus its RED-at-HEAD → GREEN proven-red test in a NEW test file (e.g. `crates/maos-bin/tests/drain_once_audit_writer.rs`), and (b) the `demo-j1` module decl + arg-parse + dispatch rows in `xtask/src/main.rs` (mirroring demo-reza) calling a compiling `xtask/src/demo_j1.rs` stub that forwards the raw arg slice. **Sequencing (measured 2026-08-14): j1-crosshost-1a is already in-progress and holds BOTH shared files with uncommitted work — do NOT start T0 until 1a's changes are committed.** Re-derive every line pin at that point: 1a shifts `maos-bin/src/main.rs` by ≈+181 lines at the drain region (drain branch moves from ~4164-4181 to ~4345-4362) and deletes `mod example_spirit_regen;` at `xtask/src/main.rs:108`, immediately adjacent to where `mod demo_j1;` goes. **After T0 this story never writes `crates/maos-bin/src/main.rs` or `xtask/src/main.rs` again — everything else is new-files-only.**
- [ ] **T1 — Scene runner** (AC1): implement `xtask/src/demo_j1.rs` behind the T0 dispatch. Flags `--live-codex`, `--keep-home <dir>`, `--skip-build` are parsed INSIDE `demo_j1.rs` from the forwarded arg slice, so later flag changes never re-touch `xtask/src/main.rs`.
- [ ] **T2 — Provisioning** (AC1): temp-home creation (`XDG_DATA_HOME` + `MAOS_HOME`), binary resolution (`target/debug/maos`, `worker-cli-fixture` as daemon-sibling), `--skip-build` reuse; assert zero `journal:` warnings in the captured stream.
- [ ] **T3 — Subprocess capture + event parse** (AC1): reuse the parsing approach of `journey_j1.rs:38-86` (JSON-per-line over captured stdout; ignore non-JSON banner lines) and the worker assertions of the 8.12 smoke (`child_pid` real, fixture stdout lines — `crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs:14-50` pattern). Parse events by NAME only; never parse topology TOML internals (1a will change `priority_weight`/`host` keys — the demo must not couple to them).
- [ ] **T4 — Beat model + claim table** (AC2, AC5): beat registry with executed vs declared-ABSENT beats, owner story per ABSENT beat, `EvidenceState` strings from `gate_common`, execution-order rendering, ledger consumption via `load_published_ledgers` + `validate_against` when `tests/reports/evidence-ledger-*.json` for J1 gates exist (post-1a `check-j1-loopback-delegation`, extended by 1b — consume, do not create gates here).
- [ ] **T5 — Sealed-export parity check** (AC4): demo-side assertion that `maosctl audit sealed-export` over the demo home covers the run's rows (row-count parity is acceptable secondary evidence) — pure `demo_j1.rs`/test logic; the daemon-side fix itself shipped in T0.
- [ ] **T6 — `--live-codex` leg** (AC3): preflight checks (auth.json refusal, grants file, `CODEX_API_KEY`), disposable demo dir, capture-JSON writer with exact spec fields, `record-capture` → `sealed-export --range` → `verify-bundle` invocation + claim-table row. Never store or echo key material; redact argv.
- [ ] **T7 — Runbook one-pager** (AC1, AC3): `_bmad-output/test-artifacts/runbook-j1-demo.md` — the six-line "how to run it" sheet (default free take; live signed take; what each beat means; where the home/evidence land). One page; link the Tier-2 runbook for the signed path.
- [ ] **T8 — Measurement + disclosures**: remeasure xtask tokei-CODE after the code exists (grant-after-measurement, `kloc.toml:57-65`; active xtask ceiling 37287 @ `kloc.toml:194-210`); record the measured delta in Dev Agent Record. Disclose in this story's record: the five story-file gates skip non-digit `j1-` names (operator-ratified 2026-08-14, sprint-status.yaml:246-250) — record dev model + review evidence anyway.

## Dev Notes

### What exists today vs what arrives later (beat gating)

| Surface | Available | Demo beat |
|---|---|---|
| `maos run <topology> [--live] [--once]` (`crates/maos-bin/src/main.rs:428-486`) | today | scene driver |
| Fixture worker admission + T3 grant + adapter-parsed completion (`main.rs:938-944` `DEFAULT_WORKER_TASK`; `:1284-1306` `worker_completion` println) | today | executed beats |
| `MAOS_LIVE_AGENT` + `MAOS_HOST_GRANTS` live path (`main.rs:797-867,1063-1090`) | today (operator-local) | `--live-codex` |
| `maosctl audit record-capture / sealed-export --range / verify-bundle` (`crates/maos-cli/src/subcommands.rs:1963-2112`; path resolution shared with the daemon) | today | signed take |
| Frame-borne delegation, `assign_frame_remote`, `LoopbackA2ARouter`, intent `development-task:write-workspace`, gate skeleton `check-j1-loopback-delegation` | **j1-crosshost-1a** | ABSENT beat |
| Refusal proofs (-32001 vs -32009 nonconflation), gate extension, PROVEN_BLOCKING legs | **j1-crosshost-1b** | ABSENT beat |
| Two real hosts, mTLS/TOFU, heterogeneous non-Codex worker, task_id correlation, signed reconciliation | **j1-crosshost-2** | ABSENT beat |
| Resume referential identity | **FOLLOWUP-J1-RESUME-SEAM** | ABSENT beat |

- **1a deletes `MAOS_WORKER_TASK`** (const/doc/read/env-registry rows). Do not read or document it in the demo; the fixture's default task is `DEFAULT_WORKER_TASK` (`main.rs:938-944`), and post-1a the task rides the delegation frame.
- `worker_completion` is currently a runner println, not a TL row; the TL linkage is `completion_tl_ref`. Assert on the event + nonempty ref, not on TL row shape.
- Fixture emits 3 stdout lines (first echoes the routed task; terminal line `worker: task complete` — `spirits/worker/src/bin/worker-cli-fixture.rs:25-44`, constants `spirits/worker/src/lib.rs:37-55`).

### Architecture constraints

- **ZERO kernel-Δ intent for the whole story** (xtask + maos-bin only). Kernel-core pressure → FLAG-Winston reopen; lane pin context: 24472 per 1a's record (older 23202 references are stale — verify at HEAD via `cargo run -p xtask -- check-kernel-baseline`).
- No new evidence-ledger gate in this story: gate creation/enrollment (seven surfaces) belongs to 1a/1b. The demo is a *consumer* (validated ledger reads) and a *scene runner* with its own exit discipline.
- Respect `emit_command` stdout/stderr conventions if reusing gate plumbing (`xtask/src/gate_common.rs:388-402`).
- demo-reza is the structural template (`xtask/src/demo_reza.rs:53-130` banner/preflight/scene/gates/summarize flow; ~512 physical lines) — but fix its three known flaws in demo-j1: absolute key-path print (`:187-207`), unvalidated report reads (`:319-410`), BTreeMap-order claim table.
- State-home resolution precedence (`crates/maos-audit/src/lib.rs:852-890,1382-1432`): `MAOS_HOME` → `MAOS_AUDIT_DB`/`MAOS_JOURNAL_PATH` → `XDG_DATA_HOME` → `~/.local/share/maos`. Setting both `MAOS_HOME` and `XDG_DATA_HOME` (journey-harness pattern) covers every consumer, including `maosctl`.

### Previous-story intelligence (j1-tier2 bridge, closed 2026-07-16)

- `codex exec` reads `CODEX_API_KEY` and ignores `OPENAI_API_KEY` (401 otherwise); set `CODEX_API_KEY="$OPENAI_API_KEY"`.
- codex's own sandbox needs unprivileged userns (bwrap); never "fix" with `--sandbox danger-full-access` on a signed run (T3 FS scope is declared-not-enforced at v0.1 — nothing else would bound the demo dir).
- Worker stdin EOF fix landed at `0a03468f`; a hang at "Reading additional input from stdin…" means a stale `maos` binary — rebuild.
- `record-capture` refuses credential-shaped values and control overclaims (egress "enforced" / redaction ≠ "verified") — the capture writer must emit exactly the spec fields.
- Sealed-export signs audit ROWS (one self-contained JSON bundle, embedded Ed25519); journal the capture first, cover with `--range`, verify with the keygen pubkey.

### Project Structure Notes

- New: `xtask/src/demo_j1.rs`; T0-only edits: `xtask/src/main.rs` (dispatch rows) + `crates/maos-bin/src/main.rs` (`--once` drain branch); new test file per T0 (prefer `crates/maos-bin/tests/`); doc: `_bmad-output/test-artifacts/runbook-j1-demo.md`.
- Naming: sprint key `j1-demo-one-command-scene` (keeps the ratified `j1-` lane prefix; "journey-closer" is deliberately NOT in the name — the closer claim belongs to whichever story makes `two-host-signed-run` `PROVEN_LIVE_SIGNED`).

### File-ownership boundary (Option A, operator-ratified 2026-08-14)

- Shared-write files with **j1-crosshost-1a** are exactly two: `crates/maos-bin/src/main.rs` and `xtask/src/main.rs`. This story touches both ONLY in **T0**. **1a holds both files first** (in-progress with uncommitted work as of 2026-08-14; +319/-19 across the two), so T0 queues behind 1a's commit — never interleaved in the same working tree.
- All post-T0 writes are NEW files: `xtask/src/demo_j1.rs`, the T0 proven-red test file, `runbook-j1-demo.md`.
- **Measured region-disjointness (2026-08-14, `git diff -U0` hunk ranges):** 1a's 21 hunks in `maos-bin/src/main.rs` cover ~661-680, 939-955, 1088, 1298, 3150, 3816-3869, 4215-4223, 12769-12938 — **none touch the `--once` drain branch (~4164-4181)**. 1a's entire `xtask/src/main.rs` footprint is a **single deleted line** (108, `mod example_spirit_regen;`). So once sequenced, both T0 edits land in regions 1a never wrote; the collision risk is same-file concurrency, not overlapping regions.
- READ-only touchpoints (never written here): `spirits/topologies/*.toml` (1a rewrites `priority_weight`/`host` keys — demo parses subprocess events by name only), `tests/reports/evidence-ledger-*.json`, `xtask/src/evidence_ledger.rs` + `xtask/src/gate_common.rs` (consumed as library), `discipline.yml` / `tests/coverage-matrix.yaml` (1a/1b enrollment surfaces — the demo adds NO CI step; its test rides the normal cargo-test suite).
- No overlap with **1b** (edits 1a's gate module + CI files — none of ours) or **crosshost-2** (far rung; demo only reads its future ledger).
- If a post-T0 need to edit a shared file ever emerges: STOP, sequence with 1a (never interleave), and note it in the Dev Agent Record.

### References

- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml:238-253] — bridge-lane charter, 1a/1b split (2026-08-14), crosshost-2 scope, disclosure conventions.
- [Source: _bmad-output/implementation-artifacts/spec-j1-tier-2-live-agent-demonstration.md:45-151] — T1–T6 ladder, capture fields, Tier definitions, D4 deferral.
- [Source: _bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md] — the signed-run procedure `--live-codex` automates (Phases 3–5) + abort conditions.
- [Source: _bmad-output/implementation-artifacts/j1-crosshost-1a-frame-borne-delegation.md:71-318; j1-crosshost-1b-consent-proofs-and-gate.md:46-205] — arriving surfaces, gate names, refusal codes, disclosures.
- [Source: xtask/src/demo_reza.rs; xtask/src/evidence_ledger.rs:149-278,739-884,1213-1480; xtask/src/gate_common.rs:77-216] — scene + ledger machinery to mirror/consume.
- [Source: crates/maos-journey-test/tests/journey_j1.rs:38-86,131-214] — isolation pattern + D4 seam status.
- [Source: crates/maos-bin/src/main.rs:4164-4181,2607-2613; crates/maos-kernel-core/src/memory/mod.rs:103-105,164-170; writer_task.rs:22-31] — drain-timeout root-cause chain (verify line pins at HEAD before editing).
- [Source: crates/maos-bin/src/main.rs:949-1020,1132-1166,199-242; crates/maos-kernel-core/src/capability/cap_policy/mod.rs:127-149; crates/maos-kernel-core/src/security/mod.rs:338-360; deferred-work.md:39-40] — cap-token fallback is designed pending Epic-9 (narrate, don't fix here).
- [Source: _bmad-output/implementation-artifacts/8-14a-j0-evaluator-surface-and-runtime-cli.md:92-94,129-151] — narration + subprocess-isolation conventions.

## Dev Agent Record

### Agent Model Used

_(to be filled by dev-story)_

### Debug Log References

### Completion Notes List

### File List
