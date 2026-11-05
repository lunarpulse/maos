#!/usr/bin/env bash
set -euo pipefail
dist=$(realpath "$1")
package=$2
suffix=$3
scratch=$(mktemp -d /tmp/maos-deb.XXXXXX)
trap 'rm -rf "$scratch"' EXIT
sudo dpkg -i "$package"
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml","rb"))["workspace"]["package"]["version"])')
maos --version | tee "$scratch/version"
grep -F "$version" "$scratch/version"
python3 packaging/check-installed.py "$dist" /usr/bin "$suffix"
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
for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "/usr/bin/$bin"; done
set +e
dpkg -L maos > "$scratch/list" 2> "$scratch/list.err"
rc=$?
set -e
# dpkg -L may report not-installed with exit 1; it must never list retained files.
[[ $rc == 0 || $rc == 1 ]]
test ! -s "$scratch/list"
