#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
image=${RUNNER_IMAGE:?RUNNER_IMAGE is required}
extras=${T2_RUNNER_EXTRAS:?T2_RUNNER_EXTRAS is required}
relay_extras="${extras},sendto,recvfrom"
workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

printf 'UTC %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf '+ git rev-parse HEAD\n'
git rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: real spike-runner, Rust recall guest, recall-service, candidate seccomp filter, and seeded log frames; not only probe-created state.\n'
printf 'T2_RELAY_EXTRAS=%s\n' "$relay_extras"

status=0
relay_result() {
    python3 - "$1" <<'PY'
import pathlib
import sys
lines = pathlib.Path(sys.argv[1]).read_text(errors="replace").splitlines()
install = next((line.split("=", 1)[1] for line in reversed(lines) if line.startswith("SECCOMP_INSTALL=")), "not-attempted")
for line in reversed(lines):
    if line.startswith("RESULT "):
        fields = dict(part.split("=", 1) for part in line.split()[1:])
        print(fields["rc"], fields["sig"], fields["errno"], install)
        break
else:
    print("none none none " + install)
PY
}
run_fd() {
    local label=$1
    local flags=$2
    local precompiled=$3
    local log="$workdir/fd-${label}.log"
    printf '+ env T2_SECCOMP=candidate T2_SECCOMP_ORDER=kill-first T2_RUNNER_EXTRAS=%s SPIKE_RUNNER_FLAGS=%s SPIKE_PRECOMPILED=%s python3 relay_fd.py\n' "$relay_extras" "$flags" "$precompiled"
    if env T2_SECCOMP=candidate T2_SECCOMP_ORDER=kill-first T2_RUNNER_EXTRAS="$relay_extras" SPIKE_RUNNER_FLAGS="$flags" SPIKE_PRECOMPILED="$precompiled" python3 spikes/story-17-3a-wasm-recall/t2-ladder/relay_fd.py | tee "$log"; then
        read -r target_rc target_sig target_errno install < <(relay_result "$log")
        if grep -Fq 'LADDER_RC=0 SERVICE_RC=0' "$log"; then
            printf '::notice title=t2-probe/%s/relay-fd-%s::target=spike-runner rung=receiver install=%s rc=%s sig=%s errno=%s expected inherited-descriptor recall relay completed under candidate seccomp\n' "$image" "$label" "$install" "$target_rc" "$target_sig" "$target_errno"
        else
            printf '::error title=t2-probe/%s/relay-fd-%s::probe exited zero without a clean ladder/service result\n' "$image" "$label"
            status=1
        fi
    else
        printf '::error title=t2-probe/%s/relay-fd-%s::candidate seccomp denied a required relay operation\n' "$image" "$label"
        status=1
    fi
}
run_fd_denied() {
    local label=$1
    local denied_extra=$2
    local log="$workdir/fd-denied-${label}.log"
    printf '+ env T2_SECCOMP=candidate T2_SECCOMP_ORDER=kill-first T2_RUNNER_EXTRAS=%s python3 relay_fd.py\n' "$denied_extra"
    if env T2_SECCOMP=candidate T2_SECCOMP_ORDER=kill-first T2_RUNNER_EXTRAS="$denied_extra" python3 spikes/story-17-3a-wasm-recall/t2-ladder/relay_fd.py | tee "$log"; then
        harness_rc=0
    else
        harness_rc=$?
    fi
    read -r target_rc target_sig target_errno install < <(relay_result "$log")
    if grep -Fq 'recall\":\"error' "$log"; then
        printf '::notice title=t2-probe/%s/relay-drop-%s::target=spike-runner rung=receiver install=%s rc=%s sig=%s errno=%s harness=%s required inherited-socket operation was denied\n' "$image" "$label" "$install" "$target_rc" "$target_sig" "$target_errno" "$harness_rc"
    else
        printf '::error title=t2-probe/%s/relay-drop-%s::removing the required inherited-socket operation did not produce a classified denial\n' "$image" "$label"
        status=1
    fi
}


run_fd default '' ''
run_fd no-parallel-compilation '--no-parallel-compilation' ''
run_fd no-signals-based-traps '--no-signals-based-traps' ''
run_fd no-compile-watchdog '--no-compile-watchdog' ''

precompiled="$workdir/recall.cwasm"
printf '+ spike-runner precompile --component guest --out %s\n' "$precompiled"
if target/release/spike-runner precompile --component target/wasm32-wasip2/release/story_17_3a_recall_guest.wasm --out "$precompiled"; then
    run_fd precompiled '' "$precompiled"
else
    printf '::error title=t2-probe/%s/relay-precompile::outside-sandbox precompile failed\n' "$image"
    status=1

fi
run_fd_denied sendto "${extras},recvfrom"
run_fd_denied recvfrom "${extras},sendto"

path_log="$workdir/path.log"
printf '+ env T2_SECCOMP=candidate T2_SECCOMP_ORDER=kill-first T2_RUNNER_EXTRAS=%s run-relay-path.sh\n' "$relay_extras"
if env T2_SECCOMP=candidate T2_SECCOMP_ORDER=kill-first T2_RUNNER_EXTRAS="$relay_extras" spikes/story-17-3a-wasm-recall/t2-ladder/run-relay-path.sh | tee "$path_log"; then
    read -r target_rc target_sig target_errno install < <(relay_result "$path_log")
    if grep -Fq 'recall\":\"error' "$path_log"; then
        printf '::notice title=t2-probe/%s/relay-path::target=spike-runner rung=receiver install=%s rc=%s sig=%s errno=%s candidate filter denied socket-path recall while inherited descriptor remained the tested alternative\n' "$image" "$install" "$target_rc" "$target_sig" "$target_errno"
    else
        printf '::error title=t2-probe/%s/relay-path::socket-path recall unexpectedly completed or produced no classified guest result\n' "$image"
        status=1
    fi
else
    printf '::error title=t2-probe/%s/relay-path::socket-path probe did not yield a classified guest result\n' "$image"
    status=1
fi

exit "$status"
