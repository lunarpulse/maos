# `scripts/repin.py` — the kernel exact pin per landing (story-gated)

Operator ruling `repin` (2026-11-12), shrunk by SCP 2026-11-12 §5.B7c option A (ruled 2026-11-13) and by operator
decision 11 (lean-gates WP-23, confirmed 2026-11-24). Python 3, standard library only. It **measures with the prebuilt
xtask gates and never runs cargo**, never touches `docs/ci-baselines/*` or `docs/invariants`, and never reformats a line
it does not own (every replacement is exactly-once or the run aborts; lines longer than 768 characters are handled).

`cargo run -p xtask -- bless` regenerates the same kernel pin (and `STABILITY.md`, `docs/errors/error-catalog.json`, the
ABI reference pages and the example Spirits) from their sources in one command, without the story gate and without the
refusals below. This script is the guarded path for a landing that has a live story.

| Mechanism | What it is | When it moves | Command |
|---|---|---|---|
| **Kernel exact pin** | `xtask/kernel-core-baseline.toml` (`src_lines`, `set_hash`, per-file digests); anti-drift | **every landing that changes kernel source** | `--check` / `--apply`, or `xtask bless` |
| **Kernel envelope** | the `maos-kernel-core` row of `xtask/kloc.toml`; anti-growth (ADR-069: row ≤ 24,000, alarm 23,000) | only by a human, with the row's `ENVELOPE ALARM` marker above 23,000 | never written by this script |

There are no other `kloc.toml` rows: the per-crate ceilings and `_aggregate_alarm` were removed, and `xtask kloc-check`
prints a size table for every crate instead. There is no `--rebase`.

```
scripts/repin.py --check
scripts/repin.py --apply --story 20-13 [--note "TEXT"] \
                 [--root DIR] [--xtask PATH]
```

The xtask binary is found via `--xtask`, `$REPIN_XTASK`, `$CARGO_TARGET_DIR/{debug,release}/xtask`,
then `<root>/target/{debug,release}/xtask`; if none exists the script stops (exit 3) rather than build.
`tokei` 14.0.0 must be on `PATH` (the same requirement as `kloc-check`).

## Kernel exact pin: `--check` / `--apply`

| Gate it runs | Measured | Pin location it rewrites |
|---|---|---|
| `check-kernel-baseline --json` | kernel physical lines | `kernel-core-baseline.toml` `src_lines` (the test file `kernel_pin_content_hash_16_0.rs` holds no copy: it asserts the readers agree with `read_pinned`) |
| `check-kernel-baseline --emit-pin` | `set_hash`, per-file digests | `[kernel_src] set_hash`, changed `[kernel_src.files]` digests (values only; no HISTORY block is written) |
| `kloc-check --json` | kernel tokei | nothing (read-only: checked against the `maos-kernel-core` row and the ADR-069 alarm/envelope) |
| `check-service-boundary --json` | public-surface diff | nothing (read-only; see refusals) |

`--check` prints one table line per pin location (pin vs measured) and exits `0` in sync, `1` drift, `2` a human
decision is pending (listed), `3` a gate could not be run. A clean tree must be `0`.

`--apply` exits `0` applied / nothing to do, `2` **refused (nothing written)**, `1` post-write verification failed
(`check-kernel-baseline` is re-run; all files are restored), `3` tool error. It prints a unified diff (very long
lines are truncated for display only) and is idempotent: the second `--apply` is a no-op. Needs `--story` (full key
or unique prefix, must be `in-progress`, `review` or `ready-for-dev` in
`_bmad-output/implementation-artifacts/sprint-status.yaml`; epic rows and `done` stories are refused). `--note` is
optional free text printed to stdout only; no comment is written to any file.

### Refused for a human (exit 2)

* the kernel **over its `kloc.toml` row** — raising the row is a human edit inside the ADR-069 envelope;
* a **surface-hash change** or any red `check-service-boundary` — the symbols are printed; the operator grants and a
  human refreshes `docs/ci-baselines/kernel-surface-v0.1-beta.json`, then re-run;
* a **new or removed kernel file** (moves `PINNED_ENTRIES`; pin with `xtask bless` after the charter decision);
* a **story key** that is missing, an epic, or not live;
* a kernel tokei above the ADR-069 alarm 23 000 unless the `maos-kernel-core` row comment in `kloc.toml` already
  carries `ENVELOPE ALARM` (the operator's acknowledgement; add it to the row first, a `--note` does **not**
  count because `--apply` writes no `kloc.toml` and the retained test reads the row), or above the 24 000 envelope.

## Landing procedure (kernel source changed)

1. final code, `cargo fmt --all`, rebuild xtask if it changed (the binary must match the tree);
2. `scripts/repin.py --check` — resolve every listed refusal (kernel grant, surface refresh);
3. `scripts/repin.py --apply --story KEY` (history is the commit message; `--note` is optional and stdout-only), or
   `cargo run -p xtask -- bless` when no story gate applies;
4. `scripts/repin.py --check` (rc 0) and commit. A landing that does not change kernel source needs no re-pin at all.
