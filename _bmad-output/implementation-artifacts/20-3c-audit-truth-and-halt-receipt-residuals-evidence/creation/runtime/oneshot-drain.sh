#!/bin/bash
B=/mnt/build/cargo-maos/debug/maos
cd /tmp/c203c/cwd
for m in smoke-epic-4 smoke-spirit-5 smoke-supervision-5 hello-spirit; do
 for i in 1 2 3; do
  H=$(mktemp -d /tmp/c203c/home-$m-XXXX)
  s=$(date +%s%N)
  env -i PATH=/usr/bin:/bin HOME=$H MAOS_HOME=$H/maos XDG_DATA_HOME=$H/data XDG_CONFIG_HOME=$H/config XDG_STATE_HOME=$H/state MAOS_ONE_SHOT=$m timeout 120 $B >$H/out.log 2>$H/err.log
  rc=$?
  e=$(date +%s%N)
  echo "$m run$i rc=$rc wall_ms=$(( (e-s)/1000000 )) drain_timeout_lines=$(grep -c 'drain timed out' $H/err.log) panics=$(grep -c 'panicked' $H/err.log)"
 done
done
