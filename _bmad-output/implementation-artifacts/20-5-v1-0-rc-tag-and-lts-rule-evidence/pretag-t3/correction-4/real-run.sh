#!/usr/bin/env bash
# Whole scripts/minimize-fuzz-corpus.sh against a REAL libFuzzer merge in the scratch clone.
# usage: bash real-run.sh <clone> <fuzz_dir> <target> <backup_corpus_dir> <rss_flag> [extra env: PLANT_CRASH=1 (a crash- file is placed in the merge artifact dir before the run), PLANT_INPUT=1 (adds a T3-MERGE-PLANT input to the corpus)]
# Restores the corpus from the backup before the run, so every row starts from the same files.
# Prints: before listing, the script's own output, exit status, the real merge command line
# (the `Running` line of out/minimize-<target>.log, which ends in the flags libFuzzer got),
# artifacts, marker file, after listing, oom- count.
set -u
clone=${1:?clone}; fd=${2:?fuzz dir}; t=${3:?target}; backup=${4:?backup}; rss=${5-}
cd "$clone" || exit 2
export RUSTUP_TOOLCHAIN=nightly ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0
listing() { (cd "$1" && find . -type f | LC_ALL=C sort | xargs -r sha256sum | sha256sum | cut -c1-16); }
corpus=$clone/$fd/corpus/$t
rm -rf "$corpus" "$clone/$fd/corpus-minimize-$t" "$clone/out/minimize-$t.log" "$clone/out/merge-crashes-$t" "$clone/out/merge-inconclusive-$t.txt"
mkdir -p "$corpus"; cp -a "$backup"/. "$corpus"/
[ "${PLANT_INPUT:-0}" = 1 ] && printf 'T3-MERGE-PLANT' > "$corpus/planted-input"
[ "${PLANT_CRASH:-0}" = 1 ] && { mkdir -p "$clone/out/merge-crashes-$t"; : > "$clone/out/merge-crashes-$t/crash-planted-by-vector"; }
l0=$(listing "$corpus"); n0=$(find "$corpus" -type f | wc -l)
echo "##### rss_flag='$rss'  before listing=$l0 files=$n0"
if [ "${KILL_INNER:-0}" = 1 ]; then
  ( for _ in $(seq 1 600); do
      p=; for c in $(pgrep -f -- '-merge_inner=1'); do
        tr '\0' ' ' < "/proc/$c/cmdline" 2>/dev/null | grep -q -- "$clone" && { p=$c; break; }
      done
      if [ -n "$p" ]; then sleep 3; kill -9 "$p" 2>/dev/null && echo "WATCHER: SIGKILLed inner pid $p"; break; fi
      sleep 0.1
    done ) &
  watcher=$!
fi
start=$SECONDS
taskset -c 0-7 bash scripts/minimize-fuzz-corpus.sh "$fd" "$t" "$rss"; rc=$?
echo "script exit=$rc wall=$((SECONDS - start))s"
[ "${KILL_INNER:-0}" = 1 ] && { wait "$watcher" 2>/dev/null; }
l1=$(listing "$corpus"); n1=$(find "$corpus" -type f | wc -l)
arts=$(find "$clone/out/merge-crashes-$t" -type f -printf '%f ' 2>/dev/null)
mk=no; [ -f "$clone/out/merge-inconclusive-$t.txt" ] && mk=yes
echo "after listing=$l1 files=$n1 $([ "$l0" = "$l1" ] && echo IDENTICAL || echo CHANGED) artifacts=[${arts% }] marker_file=$mk scratch-left=$(ls -d "$clone/$fd"/corpus-minimize-* 2>/dev/null | wc -l)"
echo "oom- artifacts: $(find "$clone/out/merge-crashes-$t" -type f -name 'oom-*' 2>/dev/null | wc -l)"
echo "non-40-hex names in corpus: $(ls "$corpus" | grep -vcE '^[0-9a-f]{40}$')"
echo "--- real merge command line (Running line of out/minimize-$t.log):"
grep -a 'Running `' "$clone/out/minimize-$t.log" | cut -c1-600
echo "--- merge log: MERGE-OUTER / ERROR / SUMMARY / MERGE-INNER failure lines:"
grep -a -E 'MERGE-OUTER: (attempt|successful|[0-9]+ new files)|ERROR|SUMMARY|caused a failure|Killed|signal' "$clone/out/minimize-$t.log" | cut -c1-300 | head -n 30
echo "--- marker file out/merge-inconclusive-$t.txt (first 8 lines):"
head -n 8 "$clone/out/merge-inconclusive-$t.txt" 2>/dev/null | cut -c1-260 || true
