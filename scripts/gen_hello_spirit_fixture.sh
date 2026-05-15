#!/usr/bin/env bash
# Story 1b.5b — FR4 1000-entry fixture generator wrapper.
#
# Runs `crates/maos-audit/src/bin/gen_fixture.rs` with the canonical seed
# (`SEED = 0x5BF01A5B5BF01A5B` — encoded inside the generator binary) and
# writes the deterministic NDJSON to
# `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl`.
#
# The generator is byte-deterministic given a fixed seed and reproduces the
# checked-in fixture exactly on `ubuntu-latest`.
#
# Usage:
#   bash scripts/gen_hello_spirit_fixture.sh
#
# After running, `git diff crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl`
# should be empty — regeneration drift fails CI via
# `crates/maos-audit/tests/fr4_full_mediation_test.rs::fixture_is_byte_deterministic`.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
OUT="${REPO_ROOT}/crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl"

mkdir -p "$(dirname "$OUT")"

# Build the generator once; subsequent invocations are no-ops.
cargo build --quiet -p maos-audit --bin gen_fixture --locked

# Run it. The seed lives in the generator source — keep it in sync if you
# ever rotate it (and re-check in the fixture).
"${REPO_ROOT}/target/debug/gen_fixture" "$OUT"

echo "gen_hello_spirit_fixture: wrote $OUT"
wc -l "$OUT"
