#!/usr/bin/env bash
# Exercises the pure/validating functions of scripts/minimize-fuzz-corpus.sh by sourcing
# it (the BASH_SOURCE guard keeps main from running). No root, no fuzzing.
# usage: bash function-vectors.sh <repo root containing scripts/minimize-fuzz-corpus.sh>
set -u
root=${1:?repo root}
# shellcheck disable=SC1091
source "$root/scripts/minimize-fuzz-corpus.sh"
work=$(mktemp -d /tmp/t3fix2-vectors.XXXXXX)
trap 'rm -rf "$work"' EXIT
fails=0
row() { printf '%-58s %-9s %s\n' "$1" "$2" "$3"; }
expect() { # name expected_rc actual_rc detail
  local v=PASS; [ "$2" -eq "$3" ] || { v=FAIL; fails=$((fails + 1)); }
  row "$1" "rc=$3/$v" "$4"
}

mkout() { # dir n : n content-addressed files, exactly as libFuzzer names them
  local d=$1 n=$2 i h
  mkdir -p "$d"
  for ((i = 0; i < n; i++)); do
    head -c $((64 + i * 7)) /dev/urandom > "$d/.tmp"
    h=$(sha1sum "$d/.tmp" | cut -d' ' -f1); mv "$d/.tmp" "$d/$h"
  done
}
mklog() { # file n
  printf 'MERGE-OUTER: 40 files, 0 in the initial corpus, 0 processed earlier\nMERGE-OUTER: attempt 1\nMERGE-OUTER: successful in 1 attempt(s)\nMERGE-OUTER: the control file has 1234 bytes\nMERGE-OUTER: %s new files with 99 new features added; 77 new coverage edges\n' "$2" > "$1"
}
check() { # name expected_rc dir log
  mkdir -p "$work/tmp"
  validate_merged "$3" "$4" "$work/tmp"; local rc=$?
  expect "validate: $1" "$2" "$rc" "${reject_reason:-accepted}"
}

echo "== validate_merged =="
d=$work/ok; mkout "$d" 5; mklog "$work/ok.log" 5
check "correct dir, 5 files == MERGE-OUTER 5" 0 "$d" "$work/ok.log"

d=$work/trunc; mkout "$d" 5; f=$(ls "$d" | sort | head -1)
head -c 10 "$d/$f" > "$d/$f.t" && mv "$d/$f.t" "$d/$f"      # short write: name stays, content shrinks
check "truncated file (content no longer matches SHA-1 name)" 1 "$d" "$work/ok.log"

d=$work/zero; mkout "$d" 5; f=$(ls "$d" | sort | head -1); : > "$d/$f"
check "zero-byte file (fopen ok, write failed)" 1 "$d" "$work/ok.log"

d=$work/cnt_hi; mkout "$d" 5; mklog "$work/hi.log" 6
check "count mismatch: MERGE-OUTER 6, dir has 5 (missing file)" 1 "$d" "$work/hi.log"

d=$work/cnt_lo; mkout "$d" 5; mklog "$work/lo.log" 4
check "count mismatch: MERGE-OUTER 4, dir has 5 (extra file)" 1 "$d" "$work/lo.log"

d=$work/nolog; mkout "$d" 5; printf 'MERGE-OUTER: attempt 1\nsome other output\n' > "$work/nomo.log"
check "missing 'MERGE-OUTER: N new files' line" 1 "$d" "$work/nomo.log"

d=$work/dup; mkout "$d" 5; { cat "$work/ok.log"; cat "$work/ok.log"; } > "$work/dup.log"
check "two 'new files' lines (ambiguous)" 1 "$d" "$work/dup.log"

d=$work/zero_new; mkdir -p "$d"; mklog "$work/zero.log" 0
check "MERGE-OUTER: 0 new files, empty dir" 1 "$d" "$work/zero.log"

d=$work/subdir; mkout "$d" 5; mkdir "$d/sub"
check "subdirectory in merged dir" 1 "$d" "$work/ok.log"

d=$work/emptyfile; mkdir -p "$d"; : > "$d/da39a3ee5e6b4b0d3255bfef95601890afd80709"; mklog "$work/one.log" 1
check "empty input correctly named da39a3ee... (valid libFuzzer output)" 0 "$d" "$work/one.log"

d="$work/spaces/with space+dots.v1"; mkout "$d" 3; mklog "$work/three.log" 3
check "dir path with space, '+' and '.' (name extraction)" 0 "$d" "$work/three.log"

echo "== space_check (orig_kib orig_files free_kib free_inodes total_inodes) =="
sc() { space_check "$2" "$3" "$4" "$5" "$6"; local rc=$?; expect "space: $1" "$7" "$rc" "${space_reason:-ok}"; }
# corpus 1,900,000 KiB: need 1,900,000 + 190,000 + 262,144 = 2,352,144 KiB
sc "free exactly at the requirement"           1900000 493838 2352144 600000 1000000 0
sc "free one KiB below the requirement"        1900000 493838 2352143 600000 1000000 1
sc "free == original only (no margin)"         1900000 493838 1900000 600000 1000000 1
sc "plenty of blocks, inodes 493838+1023"      1900000 493838 9999999 494861 1000000 1
sc "inodes exactly files+1024"                 1900000 493838 9999999 494862 1000000 0
sc "fs reports no inode table (total 0)"       1900000 493838 9999999 0 0 0
sc "unmeasurable free space"                   1900000 493838 "" "" "" 1
sc "unmeasurable corpus size"                  unknown unknown 9999999 600000 1000000 1
sc "tiny corpus: one KiB below 10+1+262144 KiB"  10 3 262154 100 1000 1
sc "tiny corpus: exactly 10+1+262144 KiB"         10 3 262155 1030 1000 0

echo "== merge_crash_evidence =="
ce() { merge_crash_evidence "$2" "$3" > "$work/ev.txt" 2>&1; local rc=$?; expect "evidence: $1" "$4" "$rc" "$(tr '\n' '|' < "$work/ev.txt" | cut -c1-110)"; }
mkdir -p "$work/c0" "$work/c1" "$work/c2" "$work/c3"
: > "$work/clean.log"; printf 'MERGE-OUTER: 5 new files\n#2\tINITED cov: 3\n' > "$work/clean.log"
touch "$work/c1/crash-0123"; touch "$work/c2/slow-unit-aa"; touch "$work/c3/oom-bb"
printf '==77== ERROR: libFuzzer: deadly signal\nthread panicked at src/x.rs:1\n' > "$work/marker.log"
printf 'thread '"'"'<unnamed>'"'"' panicked at src/x.rs:1\n' > "$work/panic.log"
printf '==9== ERROR: libFuzzer: out-of-memory (used: 3000Mb; limit: 2048Mb)\n' > "$work/oomlog.log"
ce "no artifact, clean log"                    "$work/c0" "$work/clean.log" 1
ce "crash- artifact, clean log (merge exit 0)" "$work/c1" "$work/clean.log" 0
ce "slow-unit- artifact counts"                "$work/c2" "$work/clean.log" 0
ce "oom- artifact counts"                      "$work/c3" "$work/clean.log" 0
ce "no artifact, 'deadly signal' marker"       "$work/c0" "$work/marker.log" 0
ce "no artifact, 'panicked at' marker"         "$work/c0" "$work/panic.log" 0
ce "no artifact, 'ERROR: libFuzzer' (oom) marker" "$work/c0" "$work/oomlog.log" 0
ce "unreadable log (missing file) fails closed" "$work/c0" "$work/nonexistent.log" 0
ce "missing artifact dir fails closed"         "$work/nodir" "$work/clean.log" 0

echo "== worst_case_seconds =="
w=$(worst_case_seconds); row "default limit ${default_limit}s" "$w s" "$(awk -v w="$w" 'BEGIN{printf "%.2f min", w/60}')"
[ "$w" -le 1680 ] || { fails=$((fails + 1)); row "worst case <= 1680 s" "FAIL" ""; }
echo "FAILS=$fails"
exit $((fails != 0))
