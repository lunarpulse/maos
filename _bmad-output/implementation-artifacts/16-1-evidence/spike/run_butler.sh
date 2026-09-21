#!/bin/sh
# usage: run_butler.sh <run-name> <seconds> [extra env assignments...]
WT=/home/lunarpulse/dev_ws/maos/.claude/worktrees/agent-aaa368320359a03ac
S=/tmp/claude-1000/-home-lunarpulse-dev-ws-maos/4e42f963-4b21-4969-b033-00b910f44c64/scratchpad
NAME=$1; SECS=$2; shift 2
R=$S/$NAME
rm -rf "$R"; mkdir -p "$R/home" "$R/maoshome" "$R/xdg" "$R/tmp"
cd "$WT"
START=$(date +%s)
env -i PATH=/usr/bin:/bin HOME="$R/home" MAOS_HOME="$R/maoshome" XDG_DATA_HOME="$R/xdg" \
  MAOS_AUDIT_DB="$R/audit.sqlite" MAOS_ARCHIVE_DIR="$R/archive" TMPDIR="$R/tmp" \
  MAOS_INFERENCE_MODE=replay \
  MAOS_REPLAY_CASSETTE=${CASSETTE:-crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json} \
  "$@" \
  timeout -s TERM "$SECS" ./target/debug/maos run ${MANIFEST:-spirits/butler/manifest.toml} \
  > "$R/stdout.log" 2> "$R/stderr.log"
RC=$?
END=$(date +%s)
echo "EXIT $RC elapsed $((END-START))s"
