# 17-11 plants (proof contract: ONE plant per AC, in place)

Plant count: **1** (AC4 — the only AC with a plant; AC1–AC3 are proven by it and by their focused suites).

| AC | Plant (file:line) | Command | Red output (first line) | Wall-clock | Restore |
|---|---|---|---|---|---|
| AC4 (proves AC1 tests 1–2) | `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:164` — `.any(\|s\| s.entitles(&capability.scope))` → `.any(\|s\| std::mem::discriminant(s) == std::mem::discriminant(&capability.scope))` (`ci/ac4-plant.diff`, sha256 `d66a75ae3aaae671ec489b246e8fc21b608f6baf5d3a8f1c87e99743a39362ba`) | `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test scope_selector_entitlement_17_11 --no-fail-fast` | `assertion `left == right` failed: declared McpCall { server: "web", tool: "search" } must not entitle McpCall { server: "web", tool: "fetch" }` (test 1, first undeclared vector, `scope_selector_entitlement_17_11.rs:295`); test 2 red at `:337` (first `/data/../etc` vector); test 3 green. Summary `3 tests run: 1 passed, 2 failed` | 10.44 s (2026-10-23T16:06:53Z, incl. incremental rebuild; tests 0.061 s) | the file carried the story's uncommitted edit, so it was copied aside first (`/tmp/cap_policy_17_11.rs`) and copied back; rerun green `3 passed`; `check-kernel-baseline` PASSED at 26361/95 with the re-pinned digest |

Local log: `plant-ac4-local.log`. CI run on `ubuntu-24.04`: owed at push (`ci/README.md`).
