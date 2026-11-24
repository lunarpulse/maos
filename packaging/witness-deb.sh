#!/usr/bin/env bash
set -euo pipefail
dist=$(realpath "$1")
package=$2
suffix=$3
scratch=$(mktemp -d /tmp/maos-deb.XXXXXX)
trap 'rm -rf "$scratch"' EXIT
pubkey=$(python3 -c 'import pathlib,re,sys; print(re.search(r"RELEASE_PUBKEY := ([0-9a-f]{64})",pathlib.Path(sys.argv[1],"recipes/debian/rules").read_text())[1])' "$dist")
python3 packaging/verify-release.py --directory "$dist" --pubkey "$pubkey" "maos-$suffix" "maosctl-$suffix" "maos-spirit-$suffix" "maos-registry-server-$suffix" "maos-wasm-runner-$suffix"
sudo dpkg -i "$package"
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml","rb"))["workspace"]["package"]["version"])')
maos --version | tee "$scratch/version"
grep -F "$version" "$scratch/version"
python3 packaging/check-installed.py "$dist" /usr/bin "$suffix"
# Story 22-2 live systemd leg (D-22-2-06 A): the hosted runner VM runs systemd. The package
# installs the unit and the account but neither enables nor starts it (D-22-2-05 A).
test -f /usr/lib/systemd/system/maos.service
test -f /usr/share/doc/maos/examples/host.toml
getent passwd maos
systemd-analyze verify /usr/lib/systemd/system/maos.service
if systemctl is-enabled --quiet maos || systemctl is-active --quiet maos; then
  echo 'the package enabled or started maos.service' >&2; exit 1
fi
cast=/tmp/maos-witness-cast
sudo rm -rf "$cast"; sudo install -d "$cast/crates/maos-journey-test/cassettes/j-butler"
sudo cp -r spirits "$cast/spirits"
sudo cp crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json "$cast/crates/maos-journey-test/cassettes/j-butler/"
sudo chmod -R a+rX "$cast"
sudo install -d /etc/maos /etc/systemd/system/maos.service.d
printf "[host]\ninference_mode = 'replay'\n\n[[host.spirits]]\nmanifest = '%s/spirits/nash/manifest.toml'\n\n[[host.spirits]]\nmanifest = '%s/spirits/observer/manifest.toml'\n" "$cast" "$cast" | sudo tee /etc/maos/host.toml >/dev/null
printf '[Service]\nWorkingDirectory=%s\nEnvironment=MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json\nEnvironment=MAOS_NOTIFY_DISABLE=1\n' "$cast" | sudo tee /etc/systemd/system/maos.service.d/witness.conf >/dev/null
sudo systemctl daemon-reload
sudo systemctl enable --now maos
as_maos() { sudo -u maos env HOME=/var/lib/maos MAOS_HOME=/var/lib/maos "$@"; }
ready=0
for _ in $(seq 1 90); do
  if as_maos maosctl status > "$scratch/status" 2>&1; then ready=1; break; fi
  sleep 1
done
if [[ $ready != 1 ]]; then
  systemctl status maos --no-pager || true; sudo journalctl -u maos --no-pager | tail -50 || true
  echo 'maosctl status never exited 0 under systemd' >&2; exit 1
fi
cat "$scratch/status"
grep -F 'spirits=2 audit_store_writable=true' "$scratch/status"
as_maos maosctl agents > "$scratch/agents.raw"
cat "$scratch/agents.raw"
# Story 22-5 widened the table (id, pid, state, form, task, originator, age, spend); the
# witness compares id and state only, the contract it has always meant.
awk -F'\t' '{print $1 "\t" $3}' "$scratch/agents.raw" | sort > "$scratch/agents"
printf 'nash\tRunning\nobserver\tRunning\n' | cmp - "$scratch/agents"
sudo systemctl stop maos
if systemctl is-failed --quiet maos; then
  sudo journalctl -u maos --no-pager | tail -50 || true
  echo 'maos.service ended failed after systemctl stop' >&2; exit 1
fi
test "$(systemctl is-active maos || true)" = inactive
sudo journalctl -u maos --no-pager | grep -F 'exiting cleanly'
sudo systemctl disable maos
sudo rm -rf /etc/systemd/system/maos.service.d /etc/maos "$cast"
sudo systemctl daemon-reload
export HOME="$scratch/home" MAOS_HOME="$scratch/home/maos" XDG_DATA_HOME="$scratch/data" XDG_CONFIG_HOME="$scratch/config" XDG_STATE_HOME="$scratch/state" XDG_RUNTIME_DIR="$scratch/run"
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
maos init --plain
set +e
maos purge --yes --receipt "$scratch/receipt.json" > "$scratch/purge.log" 2>&1
rc=$?
set -e
cat "$scratch/purge.log"
[[ $rc == 0 || $rc == 1 ]]
python3 - "$scratch/receipt.json" "$MAOS_HOME" <<'PY'
import json, pathlib, sys
receipt = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert receipt['status'] == 'complete', receipt
assert not pathlib.Path(sys.argv[2]).exists(), 'MAOS home survives purge'
assert 'dpkg -r maos' in receipt['next_step'], receipt
for row in receipt['roots']:
    if row['disposition'] == 'removed' and not row['path'].startswith(('podman:', 'cgroup:')):
        assert not pathlib.Path(row['path']).exists(), row
PY
sudo dpkg -r maos
for bin in maos maosctl maos-spirit maos-registry-server maos-wasm-runner; do
  if [[ -e "/usr/bin/$bin" ]]; then
    echo "still present after dpkg -r: /usr/bin/$bin" >&2; ls -l "/usr/bin/$bin" >&2; exit 1
  fi
done
set +e
dpkg -L maos > "$scratch/list" 2> "$scratch/list.err"
rc=$?
set -e
# dpkg -L may report not-installed with exit 1. Since 22-2 the package ships maintainer scripts
# (dh_installsystemd, dh_installsysusers), so after `dpkg -r` it stays in the config-files state
# ('rc') until purged and dpkg -L prints the message "Package 'maos' does not contain any files
# (!)" instead of failing; what must never appear is a listed path.
[[ $rc == 0 || $rc == 1 ]]
if grep -q '^/' "$scratch/list"; then
  echo 'dpkg -L lists retained files after removal:' >&2; cat "$scratch/list" >&2
  dpkg -s maos >&2 || true; exit 1
fi
state=$(dpkg-query -W -f='${db:Status-Abbrev}' maos 2>/dev/null || true)
[[ -z "$state" || "$state" == rc* ]] || { echo "unexpected package state after dpkg -r: '$state'" >&2; exit 1; }
