#!/usr/bin/env bash
# 20-4 creation — AC1 selector line run against the prebuilt binary (no cargo).
# Isolated HOME/XDG under /tmp/c204/probe (deleted at the end).
# Usage: selector-probe.sh <repo-root> <bin-dir>
set -u
ROOT=${1:?repo root}; BIN=${2:?bin dir}
W=/tmp/c204/probe; rm -rf "$W"; mkdir -p "$W/home" "$W/run"; chmod 700 "$W/run"
ENVX=(env -i PATH=/usr/bin:/bin HOME="$W/home" XDG_DATA_HOME="$W/home/.local/share"
  XDG_CONFIG_HOME="$W/home/.config" XDG_RUNTIME_DIR="$W/run" MAOS_OLLAMA_URL=skip)
row() { printf '\n### %s\n$ %s\nrc=%s\n' "$1" "$2" "$3"; sed -n '1,40p' "$4"; }
run() { local label=$1 input=$2; shift 2; local rc
  printf '%b' "$input" | timeout 40 "$@" >"$W/out" 2>&1; rc=$?
  row "$label" "$*" "$rc" "$W/out"; }
echo "# 20-4 selector probe @ $(git -C "$ROOT" rev-parse --short HEAD); binary:"
stat -c '%n %y' "$BIN/maos"
cd "$ROOT"
run "init" "" "${ENVX[@]}" "$BIN/maos" init
J0=crates/maos-journey-test/cassettes/j0/shell-intro.json
G=crates/maos-journey-test/cassettes/providers/gemini-shell-intro.json
run "S1 epic AC1 line (gemini cassette path)" "@hello-spirit hello\n" "${ENVX[@]}" MAOS_DEFAULT_PROVIDER=gemini MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=$G "$BIN/maos" shell
run "S2 selector with the existing J0 cassette" "@hello-spirit hello\n" "${ENVX[@]}" MAOS_DEFAULT_PROVIDER=gemini MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=$J0 "$BIN/maos" shell
run "S3 control: no selector" "@hello-spirit hello\n" "${ENVX[@]}" MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=$J0 "$BIN/maos" shell
echo; echo "### journal rows naming an inference intent (strings over every file under HOME)"
python3 -c "import sqlite3,sys,re;c=sqlite3.connect(sys.argv[1]);print([(n,) for (n,) in c.execute(\"select name from sqlite_master where type='table'\")]);[print(t,r) for t in [x for (x,) in c.execute(\"select name from sqlite_master where type='table'\")] for r in c.execute('select * from '+t) if re.search(r'infer:', str(r))]" "$W/home/.local/share/maos/audit/transparency.sqlite" 2>&1 | cut -c1-400 | head -20
echo; echo "### files under HOME"; find "$W/home" -type f | sed "s|$W||" | head -30
rm -rf "$W"
