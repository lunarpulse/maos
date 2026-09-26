#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
private_dir="$repo_root/spikes/story-17-3a-wasm-recall/t2-ladder/target"
private_runner="$private_dir/maos-wasm-runner-head-92911f59"

printf 'UTC %s\n' "$(date -u +%FT%TZ)"
printf '+ git rev-parse HEAD\n'
git -C "$repo_root" rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: the main-tree HEAD maos-wasm-runner rebuilt with its locked dependency graph; not only probe-created state.\n'
printf '+ cargo build --locked -p maos-wasm-host --bin maos-wasm-runner\n'
cargo build --locked -p maos-wasm-host --bin maos-wasm-runner
mkdir -p "$private_dir"
printf '+ cp target/debug/maos-wasm-runner spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59\n'
cp "$repo_root/target/debug/maos-wasm-runner" "$private_runner"
printf '+ sha256sum spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59\n'
sha256sum "$private_runner"
printf 'PRIVATE_HEAD_RUNNER=%s\n' "$private_runner"
