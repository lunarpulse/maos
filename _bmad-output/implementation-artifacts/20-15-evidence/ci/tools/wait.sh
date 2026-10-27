#!/usr/bin/env bash
# usage: wait.sh RUN MAX_SECONDS — polls until run completes or a non-expected job fails; prints status lines
RUN=$1; MAX=${2:-3500}; EXPECT_FAIL=${3:-}
gh_ro() { GH_TOKEN="$(cat ~/.config/maos-agent/gh-ci-read.token)" gh "$@"; }
end=$(( $(date +%s) + MAX ))
while :; do
  st=$(gh_ro api repos/lunarpulse/maos/actions/runs/$RUN --jq '.status+" "+(.conclusion//"-")')
  fails=$(gh_ro api "repos/lunarpulse/maos/actions/runs/$RUN/jobs?per_page=100" --paginate --jq '.jobs[] | select(.conclusion=="failure") | .name' | grep -vxF -- "${EXPECT_FAIL:-__none__}" | grep -E "${WATCH:-.}" | grep -v '^aggregate' || true)
  counts=$(gh_ro api "repos/lunarpulse/maos/actions/runs/$RUN/jobs?per_page=100" --paginate --jq '.jobs[] | .status+"/"+(.conclusion//"-")' | sort | uniq -c | tr '\n' ' ')
  echo "$(date -u +%H:%M:%S) $st | $counts"
  [ -n "$fails" ] && echo "UNEXPECTED FAILURES:" && echo "$fails" && [ -z "$CONTINUE" ] && exit 4
  case "$st" in completed*) exit 0;; esac
  [ "$(date +%s)" -ge "$end" ] && echo TIMEOUT && exit 3
  sleep 120
done
