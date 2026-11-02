# Installed-component startup incident — build-profile repair

## Failure and bounded work

Hosted [run 41456787892](https://github.com/lunarpulse/maos/actions/runs/41456787892), job125315380721, successfully published and installed the **12,453,135-byte** real TypeScript component, initialized maos, and launched the dedicated delegated service. The unit failed with `Deadline("startup")`: daemon startup14:19:11.395, terminal error14:25:11.529; service runtime360.149s, exit1. The `escape-anomaly consumer stopped` line is shutdown cleanup, not the cause. Original `/tmp/v201b.log` lines4382,4418–4423 contain these observations.

`crates/maos-bin/src/spirit_session.rs:139-144` takes the launch budget from `[budget].time_cap_seconds`; the example declares360 seconds (not the30-second fallback). After spawning the contained runner, lines370–378 start/check that budget until its initial Ready control. Runner startup includes reading the component, Wasmtime engine creation, **compilation**, binding/linker setup, instantiation and guest `on-start`; Ready is only sent after these (`crates/maos-wasm-host/src/runner.rs:172-288`). The independent compile watchdog is360s (`:166-170,335-355`), so it is not an extra allowance. Registry publication/install happen before this budget starts.

`scripts/run_17_3c_delegated.sh` does not set a timeout or build a binary: it refuses any cgroup except the dedicated system/user service, moves itself into its daemon leaf, enables CPU/memory controllers, then execs the supplied command. The real contained child receives the manifest's10% CPU /768MiB caps. Explicit undelegated test mode does **not** silently waive the declared CPU cap.

The existing17-3c proof explicitly claims a **release** CLI launch (story lines43–46). `scripts/prove_17_3c.py:170-171,185-193` asserts the10%/768MiB/360s contract and bounds its external observation at400s; it does not widen the product budget. `17-3c-evidence/matched-refusal-launch-360.json` records an actual successful capped launch at322.345s on its accepted substrate; `remote-capped-startup-before.json` retains the earlier180-second refusal. These are historical evidence, not new local or hosted passes.

## Measurements and decision

The actual local component is **12,453,178 bytes**, SHA-256 `8a0fd996671396c86a9cd257fbc1aead3dc33196432ed8e634ea731e715b2420` (43 bytes larger than the hosted build). It is the real prior jco-componentized example, not the tiny header fixture. Its build provenance/Node22 native-binding workaround is in the adjacent [previous incident](../hosted-incident/summary.md).

Temporary `Instant::now`/`elapsed` stderr probes surrounded only `compile_with_timeout`, `Spirit::instantiate` and `call_on_start`. They were removed and the runner returned to its exact original snapshot4AD2 before final build/checks; no diagnostics or timing API ships. Direct stdin-EOF runner execution reached Ready and exited0 in both profiles, **without containment or a CPU quota**:

| Phase (seconds) | Debug | Release |
|---|---:|---:|
| Compile |188.606264|13.580964|
| Instantiate |0.003944|0.005675|
| on-start |0.000617|0.000484|
| Whole subprocess wall |188.670|13.792|

See `runner-{debug,release}-timing.log`, `commands.jsonl` and `execution-env.json`. Debug compilation is13.89× slower on this observation. The local direct runners did **not** expire either360s deadline; they did not run under the admitted CPU cap. [INFERENCE] Slow unoptimized host-side compilation under that cap explains the hosted budget exhaustion; the failed hosted run did not contain per-phase probes, and these uncontained local timings alone do not establish capped release success or a universal startup bound.

**Fix:** both `.github/workflows/discipline.yml` and `evidence/ci/ci-20-1a-verify.yml` now build release maos (`wasm-host`), release runner and release worker fixture, and invoke the release daemon for init/run. `/usr/bin/time -p` records elapsed/user/system execution in the uploaded component log. `set -euo pipefail`, the dedicated delegated service and mandatory `spirit_session_complete` assertion remain. This matches the profile of the existing accepted17-3c claim instead of benchmarking debug Cranelift against a production admission budget. No deadline, manifest, cgroup policy, environment contract, compiler optimization policy or production code changed.

Compilation/conformance intentionally remains inside the T2 child. Moving it into the daemon would break that security contract; introducing persistent executable precompilation/deserialization would need a separate trust/custody design. The previously deferred independent compile/startup budget split is not needed to correct this workflow-profile mismatch and was not folded into this incident.

`installed_component_proof_uses_the_accepted_release_profile` checks both workflow copies, all three release builds, release init/run, elapsed measurement, mandatory completion and absence of debug dispatch. Before the repair it failed with exit101; after it the complete workflow suite passed3/3, exit0. There is no product-code change requiring a separate product mutation witness.

## Local leg replay and honest limits

Real release publisher/server/install binaries used an isolated HOME/registry root. Publish, signature-verified install and debug/release init each returned0; server SIGTERM returned0. Actual installed-manifest debug and release boot commands with `MAOS_17_2_CGROUP_MODE=undelegated` both returned1 with `declared CPU or PID cap requires a writable delegated cgroup`, before runner compilation. Systemd user launch returned1 (`systemd-host-control.log`). No missing substrate was fabricated and the manifest was not weakened.

The archived `local-leg.sh` adapts the actual workflow shell only for absolute paths, existing suffixed dist binaries, unavailable systemd/delegation, and unavailable GNU `/usr/bin/time` (Bash `time -p` locally instead). Its first invocation returned127 at missing GNU time **after successful publish/install/init**. The corrected local-only timing adaptation returned1 at the same truthful cgroup refusal; the completion assertion was not reached. Both logs remain. The production hosted command retains GNU time on Ubuntu.

Thus the repaired hosted leg is designed to prove release-feature-enabled, cold installed-artifact execution through admitted T2 under unchanged declared caps/deadlines. **Neither a new hosted success nor local admitted component completion has been observed.** It does not claim execution from the engine-off published binary, arbitrary-component startup performance, or external-author independence.

## Verification observations

Exact argv, cwd, command exit and wall time for every exercised command are in `commands.jsonl`. Cargo commands inherit `/mnt/build/cargo-maos`, with CI/GITHUB_ACTIONS unset; there is no target override, cargo clean, commit, push, status flip, kernel edit or `.devcontainer` edit.

- Debug WASM daemon/runner builds0; release daemon/runner/worker builds0; final workspace all-target build0; registered fixture builder0; rustfmt of the changed test and workspace fmt check0.
- Workflow regression red101 → green3/3,0. Installed dispatch2/2 and WASM-enabled boot-integrity1/1,0. These cover the installed-boundary equivalents of the multi-tenant gate relevant to this change; no multi-tenant product code changed. Kernel lib253/253,0.
- Full fresh serial nextest (`--profile ci --test-threads 1`): **5550/5550**,138 skipped, exit0; suite916.483s. Before it, compiler process inspection found no cargo/rustc/clippy-driver (ps1), but host load was115.32/81.16/104.78, not idle.
- Fresh serial junit budget gate **exit1**: observer_activity_corpus89.762s, one_daemon_one_door38.265s, participant_recall33.762s, each versus unchanged30s. Σ914.956s across593 binaries. No exception, budget widening or retry-to-green.
- Full unrestricted default nextest **exit100**:5548/5550,138 skipped,65.703s suite. Failures: `maos-bin::shell_host_16_2::repl_clarification_resolves_with_no_control_json` and `maos-kernel-core::t3_terminal_helpers_17_7::teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` (instance18 observer never entered inspect). Same tests passed in the serial run; no unrelated test fix or standalone rerun.
- kloc, kernel baseline, exit commands, decision register, dev-record completeness, epic-close coherence, env contract, serde error handling all0.
- Real non-denying workspace/all-target clippy0; ratchet0 at**1335 occurrences /873 source-distinct**, baseline**1383/897 unchanged**. stdout JSON and stderr captured separately; cargo deny0.
- Prescribed `/mnt/build/cargo-maos/debug/xtask prepush --json` **exit0**,466.475s:81 pass,9 justified skips,1 timing advisory, no red records; tracked tree preserved. Its independently mandated workspace leg passed5550/5550 (junit54.335s), kernel lib and doctests passed. This is not a retry replacing the separately requested default result: both reports remain. The prepush default timing gate reports10 findings as advisory under existing policy; it does not supersede or turn green the separate serial-budget failure.

**Exact ceilings/pins:** maos-bin33498 (≤34410), registry4248 (≤5163), xtask44406, aggregate191826 (≤193768; existing hardfail191827). No charged source changed, so no re-pin was needed. Kernel remains26841 physical /21042 tokei,95 files, set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`, changed/added/removed empty.

Open evidence is the new hosted delegated release completion, unavailable local delegation, the three fresh serial timing findings and the two default-suite failures. None is represented as green.
