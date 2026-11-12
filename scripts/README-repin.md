# `scripts/repin.py` — one-command re-pin of the line-count pins

Operator ruling `repin` (2026-11-12). After the code is final and `cargo fmt --all`
has run, one command replaces the hand-edited pin set of every kernel/crate landing.
Python 3, standard library only. It **measures with the prebuilt xtask gates and never
runs cargo**, never touches `docs/ci-baselines/*` or `docs/invariants`, and never
reformats a line it does not own (every replacement is exactly-once or the run aborts;
lines longer than 768 characters are handled).

```
scripts/repin.py --check
scripts/repin.py --apply --story 20-13 --note "AC7 KT3: SecurityManagerAdapter carries the selected crypto provider" \
                 [--adjudicated maos-bin[,crate...]] [--lower CRATE] [--base REF | --base-tree DIR] \
                 [--root DIR] [--xtask PATH]
```

The xtask binary is found via `--xtask`, `$REPIN_XTASK`, `$CARGO_TARGET_DIR/{debug,release}/xtask`,
then `<root>/target/{debug,release}/xtask`; if none exists the script stops (exit 3) rather than build.
`tokei` 14.0.0 must be on `PATH` (the same requirement as `kloc-check`).

## What it measures and pins

| Gate it runs | Measured | Pin location it rewrites |
|---|---|---|
| `check-kernel-baseline --json` | kernel physical lines | `kernel-core-baseline.toml` `src_lines`; the four `src_lines` constants in `xtask/tests/kernel_pin_content_hash_16_0.rs` |
| `check-kernel-baseline --emit-pin` | `set_hash`, per-file digests | `[kernel_src] set_hash`, changed `[kernel_src.files]` digests; one HISTORY comment block is inserted **after** `src_lines` (the line number 505 is pinned by a test and does not move) |
| `kloc-check --json` | per-crate tokei, aggregate | each exact-pinned `xtask/kloc.toml` row; `_aggregate_hardfail` = **measured aggregate + 1**; `RATIFIED_AT_EASING` in `xtask/tests/recovery_lane_ceiling_rule.rs` follows the kernel row |
| `check-service-boundary --json` | public-surface diff | nothing (read-only; see refusals) |

Every changed `kloc.toml` row gets a new first segment
`HISTORY: <story-key> -> exact formatted cargo fmt --all / kloc-check --json: OLD -> NEW (±d)…; <note>`
followed by `| PRIOR: <the previous comment>`, so the d11 ratchet
(`xtask/tests/d11_xtask_ceiling_ratchet.rs`: first segment cites a live story key, first arrow is the
exact grant) holds; the script re-checks that contract on the text it wrote before writing.

**Which rows move.** A row is lowered/raised to the measured value only if it was *exact* before the
landing (pin == measured in the pre-landing tree, `--base`, default `HEAD`; the kernel row is always
exact). Reserve rows (pin > measured, e.g. `maos-control`, `maos-spirit-abi`) are left alone unless you
pass `--lower CRATE`. Without a git checkout pass `--base-tree DIR` (a directory with the pre-landing
`crates/` and `xtask/`).

## `--check`

Prints one table line per pin location (pin vs measured) and exits `0` in sync, `1` drift,
`2` a human decision is pending (listed), `3` a gate could not be run. A clean tree must be `0`.

## `--apply`

Exit `0` applied / nothing to do, `2` **refused (nothing written)**, `1` post-write verification failed
(`check-kernel-baseline` and `kloc-check` are re-run; all files are restored), `3` tool error.
It prints a unified diff (very long lines are truncated for display only) and is idempotent: the second
`--apply` is a no-op. Needs `--story` (full key or unique prefix, must be `in-progress`, `review` or
`ready-for-dev` in `_bmad-output/implementation-artifacts/sprint-status.yaml`; epic rows and `done`
stories are refused) and `--note` (the narrative; no `|`).

### Refused for a human (exit 2)

* a crate **over its ceiling** — the overdraft is printed (row, measured, pre-landing measured); the
  orchestrator adjudicates, then re-run with `--adjudicated CRATE` (the HISTORY segment records it);
* a **surface-hash change** or any red `check-service-boundary` — the symbols are printed; the operator
  grants and a human refreshes `docs/ci-baselines/kernel-surface-v0.1-beta.json`, then re-run;
* a **new or removed kernel file** (moves `PINNED_ENTRIES`; pin by hand with `--emit-pin`);
* a **story key** that is missing, an epic, not live, or a `--note` containing `|`;
* a measured crate with **no `kloc.toml` row**;
* a kernel tokei above the ADR-069 alarm 22 000 unless `--note` carries `ENVELOPE ALARM` (the
  operator's acknowledgement), or above the 23 000 envelope.

## Proof against real landings

`scripts/repin_replay.py SHA --story KEY [--adjudicated A,B]` extracts `SHA~1`, overlays that landing's
source (`git diff SHA~1 SHA -- crates xtask/src`, no pin files), copies the landing's human-owned
surface baseline, runs `repin.py --apply`, and diffs the four pin files against the landing commit with
prose normalised: pins, hashes and digests must be identical.

```
scripts/repin_replay.py 998b36e6 --story 20-13 --adjudicated maos-bin
scripts/repin_replay.py 692b1e59 --story 20-8  --adjudicated maos-kernel-core,maos-domain,maos-bin
```

## Landing procedure

1. final code, `cargo fmt --all`, rebuild xtask if it changed (the binary must match the tree);
2. `scripts/repin.py --check` — resolve every listed refusal (adjudication, surface refresh);
3. `scripts/repin.py --apply --story KEY --note "…" [--adjudicated …]`;
4. `scripts/repin.py --check` (rc 0) and commit.
