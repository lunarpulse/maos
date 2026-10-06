#!/usr/bin/env bash
# Run only inside the dedicated, systemd-delegated proof service. Never touch an
# ancestor's policy or use this helper as a production admission fallback.
set -euo pipefail
if (( $# == 0 )); then
  echo "usage: $0 COMMAND [ARG...]  (inside the maos-17-3c-proof.service unit)" >&2
  exit 2
fi
cgroup=
while IFS=: read -r hierarchy controllers path; do
  if [[ "$hierarchy" == 0 && -z "$controllers" ]]; then cgroup=$path; break; fi
done < /proc/self/cgroup
uid=$(id -u)
# The unit's own cgroup, exactly: a system unit (`sudo systemd-run --uid=...`, CI)
# or the invoking user's manager unit (`systemd-run --user`). Nothing nested.
case "$cgroup" in
  /system.slice/maos-17-3c-proof.service) ;;
  "/user.slice/user-$uid.slice/user@$uid.service/app.slice/maos-17-3c-proof.service") ;;
  *) echo "17-3c proof requires the dedicated Delegate=cpu memory service; got cgroup '$cgroup'" >&2; exit 1 ;;
esac
root=/sys/fs/cgroup$cgroup
# Idempotent: an uncollected unit may keep its empty `daemon` leaf between runs.
mkdir -p -- "$root/daemon"
printf '%s\n' "$$" > "$root/daemon/cgroup.procs"
printf '+cpu +memory\n' > "$root/cgroup.subtree_control"
exec "$@"
