#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
ladder="$repo_root/target/debug/t2-ladder"
spike_runner="$repo_root/target/release/spike-runner"
recall_service="$repo_root/target/release/recall-service"
frame_driver="$repo_root/target/release/frame-driver"
guest="$repo_root/target/wasm32-wasip2/release/story_17_3a_recall_guest.wasm"
seccomp=${T2_SECCOMP:-off}
seccomp_order=${T2_SECCOMP_ORDER:-verbatim}
extra=${T2_RUNNER_EXTRAS:-}
ladder_args=(--landlock rights --execute-loader --seccomp "$seccomp" --seccomp-order "$seccomp_order")
if [ "$seccomp" = candidate ]; then
    if [ -z "$extra" ]; then
        printf 'T2_RUNNER_EXTRAS is required with T2_SECCOMP=candidate\n'
        exit 64
    fi
    ladder_args+=(--extra "$extra")
fi
workdir=$(mktemp -d)
fifo="$workdir/runner.stdin"
socket="$workdir/recall.sock"
pid_file="$workdir/runner.pid"
ids_file="$workdir/ids.json"
ladder_pid=''
service_pid=''
cleanup() {
    set +e
    if [ -n "$ladder_pid" ] && kill -0 "$ladder_pid" 2>/dev/null; then
        kill "$ladder_pid"
        wait "$ladder_pid"
    fi
    if [ -n "$service_pid" ] && kill -0 "$service_pid" 2>/dev/null; then
        kill "$service_pid"
        wait "$service_pid"
    fi
    rm -rf "$workdir"
}
trap cleanup EXIT

printf 'UTC %s\n' "$(date -u +%FT%TZ)"
printf '+ git rev-parse HEAD\n'
git -C "$repo_root" rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: real spike-runner, Rust recall guest, recall-service, socket path, and seeded log frames; not only probe-created state.\n'
printf '+ mkfifo %s\n' "$fifo"
mkfifo "$fifo"
printf '+ t2-ladder --landlock rights --execute-loader --seccomp %s --seccomp-order %s --read guest --stdin fifo --pid-out pid -- spike-runner --component guest --recall-socket socket\n' "$seccomp" "$seccomp_order"
"$ladder" "${ladder_args[@]}" --read "$guest" --stdin "$fifo" --pid-out "$pid_file" -- "$spike_runner" --component "$guest" --recall-socket "$socket" >"$workdir/ladder.out" 2>"$workdir/ladder.err" &
ladder_pid=$!
exec 3>"$fifo"
for _ in $(seq 1 50); do
    if [ -s "$pid_file" ]; then break; fi
    sleep 0.1
done
if [ ! -s "$pid_file" ]; then
    printf 'RUNNER_PID_UNAVAILABLE\n'
    exit 70
fi
runner_pid=$(cat "$pid_file")
printf 'RUNNER_PID=%s\n' "$runner_pid"
printf '+ timeout 15s recall-service --socket %s --caller-pid %s --seed-pids %s,424242 --ids-out %s --one-shot\n' "$socket" "$runner_pid" "$runner_pid" "$ids_file"
timeout 15s "$recall_service" --socket "$socket" --caller-pid "$runner_pid" --seed-pids "$runner_pid,424242" --ids-out "$ids_file" --one-shot >"$workdir/service.out" 2>"$workdir/service.err" &
service_pid=$!
for _ in $(seq 1 50); do
    if [ -s "$ids_file" ]; then break; fi
    sleep 0.1
done
if [ ! -s "$ids_file" ]; then
    printf 'RECALL_SERVICE_UNAVAILABLE\n'
    exit 70
fi
frame_id=$(python3 - "$ids_file" "$runner_pid" <<'PY'
import json
import sys
print(json.load(open(sys.argv[1], encoding="utf-8"))[sys.argv[2]])
PY
)
printf 'SEED_FRAME_ID=%s\n' "$frame_id"
printf '+ frame-driver encode --data %s >&3\n' "$frame_id"
"$frame_driver" encode --data "$frame_id" >&3
exec 3>&-
if wait "$ladder_pid"; then
    ladder_rc=0
else
    ladder_rc=$?
fi
if wait "$service_pid"; then
    service_rc=0
else
    service_rc=$?
fi
printf 'LADDER_RC=%s SERVICE_RC=%s\n' "$ladder_rc" "$service_rc"
printf '+ frame-driver decode < extracted ladder output\n'
python3 - "$workdir/ladder.out" <<'PY' | "$frame_driver" decode
import pathlib
import sys
raw = pathlib.Path(sys.argv[1]).read_bytes()
start = raw.index(b"Content-Length: ")
header_end = raw.index(b"\r\n\r\n", start) + 4
length = int(raw[start:header_end].split(b":", 1)[1].strip())
sys.stdout.buffer.write(raw[start:header_end + length])
PY
printf '+ cat service stdout and stderr\n'
cat "$workdir/service.out"
cat "$workdir/service.err"
printf '+ cat ladder stderr and RESULT\n'
cat "$workdir/ladder.err"
python3 - "$workdir/ladder.out" <<'PY'
import pathlib
import sys
text = pathlib.Path(sys.argv[1]).read_bytes().decode("utf-8", "replace")
for line in text.splitlines():
    if line.startswith("RESULT "):
        print(line)
PY
