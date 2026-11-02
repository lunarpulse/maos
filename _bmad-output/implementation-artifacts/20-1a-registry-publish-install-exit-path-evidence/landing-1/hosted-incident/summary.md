# Hosted exit incident — local binary reproduction and repair

## Root cause and wire evidence

Hosted verification run **41424027458**, job **125206155549**, failed at `maos-spirit publish` after release creation/verification passed. The old in-process witness published an eight-byte component header, so it never exercised production-sized durable persistence.

The real example was built locally with `npm ci`, `npm run build`, and componentization. System Node was v20.19.2, below the example's declared Node requirement; the first componentization also exposed npm's missing optional `@napi-rs/lzma-linux-x64-gnu` binding (exit1). Installing Node22 and that native binding **only into ignored node_modules** with `--no-save --package-lock=false` allowed componentization to complete (exit0); no package manifest or lock was edited.

The actual example component is **12,453,178 bytes**. The real release publisher sent **34,782,286 body bytes**, with valid `POST /mcp HTTP/1.1`, loopback Host and `Content-Type: application/json`; the captured request has no Expect header. A transparent TCP relay recorded both directions, not a mock server:

| Observation | Before | After |
|---|---:|---:|
| Request bytes including headers | 34,782,539 | 34,782,539 |
| Response bytes | **0** | **198** |
| Response header | absent | HTTP/1.1 200 OK; Content-Length107; Connection: close |
| Relay response lifetime | 40.963s | 0.457s |
| Release CLI command | exit1,30.711s | exit0,0.915s |

See `tcp-before.json`, `tcp-after.json`, `registry-before.log`, `registry-direct-before.log`, `registry-after.log`, and `publish-real-{before,after}.log`. The direct real-example before-control also failed (exit1,30.911s, client read timeout). The deterministic **4MiB real-binary test** reproduced the exact hosted `decode error: truncated or oversized HTTP header` (exit101,15.70s test runtime), rather than merely a transport timeout.

Two interacting defects caused this:

1. `storage.rs` serialized the signed package's multi-million-element JSON byte arrays **directly to an unbuffered File**. Per-token writes made durable publication take longer than the five-second request I/O deadline (and, for the real example here, the client's30-second read timeout).
2. `server.rs` reused the already-expired request deadline for response writes and discarded the write error. Thus a publication could complete its storage work but send **zero HTTP bytes**. Deadline rejections themselves used that expired writer, and the body-read failure path also emitted an unframed400 response.

This was not a Host/Origin/content-type/path mismatch or a header-size violation. No reviewed security guard was relaxed.

## Changes and witnesses

| Location | Repair / observable witness |
|---|---|
| `crates/maos-registry/src/storage.rs:588` | Buffer atomic JSON writes; explicitly flush before existing sync_all and rename. Durability/error propagation remain intact. |
| `crates/maos-registry/src/server.rs:218,355,359` | Keep the five-second absolute read deadline; use a separate five-second absolute response-write phase. One response helper sends complete framing, propagates errors, half-closes writes, and drains unread input under a separate100ms absolute close bound. Request rejection classes, including408 and413, use it; connection errors reach stderr. |
| `crates/maos-spirit-cli/tests/publish_live_20_1.rs:30` | Real publisher executable → registered standalone server executable; valid4MiB component/custom section; checks receipt and exact persisted artifact. Transport red101 → green0 (2.023s focused test). Original in-process witness retained. |
| `crates/maos-registry/tests/registry_server_bin_20_1.rs:109,154` | Real server binary:400,403 Host/Origin,404,405,411,413,415,431 byte/count,408 header/body deadlines,503 population. Assert full header, exact Content-Length, Connection: close and clean EOF. Unframed-body400 red101 → all4 binary tests green0. |
| `tests/harness/registry_20_1.rs:166` | Reuse publisher command construction for explicit real-sized artifacts. |

The original erroneous test setup (publisher key0644) failed before transport and is explicitly **not** the transport-red evidence; it is preserved as `witness-red-real-publish.log`. The corrected key0600 control is `witness-red-real-publish-transport.log`.

## Additional hosted discipline failures

Read-only download/diff of hosted diagnostic artifacts compares **41305010381** to **41423605076**, by source/lint/message, with line locations retained in `hosted-warning-diff.json`. Newly introduced warning identities were `server.rs:246 clippy::collapsible_if` and `installed_spirits.rs:172 clippy::result_large_err`; the older `trim_split_whitespace` disappeared. The body-read refactor removes the nested conditional; `crates/maos-bin/src/installed_spirits.rs:171` now formats delivery success/error **inside** block_in_place instead of returning a large Result through that closure. No lint allow, boxing allocation or clippy baseline edit.

The FF-J6 failure was a whole-file wildcard false positive: FR35's negative independent-author disclaimer at coverage-matrix624 supplied `J6`, unrelated `Arc<dyn` at742 supplied `<`, and `seams` at775 supplied `ms`. The FR35 sentence now honestly says `Mechanism-proven only; no independent-author claim`; no latency claim or gate rule was weakened. `check-ff-j6` is green.

Fixture policy remains **no cargo-spawning in default tests**, no stale-binary fallback. `.github/workflows/discipline.yml:5512` provisions maosctl,worker-cli-fixture,maos-registry-server,maos-spirit through the registered builder before check-multi-tenant-loom. The whole publisher suite in `air-gap-import-corpus:3922` now provisions its registered siblings too. `tests/harness/fixtures.rs:471,479,493` names the missing/stale binary and exact `cargo run -p xtask -- build-test-fixtures --only <name>` command. `xtask/tests/registry_exit_20_1.rs:46` asserts setup precedes both package consumers.

**Job census:** workspace-test-suite already builds every fixture at discipline6192 before the workspace run. check-multi-tenant-loom is the only gate with an unfiltered `cargo test -p maos-bin` leg (`xtask/src/check_multi_tenant_loom.rs:1862`); repaired as above. No other direct whole-package maos-bin/maos-cli/maos-registry invocation was found in discipline.yml; those jobs use named targets or test-name filters. The unfiltered publisher invocation in air-gap-import-corpus was also repaired. Archive consumers use their explicit target sets and transported fixture manifests.

An **initially empty MAOS_FIXTURE_DIR** plus the exact repaired four-fixture setup returned0; unchanged installed dispatch/boot-integrity tests then passed **2/2** (exit0). Full local check-multi-tenant-loom was exercised but remains **exit1**: it stops earlier at unrelated `hook_pump_17_4::one_budget_overrun_quarantines_the_subscriber`, line439, expected failed1/delivered0, observed delivered1/failed0. It does not reach the installed tests. No retry-to-green, test weakening, or claimed repair of that timing observation. Its live Postgres and operator-key legs are absent locally. An earlier shared-cell execution was interrupted without a gate result; the complete1226.078s run is separately recorded.

## Local workflow and final matrix

- All four real release binaries built/staged (`release-dry-run` before/after exit0), dev-key release signature verified (exit0), release install verify-only exit0.
- Actual example publish exit0, signature-verified name install exit0, `maos init` exit0, replay shell exit0; **exact** `maos: spirit 'example-spirit-ts' refused at boot: wasm_engine_off`, never unknown. Server SIGTERM exit0.
- WASM-enabled maos, runner and worker fixture builds exit0; WASM boot-integrity1/1 exit0; supplemental root-shutdown suite exit0.
- Actual installed-component execution remains **host-blocked**, not proven: rootless Podman cannot clone (EPERM, runtime125, provisioner1), and delegated systemd-run exits1 because PID1 is not systemd. No substrate was fabricated and no successful Spirit execution is claimed.
- Fmt exit0; final workspace all-target build exit0; production fixture build exit0; focused registry/publisher **122/122**,2 skipped; workflow fixture-enrollment2/2; kernel lib253/253; registry/publisher fixture-replay145/145,2 skipped, all exit0.
- Full fresh serial workspace **5549/5549**,138 skipped,exit0; suite650.632s. Serial junit budget gate exit0:593 binaries,Σ650.247s,no findings. No compiler processes at start (ps1); host load106.48/123.09/119.84 is disclosed, not described as idle.
- Full unrestricted default workspace **5549/5549**,138 skipped,exit0;suite42.974s.
- Required kloc/kernel/exit/decision/dev-record/coherence/env/serde gates all exit0, as did FF-J6,coverage completeness/generator,invariant lock,corpus,review findings,service boundary,composition root,dependency closure,test cost,perf runner policy,ship completeness,manifest schema,pub-field constructors,unsafe,loom-substrate-drift.
- Real non-denying all-target clippy exit0; ratchet **1335 occurrences /873 source-distinct**,exit0,baseline **1383/897 unchanged**. Initial ratchet invocation accidentally received merged Cargo stderr and rejected JSON line135 (exit1); JSON records from the same successful clippy invocation were separated without rerunning clippy, then ratchet passed. Both artifacts remain. No post-fix hosted count is claimed.
- Cargo deny exit0.
- Prescribed `xtask prepush --json`, inherited target, CI/GITHUB_ACTIONS unset,CARGO_MANIFEST_DIR set,undelegated mode: **exit0**,322.540s;83 pass,8 skip,1 advisory. Default timing advisory remains: one-daemon33.575s and participant-recall40.651s versus unchanged30s budgets. The required separate serial budget gate passed; no exception or budget widening.

**Exact pins:** registry4232→**4248** (ceiling5163); aggregate191810→**191826**,hardfail**191827** (ceiling193768). Bin33498 and xtask44406 unchanged. Kernel **26841 physical /21042 tokei**,95 source files,set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`,changed/added/removed empty. Prior HISTORY retained; no kernel or FKCS baseline edits.

Every executed command/exit/wall time is in [commands.jsonl](commands.jsonl); environment custody is [execution-env.json](execution-env.json). No commit,push,status flip,cargo clean,target override,or .devcontainer edit. Hosted rerun/no-server control,actual T3 component execution,and the unrelated local whole-package hook timing failure remain open. Historical failures elsewhere in landing-1 are not erased by this matrix.
