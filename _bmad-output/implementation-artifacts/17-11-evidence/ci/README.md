# 17-11 AC4 — planted-red CI run: PROVEN RED (2026-10-24)

Status: **proven red** — run `40045099664` (workflow `17-11-proven-red`, branch `ci/17-11-verify` @ `636bed02` = `15b02d98` + workflow file), job `120724912315`, 00:38:07–00:39:04Z. Excerpt: `run-40045099664-excerpt.log`.

- Result: 3 tests run, **1 passed, 2 failed** — `fs_subtree_containment_issues_beneath_the_declaration` red at `scope_selector_entitlement_17_11.rs:354` (`/data/../etc` entitled `/etc` → `Ok(())`), `declared_selectors_issue_and_undeclared_are_denied_uncharged` red at `:304` (`McpCall{web,search}` entitled `McpCall{web,fetch}`); control `kind_only_and_unmediated_scopes_are_unchanged` green. The job's red conclusion is the proof (the step asserts the planted tree fails).
- Runner: `ubuntu-24.04`, image version `20261012.334.1`.

- Plant diff: `ac4-plant.diff` (sha256 `d66a75ae3aaae671ec489b246e8fc21b608f6baf5d3a8f1c87e99743a39362ba`), applied on top of the story's landing tree: restores the discriminant match at `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:164`.
- Image: `ubuntu-24.04` (one image, `docs/testing/proof-contract.md:10-18`).
- Command: `cargo nextest run --locked -p maos-kernel-core --test scope_selector_entitlement_17_11 --no-fail-fast`.
- Expected: `declared_selectors_issue_and_undeclared_are_denied_uncharged` and `fs_subtree_containment_issues_beneath_the_declaration` red at their first undeclared vector; `kind_only_and_unmediated_scopes_are_unchanged` green (local proof: `../plants.md`, `../plant-ac4-local.log`).
- Still owed: the final tree's one green `discipline` run (main @ `15b02d98`, run `40045055432`, queued at record time); then delete branch `ci/17-11-verify`.
