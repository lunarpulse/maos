#!/usr/bin/env bash
# Story 22-7 (D-22-7-08 A, T9) — run one recorded agent session of the Spirit authoring kit, and write its record.
#
#   agent_run_22_7.sh run    --n N --engine podman|docker --image IMAGE --bin-dir DIR --kit-dir DIR \
#                            --work DIR --harness-cmd 'CMD {PROMPT}' [--env-file FILE] [--engine-arg ARG]...
#   agent_run_22_7.sh record --n N --work DIR --harness NAME --model MODEL --kit-dir DIR \
#                            --outcome succeeded|failed --out docs/agent-runs/22-7/run-N [--ask-json FILE]
#
# `run` starts ONE clean container (IMAGE: Node 24, cargo-generate, git, a user systemd with a delegated cgroup
# v2; no MAOS source) that holds only the five release binaries (read-only, on PATH) and `kit/spirit-author/`
# (read-only, at /kit/spirit-author), pipes the fixed prompt (kit/spirit-author/RUN-PROMPT.md) to the harness,
# and captures everything it prints. `{PROMPT}` in --harness-cmd is replaced by the path of the prompt file inside
# the container (/run/prompt.txt). The run is judged by what the agent did alone: nothing is typed after the prompt.
# `record` redacts the capture, writes transcript.txt and record.toml. The operator reads the capture and states
# the outcome; `succeeded` needs the final `maosctl ask --wait` JSON (--ask-json). Failures are recorded too.
#
# The container's /tmp is the host directory <work>/scratch, so every key file the agent made is visible here and
# its contents can be redacted from the transcript.
set -euo pipefail

BINARIES=(maos maosctl maos-registry-server maos-spirit maos-wasm-runner)
MAX_BYTES=2097152

die() { echo "agent_run_22_7: $*" >&2; exit 2; }

usage() { sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }

sha256_of() { sha256sum "$1" | cut -d' ' -f1; }

# The prompt is the fenced `text` block of RUN-PROMPT.md with <KIT> replaced by the in-container kit path.
prompt_of() {
  awk '/^```text$/ {on=1; next} /^```$/ {if (on) exit} on' "$1/spirit-author/RUN-PROMPT.md" \
    | sed 's|<KIT>|/kit/spirit-author|g'
}

cmd_run() {
  local n="" engine="" image="" bin_dir="" kit_dir="" work="" harness_cmd="" env_file=""
  local -a engine_args=()
  while [ $# -gt 0 ]; do
    case "$1" in
      --n) n=$2; shift 2 ;;
      --engine) engine=$2; shift 2 ;;
      --image) image=$2; shift 2 ;;
      --bin-dir) bin_dir=$2; shift 2 ;;
      --kit-dir) kit_dir=$2; shift 2 ;;
      --work) work=$2; shift 2 ;;
      --harness-cmd) harness_cmd=$2; shift 2 ;;
      --env-file) env_file=$2; shift 2 ;;
      --engine-arg) engine_args+=("$2"); shift 2 ;;
      *) usage ;;
    esac
  done
  [ -n "$n" ] && [ -n "$engine" ] && [ -n "$image" ] && [ -n "$bin_dir" ] && [ -n "$kit_dir" ] \
    && [ -n "$work" ] && [ -n "$harness_cmd" ] || usage
  case "$engine" in podman|docker) ;; *) die "--engine must be podman or docker" ;; esac
  for b in "${BINARIES[@]}"; do [ -x "$bin_dir/$b" ] || die "missing binary $bin_dir/$b"; done
  [ -f "$kit_dir/spirit-author/AGENTS.md" ] || die "no kit at $kit_dir/spirit-author"
  case "$harness_cmd" in *'{PROMPT}'*) ;; *) die "--harness-cmd must contain {PROMPT}" ;; esac

  mkdir -p "$work/scratch" "$work/run"
  : > "$work/capture.raw"
  for b in "${BINARIES[@]}"; do echo "$b $(sha256_of "$bin_dir/$b")"; done > "$work/binaries.sha256"
  prompt_of "$kit_dir" > "$work/run/prompt.txt"
  [ -s "$work/run/prompt.txt" ] || die "empty prompt: RUN-PROMPT.md has no fenced text block"
  date -u +%Y-%m-%dT%H:%M:%SZ > "$work/start"

  local -a env_args=()
  [ -n "$env_file" ] && env_args=(--env-file "$env_file")
  set +e
  "$engine" run --rm -i "${env_args[@]}" "${engine_args[@]}" \
    -v "$bin_dir:/opt/maos/bin:ro" -v "$kit_dir:/kit:ro" \
    -v "$work/run:/run/agent:ro" -v "$work/scratch:/tmp" \
    -e PATH=/opt/maos/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    --workdir /tmp "$image" \
    bash -c "${harness_cmd//\{PROMPT\}//run/agent/prompt.txt}" < /dev/null 2>&1 | tee "$work/capture.raw"
  local status=${PIPESTATUS[0]}
  set -e
  date -u +%Y-%m-%dT%H:%M:%SZ > "$work/end"
  echo "agent_run_22_7: the harness exited $status; read $work/capture.raw, then run \`record\`" >&2
}

# Replace every literal occurrence of the secret values listed one per line on stdin.
redact_literals() {
  local file=$1 secrets=$2
  [ -s "$secrets" ] || return 0
  awk -v secrets="$secrets" '
    BEGIN { while ((getline line < secrets) > 0) if (length(line) >= 8) s[++n] = line }
    { for (i = 1; i <= n; i++) { out = ""; rest = $0
        while ((p = index(rest, s[i])) > 0) { out = out substr(rest, 1, p - 1) "[REDACTED]"; rest = substr(rest, p + length(s[i])) }
        $0 = out rest }
      print }' "$file"
}

cmd_record() {
  local n="" work="" harness="" model="" kit_dir="" outcome="" out="" ask_json=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --n) n=$2; shift 2 ;;
      --work) work=$2; shift 2 ;;
      --harness) harness=$2; shift 2 ;;
      --model) model=$2; shift 2 ;;
      --kit-dir) kit_dir=$2; shift 2 ;;
      --outcome) outcome=$2; shift 2 ;;
      --out) out=$2; shift 2 ;;
      --ask-json) ask_json=$2; shift 2 ;;
      *) usage ;;
    esac
  done
  [ -n "$n" ] && [ -n "$work" ] && [ -n "$harness" ] && [ -n "$model" ] && [ -n "$kit_dir" ] \
    && [ -n "$outcome" ] && [ -n "$out" ] || usage
  case "$outcome" in succeeded|failed) ;; *) die "--outcome must be succeeded or failed" ;; esac
  [ -f "$work/capture.raw" ] || die "no capture at $work/capture.raw (run \`run\` first)"
  local ask=""
  if [ "$outcome" = succeeded ]; then
    [ -n "$ask_json" ] && [ -s "$ask_json" ] || die "a succeeded run needs --ask-json (the final maosctl ask --wait output)"
    ask=$(cat "$ask_json")
    case "$ask" in *"'''"*) die "--ask-json contains '''" ;; esac
  fi
  local commit
  commit=$(git -C "$kit_dir" rev-parse HEAD)
  [ -z "$(git -C "$kit_dir" status --porcelain -- kit/spirit-author)" ] || die "kit/spirit-author has uncommitted changes: the run must name a commit"

  # Secrets: every *.key file the agent made, and every secret-shaped env-file value is not known here, so the
  # patterns below also catch the usual token shapes.
  local secrets="$work/secrets.txt"
  : > "$secrets"
  find "$work/scratch" -type f -name '*.key' -print0 2>/dev/null | while IFS= read -r -d '' f; do tr -d '\n' < "$f"; echo; done >> "$secrets"
  mkdir -p "$out"
  local tmp="$out/transcript.tmp"
  redact_literals "$work/capture.raw" "$secrets" \
    | sed -E 's/(sk|pk)-[A-Za-z0-9_-]{8,}/[REDACTED]/g; s/gh[pousr]_[A-Za-z0-9]{16,}/[REDACTED]/g; s/github_pat_[A-Za-z0-9_]{16,}/[REDACTED]/g; s/AKIA[0-9A-Z]{12,}/[REDACTED]/g; s/((API_KEY|_TOKEN|_SECRET|_PASSWORD)=)[^[:space:]"'\'']+/\1[REDACTED]/g' \
    | sed -E '/-----BEGIN [A-Z ]*-----/,/-----END [A-Z ]*-----/c\[REDACTED PEM BLOCK]' > "$tmp"
  if [ "$(wc -c < "$tmp")" -gt "$MAX_BYTES" ]; then
    head -c $((MAX_BYTES - 64)) "$tmp" > "$out/transcript.txt"
    printf '\n[TRUNCATED at %s bytes]\n' "$MAX_BYTES" >> "$out/transcript.txt"
  else
    cp "$tmp" "$out/transcript.txt"
  fi
  rm -f "$tmp" "$secrets"

  {
    echo 'author_kind = "agent"'
    echo "harness = \"$harness\""
    echo "model = \"$model\""
    echo "kit_commit = \"$commit\""
    echo "start = \"$(cat "$work/start")\""
    echo "end = \"$(cat "$work/end")\""
    echo "outcome = \"$outcome\""
    echo "ask_result = '''$ask'''"
    echo 'transcript = "transcript.txt"'
    echo "transcript_sha256 = \"$(sha256_of "$out/transcript.txt")\""
    echo
    echo '[binaries_sha256]'
    while read -r name digest; do echo "$name = \"$digest\""; done < "$work/binaries.sha256"
  } > "$out/record.toml"
  echo "agent_run_22_7: wrote $out/record.toml and $out/transcript.txt (run $n)" >&2
  echo "agent_run_22_7: read the transcript once more for anything the redaction could not know, then" \
       "run: cargo test -p maos-spirit-cli --test agent_runs_22_7" >&2
}

[ $# -ge 1 ] || usage
sub=$1; shift
case "$sub" in
  run) cmd_run "$@" ;;
  record) cmd_record "$@" ;;
  *) usage ;;
esac
