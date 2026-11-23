#!/usr/bin/env bash
# Story 22-3 per-PR proof (discipline.yml `container-build`): the compose file runs a Host with a
# persistent home, and NFR-Ops-14 idle memory is measured and bounded. Needs docker with compose
# v2, cgroup v2 and MAOS_IMAGE (the locally built image, e.g. maos:ci).
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${MAOS_IMAGE:?set MAOS_IMAGE}"
export MAOS_IMAGE
compose=(docker compose -f packaging/compose/compose.yaml)
trap '"${compose[@]}" logs --no-color maos || true; "${compose[@]}" down -v >/dev/null 2>&1 || true' EXIT

wait_healthy() {
  local cid status
  for _ in $(seq 40); do
    cid=$("${compose[@]}" ps -q maos || true)
    status=$([ -n "$cid" ] && docker inspect -f '{{.State.Health.Status}}' "$cid" 2>/dev/null || true)
    [ "$status" = healthy ] && return 0
    sleep 3
  done
  echo "maos never became healthy (last status: ${status:-none})" >&2
  return 1
}
# The cast by name: ids and timestamps differ across restarts, the named cast must not.
cast() { "${compose[@]}" exec -T maos /usr/local/bin/maosctl agents | grep -oE 'nash|observer' | sort -u; }
want=$'nash\nobserver'

"${compose[@]}" config -q
"${compose[@]}" up -d
wait_healthy
"${compose[@]}" exec -T maos /usr/local/bin/maosctl status
[ "$(cast)" = "$want" ] || { echo "cast after first start is not nash+observer" >&2; exit 1; }

# NFR-Ops-14 idle memory: the container cgroup's memory.current, 60 s after `maosctl status` = 0.
sleep 60
cid=$("${compose[@]}" ps -q maos)
mem=$(cat "/sys/fs/cgroup/system.slice/docker-${cid}.scope/memory.current")
limit=$(tr -d '[:space:]' < tests/integration/idle-memory-limit)
echo "NFR-Ops-14 idle memory: measured ${mem} B, bound ${limit} B (proposed bound = measured + 10 %: $((mem + mem / 10)))"
[ "$mem" -le "$limit" ] || { echo "idle memory ${mem} B exceeds the bound ${limit} B" >&2; exit 1; }

# A restart brings the same cast back.
"${compose[@]}" restart maos
wait_healthy
[ "$(cast)" = "$want" ] || { echo "cast after restart differs" >&2; exit 1; }

# The home survives `down` without -v.
"${compose[@]}" down
docker volume inspect maos_maos-home >/dev/null
"${compose[@]}" up -d
wait_healthy
[ "$(cast)" = "$want" ] || { echo "cast after down/up differs" >&2; exit 1; }
echo "compose proof passed"
