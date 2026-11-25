# Epic 21 close — owed proven-red plants and hosted receipts (2026-11-25)

Operator ruling 2026-11-25 (close sheet `epic-21-close-decision-sheet-2026-11-25.md`, S-8 C, S-4, S-5):
run the 21-6 AC4 and 21-3 AC1.g plants now; waive the 21-12 and 21-7 plants (disclosed in
`RELEASE-HOLDS.md` row 23). Base tree: `6534038f` (branch `plants-21-close-2026-11-25`, worktree
`/home/lunarpulse/dev_ws-plants21`). Marker: `E21-PLANT`.

**Deviations from the story text, disclosed.**

* The stories ask for "one image (`ubuntu-24.04`)". These plants ran on the developer workstation
  (`Linux 7.3.3-1-cachyos`, `rustc 1.100.0`), not on `ubuntu-24.04`. They show that the named tests go red
  when the control is removed; they are not a hosted-image record.
* Environment for every cargo command:
  `unset CI GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE; export CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/mnt/build/cargo-maos-plants21 MAOS_FIXTURE_DIR=$CARGO_TARGET_DIR/maos-fixtures`;
  `cargo run --locked -q -p xtask -- build-test-fixtures` first (exit 0, 18 artefacts).
* `MAOS_AUDIT_KEY` was not set for these runs (the tests named here do not read it).
* Raw logs were kept in `/tmp/p21-*.log` (not committed).
* 21-3 AC1.g: the story's plant is "delete `a2a-pins.json` between `SIGTERM` and restart", and the
  deletion is already the body of the two named tests. Its proven-red therefore has two parts, both run:
  (1) a production plant that makes the file carry nothing (reds the AC1.f/AC1.g daemon legs); (2) a
  test-side plant that removes the deletion (the file kept) and shows the AC1.g assertions
  (`generation` 0 / config seed / re-detection `Some(7)`) red. The in-process leg
  `t_21_3_a_consented_repin_survives_a_restart_only_through_the_file` is the one that re-detects: after
  the deletion, `invalidate_if_boot_nonce_differs(peer, 8)` returns `Some(7)`. The daemon-level test still
  never dials a frame; the re-detection half is shown at the pin-store level only (as before).

## 1. 21-6 AC4 — removing the quota check reds AC1 on both tiers

Baseline (no plant): `cargo test --locked -p maos-kernel-core --test memory_tier_quota_21_6` → `16 passed; 0 failed`, exit 0.

Plant (in `crates/maos-kernel-core/src/memory/mod.rs`, first statement of `fn check`, the single
check-and-charge used by the private and the shared tier):

```rust
    if tier != MemoryTier::Collective {
        return Ok(()); // E21-PLANT
    }
```

Run (2026-11-25T09:29:09Z → 09:29:12Z, compile 2.90 s, tests 0.30 s):

```
cargo test --locked -p maos-kernel-core --test memory_tier_quota_21_6
test private_write_past_quota_is_refused_typed_and_other_ops_succeed ... FAILED
test shared_write_past_quota_is_refused_typed_and_other_ops_succeed ... FAILED
   (panicked at tests/memory_tier_quota_21_6.rs:70:18: expected QuotaExceeded, got Ok(()))
... 14 more FAILED
test result: FAILED. 0 passed; 16 failed; 0 ignored; 0 measured; 0 filtered out
exit=101
```

RED on both tiers, for the right reason (`QuotaExceeded` expected, `Ok(())` returned). Restored by deleting the
three plant lines. After restore: `16 passed; 0 failed`, exit 0.

## 2. 21-3 AC1.g — the pin file, and nothing else, carries the state

Baseline: `cargo test --locked -p maos-bin --features network --test durable_pins_21_3` → `4 passed`, exit 0.

### 2a. Production plant: the pin file carries nothing

`crates/maos-persistence/src/lib.rs`, `impl PinPersistence for FilePinStore`, first statement of `fn store`:

```rust
        if !records.is_empty() {
            return Ok(()); // E21-PLANT
        }
```

Run (09:39:09Z → 09:39:30Z; rebuilt `maos-persistence` and `maos-bin`, so the real daemon carries the plant):

```
test t_21_3_a_consented_repin_survives_a_restart_only_through_the_file ... FAILED
   (durable_pins_21_3.rs:283: left: 7, right: 8)
test t_21_3_deleting_the_pin_file_reverts_the_row_to_the_config_seed ... FAILED
test t_21_3_a_promotion_not_carried_into_config_refuses_to_boot ... FAILED
test t_21_3_a_promotion_carried_into_config_survives_a_restart ... FAILED
   (durable_pins_21_3.rs:115: the promotion must be on disk before the daemon stops)
test result: FAILED. 0 passed; 4 failed
exit=101
```

### 2b. Test-side plant: the deletion removed (file kept)

`crates/maos-bin/tests/durable_pins_21_3.rs`: line 160 (daemon leg) replaced by
`carry_promotion(&rotation.fixture, &rotation.rotated); // E21-PLANT ...` (the file is kept, the promotion
carried, so the boot is the AC1.f scenario); line 316 (in-process leg) replaced by
`let _ = &file; // E21-PLANT ...`.

Run (09:39:49Z → 09:40:06Z):

```
test t_21_3_deleting_the_pin_file_reverts_the_row_to_the_config_seed ... FAILED
   (durable_pins_21_3.rs:167: row "generation":1, fingerprint = the promoted one, not the config seed)
test t_21_3_a_consented_repin_survives_a_restart_only_through_the_file ... FAILED
   (durable_pins_21_3.rs:319: left: (8, Some(SpiritRestarted { prior_boot_nonce: 8 })), right: (7, None))
test t_21_3_a_promotion_not_carried_into_config_refuses_to_boot ... ok
test t_21_3_a_promotion_carried_into_config_survives_a_restart ... ok
test result: FAILED. 2 passed; 2 failed
exit=101
```

With the file kept the row is not the seed and the approved nonce is not re-detected; only the deletion
produces the AC1.g outcome. All plants removed (3 edits reverted; the edit tool restored the original file
tags `5FB4` and `1D20`). After restore: `4 passed`, exit 0.

## 3. Removal check

```
grep -rn 'E21-PLANT' crates xtask docs RELEASE-HOLDS.md   → no output
git status --short → only RELEASE-HOLDS.md, docs/invariants/I14.md, docs/invariants/README.md,
                     xtask/tests/invariant_claims_20_3b.rs, and this evidence directory
```

(Run before this file was written; this file names the marker in prose only.)

## 4. Waived plants (S-8 C)

21-12 AC1 re-run record and its three plants, and 21-7 AC1's two plants, were **not run**: operator waiver
2026-11-25, disclosed as `RELEASE-HOLDS.md` row 23.

## 5. Hosted receipts (S-4, S-5) — read-only `gh`

Test suites run on the `workspace-test-suite` job of the `discipline` workflow (nextest, whole workspace).

| Test | Run (`discipline`, main) | Job | Log line |
|---|---|---|---|
| `maos-kernel-core::frame_bridge_poll_17_3c backpressured_writer_is_released_at_the_wall_and_the_child_is_reaped` (21-9 AC8e) | `45551558726`, success, head `6534038f`, 2026-11-25T05:09Z | `138993694324` `workspace-test-suite` success | `2026-11-25T06:04:40Z PASS [ 0.215s] (3531/6718) maos-kernel-core::frame_bridge_poll_17_3c backpressured_writer_is_released_at_the_wall_and_the_child_is_reaped`; summary `6718 tests run: 6718 passed (8 slow), 139 skipped` |
| same | `45394038840`, success, head `576bce7015e6` (PR #54 merge), 2026-11-24T12:29Z | `138444970554` `workspace-test-suite` | `2026-11-24T13:09:06Z PASS [ 0.208s] (3454/6577) maos-kernel-core::frame_bridge_poll_17_3c backpressured_writer_is_released_at_the_wall_and_the_child_is_reaped`; summary `6577 tests run: 6577 passed (5 slow), 138 skipped` |
| `maos-bin::region_reconcile_22_1` (22-1 AC4, D4a): `a_region_the_manifest_cannot_corroborate_refuses`, `boot_refuses_a_region_that_disagrees_with_the_signed_manifest`, `pinning_off_is_not_a_disagreement`, `the_cohort_boot_reconciles_the_value_that_stamps_rows` | `45394038840` (head `576bce7015e6`, the PR #54 merge) | `138444970554` | `2026-11-24T13:06:20Z PASS [ 0.008s] (1415/6577) maos-bin::region_reconcile_22_1 boot_refuses_a_region_that_disagrees_with_the_signed_manifest` and the three siblings at lines 10307-10310 of the job log |
| same four | `45551558726` (head `6534038f`) | `138993694324` | `2026-11-25T06:01:07Z PASS [ 0.015s] (1463/6718) maos-bin::region_reconcile_22_1 boot_refuses_a_region_that_disagrees_with_the_signed_manifest` (lines 10346-10349) |

Notes: the `Commit:` line in the job log header is the runner image's commit, not the repository's; the
repository SHA is the run's `headSha`. Run `45527294642` (head `86fc6020`, PR #59) concluded `failure` and
is not cited. The backpressure test is the writer/bridge-level witness (Linux-only, `/proc`), not a test
through the pre-warm caller; that residual stands as the sheet records it (S-5).

## 6. S-9 — 21-7 findings on main (`6534038f`; 22-9 landed in PR #55, merge `0c1048d4`)

* **F6-revert (auto-revert keeps the successor's consent): implemented on main.**
  `crates/maos-kernel-core/src/hot_swap/coordinator.rs:70-72` (swap record carries the predecessor's
  declaration), `:550-579` (captured at swap commit), `:988-995` (`consent_policy().restore_if_installed(...)`
  on revert); `crates/maos-iac/src/adapter/consent_policy.rs` (`restore_if_installed`). Tests:
  `crates/maos-iac/tests/consent_restore_22_9.rs`:
  `restore_puts_the_predecessors_declaration_back_over_the_successors`,
  `restore_withdraws_a_declaration_the_predecessor_did_not_have`,
  `restore_never_overwrites_another_pids_declaration_or_the_entry_it_would_restore`;
  coordinator level `crates/maos-kernel-core/tests/unit/coordinator_publication.rs:349`
  `revert_restores_the_predecessors_consent_declaration` and `:367`
  `revert_leaves_a_consent_declaration_a_later_swap_installed`. `RELEASE-HOLDS.md` carries 22-9.a (rows 13
  lifted). Not run by me.
* **F4 (`[consent.recall]` schema floor / roster): implemented on main; NOT ratified.**
  `crates/maos-spirit-abi/src/lib.rs:134` `MANIFEST_SCHEMA_VERSION: u32 = 9`;
  `AdmissionRefusal::ConsentRecipientUnknown` → `"consent_recipient_unknown"`
  (`crates/maos-bin/src/admission.rs:435`, message `:528`), `check_consent_recipients`
  (`admission.rs:575`) called at the admission site `admission.rs:1554` and the spawned path
  `spawned_lifecycle.rs:1244`. Tests `crates/maos-bin/tests/consent_declaration_21_7.rs`:
  `a_consent_table_below_schema_9_is_refused_typed_at_the_gate` (`:87`),
  `a_consent_recipient_the_host_does_not_know_is_refused_typed` (`:99`, asserts
  `consent_recipient_unknown` at `:107`), `a_recipient_installed_after_its_sender_is_admitted` (`:114`).
  The ratification row `proposal_id = "22-9-manifest-schema-9"` in `xtask/abi-ratifications.toml:354` is
  still `status = "proposed"`: **operator ratification pending; I did not change it.** The sheet's caveat
  ("admission call sites have no removal-sensitive test") was not re-examined here; no plant was run on
  these call sites.
* F7, F8, F9, F3 residual: no code change in this lane (S-9 as the sheet recommends; F9 hosted receipt for
  the binary-level consent witness `consent_gate_21_7.rs::production_consent_granter_and_gate_without_postgres`
  was not looked up in this lane).
