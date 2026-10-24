# 17-11 AC4 — planted-red CI run: OWED AT PUSH

Status: **owed at push** — the integrator pushes; the dev host does not push.

- Plant diff: `ac4-plant.diff` (sha256 `d66a75ae3aaae671ec489b246e8fc21b608f6baf5d3a8f1c87e99743a39362ba`), applied on top of the story's landing tree: restores the discriminant match at `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:164`.
- Image: `ubuntu-24.04` (one image, `docs/testing/proof-contract.md:10-18`).
- Command: `cargo nextest run --locked -p maos-kernel-core --test scope_selector_entitlement_17_11 --no-fail-fast`.
- Expected: `declared_selectors_issue_and_undeclared_are_denied_uncharged` and `fs_subtree_containment_issues_beneath_the_declaration` red at their first undeclared vector; `kind_only_and_unmediated_scopes_are_unchanged` green (local proof: `../plants.md`, `../plant-ac4-local.log`).
- To record here after the run: run id, commit, image, plant sha256 (above); then the final tree's one green `discipline` run.
