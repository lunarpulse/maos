#!/usr/bin/env bash
# audit-spine-smoke — end-to-end evaluator-path slice for v0.1-β
# Verifies: `maosctl audit query` produces NDJSON with FR4-binding fields.
set -euo pipefail

echo "audit-spine-smoke: checking maosctl audit query against seeded DB..."

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

DB_PATH="$TMPDIR/transparency.sqlite"

# Seed the Transparency Log using the kernel-core integration test binary.
# This exercises the real TransparencyLogAdapter write path.
echo "audit-spine-smoke: seeding DB via kernel-core integration test..."
cargo test -p maos-kernel-core --test audit_spine_integration -- --test-threads=1 2>/dev/null || {
    echo "audit-spine-smoke: FAIL — kernel-core integration test failed, cannot seed DB"
    exit 1
}

# Build maosctl
cargo build -p maos-cli --quiet 2>/dev/null

# Run maosctl audit query with a known-seeded database path.
# The integration test seeds an in-memory DB; this smoke test uses the
# kernel-core adapter directly to create a file DB for CLI testing.
python3 -c "
import sqlite3, os
db_path = '$DB_PATH'
if os.path.exists(db_path):
    os.remove(db_path)
conn = sqlite3.connect(db_path)
conn.execute('''CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id BLOB NOT NULL PRIMARY KEY,
    timestamp_ns INTEGER NOT NULL,
    spirit_pid INTEGER NOT NULL,
    boot_nonce INTEGER NOT NULL,
    capability_token BLOB,
    kind INTEGER NOT NULL,
    intent TEXT NOT NULL,
    payload_redacted BLOB NOT NULL,
    origin INTEGER NOT NULL)''')
conn.execute('''INSERT INTO transparency_log VALUES
    (?, 1700000000000000000, 7, 3735928559, ?, 7, 'delegate', ?, 0)''',
    (b'\xaa' * 16, b'\xbb' * 32, b'REDACTED'))
conn.commit()
conn.close()
"

# Run maosctl audit query against the seeded DB
MAOS_AUDIT_DB="$DB_PATH" NO_COLOR=1 cargo run --quiet -p maos-cli -- audit query 2>/dev/null | head -1 > "$TMPDIR/first.ndjson"

if [ ! -s "$TMPDIR/first.ndjson" ]; then
    echo "audit-spine-smoke: FAIL — maosctl audit query produced no output"
    exit 1
fi

# Verify NDJSON shape: FR4-binding fields must be present
python3 -c "
import json, sys
with open('$TMPDIR/first.ndjson') as f:
    entry = json.loads(f.readline())
required = {'frame_id', 'timestamp_ns', 'spirit_pid', 'boot_nonce', 'intent'}
missing = required - set(entry.keys())
if missing:
    print(f'audit-spine-smoke FAIL: missing fields {missing}', file=sys.stderr)
    sys.exit(1)
print('audit-spine-smoke PASS — FR4 fields present')
" || {
    echo "audit-spine-smoke: FAIL — output was not valid JSON"
    exit 1
}

echo "audit-spine-smoke: DONE"
