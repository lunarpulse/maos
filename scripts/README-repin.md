# `scripts/repin.py` — the kernel exact pin per landing, the crate ceilings per retrospective

Operator ruling `repin` (2026-11-12), shrunk by SCP 2026-11-12 §5.B7c option A (ruled 2026-11-13).
Python 3, standard library only. It **measures with the prebuilt xtask gates and never runs cargo**,
never touches `docs/ci-baselines/*` or `docs/invariants`, and never reformats a line it does not own
(every replacement is exactly-once or the run aborts; lines longer than 768 characters are handled).

There are two different mechanisms and the script keeps them apart:

| Mechanism | What it is | When it moves | Command |
|---|---|---|---|
| **Kernel exact pin** | `xtask/kernel-core-baseline.toml` (`src_lines`, `set_hash`, per-file digests, HISTORY block) and the four `src_lines` constants in `xtask/tests/kernel_pin_content_hash_16_0.rs`; anti-drift | **every landing that changes kernel source** | `--check` / `--apply` |
| **Crate ceilings** | the `xtask/kloc.toml` crate rows: ceilings **with stated headroom** | **only at an epic retrospective** (and the B7c-A landing itself); never per landing | `--rebase` |

A landing does **not** re-pin any `kloc.toml` row, does not write a per-landing `kloc.toml` HISTORY segment, and
there is no `_aggregate_hardfail` (retired; `_aggregate_alarm` is a warning only). The d11 ceiling ratchet and the
per-landing `RATIFIED_AT_EASING` figure are retired too.

```
scripts/repin.py --check
scripts/repin.py --apply --story 20-13 --note "AC7 KT3: SecurityManagerAdapter carries the selected crypto provider" \
                 [--root DIR] [--xtask PATH]
scripts/repin.py --rebase --note "Epic-21 retrospective ceiling re-base" [--measure-root DIR | --measure-json FILE] \
                 [--lower-kernel] [--date YYYY-MM-DD] [--dry-run] [--root DIR] [--xtask PATH]
```

The xtask binary is found via `--xtask`, `$REPIN_XTASK`, `$CARGO_TARGET_DIR/{debug,release}/xtask`,
then `<root>/target/{debug,release}/xtask`; if none exists the script stops (exit 3) rather than build.
`tokei` 14.0.0 must be on `PATH` (the same requirement as `kloc-check`).

## Kernel exact pin: `--check` / `--apply`

| Gate it runs | Measured | Pin location it rewrites |
|---|---|---|
| `check-kernel-baseline --json` | kernel physical lines | `kernel-core-baseline.toml` `src_lines`; the four `src_lines` constants in `xtask/tests/kernel_pin_content_hash_16_0.rs` |
| `check-kernel-baseline --emit-pin` | `set_hash`, per-file digests | `[kernel_src] set_hash`, changed `[kernel_src.files]` digests; one HISTORY comment block is inserted **after** `src_lines` (the line number 505 is pinned by a test and does not move) |
| `kloc-check --json` | kernel tokei | nothing (read-only: checked against the `maos-kernel-core` ceiling row and the ADR-069 alarm/envelope) |
| `check-service-boundary --json` | public-surface diff | nothing (read-only; see refusals) |

`--check` prints one table line per pin location (pin vs measured) and exits `0` in sync, `1` drift, `2` a human
decision is pending (listed), `3` a gate could not be run. A clean tree must be `0`.

`--apply` exits `0` applied / nothing to do, `2` **refused (nothing written)**, `1` post-write verification failed
(`check-kernel-baseline` is re-run; all files are restored), `3` tool error. It prints a unified diff (very long
lines are truncated for display only) and is idempotent: the second `--apply` is a no-op. Needs `--story` (full key
or unique prefix, must be `in-progress`, `review` or `ready-for-dev` in
`_bmad-output/implementation-artifacts/sprint-status.yaml`; epic rows and `done` stories are refused) and `--note`
(the narrative cited by the HISTORY block).

### Refused for a human (exit 2)

* the kernel **over its `kloc.toml` ceiling** — kernel growth needs its own measured FLAG-Winston grant on the row;
* a **surface-hash change** or any red `check-service-boundary` — the symbols are printed; the operator grants and a
  human refreshes `docs/ci-baselines/kernel-surface-v0.1-beta.json`, then re-run;
* a **new or removed kernel file** (moves `PINNED_ENTRIES`; pin by hand with `--emit-pin`);
* a **story key** that is missing, an epic, or not live;
* a kernel tokei above the ADR-069 alarm 22 000 unless the `maos-kernel-core` row comment in `kloc.toml` already
  carries `ENVELOPE ALARM` (the operator's acknowledgement; add it to the row first, a `--note` does **not**
  count because `--apply` writes no `kloc.toml` and the retained test reads the row), or above the 23 000 envelope.

## Crate ceilings: `--rebase` (retrospectives and the B7c-A landing only)

```
scripts/repin.py --rebase --note "REASON" [--measure-root DIR | --measure-json FILE] [--lower-kernel] [--date YYYY-MM-DD] [--dry-run]
```

1. Per-crate tokei comes from the prebuilt `xtask kloc-check --json` run in `--measure-root` (default `--root`), or
   from a saved report with `--measure-json FILE` (see "The B7c-A landing re-base"; the two are mutually exclusive).
2. Rewrites **every crate row** of `--root`'s `xtask/kloc.toml` to `measured + max(100, ceil(0.02 * measured))`,
   never below what `--root` itself measures (when `--measure-root` or `--measure-json` is given, `--root` is
   measured too with `--xtask` and acts as a floor). `_aggregate_*` rows are not crate rows and are untouched; a
   crate with no code at all (the phantom `= 0` tripwire rows) is kept.
3. Comments stay intact. Each *changed* row gets exactly one `# RE-BASED <date> (<reason>)` segment at the head of its
   inline comment (`… = N   # RE-BASED 2026-11-13 (reason) | <previous comment>`); a re-base replaces the previous
   `RE-BASED` segment instead of stacking them. `--note` is the reason (no `|`); `--date` defaults to today.
4. `maos-kernel-core` is a **zero-headroom crate** (`ZERO_HEADROOM_CRATES` in `recovery_lane_ceiling_rule.rs`): its
   row is **held** (`held (zero headroom)` in the table), never raised. `--lower-kernel` lowers it to its measured
   value. It is refused (exit 2) if the kernel measures above its row, if the resulting row would exceed the ADR-069
   envelope 23 000, or if it sits above the alarm 22 000 without `ENVELOPE ALARM` in the row comment (add it to the
   `maos-kernel-core` row of `kloc.toml` first; the retained test reads it there).
5. Prints the before/after table (`CRATE BEFORE MEASURED ROOT-FLOOR AFTER STATUS`) and the diff, writes the file
   atomically, re-runs `kloc-check --json` and restores the file (exit 1) if it is not green. `--dry-run` prints the
   table and writes nothing. A measured crate with no row is refused (a new row is a budget decision).

The kernel exact pin is not touched by `--rebase`.

## Landing procedure (kernel source changed)

1. final code, `cargo fmt --all`, rebuild xtask if it changed (the binary must match the tree);
2. `scripts/repin.py --check` — resolve every listed refusal (kernel grant, surface refresh);
3. `scripts/repin.py --apply --story KEY --note "…"`;
4. `scripts/repin.py --check` (rc 0) and commit. A landing that does not change kernel source needs no re-pin at all.

## Retrospective procedure

1. `scripts/repin.py --rebase --note "Epic-N retrospective: <ruling>" --dry-run`, review the table;
2. run it without `--dry-run`, then `scripts/repin.py --check` (the kernel pin must still be in sync);
3. produce the *Retrospective size trend table* (`docs/testing/proof-contract.md`) and commit it with the re-base.

The retrospective re-base uses the default measure source (`--root` itself): no `--measure-root` / `--measure-json`
is needed, and the `--root` tree's own xtask and `kloc.toml` measure and verify.

## The B7c-A landing re-base (measuring a batch tree)

B7c-A's initial ceilings are measured on the **batch tree** (the tree carrying the work being landed), but the
B7c-A tree's xtask gate refuses a `kloc.toml` that still carries the retired `_aggregate_hardfail`. So
`--measure-root <batch tree>` fails with a tool error (exit 3) unless that tree already has B7c-A's configuration.
Measure with the batch tree's **own** xtask instead and hand the report over as a file:

```
# 1. in the batch tree, with the batch tree's own prebuilt xtask (its own gate accepts its own kloc.toml);
#    write the report to an ABSOLUTE path both trees can read:
<batch xtask> kloc-check --json > /tmp/batch-kloc.json
# 2. in the B7c-A tree, with the same absolute path:
scripts/repin.py --rebase --measure-json /tmp/batch-kloc.json --root <b7c tree> --xtask <b7c xtask> \
                 --note "B7c-A landing: initial ceiling re-base" [--dry-run]
```

`--measure-json FILE` (exclusive with `--measure-root`) reads the per-crate tokei from the report's `per_crate` map.
Extra keys are ignored and a `"passed": false` report (the batch tree over some budget) is accepted, since only the
measurement matters; a file that is unreadable, is not JSON, or has no usable `per_crate` map is refused (exit 3).
`--root` is still measured with `--xtask` and is the floor of every row, and the post-write verification still runs
`kloc-check --json` on `--root`, so the rewritten rows must be green under the B7c-A gate.
