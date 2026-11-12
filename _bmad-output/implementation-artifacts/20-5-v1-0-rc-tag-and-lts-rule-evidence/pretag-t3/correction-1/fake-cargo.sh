#!/usr/bin/env bash
# Test double for `cargo fuzz run --fuzz-dir D TARGET MERGED CORPUS -- -merge=1 -artifact_prefix=P/ ...`.
# FAKE_MODE selects what the "merge" leaves behind; it always exits 0 (the dangerous case).
set -u
args=("$@"); merged=${args[5]}; corpus=${args[6]}
for a in "${args[@]}"; do case $a in -artifact_prefix=*) prefix=${a#-artifact_prefix=};; esac; done
mapfile -t src < <(find "$corpus" -type f | LC_ALL=C sort | head -n 8)   # real, correctly SHA-1-named inputs
for f in "${src[@]}"; do cp "$f" "$merged/"; done
n=${#src[@]}
case "${FAKE_MODE:?}" in
  good)      printf 'MERGE-OUTER: %s new files with 1 new features added; 1 new coverage edges\n' "$n" ;;
  truncated) for f in "${src[@]}"; do s=$(stat -c %s "$f"); if [ "$s" -ge 2 ]; then v=${f##*/}; head -c $((s - 1)) "$merged/$v" > "$merged/$v.t"; mv "$merged/$v.t" "$merged/$v"; break; fi; done   # short write: last byte lost, name unchanged
             printf 'MERGE-OUTER: %s new files with 1 new features added; 1 new coverage edges\n' "$n" ;;
  missing)   rm -f "$merged/${src[0]##*/}"
             printf 'MERGE-OUTER: %s new files with 1 new features added; 1 new coverage edges\n' "$n" ;;
  nomerge)   printf 'MERGE-OUTER: attempt 1\nMERGE-OUTER: successful in 1 attempt(s)\n' ;;
  crash)     printf 'x' > "${prefix}crash-fakefakefakefake"
             printf '==1== ERROR: libFuzzer: deadly signal\nMERGE-OUTER: %s new files with 1 new features added; 1 new coverage edges\n' "$n" ;;
  marker)    printf 'thread panicked at fuzz_targets/x.rs:1:1\nMERGE-OUTER: %s new files with 1 new features added; 1 new coverage edges\n' "$n" ;;
esac
exit 0
