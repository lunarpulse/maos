#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
ladder="$repo_root/target/debug/t2-ladder"
probe_target="$repo_root/target/t2-ladder"
runner="$repo_root/spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59"
if [ "${SPIKE_CI_BRANCH_BUILD:-false}" = true ]; then
    runner="$repo_root/target/debug/maos-wasm-runner"
fi
component="$repo_root/tests/fixtures/wasm/echo_spirit_component.wasm"
forbidden_probe="$repo_root/crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/target/release/forbidden-syscall-probe"
frame="$probe_target/runner.frame"
runner_image=${RUNNER_IMAGE:?RUNNER_IMAGE is required}
mode=${1:?mode is required}
contradictions=0
printf 'UTC %s\n' "$(date -u +%FT%TZ)"
printf '+ git rev-parse HEAD\n'
git -C "$repo_root" rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: real Rust targets, the HEAD runner, pinned component, Landlock, seccomp, and strace; not only probe-created state.\n'


notice() {
    printf '::notice title=t2-probe/%s/%s::target=%s install=%s rc=%s sig=%s errno=%s\n' "$runner_image" "$1" "$2" "$3" "$4" "$5" "$6"
}

contradiction() {
    printf '::error title=t2-probe/%s::%s rc=%s sig=%s\n' "$runner_image" "$1" "$2" "$3"
    contradictions=$((contradictions + 1))
}

result_field() {
    python3 - "$1" <<'PY'
import pathlib
import sys
text = pathlib.Path(sys.argv[1]).read_bytes().decode("utf-8", "replace")
install = next((line.split("=", 1)[1] for line in reversed(text.splitlines()) if line.startswith("SECCOMP_INSTALL=")), "not-attempted")
for line in reversed(text.splitlines()):
    if line.startswith("RESULT "):
        fields = dict(part.split("=", 1) for part in line.split()[1:])
        print(fields["rc"], fields["sig"], fields["errno"], install)
        break
else:
    print("none none none", install)
PY
}

record() {
    label=$1
    expectation=$2
    shift 2
    target=''
    program_next=false
    for argument in "$@"; do
        if [ "$program_next" = true ]; then
            target=$argument
            break
        fi
        if [ "$argument" = -- ]; then
            program_next=true
        fi
    done
    log="/tmp/t2-${label}.log"
    printf '+ %q ' "$@"
    printf '\n'
    if "$@" >"$log" 2>&1; then
        harness_rc=0
    else
        harness_rc=$?
    fi
    cat "$log"
    read -r target_rc target_sig target_errno install < <(result_field "$log")
    notice "$label" "$target" "$install" "$target_rc" "$target_sig" "$target_errno"
    case "$expectation" in
        success)
            if [ "$target_rc" != 0 ] || [ "$target_sig" != none ]; then
                contradiction "$label expected-success errno=$target_errno harness=$harness_rc" "$target_rc" "$target_sig"
            fi
            ;;
        eacces)
            if [ "$target_errno" != 13 ]; then
                contradiction "$label expected-EACCES errno=$target_errno harness=$harness_rc" "$target_rc" "$target_sig"
            fi
            ;;
        failure)
            if [ "$target_rc" = 0 ] && [ "$target_sig" = none ]; then
                contradiction "$label leave-one-out-still-succeeded" "$target_rc" "$target_sig"
            fi
            ;;
        sigsys)
            if [ "$target_sig" != 31 ]; then
                contradiction "$label expected-SIGSYS errno=$target_errno harness=$harness_rc" "$target_rc" "$target_sig"
            fi
            ;;
        *)
            printf 'unknown record expectation %s\n' "$expectation"
            exit 64
            ;;
    esac
}

prepare_targets() {
    mkdir -p "$probe_target"
    printf '+ rustc --crate-name ladder_hello t2-ladder/hello.rs.txt -o target/t2-ladder/hello-dynamic\n'
    rustc --crate-name ladder_hello "$repo_root/spikes/story-17-3a-wasm-recall/t2-ladder/hello.rs.txt" -o "$probe_target/hello-dynamic"
    printf '+ rustc --crate-name ladder_hello -C target-feature=+crt-static t2-ladder/hello.rs.txt -o target/t2-ladder/hello-static\n'
    rustc --crate-name ladder_hello -C target-feature=+crt-static "$repo_root/spikes/story-17-3a-wasm-recall/t2-ladder/hello.rs.txt" -o "$probe_target/hello-static"
    printf '+ cargo run --locked --offline --manifest-path t2-ladder/Cargo.toml --bin framegen > target/t2-ladder/runner.frame\n'
    cargo run --locked --offline --quiet --manifest-path "$repo_root/spikes/story-17-3a-wasm-recall/t2-ladder/Cargo.toml" --bin framegen > "$frame"
}

write_trace_extras() {
    printf '+ strace -f -qq -o /tmp/t2-runner.strace target/debug/maos-wasm-runner --component tests/fixtures/wasm/echo_spirit_component.wasm < target/t2-ladder/runner.frame\n'
    strace -f -qq -o /tmp/t2-runner.strace "$runner" --component "$component" < "$frame" >/tmp/t2-runner.stdout
    python3 - /tmp/t2-runner.strace /tmp/t2-runner-extras <<'PY'
import pathlib
import re
import sys
known = [
    "clone3", "set_robust_list", "gettid", "tgkill", "sched_getaffinity",
    "epoll_create1", "epoll_ctl", "epoll_wait", "eventfd2", "readv", "prctl",
    "getcwd", "uname", "mremap", "ftruncate", "membarrier", "memfd_create",
]
text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
seen = set(re.findall(r"(?:^|\s)([a-zA-Z0-9_]+)\(", text))
candidate = ["poll", "ppoll", "rt_sigaction", "clone3", "memfd_create"]
pathlib.Path(sys.argv[2]).write_text(",".join(candidate), encoding="utf-8")
recognized = sorted(name for name in seen if name in {"poll", "ppoll", "rt_sigaction"} or name in known)
print("STRACE_CANDIDATE_EXTRAS=" + ",".join(candidate) + " (all five entries are leave-one-out measured)")
print("STRACE_RECOGNIZED=" + ",".join(recognized))
PY
}

case "$mode" in
    landlock)
        prepare_targets
        record hello-as-is eacces "$ladder" --landlock as-is --seccomp off -- "$probe_target/hello-dynamic"
        record hello-rights-loader success "$ladder" --landlock rights --execute-loader --seccomp off -- "$probe_target/hello-dynamic"
        record hello-static success "$ladder" --landlock rights --static-target --seccomp off -- "$probe_target/hello-static"
        record runner-as-is eacces "$ladder" --landlock as-is --seccomp off --stdin "$frame" -- "$runner" --component "$component"
        record runner-rights-loader success "$ladder" --landlock rights --execute-loader --seccomp off --read "$component" --stdin "$frame" -- "$runner" --component "$component"
        ;;
    prepare-seccomp)
        prepare_targets
        write_trace_extras
        candidate=$(cat /tmp/t2-runner-extras)
        printf 'T2_RUNNER_EXTRAS=%s\n' "$candidate"
        notice "strace-additions" "$runner" "not-applicable" "0" "none" "$candidate"
        if [ -n "${GITHUB_OUTPUT:-}" ]; then
            printf 'runner_extras=%s\n' "$candidate" >> "$GITHUB_OUTPUT"
        fi
        ;;
    seccomp)
        prepare_targets
        candidate=${T2_RUNNER_EXTRAS:?T2_RUNNER_EXTRAS is required}
        record runner-as-is-verbatim failure "$ladder" --landlock rights --execute-loader --seccomp as-is --seccomp-order verbatim --read "$component" --stdin "$frame" -- "$runner" --component "$component"
        record runner-as-is-kill-first failure "$ladder" --landlock rights --execute-loader --seccomp as-is --seccomp-order kill-first --read "$component" --stdin "$frame" -- "$runner" --component "$component"
        record hello-dynamic-candidate success "$ladder" --landlock rights --execute-loader --seccomp candidate --seccomp-order kill-first --extra "$candidate" -- "$probe_target/hello-dynamic"
        record hello-static-candidate success "$ladder" --landlock rights --static-target --seccomp candidate --seccomp-order kill-first --extra "$candidate" -- "$probe_target/hello-static"
        record runner-candidate-kill-first success "$ladder" --landlock rights --execute-loader --seccomp candidate --seccomp-order kill-first --extra "$candidate" --read "$component" --stdin "$frame" -- "$runner" --component "$component"
        record benign-candidate success "$ladder" --landlock rights --execute-loader --seccomp candidate --seccomp-order kill-first --extra "$candidate" -- /bin/true
        record forbidden-ptrace-kill sigsys "$ladder" --landlock rights --execute-loader --seccomp candidate --seccomp-order kill-first --extra "$candidate" -- "$forbidden_probe"
    leave-one-out)
        prepare_targets
        candidate=${T2_RUNNER_EXTRAS:?T2_RUNNER_EXTRAS is required}
        start=${2:?start is required}
        count=${3:?count is required}
        IFS=, read -r -a extras <<< "$candidate"
        end=$((start + count))
        if [ "$end" -gt "${#extras[@]}" ]; then
            end=${#extras[@]}
        fi
        for ((index=start; index<end; index++)); do
            dropped=${extras[index]}
            retained=()
            for ((inner=0; inner<${#extras[@]}; inner++)); do
                if [ "$inner" -ne "$index" ]; then
                    retained+=("${extras[inner]}")
                fi
            done
            joined=$(IFS=,; printf '%s' "${retained[*]}")
            record "runner-drop-${dropped}" failure "$ladder" --landlock rights --execute-loader --seccomp candidate --seccomp-order kill-first --extra "$joined" --read "$component" --stdin "$frame" -- "$runner" --component "$component"
        done
        ;;
    *)
        printf 'unknown mode %s\n' "$mode"
        exit 64
        ;;
esac

if [ "$contradictions" -gt 0 ]; then
    exit 1
fi
