#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
ladder_dir="$repo_root/spikes/story-17-3a-wasm-recall/t2-ladder"
probe_target="$repo_root/target/t2-ladder"
runner="$ladder_dir/target/maos-wasm-runner-head-92911f59"
component="$repo_root/tests/fixtures/wasm/echo_spirit_component.wasm"

printf 'UTC %s\n' "$(date -u +%FT%TZ)"
printf '+ git rev-parse HEAD\n'
git -C "$repo_root" rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: real Rust executables, the HEAD runner, the pinned component, dynamic loader, and ldd-resolved libraries; not only probe-created state.\n'
mkdir -p "$probe_target"
printf '+ rustc --crate-name ladder_hello hello.rs.txt -o target/t2-ladder/hello-dynamic\n'
rustc --crate-name ladder_hello "$ladder_dir/hello.rs.txt" -o "$probe_target/hello-dynamic"
printf '+ rustc --crate-name ladder_hello -C target-feature=+crt-static hello.rs.txt -o target/t2-ladder/hello-static\n'
rustc --crate-name ladder_hello -C target-feature=+crt-static "$ladder_dir/hello.rs.txt" -o "$probe_target/hello-static"
printf '+ cargo run --offline --manifest-path t2-ladder/Cargo.toml --bin framegen > target/t2-ladder/runner.frame\n'
cargo run --offline --quiet --manifest-path "$ladder_dir/Cargo.toml" --bin framegen > "$probe_target/runner.frame"
printf '+ ldd target/t2-ladder/hello-dynamic; ldd target/t2-ladder/hello-static\n'
ldd "$probe_target/hello-dynamic"
ldd "$probe_target/hello-static"
printf '+ t2-ladder --landlock off --seccomp off -- target/t2-ladder/hello-dynamic\n'
"$repo_root/target/debug/t2-ladder" --landlock off --seccomp off -- "$probe_target/hello-dynamic"
printf '+ t2-ladder --landlock as-is --seccomp off -- target/t2-ladder/hello-dynamic\n'
"$repo_root/target/debug/t2-ladder" --landlock as-is --seccomp off -- "$probe_target/hello-dynamic"
printf '+ t2-ladder --landlock rights --seccomp off -- target/t2-ladder/hello-dynamic\n'
"$repo_root/target/debug/t2-ladder" --landlock rights --seccomp off -- "$probe_target/hello-dynamic"
printf '+ t2-ladder --landlock rights --static-target --seccomp off -- target/t2-ladder/hello-static\n'
"$repo_root/target/debug/t2-ladder" --landlock rights --static-target --seccomp off -- "$probe_target/hello-static"
printf '+ t2-ladder --landlock off --seccomp off --stdin target/t2-ladder/runner.frame -- spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59 --component tests/fixtures/wasm/echo_spirit_component.wasm\n'
"$repo_root/target/debug/t2-ladder" --landlock off --seccomp off --stdin "$probe_target/runner.frame" -- "$runner" --component "$component"
printf '+ t2-ladder --landlock as-is --seccomp off --stdin target/t2-ladder/runner.frame -- spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59 --component tests/fixtures/wasm/echo_spirit_component.wasm\n'
"$repo_root/target/debug/t2-ladder" --landlock as-is --seccomp off --stdin "$probe_target/runner.frame" -- "$runner" --component "$component"
printf '+ t2-ladder --landlock rights --seccomp off --read tests/fixtures/wasm/echo_spirit_component.wasm --stdin target/t2-ladder/runner.frame -- spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59 --component tests/fixtures/wasm/echo_spirit_component.wasm\n'
"$repo_root/target/debug/t2-ladder" --landlock rights --seccomp off --read "$component" --stdin "$probe_target/runner.frame" -- "$runner" --component "$component"
