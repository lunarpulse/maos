#!/bin/bash
# check-only fake C compiler for aarch64-apple-darwin: -E goes to the real
# clang (cc-rs family detection); every compile emits an EMPTY arm64 Mach-O
# object (cargo check never links).
out=""; compile=0; pre=0
args=("$@")
for ((i=0;i<${#args[@]};i++)); do
  case "${args[$i]}" in
    -Fo*) out="${args[$i]#-Fo}";;
    -o) out="${args[$((i+1))]}";;
    -c) compile=1;;
    -E) pre=1;;
  esac
done
if [ "$pre" = 1 ]; then exec /usr/bin/clang "$@"; fi
if [ "$compile" = 1 ] && [ -n "$out" ]; then
  exec /usr/bin/clang --target=arm64-apple-macos11 -x c -c /dev/null -o "$out"
fi
exit 0
