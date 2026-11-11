#!/usr/bin/env bash
# 20-9 L1 review-correction plants. usage: plants.sh <label> <nextest-filter> <file>=<old>@@<new> [<file>=<old>@@<new> ...]
# Each mutation must match exactly once. The proof runs in the final tree; every file is then restored
# byte-for-byte from a backup taken before the plant (copy-back + cmp + touch). Raw log: plants/<label>.log
set -u
cd /home/lunarpulse/dev_ws
LABEL=$1; FILTER=$2; shift 2
R=_bmad-output/implementation-artifacts/20-9-spawned-form-lifecycle-hooks-evidence/landing-1/review-corrections
OUT=${PLANT_DIR:-$R/plants}/$LABEL.log
BK=/tmp/209-plantbk-$LABEL; rm -rf "$BK"; mkdir -p "$BK"
: > "$OUT"; FILES=()
for spec in "$@"; do
  f=${spec%%=*}; rest=${spec#*=}; old=${rest%%@@*}; new=${rest#*@@}
  mkdir -p "$BK/$(dirname "$f")"; [ -e "$BK/$f" ] || cp -p "$f" "$BK/$f"; FILES+=("$f")
  python3 - "$f" "$old" "$new" <<'P' >> "$OUT" 2>&1 || { echo "# PLANT DID NOT APPLY" >> "$OUT"; }
import sys
f,old,new=sys.argv[1:4]; s=open(f).read()
assert s.count(old)==1, f"{f}: {old!r} matches {s.count(old)} times"
open(f,'w').write(s.replace(old,new)); print(f"# planted {f}: {old!r} @@ {new!r}")
P
done
{ echo "# plant $LABEL"; for f in "${FILES[@]}"; do echo "# diff $f"; diff "$BK/$f" "$f"; done; } >> "$OUT"
echo "# command: nextest -E '$FILTER'" >> "$OUT"
source /tmp/verify209/venv/bin/activate 2>/dev/null
env -u CI -u GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE=undelegated cargo nextest run --locked -p maos-bin --features maos-bin/network,maos-bin/wasm-host --no-fail-fast --profile ci -E "$FILTER" >> "$OUT" 2>&1
echo "# exit=$?" >> "$OUT"
for f in "${FILES[@]}"; do cp -p "$BK/$f" "$f" && cmp "$BK/$f" "$f" && touch "$f" && echo "# restored $f: cmp clean, touched" >> "$OUT"; done
grep -E "^\s+(FAIL|PASS)|Summary|restored|DID NOT" "$OUT" | sort -u | tail -8
