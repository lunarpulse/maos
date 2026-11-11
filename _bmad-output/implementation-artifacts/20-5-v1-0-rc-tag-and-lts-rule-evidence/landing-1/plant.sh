#!/usr/bin/env bash
# Plant recipe used for the 20-5 L1 plants (copy-back + cmp + touch). Usage:
#   plant.sh NAME FILE LOG -- <cargo command...>   (the plant edit itself is applied beforehand by /tmp/d205/plant.py)
# The pristine copy lives in /tmp/d205/plants/NAME.orig; restore.sh copies it back with cp -p, cmp's it, touches it.
set -uo pipefail
name=$1; file=$2; log=$3; shift 4
{
  echo "## plant $name on $file"
  diff -u "/tmp/d205/plants/$name.orig" "$file" || true
  echo "## command: $*"
  "$@"
  echo "exit=$?"
} > "$log" 2>&1
/tmp/d205/restore.sh "$name" "$file" >> "$log" 2>&1
