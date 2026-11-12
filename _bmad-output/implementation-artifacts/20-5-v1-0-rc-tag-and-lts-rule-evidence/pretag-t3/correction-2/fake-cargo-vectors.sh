#!/usr/bin/env bash
# Runs the whole scripts/minimize-fuzz-corpus.sh against the test double fake-cargo.sh for each mode.
# usage: bash fake-cargo-vectors.sh <scratch clone with scripts/minimize-fuzz-corpus.sh> <fake-cargo.sh> <mode>...
# cwd of the script under test = the clone (its out/ receives log, artifacts and the inconclusive marker).
set -u
clone=${1:?clone}; fake=${2:?fake-cargo.sh}; shift 2
cd "$clone" || exit 2
bin=$(mktemp -d /tmp/t3fix3-fakebin.XXXXXX); cp "$fake" "$bin/cargo"; chmod +x "$bin/cargo"
trap 'chmod -R u+rwx "$clone/out" 2>/dev/null; rm -rf "$bin"' EXIT
listing() { (cd "$1" && find . -type f | LC_ALL=C sort | xargs -r sha256sum | sha256sum | cut -c1-16); }
for mode in "$@"; do
  t=fake_$mode; fz=$clone/fz-$mode
  rm -rf "$fz" "$clone/out/minimize-$t.log" "$clone/out/merge-crashes-$t" "$clone/out/merge-inconclusive-$t.txt"
  mkdir -p "$fz/corpus/$t" "$clone/out"
  for i in $(seq 1 20); do
    head -c $((64 + i * 7)) /dev/urandom > "$fz/.tmp"; h=$(sha1sum "$fz/.tmp" | cut -d' ' -f1); mv "$fz/.tmp" "$fz/corpus/$t/$h"
  done
  n0=$(find "$fz/corpus/$t" -type f | wc -l); l0=$(listing "$fz/corpus/$t")
  echo "### FAKE_MODE=$mode  files before=$n0 listing=$l0"
  PATH="$bin:$PATH" FAKE_MODE=$mode bash scripts/minimize-fuzz-corpus.sh "$fz" "$t" ''; rc=$?
  chmod -R u+rwx "$clone/out" 2>/dev/null
  n1=$(find "$fz/corpus/$t" -type f | wc -l); l1=$(listing "$fz/corpus/$t")
  arts=$(find "$clone/out/merge-crashes-$t" -type f 2>/dev/null | wc -l)
  left=$(ls -d "$fz"/corpus-minimize-* 2>/dev/null | wc -l)
  mk=no; [ -f "$clone/out/merge-inconclusive-$t.txt" ] && mk=yes
  echo "script exit=$rc  files after=$n1 listing=$l1 $([ "$l0" = "$l1" ] && echo IDENTICAL || echo CHANGED) scratch-left=$left crash-artifacts=$arts marker-file=$mk"
  if [ "$mk" = yes ]; then echo "--- out/merge-inconclusive-$t.txt"; cat "$clone/out/merge-inconclusive-$t.txt"; echo "---"; fi
  rm -rf "$fz"
done
