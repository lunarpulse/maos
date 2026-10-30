# 20-3b landing-2 proven-red plants

All four FR31 mutations ran in the real checkout, sequentially, after green witnesses. No test expectation was changed. Original bytes were copied to a temporary directory; every copy-back, `cmp` and `touch` command exited 0 (exact commands in `commands.jsonl`). The temporary backups were removed after restored-green runs.

## plant-disclosure-owner-pid

```sh
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test principal_identity_20_3b recycled_pid_cannot_disclose_another_spirits_principal_spill -- --exact --nocapture
```

Exit **101**, **9.141 s** (including compilation). Red assertion:

```text
panicked at crates/maos-kernel-core/tests/principal_identity_20_3b.rs:47:5:
assertion `left == right` failed
  left: Some(Markdown("A only"))
 right: None
```

## plant-butler-restore-poisoning

```sh
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --test butler_scene_18_4 recycled_pid_cannot_poison_butlers_acceptance_restore -- --exact --nocapture
```

Exit **101**, **14.062 s** (including compilation). Red assertion:

```text
panicked at crates/maos-bin/tests/butler_scene_18_4.rs:1009:5:
assertion `left == right` failed: Butler must restore zero attacker records
  left: 7
 right: 0
```

## plant-principal-index-export-pid

```sh
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test principal_identity_20_3b subject_access_export_preserves_both_owners_when_pid_and_key_are_reused -- --exact --nocapture
```

Exit **101**, **5.490 s** (including compilation). Red assertion:

```text
panicked at crates/maos-kernel-core/tests/principal_identity_20_3b.rs:111:5:
assertion `left == right` failed: the index key must include stable owner, not pid
  left: 1
 right: 2
```

## plant-small-principal-durability

```sh
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test principal_identity_20_3b small_non_markdown_principal_values_survive_restart_and_pid_change -- --exact --nocapture
```

Exit **101**, **5.623 s** (including compilation). Red assertion:

```text
panicked at crates/maos-kernel-core/tests/principal_identity_20_3b.rs:76:9:
assertion `left == right` failed
  left: None
 right: Some(Text("small"))
```

## Cut checksums and restoration

The exact unified diffs are the four `*-plant.diff` files. `plant-cuts.sha256` is consumed by the branch-only hosted evidence workflow.

```text
7a2bf3ff7a2b290126703e33fcf3338a8e23b736ebd969eabb2b5005e9247bc5  disclosure-owner-pid-plant.diff
571f7040b7a672b4a3ff97c88394ac12a70b01b89c06df9a64331171f6e4bc63  principal-index-export-pid-plant.diff
c889df8965f76b7cdb078065a1ef3d7133bc110deb9f5ee3608ed56802af815e  small-principal-durability-plant.diff
7a2bf3ff7a2b290126703e33fcf3338a8e23b736ebd969eabb2b5005e9247bc5  butler-restore-poisoning-plant.diff
```

- `env -u CI -u GITHUB_ACTIONS cp /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/private.rs /tmp/20-3b-plant-backup-sgvb86z5/private.rs` — exit 0, 0.004 s.
- `env -u CI -u GITHUB_ACTIONS cp /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/principal.rs /tmp/20-3b-plant-backup-sgvb86z5/principal.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cp /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/mod.rs /tmp/20-3b-plant-backup-sgvb86z5/mod.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cp /tmp/20-3b-plant-backup-sgvb86z5/private.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/private.rs` — exit 0, 0.002 s.
- `env -u CI -u GITHUB_ACTIONS cmp /tmp/20-3b-plant-backup-sgvb86z5/private.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/private.rs` — exit 0, 0.005 s.
- `env -u CI -u GITHUB_ACTIONS touch /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/private.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cp /tmp/20-3b-plant-backup-sgvb86z5/principal.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/principal.rs` — exit 0, 0.002 s.
- `env -u CI -u GITHUB_ACTIONS cmp /tmp/20-3b-plant-backup-sgvb86z5/principal.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/principal.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS touch /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/principal.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cp /tmp/20-3b-plant-backup-sgvb86z5/mod.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/mod.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cmp /tmp/20-3b-plant-backup-sgvb86z5/mod.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/mod.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS touch /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/mod.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cp /tmp/20-3b-plant-backup-sgvb86z5/private.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/private.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cmp /tmp/20-3b-plant-backup-sgvb86z5/private.rs /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/private.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS touch /home/lunarpulse/dev_ws/crates/maos-kernel-core/src/memory/private.rs` — exit 0, 0.001 s.
- `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test principal_identity_20_3b` — exit 0, 4.807 s.
- `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --test butler_scene_18_4 recycled_pid_cannot_poison_butlers_acceptance_restore -- --exact` — exit 0, 10.131 s.

## Other required negative vectors

- AC2: NO-GO is the ruled closure, not an unrun generated-YAML drift claim. `coverage_matrix_20_3b` passed its historical byte-oracle, hard-mode unknown-reference/phase refusal and NO-GO-retains-completeness vectors in `xtask-new-vectors.log`.
- AC3 real-tree `if: false`: built `xtask check-epic-close-coherence --json` clean **0** / planted **1** naming `disabled-workflow-job` / restored **0**; respective wall times 0.024 / 0.020 / 0.020 s. Scratch workflow removed, not enrolled. Exact commands and outputs: `commands.jsonl`, `plant-ac3-{clean,disabled,restored}.log`.
- Invariant ledger: `guaranteed_requires_property_corpus_and_ci_job_together` rejects missing property test/corpus/CI job, nonexistent paths/test, and imaginary claim state; `asserted_claim_without_owner_is_not_an_honest_ledger_entry` rejects absent/blank owner; `forged_id_or_missing_register_cannot_shrink_the_ledger` rejects forged/missing I14. All passed in the real in-tree integration binary.
- Clippy ratchet: occurrence growth **1412/884** and source-distinct growth **1411/885** are refused independently; equal/lower counts accepted; malformed/truncated/unsuccessful streams refused. These in-tree tests passed; they are not changes to the live baseline.

Exact combined negative-vector command (exit 0) and time are recorded under `xtask-new-vectors` in `commands.jsonl`; its log names every executed test.

## Hosted status

`../ci/ci-20-3b-verify.yml` now wires the FR31 green vectors, all four checksum-pinned mutations with copy-back/cmp/touch, restored-green witnesses, coverage NO-GO/hard-mode witnesses, and the real non-denying clippy stream/ratchet. It is evidence-only, branch-only, and has **not been pushed or run**. No additional hosted-only cut was requested (Main ruling, 2026-10-30). No hosted run id or ledger-consuming proof is claimed.
