#!/usr/bin/env bash
# 20-1 creation — exit-block lines 2, 4, 5, 7, 8, 9 run against the prebuilt binaries
# (no cargo). Isolated HOME/XDG per run under /tmp/c201 (deleted at the end).
# Usage: exit-probe.sh <repo-root> <bin-dir>
set -u
ROOT=${1:?repo root}; BIN=${2:?bin dir}
W=/tmp/c201; rm -rf "$W"; mkdir -p "$W/home" "$W/dist" "$W/reg"
ENVX=(env -i PATH=/usr/bin:/bin HOME="$W/home" XDG_DATA_HOME="$W/home/.local/share"
  XDG_CONFIG_HOME="$W/home/.config" XDG_RUNTIME_DIR="$W/run")
mkdir -p "$W/run"; chmod 700 "$W/run"
row() { printf '\n### %s\n$ %s\nrc=%s (%s ms)\n' "$1" "$2" "$3" "$4"; sed -n '1,12p' "$5"; }
run() { # label, cmd...
  local label=$1; shift; local t0 t1 rc
  t0=$(date +%s%N); timeout 25 "$@" >"$W/out" 2>&1 </dev/null; rc=$?; t1=$(date +%s%N)
  row "$label" "$*" "$rc" "$(( (t1-t0)/1000000 ))" "$W/out"; }

echo "# 20-1 exit probe @ $(git -C "$ROOT" rev-parse --short HEAD); binaries:"
for b in maos maosctl maos-registry-server maos-spirit xtask; do
  stat -c '%n %y' "$BIN/$b"; done

# Line 2 + 4: stage a two-binary dist from the prebuilt binaries, dev-sign, verify.
cp "$BIN/maos" "$W/dist/maos-linux-amd64"; cp "$BIN/maosctl" "$W/dist/maosctl-linux-amd64"
( cd "$W/dist" && sha256sum maos-linux-amd64 maosctl-linux-amd64 > SHA256SUMS )
run "L2 release-verify --sign (dev seed)" env -i PATH=/usr/bin:/bin \
  RELEASE_SIGNING_KEY=794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d \
  CARGO_MANIFEST_DIR="$ROOT/xtask" "$BIN/xtask" release-verify --sign --key-env RELEASE_SIGNING_KEY \
  --sha256sums "$W/dist/SHA256SUMS" --output "$W/dist/SHA256SUMS.sig"
run "L4 maosctl install --from-local --verify-only" "${ENVX[@]}" "$BIN/maosctl" install --from-local "$W/dist" --verify-only

# Line 5: the registry-server binary.
run "L5 maos-registry-server --bind --root" "${ENVX[@]}" "$BIN/maos-registry-server" --bind 127.0.0.1:0 --root "$W/reg"

# Line 5 (alternative at HEAD): the daemon one-shot arm, fixed 127.0.0.1:6789, store at ~/.local/share/maos/registry.
if (exec 3<>/dev/tcp/127.0.0.1/6789) 2>/dev/null; then echo "port 6789 busy — daemon arm skipped"; else
  "${ENVX[@]}" MAOS_ONE_SHOT=registry-server "$BIN/maos" >"$W/srv.log" 2>&1 & SRV=$!
  ok=0; for i in $(seq 1 50); do (exec 3<>/dev/tcp/127.0.0.1/6789) 2>/dev/null && { ok=1; break; }; sleep 0.1; done
  printf '\n### L5-alt MAOS_ONE_SHOT=registry-server maos\nready=%s after %s polls\n' "$ok" "$i"
  python3 - <<'PY'
import json, http.client
c = http.client.HTTPConnection("127.0.0.1", 6789, timeout=5)
body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "registry.search", "params": {"text": "example"}})
c.request("POST", "/mcp", body, {"Content-Type": "application/json"})
r = c.getresponse(); print("registry.search ->", r.status, r.read()[:300].decode())
c = http.client.HTTPConnection("127.0.0.1", 6789, timeout=5)
body = json.dumps({"jsonrpc": "2.0", "id": 2, "method": "registry.deprecate",
                   "params": {"spirit_id": "anyone-elses-spirit", "version": "0.1.0", "reason": "unauthenticated yank probe"}})
c.request("POST", "/mcp", body, {"Content-Type": "application/json"})
r = c.getresponse(); print("registry.deprecate (no credential) ->", r.status, r.read()[:300].decode())
PY
  kill "$SRV" 2>/dev/null; wait "$SRV" 2>/dev/null; sed -n '1,4p' "$W/srv.log"
  echo "yanks.json after probe:"; cat "$W/home/.local/share/maos/registry/yanks.json" 2>/dev/null | head -c 400; echo
fi

# Line 7: publish (third-party key, public_untrusted) — and the same at the manifest's own tier.
KEY=$(od -An -tx1 -N32 /dev/urandom | tr -d ' \n')
M="$ROOT/examples/example-spirit-ts/manifest.toml"; A="$ROOT/examples/example-spirit-ts/dist/spirit.wasm"
run "L7 maos-spirit publish --tier=public_untrusted" "${ENVX[@]}" MAOS_SPIRIT_SIGNING_KEY="$KEY" "$BIN/maos-spirit" publish \
  --registry-uri http://127.0.0.1:6789/mcp --tier=public_untrusted --manifest "$M" --artifact "$A" --signing-key-env MAOS_SPIRIT_SIGNING_KEY
run "L7b maos-spirit publish --tier=local (manifest's tier)" "${ENVX[@]}" MAOS_SPIRIT_SIGNING_KEY="$KEY" "$BIN/maos-spirit" publish \
  --registry-uri http://127.0.0.1:6789/mcp --tier=local --manifest "$M" --artifact "$A" --signing-key-env MAOS_SPIRIT_SIGNING_KEY

# Line 8: install.
run "L8 maosctl spirit install" "${ENVX[@]}" "$BIN/maosctl" spirit install example-spirit-ts@0.1.0 --registry-uri http://127.0.0.1:6789/mcp

# Line 9: init + replay shell, the installed-Spirit dispatch.
run "L9a maos init" "${ENVX[@]}" "$BIN/maos" init --plain
printf '@example-spirit-ts hello\n' > "$W/in"
t0=$(date +%s%N); ( cd "$ROOT" && env -i PATH=/usr/bin:/bin HOME="$W/home" XDG_DATA_HOME="$W/home/.local/share" \
  XDG_CONFIG_HOME="$W/home/.config" XDG_RUNTIME_DIR="$W/run" MAOS_INFERENCE_MODE=replay \
  MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j0/shell-intro.json \
  timeout 25 "$BIN/maos" shell --plain <"$W/in" >"$W/out" 2>&1 ); rc=$?; t1=$(date +%s%N)
row "L9b replay maos shell, @example-spirit-ts" "printf '@example-spirit-ts hello\n' | MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=… maos shell --plain" "$rc" "$(( (t1-t0)/1000000 ))" "$W/out"
echo; grep -n -i 'example-spirit-ts\|unknown\|expected @\|engine' "$W/out" | head -8

rm -rf "$W"
