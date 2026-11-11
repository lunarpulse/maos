#!/usr/bin/env bash
# 20-9 L1 failing-before capture: drive the REAL operator door of a `maos` build (HEAD fc7373f0
# in a scratch copy; the binaries sit in a scratch target dir, never the shared one) with a
# spawned `wasm-component` Spirit, then ask for a hot swap and a cold swap, as
# `spawned_door_20_9.rs` does on the final tree.
#   usage: head-door.sh <bin-dir> <artifact.wasm> <workdir>
set -u
BIN=$1; ART=$2; WORK=$3
rm -rf "$WORK"; mkdir -p "$WORK/package" "$WORK/home" "$WORK/xdg"
cp "$ART" "$WORK/package/guest.wasm"
export HOME="$WORK/home" MAOS_HOME="$WORK/maos-home" XDG_DATA_HOME="$WORK/xdg" MAOS_NOTIFY_DISABLE=1
manifest() { # version artifact
cat <<EOF
[class]
name = "lifecycle-probe"
description = "lifecycle witness"
version = "$1"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0-alpha.1"
forms = ["wasm-component"]
artifact = "$2"
trust_tier = "local"
[posture]
default = "assistive"
allowed_max = "assistive"
[output_shape]
required_fields = ["introduction"]
[budget]
context_window_size = 4096
time_cap_seconds = 30
[resources]
fd_max = 64
[sandbox]
tier = "T2"
[scheduling]
priority_weight = 100
idle_window_ms = 2000
[lifecycle]
enabled_hooks = []
[capabilities.required.iac]
send = ["spirit:peer"]
[capabilities.required.log]
recall = true
EOF
}
manifest 1.0.0 guest.wasm > "$WORK/package/v1.toml"
manifest 1.1.0 guest.wasm > "$WORK/package/v2.toml"
"$BIN/maos" init > "$WORK/init.log" 2>&1
"$BIN/maos" run "$WORK/package/v1.toml" > "$WORK/daemon.out" 2> "$WORK/daemon.err" &
DAEMON=$!
for i in $(seq 1 600); do
  if "$BIN/maosctl" spirit inspect lifecycle-probe 2>/dev/null | grep -q '^lifecycle_state: Running'; then break; fi
  sleep 0.1
done
echo "== inspect"; "$BIN/maosctl" spirit inspect lifecycle-probe 2>&1
for policy in hot-swap cold-swap; do
  echo "== upgrade --policy $policy"
  "$BIN/maosctl" spirit upgrade lifecycle-probe --to "$WORK/package/v2.toml" --policy "$policy" 2>&1
  echo "exit=$?"
done
echo "== inspect after"; "$BIN/maosctl" spirit inspect lifecycle-probe 2>&1
kill "$DAEMON" 2>/dev/null; wait "$DAEMON" 2>/dev/null
echo "== daemon stderr (tail)"; tail -n 5 "$WORK/daemon.err"
