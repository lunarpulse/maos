# 20-11 landing 1 — T0 STOP

Date: 2026-11-04. Agent: Dev2011 (`openai-codex/gpt-6.1-sol`).
Baseline: `73e7afbcae81f4fb4858e9d52ded3d592bed636d` (initial tracked tree clean).

**Subsequent ruling (2026-11-04):** ask `halt_grant` grants O4 (D-20-11-11 A) and shared measured STOP
21250; ask `halt_landing` moves AC3 to the final kernel KS + O4 landing. The initial STOP disposition below is
historical, not an outstanding request for a ruling. L1 AC1, AC2, AC4–AC7 may resume kernel-free when the
orchestrator grants the cargo slot. The observed red and original command outputs remain unchanged.

## Disposition

**STOP required by creation measurement 2, T0 and D-20-11-03 B.** The suspected second unreceipted unload
drain is real. No L1 runtime implementation, kernel edit, pin update, status transition or acceptance closure was
made. D-20-11-11 is PROPOSED; the existing KS grant does not authorize this repair. The cargo slot was released
after the one witness run; there was no retry-to-green.

Missing prerequisite: an operator ruling on D-20-11-11, including an explicitly measured/granted kernel repair
and the revised landing dependency, or an independently granted repair landing first. Reordering kernel work or
inventing a second receipt writer would violate the assignment.

## Re-measured cites at HEAD

| Source | Observation |
|---|---|
| `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:531-593` | Real unload: `Unloaded` transition at 542–545; `terminate_spirit` at 562–569; second unreceipted drain at 574; SCB removal at 577. |
| `crates/maos-kernel-core/src/halt/termination.rs:26-122` | Single receipt writer drains once at 39; its subsequent writes iterate only that captured set, or write a synthetic `term-…` receipt. |
| `crates/maos-kernel-core/src/halt/mod.rs:523-585` | Real invocation writes TL, then Journal, then registry metadata; it has no scheduler/SCB lifecycle admission fence. |
| `crates/maos-kernel-core/src/halt/mod.rs:347-381` | Drain removes and returns the matching registry entries; this function does not write receipts. |
| `crates/maos-iac/src/adapter/transparency_log.rs:1269-1275` | Existing redaction-policy callback runs before acquiring the TL lock, enabling a deterministic real concurrent writer without modifying production code. |
| `crates/maos-bin/src/shell_host.rs:131-153` | Production halt caller directly invokes `invoke_halt`; it does not consult SCB lifecycle state. |
| `deferred-work.md:1396` | Existing R3 STOP-risk upgraded to confirmed; owner remains 20-11. The supplied pid-reuse cite was not used to claim closure. |

The complete story (including long D-row cells), creation measurement requirements and records-owed were read.
T0 measurements 1 and 3–7, remaining implementation cite remeasurement and inventory are not claimed complete:
measurement 2 triggered the explicit STOP before implementation. The purge-proof-retention observation remains
routed to 20-10 under creation records-owed §C R1; no purge behavior was changed.

## Real failing-before witness

Source: [t0-unload-race-witness.rs.txt](t0-unload-race-witness.rs.txt).
Executed temporarily as `crates/maos-bin/tests/erasure_unload_race_20_11.rs`, then removed from the normal test
suite after preserving the exact formatted source here.
The evidence-only `.rs.txt` suffix keeps the preserved witness outside production Rust/KLOC scanning;
the source bytes are unchanged by this rename.

The harness loads a real SCB. The existing TL redaction-policy seam pauses the first `PlannedUnload` receipt
serialization after `terminate_spirit` has drained, before the second drain can execute. A concurrent OS thread:

1. Asserts the real SCB is already `Unloaded` (source lines 130–134).
2. Calls real `invoke_halt` for `late-20-11`, committing TL and Lifecycle Journal rows (144–153).
3. Checks that halt's pending metadata exists before resuming unload (154–157).

The real unload returns success (159); durable read-back proves the invocation remains (177–180), registry
metadata is empty (181–184), and the SCB has been removed (185). The terminal receipt assertion (187–190)
then fails. Thus an empty post-unload registry is not evidence that all pending halts were receipted.

Observed output, [full log:589](t0-unload-race-cargo-test.log#L589):

```text
Unloaded SCB accepted late-20-11; durable invocation, registry empty, SCB removed; receipt_ids=["term-race-20-11-1-1793782986345734392"]
unload discarded late-20-11 without a terminal receipt
```

No production plant, mock receipt writer, simulated drain or kernel edit was used. The policy delegates normal
redaction to `CorpusBackedRedactionPolicy`; only the interleaving is controlled. This establishes the permitted
interleaving, not its natural occurrence frequency.

## AC witness table

| AC / witness (`name:line`) | Before / plant | Final |
|---|---|---|
| AC3 prerequisite: `t0_unload_must_receipt_halt_invoked_after_termination_drain:48`, receipt oracle `:187` | **Observed red** at HEAD, cargo exit 101; all preceding real-state assertions passed. No plant needed for T0 STOP. | Blocked: cannot claim AC3 with D-03 B's pre-read/post-empty check. |
| AC1, AC2 | Not run: stopped at T0 before replacing constants, changing proof schema or touching lineage. | Open; no final or plant claimed. |
| AC4 | Not run: installed-package and schedule witnesses not implemented. KS remains kernel-last and unlanded. | Open; no final or plant claimed. |
| AC5 | Not run: large-log calibration, hold serialization and window plant not implemented. | EC-13 remains open. |
| AC6 | Hosted T3 run not started. | Hosted-only leg unverified. |
| AC7 | Inventory typed-list witness/plant not implemented. | Open. |
| AC8 | STOP evidence, Dev Agent Record, new proposed D-row, grant boundary and existing deferred owner recorded. | Partial records only; no closure, coverage-matrix, ADR or pin claim. |

## Exact baseline measurements versus pins

Prebuilt gates, each run once before the witness; raw JSON retained alongside this report. No charged source
remains changed, so no exact re-pin or HISTORY row was added. The measurements are baseline results, not a
fictional completed L1 landing.

| Row | Measured | Exact current pin | Story STOP ceiling / disposition |
|---|---:|---:|---|
| Kernel physical | 26841 | 26841 | 26861, KS only; 95 files, unchanged set hash. |
| Kernel tokei | 21042 | 21042 | 21054, KS only. |
| maos-bin | 34504 | 34504 | Assignment re-base: 35704 = 34504 + original 1200 allowance. No growth. |
| maos-iac | 8270 | 8270 | Frontmatter 8630. |
| maos-audit | 7721 | 7721 | Frontmatter 8261. |
| maos-registry | 4248 | 4248 | Frontmatter 4368. |
| maos-providers | 1679 | 1679 | Untouched. |
| maos-domain | 10107 | 10107 | Untouched. |
| xtask | 44452 | 44452 | Untouched. |
| Aggregate | 193140 | hardfail 193141 | Frontmatter 194058 unchanged; no allowance consumed or re-base needed for STOP. |

Kernel set hash: `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`;
changed/added/removed kernel files all empty. KLOC `over_budget=[]`; aggregate alarm remains true as at baseline.

## Observed commands and checks

[commands.json](commands.json) records exact commands and exit codes. Commands using the prebuilt gate binary
set `CARGO_MANIFEST_DIR` and unset CI/GITHUB_ACTIONS as prescribed; no cargo target override was set.
The Bash environment inherited `CARGO_TARGET_DIR=/mnt/build/cargo-maos` (observed via `printenv`).

| Command / gate | Exit | Observed result |
|---|---:|---|
| `git status --short && git rev-parse HEAD` | 0 | Initial tree clean; HEAD above. |
| `rustfmt --edition 2021 crates/maos-bin/tests/erasure_unload_race_20_11.rs` | 0 | Formatted witness before its only run. Not a workspace fmt claim. |
| `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --test erasure_unload_race_20_11 -- --nocapture` | **101** | Compiled successfully; 0 passed / 1 failed in 0.29 s. Intended terminal-receipt assertion fails at line 187. |
| `check-kernel-baseline --json` | 0 | Exact unchanged baseline above. |
| `kloc-check --json` | 0 | Aggregate 193140; no over-budget crates. |
| `check-service-boundary --json` | 0 | No violations. |
| `check-dev-record-completeness --json` | 0 | 197 done stories; 178 deferred-owner assertions; 0 violations. |
| `check-epic-close-coherence --json` | 0 | 21 epics; authoritative physical kernel pin 26841. |
| `check-decision-register --json` | 0 | 23 epic-14 register rows / 6 open; no findings. This gate is not an adjudication of D-20-11-11. |
| `check-exit-commands --json` | 0 | 2 blocks / 10 tokens; no findings. |

The structural gates ran once after the story/deferred/decision record edits. They do not prove any runtime AC.
The auxiliary skill customization resolver exited 0. An Eval measurement attempt initially found no
`CARGO_TARGET_DIR` in the Python kernel environment and stopped before running either gate; recovery used the
known absolute prebuilt binary. The actual cargo witness used Bash's inherited target environment.

**Not run after STOP:** prototype compilation beyond the witness's actual dependencies, affected-binary build,
full workspace `cargo fmt --all`, clippy delta, fixture rebuild, workspace nextest, prepush, hosted discipline or
hosted T3 legs. No verification run is borrowed from an ancestor or another story, and no failing witness was
rerun. Prepush is not claimed: this was a mandatory pre-implementation STOP, not a landing.

## Cleanup

[cleanup.json](cleanup.json) records the /proc scan and exact removed incremental session paths.
No cargo/rustc process was observed before pruning. Only stale `debug/incremental/*/s-*` session directories
were pruned; newest per crate artifact directory and every session with a descendant touched within two hours
were retained. `deps`, `build`, fingerprints and other worktrees were not touched.

Freed allocated bytes: **10136645632**. No named `/tmp` scratch was created; the witness's `TempDir` is scoped
to the test and drops on assertion unwind. The temporary normal-suite integration target was removed; its
formatted source and complete failure output remain only as requested evidence.
