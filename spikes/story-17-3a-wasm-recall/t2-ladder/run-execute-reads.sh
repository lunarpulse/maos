#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
ladder="$repo_root/target/debug/t2-ladder"
hello="$repo_root/target/t2-ladder/hello-dynamic"
runner="$repo_root/spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59"
component="$repo_root/tests/fixtures/wasm/echo_spirit_component.wasm"
frame="$repo_root/target/t2-ladder/runner.frame"

printf 'UTC %s\n' "$(date -u +%FT%TZ)"
printf '+ git rev-parse HEAD\n'
git -C "$repo_root" rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: real dynamic hello and the HEAD runner plus their ldd paths and component; not only probe-created state.\n'
printf '+ t2-ladder --landlock rights --execute-reads --seccomp off -- target/t2-ladder/hello-dynamic\n'
"$ladder" --landlock rights --execute-reads --seccomp off -- "$hello"
printf '+ t2-ladder --landlock rights --execute-reads --seccomp off --read tests/fixtures/wasm/echo_spirit_component.wasm --stdin target/t2-ladder/runner.frame -- target/debug/maos-wasm-runner --component tests/fixtures/wasm/echo_spirit_component.wasm\n'
"$ladder" --landlock rights --execute-reads --seccomp off --read "$component" --stdin "$frame" -- "$runner" --component "$component"
