#!/usr/bin/env python3
"""Continuation proof commands, always one cargo process at a time."""
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time

here = pathlib.Path(__file__).resolve().parent
root = here.parents[3]
cargo = '/home/lunarpulse/.cargo/bin/cargo'
target = pathlib.Path(os.environ['CARGO_TARGET_DIR'])
xtask = str(target / 'debug/xtask')
os.environ['CARGO_MANIFEST_DIR'] = str(root / 'xtask')
failed = []
def run(label, command):
    result = subprocess.run([sys.executable, str(here / 'capture.py'), label, *command], cwd=root)
    if result.returncode:
        failed.append(label)
    return result.returncode

base = [cargo, 'nextest', 'run', '--locked', '--manifest-path', str(root / 'Cargo.toml')]
run('workspace-serial-repaired', [*base, '--workspace', '--no-fail-fast', '--profile', 'ci', '--test-threads', '1'])
shutil.copyfile(target / 'nextest/ci/junit.xml', here / 'serial-repaired-junit.xml')
run('workspace-default', [*base, '--workspace', '--no-fail-fast'])
run('wasm-feature-build', [cargo, 'build', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'maos-bin', '--features', 'wasm-host', '--all-targets'])
run('wasm-feature-tests', [*base, '-p', 'maos-bin', '--features', 'wasm-host', '--no-fail-fast'])
run('registry-fixture-replay', [*base, '-p', 'maos-registry', '-p', 'maos-spirit-cli', '--features', 'maos-registry/fixture_replay,maos-spirit-cli/fixture_replay', '--no-fail-fast'])
run('kernel-lib', [cargo, 'test', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'maos-kernel-core', '--lib'])
run('spirit-boundary-invariants', [cargo, 'test', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'maos-kernel-core', '--test', 'spirit_boundary_invariants'])
run('perf-hosted-edit-tests', [*base, '-p', 'xtask', '--test', 'perf_gates_20_7', '--test', 'perf_budgets_20_7', '--no-fail-fast'])
run('fanout-hosted-edit-tests', [*base, '-p', 'maos-bin', '--test', 'nfr_perf_8_real_spawn_fanout_20_7', '--no-fail-fast'])
run('perf-counts-six-binding', [cargo, 'run', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '-p', 'xtask', '--', 'check-perf-counts', '--dir', '/tmp/v207-tier-c-counts', '--json'])
run('check-exit-commands', [xtask, 'check-exit-commands'])
run('check-decision-register', [xtask, 'check-decision-register'])
command = [cargo, 'clippy', '--workspace', '--all-targets', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '--message-format=json']
start = time.monotonic()
with (here / 'clippy-live.jsonl').open('w') as output, (here / 'clippy-live.stderr').open('w') as errors:
    result = subprocess.run(command, cwd=root, stdout=output, stderr=errors)
record = {'command': ['env', '-u', 'CI', '-u', 'GITHUB_ACTIONS', *command], 'exit_code': result.returncode, 'seconds': round(time.monotonic()-start, 3), 'log': 'clippy-live.jsonl', 'stderr': 'clippy-live.stderr'}
with (here / 'captured-commands.jsonl').open('a') as output:
    output.write(json.dumps(record) + '\n')
print(json.dumps(record), flush=True)
if result.returncode:
    failed.append('clippy-live')
else:
    run('clippy-ratchet-live', [xtask, 'clippy-ratchet', '--input', str(here / 'clippy-live.jsonl'), '--baseline', str(root / 'xtask/clippy-baseline.toml')])
print('Failed continuation commands: ' + ', '.join(failed), flush=True)
sys.exit(bool(failed))
