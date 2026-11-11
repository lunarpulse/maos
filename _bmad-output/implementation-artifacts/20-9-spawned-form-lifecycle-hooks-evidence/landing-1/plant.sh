#!/usr/bin/env bash
# 20-9 L1 plant harness. usage: plant.sh <label> <file> <log> -- <command...>
# The plant itself is applied beforehand by the caller; this runs the proof, keeps the raw log,
# then restores the file byte-for-byte from /tmp/209-plants (copy-back + cmp + touch).
set -u
LABEL=$1; FILE=$2; LOG=$3; shift 4
BACKUP=/tmp/209-plants/$(echo "$FILE" | tr / _)
{ echo "# plant $LABEL on $FILE"; echo "# diff vs final tree:"; diff "$BACKUP" "$FILE"; echo "# command: $*"; } > "$LOG"
"$@" >> "$LOG" 2>&1; echo "# exit=$?" >> "$LOG"
cp -p "$BACKUP" "$FILE" && cmp "$BACKUP" "$FILE" && touch "$FILE" && echo "# restored: cmp clean, touched" >> "$LOG"
tail -n 3 "$LOG"
