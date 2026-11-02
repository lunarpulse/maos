# 20-1a landing 1 — mutation evidence

All mutations ran in the real checkout, one cargo process at a time. No worktree,
secondary target directory, kernel mutation, commit or push was used. The shared
creation evidence remains in `../../20-1-registry-client-install-verb-vetter-and-yank-evidence/creation/`.
`captured-commands.jsonl` records the exact later commands, exits and wall clocks;
`commands.json` records the earlier captures. Every temporary source edit below
was restored by copy-back, `cmp`, then `touch` (all restoration commands exited 0).

## Executed local controls

| AC | Witness (`name:line` in the final source) | Temporary defect / reconstructed before state | Observed red | Wall clock |
|---|---|---|---|---:|
| AC1 | `crates/maos-registry/tests/registry_server_bin_20_1.rs:7` `standalone_server_roundtrip_isolates_root_and_exits_cleanly` | `Some(root) => at_path(root)` replaced with `Some(_root) => new()` in shared server construction | `ac1-plant.log`: exit 101, `--root must own the package` assertion; the isolated caller HOME was the only fallback store | 6.471 s |
| AC2 | `crates/maos-spirit-cli/tests/publish_live_20_1.rs:5` `publish_real_address_returns_receipt_and_manifest` | Restore the pre-story non-stub publisher guard | `ac2-red-plant.log`: exit 101, real-address publish refused by the guard | 44.770 s |
| AC3 | `crates/maos-cli/tests/spirit_install_20_1.rs:61` `install_refuses_registry_rust_inproc_before_persisting` | Return an admitting decision instead of calling admission on the no-attestation arm | `ac3-plant.log`: exit 101, signed `spirits/researcher` package incorrectly installed and `!output.status.success()` failed | 4.132 s |
| AC4 | `crates/maos-bin/tests/installed_spirit_dispatch_20_1.rs:6` `installed_component_dispatch_reports_typed_boot_refusal` | Remove the installed-name dispatch branch, restoring the hello-only fallback | `ac4-plant.log`: exit 101; stdout contained `maos: unknown spirit 'example-spirit-ts' — known: hello-spirit` instead of the exact typed refusal | 38.257 s |
| AC5 before witness | `xtask/tests/registry_exit_20_1.rs:4` `registry_release_set_and_exit_job_are_enrolled` | Reconstruct the pre-landing two-binary `ARTIFACTS` table | `ac5-red-before.log`: exit 101 at the explicit four-binary denominator assertion | 14.081 s |

AC1 also has an actual unmodified-source red (`ac1-red.log`: exit 101, server stub
exited 1). AC3 has the prebuild CLI red (`ac3-red.log`: exit 2, unrecognized
`spirit install`). AC4's original before observation is creation
`runtime/exit-probe-a4c00897.txt:71-87`; the local reconstruction above independently
falsifies the same fallback. AC2's guard reconstruction is both its red-before
witness and its local plant. An initial AC5 capture named `ac5-red` actually
passed: compilation observed the newly edited denominator/job. It is **not** red
evidence; `ac5-red-before.log` is the explicit reconstructed-before run.

Thus there are four executed AC1–AC4 plants plus one separately named AC5
before-state reconstruction. AC5's required no-server plant is hosted-only and
has not executed locally.

## Byte-identical restoration

For AC1, AC3, AC4 and AC5-before respectively, the successful restoration commands
were (absolute paths in the actual invocations):

```text
cp /tmp/20-1a-server.snapshot <repo>/crates/maos-registry/src/server.rs && cmp /tmp/20-1a-server.snapshot <repo>/crates/maos-registry/src/server.rs && touch <repo>/crates/maos-registry/src/server.rs
cp /tmp/20-1a-install.snapshot <repo>/crates/maos-registry/src/install.rs && cmp /tmp/20-1a-install.snapshot <repo>/crates/maos-registry/src/install.rs && touch <repo>/crates/maos-registry/src/install.rs
cp /tmp/20-1a-shell.snapshot <repo>/crates/maos-shell/src/lib.rs && cmp /tmp/20-1a-shell.snapshot <repo>/crates/maos-shell/src/lib.rs && touch <repo>/crates/maos-shell/src/lib.rs
cp /tmp/20-1a-release.snapshot <repo>/xtask/src/release_dry_run.rs && cmp /tmp/20-1a-release.snapshot <repo>/xtask/src/release_dry_run.rs && touch <repo>/xtask/src/release_dry_run.rs
```

AC2 used `cp`, `cmp`, and `touch` as three individually captured commands
(`ac2-restore-copy`, `ac2-restore-cmp`, `ac2-restore-touch`; each exit 0). Its backup
was the exact then-current `publish.rs`, before subsequent normal implementation
and formatting. Restore snapshot SHA-256 values:

```text
AC1 ff1d6a545b9a2a903d34b367aced0b68f8a1addf5130b12a6a9aca738632fc02
AC2 96168bc9ba890ec6f8ce7a1028743e9419ea1499672af7ba15d21789bb216177
AC3 1e2a6970261791110032d819261c1bd2746c1f5830e5ef7c7b74aa27de046522
AC4 a7e76d1fde5a33be09ec366a2b36f1c26a503313bd115cef4688cc73ab8ab489
AC5 17e9ac47354bd6ebb0b2d5fc5b625c1f954f4cbfbb10c942cc9dd40c066c1bc1
```

## Hosted-only control (not run, never pushed)

`../ci/ci-20-1a-verify.yml` carries the production exit job, the no-server plant,
and the WASM-enabled installed-manifest execution leg with delegated cgroups and
existing T3 root-shutdown witnesses. The exact no-server patch is
`../ci/ac5-no-server.diff`, SHA-256
`8cdf9a4ca1acf57083cdbc03f18d065b25d9c1541b8324b8d7f2643785fb33dc`.
The plant requires nonzero exit **and** `registry readiness failed within 5 s`;
a failure for any other reason is not accepted. No hosted green/red run is claimed.

## Subsequent ordinary repair

After all copy-back/cmp/touch restorations above, the serde gate exposed two
pre-existing empty-vector serialization fallbacks in the now-runnable server.
They were replaced with direct error propagation, removing 20 production lines.
The snapshot hashes above certify restoration at plant time, not a claim that
later legitimate repairs left those files unchanged. The final fresh serial run
passed all 5,526 tests, including all AC witnesses; no mutation remains applied.
