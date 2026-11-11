# 20-1b landing 2 — final kernel landing (row R = KQ1 + P) and the AC8 quarantine effect

Dev201bK, `anthropic/claude-sonnet-5-5`, base `c2beedb4`, 2026-11-11, uncommitted. Decision record:
`../../20-1b-flag-winston-decision.md`. Story record: `../../20-1b-vetting-yank-policy-and-quarantine.md`
§"Landing 2". Exact commands, exits and timings: `commands.md`.

| Directory | Contents |
|---|---|
| `t0/` | `t0-service-boundary-before-surface-refresh.json` (the one flagged symbol, `CapabilityRegistryAdapter`, on the T0 copy), `sb-new.json` (current surface used to take the new hash) |
| `red-head/` | failing-before logs at HEAD (kernel: 4 new vectors + the updated existing action test; bin: AC8 policy matrix and the AC8/hostile vector; AC12 poller with 12 repeated lines) and the genuine HEAD build |
| `green/` | the same witnesses green on the landing tree, kernel `--lib` 253/253, the genuine build |
| `plants/` | six plants: unified diff, raw log, `plants.json` (exit, seconds, restore `cmp`); each restored by copy-back + `cmp` + `touch` |
| `diff/` | `kernel-kq1-p-landing.diff` (the 4 kernel source files), `bin-ac8-effect.diff` |
| `gates/` | gate JSON/stderr, the AC13 negative control (`neg-control-record-completeness.*`), `changed-files.txt` |

Retained caveats (not hidden):

- `red-head/red-head-bin-policy.log` carries four `PoisonError` cascade failures from the shared `ENV` mutex after the
  first (real) red; the policy-matrix red is the separate `red-head-bin-policy-ac8-warn-quarantine-crl.log`.
- Red-run line numbers refer to the test files before `cargo fmt --all`; names are stable, final lines are in the story.
- The first kernel red run was a single combined test; it was split into four vectors and re-run before the retained log.
- AC11.5 (hosted T3 container leg) was **not run**. Quarantine persistence across a process restart is **not delivered**
  (in-memory pid fence, KQ1 as ruled); see the deferred-work section "20-1b landing 2".
