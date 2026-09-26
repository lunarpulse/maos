#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
printf 'UTC %s\n' "$(date -u +%FT%TZ)"
printf '+ git rev-parse HEAD\n'
git -C "$repo_root" rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: the real compiled hello target and current kernel seccomp nesting state; not only probe-created state.\n'
printf '+ t2-ladder --landlock off --seccomp candidate --extra rust-start -- target/t2-ladder/hello-dynamic\n'
"$repo_root/target/debug/t2-ladder" --landlock off --seccomp candidate --extra rust-start -- "$repo_root/target/t2-ladder/hello-dynamic"
