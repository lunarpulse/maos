#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
count=${Q4_COUNT:-1}
fetch_owner=${Q4_FETCH_OWNER:-A}
fuel=${Q4_FUEL:-1000000000}
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
fifo="$tmpdir/input"
socket="$tmpdir/recall.sock"
ids="$tmpdir/ids.json"
mkfifo "$fifo"

"$root/target/release/spike-runner" \
  --component "$root/target/wasm32-wasip2/release/story_17_3a_recall_guest.wasm" \
  --fuel "$fuel" --recall-socket "$socket" <"$fifo" >"$tmpdir/runner.out" 2>"$tmpdir/runner.err" &
runner_pid=$!

mkfifo "$tmpdir/ready"
"$root/target/release/recall-service" \
  --socket "$socket" --caller-pid "$runner_pid" --seed-pids "$runner_pid,424242" \
  --ids-out "$ids" --one-shot >"$tmpdir/ready" 2>"$tmpdir/service.err" &
service_pid=$!
IFS= read -r ready_line <"$tmpdir/ready"
printf '%s\n' "$ready_line"

if [[ "$fetch_owner" == A ]]; then
  id=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$ids" "$runner_pid")
elif [[ "$fetch_owner" == B ]]; then
  id=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$ids" 424242)
else
  printf 'Q4_FETCH_OWNER must be A or B\n' >&2
  exit 2
fi
"$root/target/release/frame-driver" encode --data "$id" --count "$count" >"$fifo"
wait "$runner_pid"
wait "$service_pid"
"$root/target/release/frame-driver" decode <"$tmpdir/runner.out"
printf '%s\n' '-- runner stderr --'
cat "$tmpdir/runner.err"
printf '%s\n' '-- service stderr --'
cat "$tmpdir/service.err"
