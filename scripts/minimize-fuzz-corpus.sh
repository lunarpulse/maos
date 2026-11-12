#!/usr/bin/env bash
# Shrinks a link's carried-over fuzz corpus before the upload/download hand-off.
# usage: minimize-fuzz-corpus.sh <fuzz_dir> <target> <rss_flag> [timeout_seconds]
#
# A libFuzzer `-merge=1` pass keeps only inputs that add coverage. It writes into a
# FRESH sibling directory and the original corpus is replaced only after ALL of:
#   1. no CONFIRMED crash. classify_merge looks at the merge's artifact dir out/merge-crashes-
#      <target>/ and its log and returns one of three results, whatever the merge's exit status
#      (libFuzzer's crash-resistant merge skips an input that crashes the target, restarts its
#      inner process and can still exit 0, FuzzerMerge.cpp "Execute the inner process until it
#      passes", so the exit status proves nothing):
#        CONFIRMED     a regular file in the artifact dir whose basename does NOT start with
#                      `oom-` (crash-, leak-, timeout-, slow-unit-, anything unknown), or any such
#                      file next to oom- files: libFuzzer wrote an input artifact in the inner
#                      `-merge_inner` process, whose crash and alarm handlers call
#                      Fuzzer::DumpCurrentUnit (FuzzerLoop.cpp:236, 301) on a buffer that exists
#                      because FuzzerDriver.cpp:892-894 sets -max_len for the inner. The original
#                      corpus is kept, `::error::` names the artifacts, exit 1, exactly like
#                      run-fuzz-link.sh does for its own crashes. slow-unit- is only written by
#                      Fuzzer::RunOne (PrintPulseAndReportSlowInput) and the merge path calls
#                      ExecuteCallback directly, so it cannot occur here; counting it is harmless.
#        INCONCLUSIVE  (a) the artifact dir holds only `oom-` files. They are libFuzzer's RSS /
#                      malloc-limit dump (FuzzerLoop.cpp:311-323, DumpCurrentUnit), written for
#                      whatever unit is current, even the empty one (measured in
#                      pretag-t3/correction-2/v4-real-outer-oom-attempts.log), and the inner
#                      merger's RSS (per-file MergeFileInfo + paths, AllFeatures/AllPCs, no
#                      PurgeAllocator) is not bounded by fuzz-worker success, so an oom- file is
#                      not attributable to an input. The merge inherits the matrix rss_flag
#                      exactly like the fuzz step (no -rss_limit_mb override: removing every
#                      memory bound lets a runner-wide OOM kill the supervising shell and lose the
#                      link), so it is never less bounded than the fuzzing it follows.
#                      A genuine single-input OOM therefore stays in the kept original corpus and
#                      is re-executed by the next link's workers under the same limit, where
#                      run-fuzz-link.sh refuses evidence; link 5 has no next link and relies on
#                      the marker file alone. (b) no artifact, but the log carries a crash/resource
#                      marker (`ERROR: libFuzzer`, `ERROR: *Sanitizer`, `deadly signal`,
#                      `panicked at`, `out-of-memory`), or the artifact-dir / log scan itself failed
#                      or timed out. The OUTER merge process never writes an artifact: its Fuzzer is
#                      built with the default -max_len=0, so CurrentUnitData is never allocated
#                      (FuzzerLoop.cpp:158, 163-166; FuzzerDriver.cpp:690) and DumpCurrentUnit
#                      returns at FuzzerLoop.cpp:175, yet its RSS thread (FuzzerDriver.cpp:314-327,
#                      started at :832 before Merge at :889) still prints `ERROR: libFuzzer:
#                      out-of-memory` (FuzzerLoop.cpp:311-323). That is minimizer infrastructure
#                      (the control file/file lists outgrowing the limit), not evidence against the
#                      link, and neither is a scan error. The original corpus is kept (a crashing
#                      input still in it is re-executed at the next link's INITED and fails that
#                      link properly), `::warning::` carries a bounded excerpt, the marker file
#                      out/merge-inconclusive-<target>.txt records reason + excerpt (names the oom-
#                      files in case (a); uploaded by the `if: always()` logs upload; for link 5,
#                      which has no next link, the marker file and the warning are the whole
#                      record), exit 0.
#        CLEAN         neither: proceed to checks 2 and 3 and the swap.
#   2. the merge exited 0 and the output is complete (validate_merged): the log has exactly
#      one `MERGE-OUTER: N new files` line, the directory holds exactly N regular files and
#      nothing else, and sha1(content) == file name for every file. libFuzzer ignores
#      fopen/short-write/close errors when it writes the output (FuzzerIO.cpp WriteToFile)
#      and still exits 0, so a full disk yields missing or truncated files; libFuzzer names
#      every corpus file by the SHA-1 of its content, which makes truncation detectable.
#   3. before the merge: free space >= original disk usage + 10 % + 256 MiB and free inodes
#      >= original files + 1024. The output is a subset of the input files (same block
#      rounding), so the worst case is another full copy (1.0x); 10 % covers metadata and the
#      control file (one line per input, measured in correction-1/ logs), 256 MiB the logs.
# Any failed check in 2/3, an INCONCLUSIVE classification, a timeout, a merge failure, a
# missing target ... keeps the original corpus, prints a ::warning:: and exits 0: minimization
# is an optimisation; the link's evidence is the fuzz step and its out/record-*.json, which this
# script never reads or writes. Only a CONFIRMED crash found by the merge fails the step.
# `cargo fuzz cmin` is deliberately not used: it swaps with two unguarded renames, has no
# time bound, and replaces the corpus even when the merge produced nothing.
#
# Every potentially long operation (find/wc/du/df/rm/mv/grep/sha1sum) runs under
# `timeout -k 2 <bound>` and is best-effort. The bounds below sum to worst_case_seconds
# (1659 s = 27.65 min with the default 1200 s merge limit; `bash -c 'source
# scripts/minimize-fuzz-corpus.sh; worst_case_seconds'` prints it), inside the workflow
# step's `timeout-minutes: 30` and well inside the job's 355 min (300 min of fuzzing).
# Swap order keeps `corpus/<target>/` a complete corpus except for the instant between two
# renames: mv corpus -> old, mv merged -> corpus (same filesystem, checked), delete old.

# Per-call bounds in seconds (each call also gets a 2 s SIGKILL grace, B_KILL).
B_KILL=2
b_mkdir=10 b_rmscratch=15 b_find=15 b_du=15 b_df=5 b_stat=5
b_scan=10 b_nonreg=20 b_hash=60 b_awk=20 b_mv=10 b_rmold=45
b_trap_restore=10 b_trap_rm=20 b_trap_rmdir=5
merge_kill=30
default_limit=1200
# Bounded call sites on the longest path: mkdir 2, rm scratch 1, find+du before/after 4,
# df 5 (3 before, 2 after), stat 1, scans 4 (classify_merge's artifact-dir find and log
# grep, MERGE-OUTER print, validation grep), nonreg/hash/awk 3, mv 2, rm old 1, restore 2
# (start, trap), trap rm 1, trap rmdir 1 = 27. The INCONCLUSIVE path adds one `tee` (the
# marker file) but stops before validation and the swap, so it is shorter than this path.
bounded_calls=27

bounded() { local s=$1; shift; timeout -k "$B_KILL" "$s" "$@"; }

# Sum of every bound: the worst-case wall of the script when each step runs to its bound.
worst_case_seconds() {
  local limit=${1:-$default_limit}
  echo $(( 2 * b_mkdir + b_rmscratch + 2 * (b_find + b_du) + 5 * b_df + b_stat \
    + limit + merge_kill + 4 * b_scan + b_nonreg + b_hash + b_awk + 2 * b_mv + b_rmold \
    + 2 * b_trap_restore + b_trap_rm + b_trap_rmdir + bounded_calls * B_KILL ))
}

warn() { echo "::warning::minimize-fuzz-corpus ${target-?}: $*"; }

# Prints `files=N du_kib=K` ("unknown" for a figure that did not finish in its bound).
measure() {
  local files kib
  files=$(set -o pipefail; bounded "$b_find" find "$1" -type f 2>/dev/null | wc -l) || files=unknown
  kib=$(set -o pipefail; bounded "$b_du" du -sk "$1" 2>/dev/null | cut -f1) || kib=unknown
  echo "files=${files:-unknown} du_kib=${kib:-unknown}"
}

# df_field <-k|-i> <awk column> <path>: one df figure, empty when df failed or timed out.
df_field() {
  local v
  v=$(set -o pipefail; bounded "$b_df" df -P "$1" "$3" 2>/dev/null | awk -v c="$2" 'NR==2{print $c}') || v=
  echo "$v"
}

is_num() { [[ ${1-} =~ ^[0-9]+$ ]]; }

# space_check <orig_kib> <orig_files> <free_kib> <free_inodes> <total_inodes>
# 0 when the merge has room; otherwise 1 with the reason in $space_reason. A filesystem
# that reports no inode table (total 0, e.g. btrfs) skips only the inode comparison.
space_check() {
  local okib=$1 ofiles=$2 fkib=$3 finodes=$4 tinodes=$5 need_kib need_inodes
  space_reason=
  is_num "$okib" && is_num "$ofiles" && is_num "$fkib" || {
    space_reason="cannot measure the corpus size or free space (orig_kib=$okib files=$ofiles free_kib=$fkib)"
    return 1
  }
  need_kib=$(( okib + okib / 10 + 262144 ))
  if [ "$fkib" -lt "$need_kib" ]; then
    space_reason="free space ${fkib} KiB < required ${need_kib} KiB (corpus ${okib} KiB x1.1 + 256 MiB)"
    return 1
  fi
  if is_num "$tinodes" && [ "$tinodes" -gt 0 ]; then
    need_inodes=$(( ofiles + 1024 ))
    if ! is_num "$finodes" || [ "$finodes" -lt "$need_inodes" ]; then
      space_reason="free inodes ${finodes:-unknown} < required ${need_inodes} (corpus files ${ofiles} + 1024)"
      return 1
    fi
  fi
  return 0
}

# validate_merged <merged_dir> <merge_log> <tmp_dir>
# 0 when the directory is exactly what libFuzzer reported; otherwise 1 with the reason in
# $reject_reason. <dir> must not carry a trailing slash.
validate_merged() {
  local dir=$1 mlog=$2 tmp=$3 lines expected extra res count bad first
  reject_reason=
  lines=$(bounded "$b_scan" grep -E '^MERGE-OUTER: [0-9]+ new files' "$mlog" 2>/dev/null) || lines=
  if [ -z "$lines" ]; then
    reject_reason="no 'MERGE-OUTER: N new files' line in $mlog"; return 1
  fi
  if [ "$(printf '%s\n' "$lines" | wc -l)" -ne 1 ]; then
    reject_reason="more than one 'MERGE-OUTER: N new files' line in $mlog"; return 1
  fi
  expected=$(printf '%s\n' "$lines" | sed -E 's/^MERGE-OUTER: ([0-9]+) new files.*/\1/')
  if ! is_num "$expected" || [ "$expected" -lt 1 ]; then
    reject_reason="merge reported ${expected:-?} new files"; return 1
  fi
  extra=$(bounded "$b_nonreg" find "$dir" -mindepth 1 ! -type f -print -quit 2>&1) \
    || { reject_reason="non-regular-file scan of $dir failed or timed out"; return 1; }
  if [ -n "$extra" ]; then
    reject_reason="non-regular entry in merged dir: $extra"; return 1
  fi
  bounded "$b_hash" find "$dir" -mindepth 1 -type f -exec sha1sum {} + > "$tmp/sha1.txt" 2> "$tmp/sha1.err" \
    || { reject_reason="sha1sum over $dir failed or exceeded ${b_hash}s ($(head -c 200 "$tmp/sha1.err" 2>/dev/null))"; return 1; }
  # sha1sum prints "<40 hex>  <dir>/<name>": the name must equal the hash.
  res=$(bounded "$b_awk" awk -v dlen="${#dir}" '
    { n++; name = substr($0, 43 + dlen + 1);
      if ($1 != name) { b++; if (first == "") first = name } }
    END { print n + 0, b + 0, first }' "$tmp/sha1.txt") \
    || { reject_reason="SHA-1 comparison failed or timed out"; return 1; }
  read -r count bad first <<< "$res"
  if [ "$count" -ne "$expected" ]; then
    reject_reason="merged dir holds $count regular files but MERGE-OUTER reported $expected"; return 1
  fi
  if [ "$bad" -ne 0 ]; then
    reject_reason="$bad file(s) whose SHA-1 differs from the file name (first: ${first:-?}): truncated or corrupt output"; return 1
  fi
  return 0
}

# classify_merge <artifact_dir> <merge_log>
# Tri-state classification of a finished merge. The result is the exit status: CM_CLEAN (0),
# CM_CONFIRMED (1) or CM_INCONCLUSIVE (2). stdout carries the evidence, one line each:
# `reason: <why inconclusive>`, `merge artifact: <name>` (<= 20, non-oom- names first),
# `merge log marker: <line>` (<= 5 lines of <= 300 chars). Both scans are bounded; a scan error
# or timeout is never CLEAN. An artifact decides over the log; among artifacts, only a basename
# starting with `oom-` is demoted, and only when no other artifact is present.
CM_CLEAN=0 CM_CONFIRMED=1 CM_INCONCLUSIVE=2
crash_marker_re='ERROR: (libFuzzer|[A-Za-z]*Sanitizer)|deadly signal|panicked at|out-of-memory'
classify_merge() {
  local crashes=$1 mlog=$2 names markers nrc=0 mrc=0 inconclusive=0 n
  local oom=() other=() ordered=()
  names=$(bounded "$b_scan" find "$crashes" -type f -printf '%f\n' 2>/dev/null) || nrc=$?
  markers=$(bounded "$b_scan" grep -a -E -m 5 -e "$crash_marker_re" "$mlog" 2>/dev/null) || mrc=$?
  # A name is printed only for a real regular file, so it is proof even if the scan then failed.
  if [ -n "$names" ]; then
    while IFS= read -r n; do
      case $n in oom-*) oom+=("$n") ;; *) other+=("$n") ;; esac
    done < <(printf '%s\n' "$names")
    ordered=("${other[@]}" "${oom[@]}")
    if [ "${#other[@]}" -eq 0 ]; then
      echo "reason: only oom- artifact(s) were written (libFuzzer's RSS/malloc-limit dump, which it writes for whatever unit is current, even the empty one); that is a merge-process resource trip, not an attributable input crash"
      inconclusive=1
    fi
    printf 'merge artifact: %s\n' "${ordered[@]:0:20}"
  elif [ "$nrc" -ne 0 ]; then
    echo "reason: artifact dir $crashes could not be scanned (exit $nrc)"; inconclusive=1
  fi
  if [ -z "$names" ] && [ "$mrc" -gt 1 ]; then
    echo "reason: merge log $mlog could not be scanned (exit $mrc)"; inconclusive=1
  fi
  if [ -n "$markers" ]; then
    if [ -z "$names" ]; then
      echo "reason: no input artifact was written but the merge log carries a crash/resource marker (an outer-process resource limit or other minimizer infrastructure failure, not an input crash)"
      inconclusive=1
    fi
    printf '%s\n' "$markers" | cut -c1-300 | sed 's/^/merge log marker: /'
  fi
  if [ -n "$names" ] && [ "${#other[@]}" -gt 0 ]; then return "$CM_CONFIRMED"; fi
  if [ "$inconclusive" -eq 1 ]; then return "$CM_INCONCLUSIVE"; fi
  return "$CM_CLEAN"
}

# record_inconclusive <evidence> <merge_exit>: one bounded ::warning:: line plus the marker
# file $inconclusive_file (reason + excerpt) next to the other logs under out/.
record_inconclusive() {
  local evidence=$1 mexit=$2 excerpt
  excerpt=$(printf '%s\n' "$evidence" | head -n 7 | cut -c1-200 | tr '\n' '|')
  warn "merge result inconclusive, not a confirmed input crash (merge exit $mexit): ${excerpt%|}; keeping the unminimized corpus; details in $inconclusive_file"
  {
    printf 'target: %s\nmerge_exit: %s\nmerge_log: %s\nartifact_dir: %s\n' "$target" "$mexit" "$log" "$crashes"
    printf 'result: INCONCLUSIVE (original corpus kept)\n'
    printf '%s\n' "$evidence"
  } | bounded "$b_scan" tee "$inconclusive_file" > /dev/null \
    || warn "could not write $inconclusive_file"
}

# Undo a half-finished swap (only reachable by an interrupt between the two renames).
restore() {
  if [ -d "$old" ] && [ ! -e "$corpus" ]; then bounded "$b_trap_restore" mv -T "$old" "$corpus"; fi
}

cleanup() {
  restore || true
  bounded "$b_trap_rm" rm -rf "$merged" "$scratch/tmp" || true
  bounded "$b_trap_rmdir" rmdir "$scratch" 2>/dev/null || true
}

# Returns 0 (done or kept the original), 1 (unexpected error; the caller keeps the
# original), 2 (the merge found a CONFIRMED crash). Runs as `minimize || rc=$?`, so errexit
# is off here: every step that must stop the minimization checks its own status.
minimize() {
  [ -d "$corpus" ] || { warn "corpus dir $corpus missing; nothing to do"; return 0; }
  bounded "$b_mkdir" mkdir -p "$(dirname "$log")" "$crashes" || return 1
  restore || true
  bounded "$b_rmscratch" rm -rf "$scratch" || true
  if [ -e "$scratch" ]; then
    warn "stale scratch dir $scratch could not be removed in ${b_rmscratch}s; keeping the unminimized corpus"
    return 0
  fi
  bounded "$b_mkdir" mkdir -p "$merged" "$scratch/tmp" || return 1

  local before orig_kib orig_files fs_dir free_kib free_inodes total_inodes
  before=$(measure "$corpus")
  echo "minimize-fuzz-corpus $target: before $before"
  orig_files=${before#files=}; orig_files=${orig_files%% *}
  orig_kib=${before##*du_kib=}
  fs_dir=$(dirname "$corpus")
  free_kib=$(df_field -k 4 "$fs_dir")
  free_inodes=$(df_field -i 4 "$fs_dir")
  total_inodes=$(df_field -i 2 "$fs_dir")
  echo "minimize-fuzz-corpus $target: free_kib=${free_kib:-unknown} free_inodes=${free_inodes:-unknown} total_inodes=${total_inodes:-unknown}"
  if ! space_check "$orig_kib" "$orig_files" "$free_kib" "$free_inodes" "$total_inodes"; then
    warn "$space_reason; skipping the merge and keeping the unminimized corpus"
    return 0
  fi
  local devs
  devs=$(bounded "$b_stat" stat -c %d "$corpus" "$scratch" "$fs_dir" 2>/dev/null | sort -u | wc -l)
  if [ "$devs" != 1 ]; then
    warn "$corpus and $scratch are not on one filesystem (an atomic rename is impossible); keeping the unminimized corpus"
    return 0
  fi

  # The merge inherits the matrix rss_flag exactly as the fuzz step received it, and nothing is
  # appended: the merge is never less bounded than the fuzzing it follows (default 2048 MB for
  # manifest_parser / fuzz_exec_deps_parse_elf, -rss_limit_mb=0 for frame_deser, the same
  # exposure its fuzz step already has). An RSS trip of the merge shows up as an oom- artifact,
  # which classify_merge treats as INCONCLUSIVE (see the header).
  local flags=(-merge=1 "-artifact_prefix=$crashes/")
  if [ -n "$rss_flag" ]; then flags+=("$rss_flag"); fi

  local start=$SECONDS rc=0
  # timeout runs the command in its own process group and signals the whole group, so
  # libFuzzer's merge children are stopped with cargo; -k escalates to SIGKILL.
  # TMPDIR keeps libFuzzer's merge control file (~150 bytes per input) in the scratch dir, which the
  # exit trap removes, instead of leaking it into /tmp when the merge is killed.
  TMPDIR="$scratch/tmp" timeout -k "$merge_kill" "$limit" \
    cargo fuzz run --fuzz-dir "$fuzz_dir" "$target" "$merged" "$corpus" -- "${flags[@]}" \
    > "$log" 2>&1 || rc=$?
  echo "minimize-fuzz-corpus $target: merge exit=$rc wall=$((SECONDS - start))s log=$log"

  # The classification decides first, whatever the merge's exit status was.
  local evidence cls=0
  evidence=$(classify_merge "$crashes" "$log") || cls=$?
  case "$cls" in
    "$CM_CONFIRMED")
      echo "::error::T3 $target: input crashed during corpus merge; refusing evidence" >&2
      echo "$evidence" >&2
      return 2 ;;
    "$CM_INCONCLUSIVE")
      record_inconclusive "$evidence" "$rc"
      return 0 ;;
  esac

  if [ "$rc" -ne 0 ]; then
    tail -n 20 "$log" || true
    if [ "$rc" -eq 124 ] || [ "$rc" -eq 137 ]; then
      warn "merge exceeded ${limit}s; keeping the unminimized corpus"
    else
      warn "merge failed with exit $rc; keeping the unminimized corpus"
    fi
    return 0
  fi
  bounded "$b_scan" grep -h '^MERGE-OUTER: .*\(new files\|control file has\)' "$log" || true

  local vstart=$SECONDS
  if ! validate_merged "$merged" "$log" "$scratch/tmp"; then
    warn "merged corpus rejected ($reject_reason); keeping the unminimized corpus"
    return 0
  fi
  echo "minimize-fuzz-corpus $target: merged output validated (count == MERGE-OUTER, sha1 == name) in $((SECONDS - vstart))s"

  bounded "$b_mv" mv -T "$corpus" "$old" || return 1
  if ! bounded "$b_mv" mv -T "$merged" "$corpus"; then
    if [ -d "$corpus" ] && [ ! -e "$merged" ]; then
      :  # the rename completed and only the timeout raced it
    else
      restore
      warn "could not move the minimized corpus into place; kept the original"
      return 0
    fi
  fi
  echo "minimize-fuzz-corpus $target: after $(measure "$corpus")"
  # Outside corpus/<target>/ (the upload path): a delete that does not finish is harmless.
  bounded "$b_rmold" rm -rf "$old" || warn "could not delete the replaced corpus at $old within ${b_rmold}s (left outside the upload path)"
  echo "minimize-fuzz-corpus $target: free_kib=$(df_field -k 4 "$fs_dir") free_inodes=$(df_field -i 4 "$fs_dir")"
}

main() {
  set -euo pipefail
  fuzz_dir=${1:?fuzz directory}
  target=${2:?target}
  rss_flag=${3-}
  # 1200 s: measured merge of 493,838 frame_deser files (1.9 GB on disk) took 690 s on one
  # fast core (~1.4 ms/file, single process); 1200 s is 1.7x that and keeps the whole
  # script's worst case (worst_case_seconds) at 27.65 min of the job's 55 spare minutes.
  limit=${4:-$default_limit}

  local abs
  abs=$(realpath -m "$fuzz_dir")
  corpus=$abs/corpus/$target
  # Siblings of the corpus dir on the same filesystem (rename-able), outside the upload path.
  scratch=$abs/corpus-minimize-$target
  old=$scratch/old
  merged=$scratch/merged
  log=$(pwd)/out/minimize-$target.log
  # Under out/, so the `Preserve worker logs and crashes` upload (if: always()) keeps both.
  crashes=$(pwd)/out/merge-crashes-$target
  inconclusive_file=$(pwd)/out/merge-inconclusive-$target.txt

  trap cleanup EXIT
  local rc=0
  minimize || rc=$?
  case "$rc" in
    0) ;;
    2) exit 1 ;;
    *) restore || true; warn "unexpected error; kept the original corpus" ;;
  esac
  exit 0
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then main "$@"; fi
