#!/bin/bash
# usage: p1_stress_loop.sh <test-exe> <label> <iterations> <logfile>
exe=$1; label=$2; n=$3; log=$4
cd /home/lunarpulse/dev_ws
pids=()
for i in $(seq 1 40); do
  bash -c 'while :; do :; done' &
  pids+=($!)
done
pass=0; fail=0
for i in $(seq 1 $n); do
  out=$(env -u CI -u GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE=undelegated "$exe" --exact maosctl_cold_swaps_a_component_to_a_new_pid_and_restores_it_when_the_successor_is_refused 2>&1)
  if echo "$out" | grep -q "test result: ok. 1 passed"; then pass=$((pass+1)); echo "iter $i PASS" >> "$log"; else fail=$((fail+1)); echo "iter $i FAIL" >> "$log"; echo "$out" | grep -E "restore refused|already registered|panicked|cold_swap" | head -3 >> "$log"; fi
done
for p in "${pids[@]}"; do kill "$p" 2>/dev/null; done
wait 2>/dev/null
echo "$label: pass=$pass fail=$fail of $n (40 busy-loop load jobs on 32 cpus)" | tee -a "$log"
