#!/usr/bin/env bash
# Hosted AC4 shell, without unavailable systemd/delegation or GNU time; suffixed dist binaries.
set -euo pipefail
target="$(/home/lunarpulse/.cargo/bin/cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
scratch="$(mktemp -d)"
export HOME="$scratch/home" XDG_DATA_HOME="$scratch/home/.local/share" MAOS_HOME="$scratch/home/.maos"
mkdir -p "$HOME"
/home/lunarpulse/dev_ws/dist/maos-registry-server-linux-amd64 --bind 127.0.0.1:6789 --root "$scratch/registry" > /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/startup-incident/shell-component-registry.log 2>&1 &
server_pid=$!
trap 'kill -TERM "$server_pid" 2>/dev/null || true; wait "$server_pid" || true; rm -rf "$scratch"' EXIT
ready=0
for i in $(seq 1 50); do if (exec 3<>/dev/tcp/127.0.0.1/6789) 2>/dev/null; then ready=1; break; fi; sleep 0.1; done
[ "$ready" -eq 1 ] || { cat /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/startup-incident/shell-component-registry.log; exit 1; }
MAOS_SPIRIT_SIGNING_KEY="$(od -An -tx1 -N32 /dev/urandom | tr -d ' \n')" /home/lunarpulse/dev_ws/dist/maos-spirit-linux-amd64 publish --registry-uri http://127.0.0.1:6789/mcp --tier=public_untrusted --manifest /home/lunarpulse/dev_ws/examples/example-spirit-ts/manifest.toml --artifact /home/lunarpulse/dev_ws/examples/example-spirit-ts/dist/spirit.wasm --signing-key-env MAOS_SPIRIT_SIGNING_KEY
/home/lunarpulse/dev_ws/dist/maosctl-linux-amd64 spirit install example-spirit-ts@0.1.0 --registry-uri http://127.0.0.1:6789/mcp
"$target/release/maos" init --plain
manifest="$(python3 -c 'import json,os,pathlib; r=pathlib.Path(os.environ["HOME"])/".local/share/maos/registry"; print(r/"installed"/json.loads((r/"installed-index.json").read_text())["example-spirit-ts"]/"manifest.toml")')"
time -p env MAOS_17_2_CGROUP_MODE=undelegated HOME="$HOME" MAOS_HOME="$MAOS_HOME" XDG_DATA_HOME="$XDG_DATA_HOME" MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE="/home/lunarpulse/dev_ws/crates/maos-journey-test/cassettes/j0/shell-intro.json" "$target/release/maos" run "$manifest" --once 2>&1 | tee /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/startup-incident/shell-component.log
grep -F 'spirit_session_complete' /home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path-evidence/landing-1/startup-incident/shell-component.log
