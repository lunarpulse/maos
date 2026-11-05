#!/usr/bin/env bash
# Story 20-2: shipped default binary, real replay session and TL growth.
# No local missing-prerequisite result is evidence. CI cannot green-skip.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
missing() {
  echo "NOT RUN: $*" >&2
  if [[ ${CI:-} == true || ${GITHUB_ACTIONS:-} == true ]]; then exit 1; fi
  exit 77
}
if [[ ${1:-} == --workload ]]; then
  shift
  binary=$1; cassette=$2; out=$3; plant=${4:-}
  export HOME="$out/home" MAOS_HOME="$out/home/maos" XDG_DATA_HOME="$out/home/data"
  export XDG_CONFIG_HOME="$out/home/config" XDG_STATE_HOME="$out/home/state" XDG_RUNTIME_DIR="$out/home/runtime"
  export MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE="$cassette" MAOS_NOTIFY_DISABLE=1
  unset MAOS_ANTHROPIC_API_KEY MAOS_OPENAI_API_KEY ANTHROPIC_API_KEY OPENAI_API_KEY MAOS_OLLAMA_URL
  mkdir -p "$HOME" "$XDG_RUNTIME_DIR"
  chmod 700 "$XDG_RUNTIME_DIR"
  "$binary" init --plain
  python3 "$root/tests/air-gap-trace.py" rows "$MAOS_HOME/audit/transparency.sqlite" > "$out/before"
  if [[ $plant == --plant-outbound ]]; then
    python3 -c 'import socket; s=socket.socket(); s.settimeout(1); print("outbound plant"); rc=s.connect_ex(("198.51.100.1",443)); print(rc); s.close()'
  fi
  printf '@hello-spirit hello\n' | "$binary" shell --plain > "$out/session.log" 2>&1
  cat "$out/session.log"
  python3 "$root/tests/air-gap-trace.py" rows "$MAOS_HOME/audit/transparency.sqlite" > "$out/after"
  python3 - "$out" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
assert int((p/'after').read_text()) > int((p/'before').read_text()), 'no new session TL entry'
assert 'Hello! I am the MAOS hello-spirit reference implementation.' in (p/'session.log').read_text(), 'missing replay response'
print('PASS: expected replay response and new Transparency Log entries')
PY
  exit 0
fi
binary=${1:-$root/dist/maos-linux-amd64}
cassette="$root/crates/maos-journey-test/cassettes/j0/shell-intro.json"
for tool in unshare setpriv strace sudo python3; do command -v "$tool" >/dev/null || missing "$tool unavailable"; done
[[ -x $binary ]] || missing "executable $binary unavailable"
[[ -f $cassette ]] || missing "cassette unavailable"
sudo -n unshare -n -- setpriv --reuid="$(id -u)" --regid="$(id -g)" --clear-groups true || missing 'network namespace privilege unavailable'
binary=$(realpath "$binary")
out=${2:-$(mktemp -d /tmp/maos-netns.XXXXXX)}
mkdir -p "$out"; out=$(realpath "$out")
# Only namespace creation is privileged. Tracer, canary and MAOS run as runner uid.
prefix=(sudo -n unshare -n -- setpriv --reuid="$(id -u)" --regid="$(id -g)" --clear-groups)
"${prefix[@]}" strace -f -yy -s 2048 -e trace=network,sendmmsg,io_uring_enter -o "$out/canary.trace" \
  python3 -c 'import socket; s=socket.socket(); s.settimeout(1); s.connect_ex(("198.51.100.1",443)); s.close()'
python3 "$root/tests/air-gap-trace.py" canary "$out/canary.trace"
"${prefix[@]}" strace -f -yy -s 2048 -e trace=network,sendmmsg,io_uring_enter -o "$out/workload.trace" \
  bash "$0" --workload "$binary" "$cassette" "$out" "${3:-}"
python3 "$root/tests/air-gap-trace.py" workload "$out/workload.trace"
echo "PASS: no outbound attempts in the witnessed replay session; journal grew; traces $out"
