#!/bin/bash
# usage: loop.sh <label> <total> <parallel>
export PATH=$HOME/.venvs/maos-test/bin:$PATH
label=$1; total=$2; par=$3
exe=$(ls -t /home/lunarpulse/dev_ws/target/debug/deps/journey_forget_18_6-* | grep -E -- '-[0-9a-f]{16}$' | head -1)
out=/tmp/forget-flake/$label; mkdir -p $out
run1() { i=$1
  cd /home/lunarpulse/dev_ws/crates/maos-journey-test
  if timeout 300 $exe --exact live_forget_redistills_and_never_resurrects_then_pending_is_typed --nocapture > $out/run-$i.log 2>&1; then echo "$i PASS"; rm -f $out/run-$i.log; else echo "$i FAIL"; fi
}
export -f run1; export exe out
seq 1 $total | xargs -P $par -I{} bash -c 'run1 {}' > $out/results.txt
echo "total=$total pass=$(grep -c PASS $out/results.txt) fail=$(grep -c FAIL $out/results.txt)"
