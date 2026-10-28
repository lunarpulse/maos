#!/usr/bin/env bash
# usage: proof.sh OUTDIR — epic17-proof.txt, failures.txt, dirty.txt (only if any dirty refusal) in OUTDIR
set -uo pipefail
cd "$1"
grep -H 'checkout is dirty' logs/*.txt > dirty.txt || rm -f dirty.txt
{
  echo "dirty-checkout refusals across all $(ls logs/*.txt | wc -l) job logs: $(cat dirty.txt 2>/dev/null | wc -l)"
  for f in logs/*epic-17-exit*; do
    echo "== ${f#logs/}"
    grep -nE '##\[notice\]rc=|##\[error\]|maos-wasm-host::t2_sandbox_kill|worker_terminal_facts_17_7 (runtime_scenes::fallback_oom_marker|s4_oom_marker)|Summary \[|##\[notice\]LINE-2B|^[^ ]+ SKIP' "$f" | grep -E 'PASS|FAIL|SKIP|Summary|notice|error' | cut -c1-260
    echo "  'checkout is dirty' hits: $(grep -c 'checkout is dirty' "$f")"
  done
} > epic17-proof.txt
../.venv/bin/python - <<'PY' > failures.txt
import json
for l in open("jobs.jsonl"):
    j = json.loads(l)
    if j["conclusion"] == "failure":
        print(j["name"], "| failed steps:", [s["name"] for s in j["steps"] if s["conclusion"] == "failure"])
PY
cat epic17-proof.txt failures.txt
