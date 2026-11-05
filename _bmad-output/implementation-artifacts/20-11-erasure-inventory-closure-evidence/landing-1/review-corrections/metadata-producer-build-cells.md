# Final workspace metadata producer: bounded build cells

The registered final fixture builder exited0, followed by the genuine final native build `cargo build --locked -p maos-bin -p maos-cli` (exit0,370.4357s). No production source changed afterward; final-source control preimages were already restored.

The next tool cell had a2900s deadline and ran, in dependency order:

1. `cargo metadata --locked --format-version 1` — exit0,13.1432s; exact argv/environment in `workspace-final-cargo-metadata.json.command.json`.
2. `cargo nextest list --locked --workspace --message-format json --list-type binaries-only` — compilation did not deliver a completed producer status before the cell deadline. The tool reported `[Command timed out after 2900 seconds]` and `error: interrupted`; total observed tool wall2902.09s. No numeric Cargo/nextest exit status was delivered for this interrupted producer, so none is invented.

Raw build diagnostics are retained in `workspace-final-binaries-metadata.stderr`. This was a build/metadata cell, **not a workspace test execution**: no tests are counted as run or passed by it. An identical build-only producer was continued in a new2900s cell using the warm Cargo artifacts, writing `workspace-final-binaries-metadata-resumed.{json,stderr}` rather than overwriting the interrupted evidence. No package/target filtering, test retry, runtime-concurrency change or larger cell deadline is introduced by that continuation.

Before continuation, `ps -C cargo,rustc -o pid,ppid,stat,etime,args` exited1 because the harness ps implements only a subset of procps flags. A direct `/proc` scan of process comm/status/cwd/cmdline then observed **no cargo or rustc processes**. Only after that observation was the producer restarted. This process check is interruption/ownership diagnosis, not polling an active job.

The default workspace test-target set is intentional. The earlier `--all-targets` metadata included a custom benchmark whose MAOS-PERF output is not a libtest listing; that exit104 was a setup error, not a feature failure. The normal workspace producer has no `--all-targets`, matching ordinary `cargo nextest --workspace`; all-target compilation/clippy was separately exercised. Neither this setup correction nor the bounded compilation continuation is claimed as the one actual unfiltered workspace nextest run.

## Second completed compilation cell

The identical `cargo nextest list --locked --workspace --message-format json --list-type binaries-only` continuation also reached its2900s tool deadline, observed wall2900.20s, with `error: interrupted` and no delivered numeric Cargo/nextest exit. Its retained stderr ends with compilation of `maos-cli`; the earlier cell ended among the kernel/Spirit/PDP packages. Searches of both completed stderr files found no lines beginning `error:`/`error[`. This records observed build progress/diagnostics, not a claim that compilation or tests passed.

After interruption a fresh `/proc` comm/status/cmdline scan observed no cargo, rustc or cargo-nextest processes. A resource probe reported32 visible CPUs (`nproc`), `cpu.max = max 100000`, and `memory.max = 51539607552` (48GiB). The environment probe had no inherited CARGO_BUILD_JOBS. These are resource facts, not proof of the timeout's cause.

The next build-only cell explicitly uses `env CARGO_BUILD_JOBS=8 cargo nextest list --locked --workspace --message-format json --list-type binaries-only`, retaining the2900s deadline and the complete default workspace target set. This is compiler-job control only: no nextest runtime thread count, test filter, threshold, deadline or retry policy changes. Exact output is `workspace-final-binaries-metadata-j8.{json,stderr}`; it does not overwrite either interrupted cell's evidence.

## Completed final producer

The explicit-eight-compiler-job cell completed **exit0**,762.2965s (tool wall765.18s). Its exact command and environment are in `workspace-final-binaries-metadata-j8.json.command.json`. The fresh Cargo metadata contains58 workspace members. The completed binaries metadata contains669 default test targets:56 lib,596 integration-test,2 proc-macro and15 bin targets; no bench/example targets. These match the normal workspace target set, not a narrowed package filter.

The actual unfiltered workspace consumer uses this completed metadata plus the fresh Cargo metadata, with no package/build filter and no nextest thread override. It explicitly disables retries. The producer's CARGO_BUILD_JOBS=8 setting is not applied as a test-concurrency setting. The two interrupted compilation cells remain failed/interrupted evidence, never test passes.
