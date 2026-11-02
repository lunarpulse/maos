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

## Review corrections after `850e4c00`

The new regression vectors ran against the pre-repair production tree before
implementation. These are runtime failures, not compile-error witnesses:

| Finding | Captured red boundary | Exit |
|---|---|---:|
| P1 / F2 | `review-red-boot`: modified manifest still reached `wasm_engine_off`; `review-red-registry`: inbox publish became boot-visible | 100 |
| F1 | `browser_and_rebinding_requests_cannot_publish`: rebinding Host returned HTTP 200 | 100 |
| F3 | `traversal_publish_is_an_rpc_error_without_escape_files`: traversal returned a publish receipt | 100 |
| F7 / P11 / deferred 3 | Header and connection limits missing; trickling request exceeded its absolute bound; active-client SIGTERM failed the two-second drain assertion | 100 |
| P2 | Both Cargo-target/staging consistency and missing executable-receipt vectors accepted stale bytes | 100 |
| P3 | PublicUntrusted was accepted under a PublicVetted floor | 100 |
| P4 | Materialization failure activated the partial install | 100 |
| P5 | Malformed `tools/call` closed without a JSON-RPC response | 100 |
| P6 | Executable exit job lacked the WASM-enabled execution leg | 100 |
| P8 | An unrecorded sibling executable passed the presence-only resolver | 100 |
| P10 | Reserved `hello-spirit` installed successfully | 100 |
| deferred 2 | Failed index persistence returned success and mutated the in-memory index | 100 |

Logs: `review-red-{registry,order-shutdown,install,boot,release}.log`.
Initial repaired vectors subsequently passed in `review-green-*.log`.

Two additional production mutations establish the strengthened witnesses:

1. **P9 signature-presence-only plant.** Replaced the existing server-signature
   verification expression with a no-op that retained signature presence and
   configured-key handling. Rebuilt registered fixture provenance on that exact
   planted tree (exit 0), then ran
   `independently_keyed_server_is_verified_and_wrong_key_never_persists`.
   It failed at wrong-key acceptance (`true != false`), **exit 100**,
   2.020 s including build. This did not fail on stale fixture provenance.
2. **P12 first-address-only plant.** Replaced the bounded address loop with the
   former first-address-only behavior. The deterministic two-address vector
   failed with `ConnectionRefused` on the first address, **exit 101**, 1.984 s.

Each mutation was restored by separately captured **cp → cmp → touch**, all
exit 0. Original snapshots and restored source had the same SHA-256:

```text
client.rs 3e84e63d8e55f556a445a985e0ff565011d797330eaeb4eaf9a22f981c768ed8
http.rs   0661a61ce844dc3072d3e020ef33045e5767468416bb0bd2a0b92bb5bde12b45
```

The unique `/tmp/20-1a-review-2AHAwqBt` snapshots and directory were removed
after comparison (both cleanup commands exit 0). Full commands and exits are
in `captured-commands.jsonl`; logs use `review-p9-*` / `review-p12-*`.
The hosted P7 readiness diff remains current with the unchanged SHA-256 above;
its remote execution is still owed and was never replaced by a local claim.

### Absolute-deadline control after TCP-reset oracle correction

The initial absolute-deadline test treated a genuine TCP reset as an unexpected
read error. Main approved accepting **EOF or ConnectionReset** as peer closure,
while retaining the seven-second bound and rejecting WouldBlock/TimedOut.
The corrected oracle was then planted against a sliding per-read timeout:
`DeadlineStream::remaining` returned the full five-second duration each time,
instead of the remaining absolute deadline. The real nextest witness
`trickling_request_has_an_absolute_deadline`
(`crates/maos-registry/tests/registry_security_20_1a.rs:155`) failed **100**
(`review-deadline-sliding-plant.log`,16.985 s command wall time).
Thus TCP-reset acceptance does not let the missing absolute deadline pass.

The original server source was copy-backed, compared with `cmp`, and touched;
each command returned0. Snapshot and restored SHA-256:

```text
server.rs 725c3cfe8f7e62359aaa2009a84caa19e049b5b5953508f516f3d04ffba22172
```

Snapshot-file and temporary-directory cleanup both returned0. All eleven
registry security vectors passed on restored source, as did the final serial
and default workspace runs. Exact argv/exits and immutable logs are retained
under `review-deadline-*` and `review-repaired-*`. Final prepush still exposed
the separately recorded known20-7 observer timing failure; it was not retried.
