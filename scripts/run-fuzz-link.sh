#!/usr/bin/env bash
# Independent workers make every libFuzzer counter and exit status observable.
set -euo pipefail
fuzz_dir=${1:?fuzz directory}
target=${2:?target}
duration=${3:?seconds}
tier=${4:?T2 or T3}
link=${5:?link identifier}
rss_flag=${6:-}
workers=$(nproc)
corpus=$(realpath "$fuzz_dir")/corpus/$target
logs=$(pwd)/out/logs-$target-$link
artifacts=$(pwd)/out/crashes-$target-$link
mkdir -p "$corpus" "$logs" "$artifacts"
cargo fuzz build --fuzz-dir "$fuzz_dir" "$target"
pids=()
for ((worker=0; worker<workers; worker++)); do
  flags=(-max_total_time="$duration" -print_final_stats=1 -artifact_prefix="$artifacts/")
  if [ -n "$rss_flag" ]; then flags+=("$rss_flag"); fi
  cargo fuzz run --fuzz-dir "$fuzz_dir" "$target" "$corpus" -- "${flags[@]}" > "$logs/worker-$worker.log" 2>&1 &
  pids+=("$!")
done
failed=0
for pid in "${pids[@]}"; do
  if ! wait "$pid"; then failed=1; fi
done
if [ "$failed" -ne 0 ] || [ -n "$(find "$artifacts" -type f -print -quit)" ]; then
  echo "::error::$tier $target link $link crashed or a worker failed; refusing evidence" >&2
  exit 1
fi
python3 scripts/record-fuzz.py --target "$target" --logs "$logs" --corpus "$corpus" \
  --workers "$workers" --tier "$tier" --commit "$GITHUB_SHA" \
  --run-id "$GITHUB_RUN_ID-$link" --output "out/record-$target-$link.json"
