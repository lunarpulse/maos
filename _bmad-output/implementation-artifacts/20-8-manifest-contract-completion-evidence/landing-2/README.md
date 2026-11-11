# 20-8 landing 2 — AC8 swap-invariant refusal (FLAG-Winston KS, final kernel landing)

Dev208K, `anthropic/claude-sonnet-5-5`, base `608735d2` (the 20-1b L2 kernel landing, kernel 26967 / 21141), 2026-11-11,
uncommitted. Decision record: `../../20-8-flag-winston-decision.md` (§L2). Story record: `../../20-8-manifest-contract-completion.md`
(AC8 "LANDED", Dev Agent Record). Exact commands, exits and timings: `commands.md`.

| Directory | Contents |
|---|---|
| `t0/` | `t0-service-boundary-before-surface-refresh.json` (the T0 prototype's gate run: the one flagged symbol, `SpiritManifestBundle`), `emit-pin-after.txt` (`check-kernel-baseline --emit-pin` on the clean overlay; set hash and the three digests) |
| `red-head/` | failing-before: `red-head-kernel-compile.log` (pristine HEAD: the test names a field and a variant HEAD lacks, E0609/E0599) and `red-head-kernel-behavioral.log` (field + parser + variant, NO coordinator check: the drop vector and the undeclared vector `Ok(Completed)`, exit 101) |
| `green/` | the kernel witnesses green on the landing tree (3/3), the genuine `maos-bin`/`maosctl` build, the door witness green |
| `plants/` | four plants: unified diff, raw log, `plants.json` (exit, seconds, restore `cmp`); each restored by copy-back + `cmp` (rc 0) + `touch` |
| `diff/` | `kernel-ks-landing.diff` (kernel + domain source), `bin-door-mapping.diff` (bin source) |
| `gates/` | gate JSON/stderr on the final tree, `changed-files.txt`, the clean-overlay `kloc-check`/`check-service-boundary` |
| `final/` | clippy stderr + ratchet, kernel `--lib`, focused kernel suite, workspace nextest |

## Witness table (AC8)

| AC8 item | Witness | Red before | Plant | Final |
|---|---|---|---|---|
| 8.2 drop ⇒ typed refusal naming `review_queue`, predecessor restored and Running, `on_swap_out`/`snapshot` not fired, journaled + receipted | `swap_invariants_20_8.rs::a_successor_that_drops_a_declared_invariant_is_refused_before_any_hook` | `red-head/*` | `plants/plant-kernel-check-dropped` (red) | `green/green-kernel-swap-invariants.log` |
| 8.2 undeclared / pre-schema-7 successor drops all (schema rule, as the gate) | `…::an_undeclared_or_pre_schema_7_successor_drops_every_invariant` | `red-head/*` | `plants/plant-schema7-filter-dropped`, `plant-kernel-check-dropped` (red) | same |
| 8.2 superset swaps, SCB carries the successor's declaration; a predecessor declaring nothing swaps freely | `…::a_superset_successor_swaps_and_the_scb_carries_its_declaration` | (compile-red) | n/a (guards over-refusal) | same |
| 8.1 door maps to `409 swap_invariant_dropped` | `one_daemon_one_door_16_1.rs::ac8_20_8_a_dropped_swap_invariant_is_a_typed_409_and_the_predecessor_keeps_serving` | via `plants/plant-door-kernel-check-dropped` | `plant-door-mapping-dropped`, `plant-door-kernel-check-dropped` (red) | `green/green-door.log` |

Retained caveats (not hidden):

- The red at HEAD is two-stage: HEAD has no `swap_invariants` field to declare on, so the test cannot compile there (`red-head-kernel-compile.log`);
  the behavioural red is the tree with the carriage but without the check (`red-head-kernel-behavioral.log`), where the drop vector completes the swap.
  The superset vector passes there (carriage only); it guards over-refusal, it is not a control.
- The door witness was not run red at pristine HEAD; its reds are the two plants (kernel check removed ⇒ upgrade exits 0; door mapping removed ⇒ code `upgrade_failed`).
- Not witnessed: the spawned (WASM component) hop's refusal, the dry-run `precheck` verdict (deferred rows), the hosted legs.
- The behavioural red ran before `cargo fmt --all`; formatting changed no line of the landing (`git status` identical before/after).
