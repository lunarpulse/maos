#!/usr/bin/env bash
# usage: capture-landing.sh RUN OUTDIR — raw evidence into OUTDIR
set -uo pipefail
REPO=lunarpulse/maos; RUN=$1
H=/tmp/maos-20-15-hosted; PY=$H/.venv/bin/python
METHOD=/home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-15-evidence/creation/job-pricing.method.py
OUT=$2; mkdir -p "$OUT"; cd "$OUT"
gh_ro() { GH_TOKEN="$(cat ~/.config/maos-agent/gh-ci-read.token)" gh "$@"; }
gh_ro api repos/$REPO/actions/runs/$RUN > run.json
SHA=$($PY -c 'import json;print(json.load(open("run.json"))["head_sha"])')
gh_ro api "repos/$REPO/actions/runs/$RUN/jobs?per_page=100" --paginate --jq '.jobs[] | tojson' > jobs.jsonl
rm -rf logs logs.zip; $PY $H/fetch_logs.py
git -C /home/lunarpulse/dev_ws show "$SHA:.github/workflows/discipline.yml" \
  | $PY -c 'import sys,json,yaml; json.dump(yaml.safe_load(sys.stdin), sys.stdout)' > discipline.json
$PY - <<'PY'
import glob, json, os
have = {os.path.basename(f).split("_",1)[1][:-4] for f in glob.glob("logs/*.txt")}
jobs = [json.loads(l) for l in open("jobs.jsonl")]
keep = [j for j in jobs if j["name"] in have and j.get("steps")]
open("jobs.run.jsonl","w").write("".join(json.dumps(j)+"\n" for j in keep))
print("prefilter:", len(keep), "of", len(jobs), "jobs have logs+steps; dropped:", sorted(j["name"] for j in jobs if j not in keep))
PY
$PY "$METHOD" jobs.run.jsonl logs discipline.json > per-job-compile.json
$PY $H/totals.py jobs.jsonl > totals.txt
ls logs/*.txt | grep -E '/[0-9]+_(wasm-host-tests|cgroup-ceiling-(un)?delegated|epic-17-exit|epic-18-exit)( \(.*\))?\.txt$' > consumer-logs.list
{
  echo "consumer logs: $(wc -l < consumer-logs.list)"
  xargs -d '\n' grep -nP '^\S+Z\s+(?:\x1b\[[0-9;]*m)*\s*(Compiling \S+ v\d|Finished\s*(?:\x1b\[[0-9;]*m)*\s*`[^`]+` profile)' < consumer-logs.list; echo "cargo-line grep rc=$? (xargs 123 / grep 1 with no hit lines above = none found = PASS)"
  xargs -d '\n' grep -c 'Compiling' < consumer-logs.list | grep -v ':0$' || echo "no 'Compiling' substring anywhere in consumer logs"
  echo "--- pins: N tests run lines"
  xargs -d '\n' grep -HoP '\d+ tests? run: \d+ passed[^\r\n]*' < consumer-logs.list | sed 's#^logs/##'
  echo "--- pins: marker counts (VERDICT/OUTCOME-style tokens per log)"
  while IFS= read -r f; do printf '%s ' "${f#logs/}"; grep -oP '\b[A-Z][A-Z0-9]+(?:-[A-Z0-9]+)*-(?:VERDICT|OUTCOME|MARKER)\b' "$f" | sort | uniq -c | tr -s ' \n' ' '; echo; done < consumer-logs.list
} > ac1.2.txt 2>&1
echo "captured $OUT ($SHA)"
