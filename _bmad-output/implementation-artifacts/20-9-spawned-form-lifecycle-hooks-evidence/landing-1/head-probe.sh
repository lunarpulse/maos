#!/usr/bin/env bash
# 20-9 AC1 failing-before: HEAD's runner against the 2.4.0 lifecycle-probe component.
#   usage: head-probe.sh <bin-dir> <lifecycle_probe.wasm> <workdir>
set -u
BIN=$1; ART=$2; WORK=$3
rm -rf "$WORK"; mkdir -p "$WORK/package" "$WORK/home" "$WORK/xdg"
cp "$ART" "$WORK/package/guest.wasm"
export HOME="$WORK/home" MAOS_HOME="$WORK/maos-home" XDG_DATA_HOME="$WORK/xdg" MAOS_NOTIFY_DISABLE=1
sed -n '/^manifest()/,/^}/p' "$(dirname "$0")/head-door.sh" > "$WORK/m.sh"; . "$WORK/m.sh"
manifest 1.0.0 guest.wasm > "$WORK/package/v1.toml"
"$BIN/maos" init > "$WORK/init.log" 2>&1
timeout 60 "$BIN/maos" run "$WORK/package/v1.toml" > "$WORK/daemon.out" 2> "$WORK/daemon.err"
echo "maos run exit=$?"; echo "== stdout"; cat "$WORK/daemon.out"; echo "== stderr (tail)"; tail -n 6 "$WORK/daemon.err"
