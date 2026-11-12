#!/usr/bin/env bash
# Exercises the pure/validating functions of scripts/minimize-fuzz-corpus.sh by sourcing
# it (the BASH_SOURCE guard keeps main from running). No root, no fuzzing.
# usage: bash function-vectors.sh <repo root containing scripts/minimize-fuzz-corpus.sh>
set -u
root=${1:?repo root}
# shellcheck disable=SC1091
source "$root/scripts/minimize-fuzz-corpus.sh"
work=$(mktemp -d /tmp/t3fix3-vectors.XXXXXX)
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

echo "== classify_merge (0 CLEAN, 1 CONFIRMED, 2 INCONCLUSIVE) =="
# cl <name> <dir> <log> <expected rc> <expected substring of the evidence or ->
cl() {
  local out rc=0
  out=$(PATH="${scan_path:-$PATH}" classify_merge "$2" "$3" 2>&1) || rc=$?   # scan_path: optional PATH shims, scoped to the classifier only
  if [ "$5" != "-" ] && ! grep -qF -- "$5" <<< "$out"; then rc=99; fi   # wrong/missing evidence line
  expect "classify: $1" "$4" "$rc" "$(tr '\n' '|' <<< "$out" | cut -c1-120)"
}
mkdir -p "$work/c0" "$work/c1" "$work/c2" "$work/c3" "$work/c4" "$work/c5" "$work/c6"
printf 'MERGE-OUTER: 5 new files\n#2\tINITED cov: 3\n' > "$work/clean.log"
touch "$work/c1/crash-0123"; touch "$work/c2/slow-unit-aa"; touch "$work/c3/oom-bb"
touch "$work/c4/timeout-cc"; touch "$work/c5/leak-dd"
printf '==77== ERROR: libFuzzer: deadly signal\nthread panicked at src/x.rs:1\n' > "$work/marker.log"
printf 'thread '"'"'<unnamed>'"'"' panicked at src/x.rs:1\n' > "$work/panic.log"
printf '==9== ERROR: libFuzzer: out-of-memory (used: 3000Mb; limit: 2048Mb)\n   To change the out-of-memory limit use -rss_limit_mb=<N>\nSUMMARY: libFuzzer: out-of-memory\n' > "$work/oomlog.log"
printf '==9== ERROR: AddressSanitizer: heap-buffer-overflow\n' > "$work/asan.log"
printf 'SUMMARY: libFuzzer: out-of-memory\n' > "$work/oomonly.log"
printf 'MERGE-OUTER: 5 new files\n\xff\xfe binary \x00 junk\n' > "$work/binary.log"
# CONFIRMED: an artifact decides, whatever the log says
cl "crash- artifact, clean log (merge exit 0)"   "$work/c1" "$work/clean.log"  1 "merge artifact: crash-0123"
cl "slow-unit- artifact counts"                  "$work/c2" "$work/clean.log"  1 "merge artifact: slow-unit-aa"
cl "oom- artifact counts"                        "$work/c3" "$work/clean.log"  1 "merge artifact: oom-bb"
cl "timeout- artifact counts"                    "$work/c4" "$work/clean.log"  1 "merge artifact: timeout-cc"
cl "leak- artifact counts"                       "$work/c5" "$work/clean.log"  1 "merge artifact: leak-dd"
cl "artifact + 'deadly signal' log"              "$work/c1" "$work/marker.log" 1 "merge log marker:"
cl "artifact + OOM log (inner-process OOM)"      "$work/c3" "$work/oomlog.log" 1 "merge artifact: oom-bb"
cl "artifact + unreadable log: artifact decides" "$work/c1" "$work/nonexistent.log" 1 "merge artifact: crash-0123"
# INCONCLUSIVE: no artifact, but a marker or a scan failure
cl "no artifact, 'ERROR: libFuzzer: out-of-memory' (outer RSS)" "$work/c0" "$work/oomlog.log" 2 "no input artifact was written"
cl "no artifact, 'deadly signal' marker"        "$work/c0" "$work/marker.log" 2 "merge log marker: ==77== ERROR: libFuzzer: deadly signal"
cl "no artifact, 'panicked at' marker"          "$work/c0" "$work/panic.log"  2 "panicked at"
cl "no artifact, 'ERROR: AddressSanitizer'"     "$work/c0" "$work/asan.log"   2 "AddressSanitizer"
cl "no artifact, bare 'out-of-memory' (SUMMARY)" "$work/c0" "$work/oomonly.log" 2 "out-of-memory"
cl "unreadable log (missing file)"              "$work/c0" "$work/nonexistent.log" 2 "reason: merge log $work/nonexistent.log could not be scanned"
cl "unreadable artifact dir (missing)"          "$work/nodir" "$work/clean.log" 2 "reason: artifact dir $work/nodir could not be scanned"
mkdir "$work/c6/locked"; chmod 000 "$work/c6"
cl "unreadable artifact dir (chmod 000)"        "$work/c6" "$work/clean.log" 2 "could not be scanned (exit 1)"
chmod 755 "$work/c6"
: > "$work/c7.log"; chmod 000 "$work/c7.log"
cl "unreadable log (chmod 000)"                 "$work/c0" "$work/c7.log" 2 "reason: merge log"
chmod 644 "$work/c7.log"
# scan timeout: shims that sleep (one dir per tool), bound cut to 1 s; PATH is shimmed for the classifier call only
for t in find grep; do mkdir "$work/shim-$t"; printf '#!/bin/sh\nsleep 30\n' > "$work/shim-$t/$t"; chmod +x "$work/shim-$t/$t"; done
b_scan=1 B_KILL=1
scan_path="$work/shim-find:$PATH" cl "artifact-dir scan timeout (find sleeps, b_scan=1)" "$work/c0" "$work/clean.log" 2 "reason: artifact dir $work/c0 could not be scanned (exit 124)"
scan_path="$work/shim-grep:$PATH" cl "log scan timeout (grep sleeps, b_scan=1)"           "$work/c0" "$work/clean.log" 2 "reason: merge log $work/clean.log could not be scanned (exit 124)"
b_scan=10 B_KILL=2
# CLEAN
cl "no artifact, clean log"                     "$work/c0" "$work/clean.log"  0 -
cl "no artifact, log with binary junk, no marker" "$work/c0" "$work/binary.log" 0 -
mkdir -p "$work/c8/sub"; cl "dir with only an empty subdirectory (no regular file)" "$work/c8" "$work/clean.log" 0 -

echo "== record_inconclusive (warning + marker file) =="
target=vec; log=$work/minimize-vec.log; crashes=$work/c0; inconclusive_file=$work/merge-inconclusive-vec.txt
ev=$(classify_merge "$work/c0" "$work/oomlog.log"); warnline=$(record_inconclusive "$ev" 0)
row "warning line" "-" "$(cut -c1-250 <<< "$warnline")"
echo "--- $inconclusive_file"; cat "$inconclusive_file"; echo "---"
[ "$(grep -c '^::warning::' <<< "$warnline")" -eq 1 ] && grep -q 'out-of-memory' "$inconclusive_file" && grep -q '^result: INCONCLUSIVE' "$inconclusive_file" \
  && row "single ::warning:: line, marker file has reason + excerpt" "rc=0/PASS" "" || { fails=$((fails + 1)); row "record_inconclusive" "FAIL" ""; }
inconclusive_file=$work/no-such-dir/m.txt; warnline=$(record_inconclusive "$ev" 0)
grep -q 'could not write' <<< "$warnline" && row "unwritable marker file only warns" "rc=0/PASS" "" || { fails=$((fails + 1)); row "unwritable marker file" "FAIL" ""; }

echo "== worst_case_seconds =="
w=$(worst_case_seconds); row "default limit ${default_limit}s" "$w s" "$(awk -v w="$w" 'BEGIN{printf "%.2f min", w/60}')"
[ "$w" -le 1680 ] || { fails=$((fails + 1)); row "worst case <= 1680 s" "FAIL" ""; }
echo "FAILS=$fails"
exit $((fails != 0))
