#!/usr/bin/env python3
"""Final continuation: repaired prerequisites, authorized lint fixes, gates, prepush."""
import json
import os
import pathlib
import subprocess
import sys
import time
here = pathlib.Path(__file__).resolve().parent
root = here.parents[3]
cargo = '/home/lunarpulse/.cargo/bin/cargo'
xtask = str(pathlib.Path(os.environ['CARGO_TARGET_DIR']) / 'debug/xtask')
os.environ['CARGO_MANIFEST_DIR'] = str(root / 'xtask')
failed = []
def run(label, command):
    result = subprocess.run([sys.executable, str(here / 'capture.py'), label, *command], cwd=root)
    if result.returncode:
        failed.append(label)
    return result.returncode
run('fmt-authorized-fixes', [cargo, 'fmt', '--all', '--manifest-path', str(root / 'Cargo.toml')])
if run('post-bench-kloc', [xtask, 'kloc-check', '--json']) or run('post-bench-kernel', [xtask, 'check-kernel-baseline', '--json']):
    sys.exit('STOP: authorized benchmark cleanup moved a pin')
base = [cargo, 'nextest', 'run', '--locked', '--manifest-path', str(root / 'Cargo.toml')]
run('perf-hosted-edits-corrected-selector', [*base, '-p', 'xtask', '--test', 'perf_gates_20_7', '--no-fail-fast'])
run('fanout-authorized-lint-fix', [*base, '-p', 'maos-bin', '--test', 'nfr_perf_8_real_spawn_fanout_20_7', '--no-fail-fast'])
os.environ['MAOS_17_2_CGROUP_MODE'] = 'undelegated'
run('wasm-feature-undelegated-serial', [*base, '-p', 'maos-bin', '--features', 'wasm-host', '--no-fail-fast', '--test-threads', '1'])
run('all-targets-after-authorized-fixes', [cargo, 'build', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '--workspace', '--all-targets'])
command = [cargo, 'clippy', '--workspace', '--all-targets', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '--message-format=json']
start = time.monotonic()
with (here / 'clippy-final.jsonl').open('w') as output, (here / 'clippy-final.stderr').open('w') as errors:
    result = subprocess.run(command, cwd=root, stdout=output, stderr=errors)
record = {'command': ['env', '-u', 'CI', '-u', 'GITHUB_ACTIONS', *command], 'exit_code': result.returncode, 'seconds': round(time.monotonic()-start, 3), 'log': 'clippy-final.jsonl', 'stderr': 'clippy-final.stderr'}
with (here / 'captured-commands.jsonl').open('a') as output:
    output.write(json.dumps(record) + '\n')
print(json.dumps(record), flush=True)
if result.returncode:
    failed.append('clippy-final')
else:
    run('clippy-ratchet-final', [xtask, 'clippy-ratchet', '--input', str(here / 'clippy-final.jsonl'), '--baseline', str(root / 'xtask/clippy-baseline.toml')])
result = subprocess.run([sys.executable, str(here / 'verify-gates.py')], cwd=root)
if result.returncode:
    failed.append('final-gates')
print('Failed final continuation commands: ' + ', '.join(failed), flush=True)
sys.exit(bool(failed))
