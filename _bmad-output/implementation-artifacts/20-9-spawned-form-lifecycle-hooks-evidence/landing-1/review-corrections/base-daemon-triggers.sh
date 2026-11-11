#!/usr/bin/env bash
# Baseline-compatible executable witness (AC1/AC2/AC3): a real `maos run` of the lifecycle-probe component
# under fast clocks with a [[schedule]]. At the pre-20-9 base nothing reaches a lifecycle export, so the
# daemon's own Transparency Log has no `log.recall` row (the probe's recall import runs inside each export)
# and no `schedule.fire:tick` row; at the final tree the same script shows both.
#   usage: base-daemon-triggers.sh <bin-dir> <lifecycle_probe.wasm> <workdir> [seconds]
set -u
BIN=$1; ART=$2; WORK=$3; SECS=${4:-25}
rm -rf "$WORK"; mkdir -p "$WORK/package" "$WORK/home" "$WORK/xdg"
cp "$ART" "$WORK/package/guest.wasm"
export HOME="$WORK/home" MAOS_HOME="$WORK/maos-home" XDG_DATA_HOME="$WORK/xdg" MAOS_NOTIFY_DISABLE=1 MAOS_IDLE_FAST=1 MAOS_SCHEDULE_FAST=1
cat > "$WORK/package/v1.toml" <<EOF
[class]
name = "lifecycle-probe"
description = "lifecycle witness"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0-alpha.1"
forms = ["wasm-component"]
artifact = "guest.wasm"
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
[[schedule]]
id = "tick"
cadence_secs = 1
payload_b64 = "dGljaw=="
principal_revocability = false
EOF
"$BIN/maos" init > "$WORK/init.log" 2>&1
"$BIN/maos" run "$WORK/package/v1.toml" > "$WORK/daemon.out" 2> "$WORK/daemon.err" &
D=$!
for i in $(seq 1 600); do "$BIN/maosctl" spirit inspect lifecycle-probe 2>/dev/null | grep -q '^lifecycle_state: Running' && break; sleep 0.1; done
sleep "$SECS"
kill -TERM "$D" 2>/dev/null; wait "$D" 2>/dev/null
DB="$WORK/maos-home/audit/transparency.sqlite"
echo "== rows by intent (log.recall = a lifecycle export ran its recall import; schedule.fire:tick = the schedule caller)"
python3 - "$DB" <<'P'
import sqlite3,sys,collections
c=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro",uri=True)
n=collections.Counter(r[0] for r in c.execute("select intent from transparency_log"))
for k in ("log.recall","schedule.fire:tick"): print(f"{k}: {n.get(k,0)}")
lim=collections.Counter(r[0] for r in c.execute("select payload_redacted from transparency_log where intent='log.recall'"))
print("recall payloads:",dict(lim))
P
