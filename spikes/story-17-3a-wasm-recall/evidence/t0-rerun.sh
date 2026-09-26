#!/usr/bin/env bash
# 17-3a T0 — re-run the two creation-time experiments (P19, P21) against HEAD.
# Run from the repository root:  bash spikes/story-17-3a-wasm-recall/evidence/t0-rerun.sh
# Builds everything it runs (HEAD runner, forbidden-syscall probe, shim, hello,
# the P21 repro) so the output reads the tree, not a stale binary (E15-A6).
set -u
EVD="$(cd "$(dirname "$0")" && pwd)"
SCRATCH="${TMPDIR:-/tmp}/17-3a-t0"
mkdir -p "$SCRATCH"

echo "# 17-3a T0 re-run — $(date -u +%FT%TZ) — HEAD $(git rev-parse HEAD)"
echo "kernel: $(uname -r)"
grep -E '^(Seccomp|NoNewPrivs):' /proc/self/status
echo "landlock LSM active: $(cat /sys/kernel/security/lsm 2>/dev/null || echo unknown)"

echo; echo '## build'
cargo build -q -p maos-wasm-host --bin maos-wasm-runner || exit 1
cargo build -q --release --manifest-path crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/Cargo.toml || exit 1
gcc -shared -fPIC -o "$SCRATCH/shim.so" "$EVD/p19-shim.c" || exit 1
rustc -O --edition 2021 --crate-name hello -o "$SCRATCH/hello" "$EVD/p19-hello.rs.txt" || exit 1
cargo build -q --offline --manifest-path "$EVD/p21-landlock-repro/Cargo.toml" --target-dir "$SCRATCH/p21-target" || exit 1

RUNNER="$PWD/target/debug/maos-wasm-runner"
PROBE="$PWD/crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/target/release/forbidden-syscall-probe"
FIXTURE="$PWD/tests/fixtures/wasm/echo_spirit_component.wasm"
P21="$SCRATCH/p21-target/debug/p21-landlock-repro"

# rc of a command whose stdin is /dev/null; 128+N means killed by signal N.
rc_of() { "$@" </dev/null >"$SCRATCH/out" 2>"$SCRATCH/err"; echo $?; }

echo; echo '## P19 — seccomp EPERM emulated by LD_PRELOAD (poll / signal)'
for target in hello probe runner; do
  case $target in
    hello)  cmd=("$SCRATCH/hello") ;;
    probe)  cmd=("$PROBE") ;;
    runner) cmd=("$RUNNER" --component "$FIXTURE") ;;
  esac
  base=$(rc_of "${cmd[@]}")
  ctl=$(LD_PRELOAD="$SCRATCH/shim.so" rc_of "${cmd[@]}")
  pol=$(LD_PRELOAD="$SCRATCH/shim.so" P19_POLL=1 rc_of "${cmd[@]}"); polerr=$(head -c 160 "$SCRATCH/err" | tr '\n' ' ')
  sig=$(LD_PRELOAD="$SCRATCH/shim.so" P19_SIGNAL=1 rc_of "${cmd[@]}"); sigerr=$(head -c 160 "$SCRATCH/err" | tr '\n' ' ')
  echo "$target: baseline rc=$base | shim control rc=$ctl | P19_POLL rc=$pol [$polerr] | P19_SIGNAL rc=$sig [$sigerr]"
done

echo; echo '## P21 — kernel T2 Landlock ruleset (verbatim), restrict_self in pre_exec'
for prog in /bin/true "$SCRATCH/hello" "$PROBE" "$RUNNER"; do
  "$P21" "$prog"
done
