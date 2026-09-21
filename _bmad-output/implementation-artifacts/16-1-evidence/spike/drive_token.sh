#!/bin/sh
# usage: drive_token.sh <run-name> <secs>   (MANIFEST / CASSETTE env pass through)
S=/tmp/claude-1000/-home-lunarpulse-dev-ws-maos/4e42f963-4b21-4969-b033-00b910f44c64/scratchpad
NAME=$1; SECS=$2
TOK=$S/tokens/$NAME.hex
mkdir -p $S/tokens; rm -f "$TOK"
sh $S/run_butler.sh "$NAME" "$SECS" MAOS_SPIKE_16_1=1 MAOS_SPIKE_TOKEN_FILE="$TOK" &
BG=$!
sleep ${WAIT:-4}
echo "--- TL mid-run (daemon still up) ---"
sqlite3 "$S/$NAME/maoshome/audit/transparency.sqlite" \
  "select spirit_pid, intent, hex(capability_token) from transparency_log where capability_token is not null"
ID=$(sqlite3 "$S/$NAME/maoshome/audit/transparency.sqlite" \
  "select lower(hex(substr(capability_token,1,16))) from transparency_log where intent like 'cap.issue%' order by timestamp_ns limit 1")
echo "token id read from TL: '$ID'"
printf '%s' "$ID" > "$TOK"
wait $BG
