#!/usr/bin/env bash
# Existing one-hour wait, but never accept another tag/run or an older attempt.
set -euo pipefail
sha=$1; tag=$2
binding=''
for poll in $(seq 1 180); do
  if [[ -z $binding ]]; then
    runs=$(gh api "repos/$GITHUB_REPOSITORY/actions/workflows/release.yml/runs?event=push&head_sha=$sha&per_page=100")
    set +e
    binding=$(python3 packaging/release-run.py select "$sha" "$tag" <<< "$runs")
    rc=$?
    set -e
    [[ $rc == 0 || $rc == 3 ]] || exit "$rc"
  fi
  if [[ -n $binding ]]; then
    read -r run_id attempt <<< "$binding"
    current=$(gh api "repos/$GITHUB_REPOSITORY/actions/runs/$run_id")
    set +e
    python3 packaging/release-run.py check "$sha" "$tag" "$run_id" "$attempt" <<< "$current"
    rc=$?
    set -e
    [[ $rc == 0 || $rc == 3 ]] || exit "$rc"
    if [[ $rc == 0 ]]; then
      exact=$(gh api "repos/$GITHUB_REPOSITORY/actions/runs/$run_id/attempts/$attempt")
      python3 packaging/release-run.py check "$sha" "$tag" "$run_id" "$attempt" <<< "$exact"
      echo "release success: run=$run_id attempt=$attempt sha=$sha tag=$tag"
      exit 0
    fi
  fi
  sleep 20
done
echo 'signed release attempt did not succeed within the one-hour wait' >&2
exit 1
