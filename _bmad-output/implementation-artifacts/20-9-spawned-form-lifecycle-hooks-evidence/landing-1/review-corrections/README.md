# 20-9 L1 review corrections — evidence index

`evidence.md` is the witness table. Raw logs: `red/red-cdf3937d.log` (new tests against a scratch `git archive cdf3937d`,
13/13 red), `plants/*.log` (13 plants, each restored cmp-clean), `baseline-daemon-triggers.log` / `final-daemon-triggers.log`
(`base-daemon-triggers.sh`, base `fc7373f0` vs final), `final-focused.log`, `workspace-nextest.log`, `gate-*.log`,
`clippy-ratchet.log`, `verb-table-no-default.log`, `fmt-check.log`. `final-swap-lifecycle-{1,2}.log` are superseded red runs
(guest stderr crashed the runner under the T2 seccomp filter; the probe no longer writes stderr); `-3.log` is the run after.
`suites-1.log` (two failures, fixed) and `suites-2.log` (71/71) are the door/installed/pump reruns. `gate-kloc-check.log` is red only because of a foreign untracked `spikes/gpui-operator-shell/` (+127 aggregate lines, `(unknown:spikes)` 271 > 244), not this change: measured without it, aggregate 201197.

Later rounds keep the same layout in their own directories, each with its own `evidence.md`: `round3/` (reviews of `8c669a21`: red at `c8e5d035`, nine plant runs) and `round4/` (reviews of `f56959c2`: red at `0e455c4e`, nine plant runs). `plants.sh` takes `PLANT_DIR` to choose where a round's plant logs go.
