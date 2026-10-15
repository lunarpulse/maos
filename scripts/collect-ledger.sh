#!/usr/bin/env bash
# Run from the source checkout; only an operator may provision ledger branches.
set -euo pipefail
kind=${1:?usage: collect-ledger.sh fuzz|rto INPUT_DIRECTORY}
inputs=$(realpath "${2:?input directory}")
case "$kind" in
  fuzz) branch=fuzz-ledger; ledger=fuzz-ledger.json ;;
  rto) branch=rto-ledger; ledger=rto-evidence.toml ;;
  *) echo "::error::unknown collector: $kind" >&2; exit 1 ;;
esac
scratch=$(mktemp -d)
trap 'if [ -d "$scratch/tree" ]; then git worktree remove --force "$scratch/tree"; fi; rm -rf "$scratch"' EXIT

append_records() {
if [ ! -f "$ledger" ]; then
  echo "::error::$ledger missing on $branch — ops-provisioning-secrets-and-accounts must provision it" >&2
  exit 1
fi
if [ "$kind" = fuzz ]; then
  jq -e '.schema_version == 1 and (.records | type == "array")' "$ledger" >/dev/null
  shopt -s nullglob
  records=("$inputs"/record-*.json)
  if [ "${#records[@]}" -eq 0 ]; then
    echo '::error::no fuzz record artifacts' >&2
    exit 1
  fi
  for record in "${records[@]}"; do
    jq -se 'length == 1 and (.[0] | type == "object" and
      (.target | IN("manifest_parser", "frame_deser", "fuzz_exec_deps_parse_elf")) and
      (.cpu_seconds | type == "number" and . >= 0 and floor == .) and
      (.corpus_size | type == "number" and . >= 0 and floor == .) and
      (.timestamp | fromdateiso8601 | type == "number"))' "$record" >/dev/null
    if jq -e --slurpfile rec "$record" '.records | index($rec[0]) != null' "$ledger" >/dev/null; then
      echo "record already present: $record"
      continue
    fi
    # Keep jq separate: an append error MUST stop the collector, not an && list.
    jq --slurpfile rec "$record" '.records += [$rec[0]]' "$ledger" > "$scratch/next.json"
    mv "$scratch/next.json" "$ledger"
  done
else
  python3 - "$ledger" "$inputs/rto-evidence-entry.toml" <<'PY'
import datetime
import pathlib
import sys
import tomllib
ledger, record = map(pathlib.Path, sys.argv[1:])
old = tomllib.loads(ledger.read_text())
new = tomllib.loads(record.read_text())
assert isinstance(old.get("evidence"), list), "malformed RTO ledger"
assert len(new.get("evidence", [])) == 1, "expected exactly one RTO record"
entry = new["evidence"][0]
datetime.date.fromisoformat(entry["drill_date"])
assert type(entry["rto_seconds"]) is int and entry["rto_seconds"] >= 0, "invalid RTO seconds"
assert type(entry["drill_success"]) is bool, "invalid drill success"
if not old["evidence"]:
    ledger.write_text(record.read_text() + "\n")
else:
    with ledger.open("a") as stream:
        stream.write("\n" + record.read_text() + "\n")
PY
fi
}

# Only a concurrent branch advance is retryable; policy/auth/hook failures are not.
for attempt in 1 2 3 4 5; do
  if ! git fetch origin "refs/heads/$branch"; then
    echo "::error::$branch unavailable — ops-provisioning-secrets-and-accounts must provision the ledger branch and write access" >&2
    exit 1
  fi
  if [ "$attempt" -eq 1 ]; then
    git worktree add --detach "$scratch/tree" FETCH_HEAD
    cd "$scratch/tree"
    git config user.name "github-actions[bot]"
    git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
  else
    # Our previous attempt is committed; switch the disposable worktree to the
    # newly fetched tip and reapply the original artifacts with deduplication.
    git checkout --detach FETCH_HEAD
  fi
  append_records
  git add "$ledger"
  if git diff --cached --quiet; then
    echo 'no new records to append'
    exit 0
  fi
  git commit -m "$kind-cadence: append measured evidence (run ${GITHUB_RUN_ID:-local})"
  if LC_ALL=C git push --porcelain origin "HEAD:refs/heads/$branch" > "$scratch/push.out" 2> "$scratch/push.err"; then
    cat "$scratch/push.out" "$scratch/push.err"
    exit 0
  fi
  cat "$scratch/push.out" "$scratch/push.err" >&2
  retry=false
  while IFS=$'\t' read -r flag refspec summary; do
    if [ "$flag" = '!' ] && [[ "$summary" =~ ^\[rejected\]\ \((fetch\ first|non-fast-forward)\)$ ]]; then
      retry=true
    fi
  done < "$scratch/push.out"
  if [ "$retry" != true ] || [ "$attempt" -eq 5 ]; then
    exit 1
  fi
  echo "ledger advanced concurrently; retrying append (attempt $((attempt + 1))/5)"
done
