#!/usr/bin/env bash
cd /home/lunarpulse/dev_ws
N=/tmp/dev205l2/new
run() { # name file sed-expr filterexpr
  name=$1; f=$2; expr=$3; filt=$4
  cp "$f" "$N/$f.orig-$name" 2>/dev/null || true
  sed -i "$expr" "$f"
  if cmp -s "$f" "$N/$f"; then echo "PLANT $name: sed changed nothing" > /tmp/dev205l2/plants/$name.log; return; fi
  touch "$f"
  env -u CI -u GITHUB_ACTIONS cargo nextest run -p xtask --locked --no-fail-fast --test rc_readiness_20_5 --test release_gates_20_5r1 --test release_precondition_perf_20_5 --test release_precondition_t3_20_5 --bins -E "$filt" > /tmp/dev205l2/plants/$name.log 2>&1
  cp "$N/$f" "$f"; cmp "$f" "$N/$f" && touch "$f" && echo "restored $f" >> /tmp/dev205l2/plants/$name.log
}
F='binary(/rc_readiness_20_5|release_gates_20_5r1|release_precondition_perf_20_5|release_precondition_t3_20_5/) | test(/lts_clock/)'
run p1-absent-without-flag xtask/src/check_release_precondition.rs 's/!(absent_ok \&\& row.verdict == "ABSENT")/!(row.verdict == "ABSENT")/' "$F"
run p2-no-sha-check xtask/src/check_release_precondition.rs 's/report.sha != sha || report.verdicts/report.verdicts/' "$F"
run p3-waive-any-version xtask/src/check_rc_readiness.rs 's/let waive = is_rc_version(&version) \&\& perf.passed;/let waive = perf.passed;/' "$F"
run p4-waive-any-status xtask/src/check_rc_readiness.rs 's/waive \&\& key == WAIVED_KEY \&\& value == "in-progress"/waive \&\& key == WAIVED_KEY/' "$F"
run p5-waive-without-label xtask/src/check_rc_readiness.rs 's/let waive = is_rc_version(&version) \&\& perf.passed;/let waive = is_rc_version(\&version);/' "$F"
run p6-warn-without-label xtask/src/check_rc_readiness.rs 's/Some("`absent`") if has_line(notes, ENV_LABEL) => true/Some("`absent`") => true/' "$F"
run p7-label-contains xtask/src/check_rc_readiness.rs 's/line.trim_end() == label/line.contains(label)/' "$F"
run p8-ga-absent-ok .github/workflows/release.yml 's/perf_args=(--perf-verdicts "$RUNNER_TEMP\/perf-verdicts.json") ;;/perf_args=(--perf-verdicts "$RUNNER_TEMP\/perf-verdicts.json" --perf-absent-ok) ;;/' "$F"
run p9-any-failure-is-empty .github/workflows/release.yml "s/elif grep -q 'HTTP 404' \"\$perf_err\"; then/elif true; then/" "$F"
run p10-placeholder-reverted xtask/src/stability_matrix.rs 's/(RELEASE-HOLDS Holds 1–2 are explicit claim boundaries, not preconditions)/after RELEASE-HOLDS Holds 1–2 clear/' "$F"
echo done > /tmp/dev205l2/plants/DONE
