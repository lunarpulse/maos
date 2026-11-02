#!/usr/bin/env python3
"""One post-contention serial sample, with final serde repair and exact pins."""
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
xtask = str(pathlib.Path(os.environ['CARGO_TARGET_DIR']) / 'debug/xtask')
os.environ['CARGO_MANIFEST_DIR'] = str(root / 'xtask')
os.environ['MAOS_17_2_CGROUP_MODE'] = 'undelegated'
failed = []
def run(label, command):
    result = subprocess.run([sys.executable, str(here / 'capture.py'), label, *command], cwd=root)
    if result.returncode:
        failed.append(label)
    return result.returncode
for gate in ('kloc-check', 'check-kernel-baseline', 'check-serde-error-handling'):
    run('repaired-' + gate, [xtask, gate, '--json'])
run('repaired-all-targets', [cargo, 'build', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '--workspace', '--all-targets'])
run('prepush-repaired', [xtask, 'prepush', '--json'])
run('host-idle-load', ['/usr/bin/uptime'])
processes = subprocess.run([sys.executable, str(here / 'capture.py'), 'host-idle-compilers', '/usr/bin/ps', '-C', 'cargo,rustc,clippy-driver', '-o', 'pid,etime,args'], cwd=root)
if processes.returncode == 0:
    sys.exit('STOP: another compiler is active; no fresh serial sample taken')
if processes.returncode != 1:
    sys.exit('STOP: could not determine compiler idleness')
run('workspace-serial-idle-final', [cargo, 'nextest', 'run', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '--workspace', '--no-fail-fast', '--profile', 'ci', '--test-threads', '1'])
junit = pathlib.Path(os.environ['CARGO_TARGET_DIR']) / 'nextest/ci/junit.xml'
shutil.copyfile(junit, here / 'serial-idle-final-junit.xml')
run('test-budget-serial-idle-final', [xtask, 'check-test-budget', '--junit', str(here / 'serial-idle-final-junit.xml'), '--json'])
command = [cargo, 'clippy', '--workspace', '--all-targets', '--locked', '--manifest-path', str(root / 'Cargo.toml'), '--message-format=json']
start = time.monotonic()
with (here / 'clippy-serde-final.jsonl').open('w') as output, (here / 'clippy-serde-final.stderr').open('w') as errors:
    result = subprocess.run(command, cwd=root, stdout=output, stderr=errors)
record = {'command': ['env', '-u', 'CI', '-u', 'GITHUB_ACTIONS', *command], 'exit_code': result.returncode, 'seconds': round(time.monotonic()-start, 3), 'log': 'clippy-serde-final.jsonl', 'stderr': 'clippy-serde-final.stderr'}
with (here / 'captured-commands.jsonl').open('a') as output:
    output.write(json.dumps(record) + '\n')
print(json.dumps(record), flush=True)
if result.returncode:
    failed.append('clippy-serde-final')
else:
    run('clippy-ratchet-serde-final', [xtask, 'clippy-ratchet', '--input', str(here / 'clippy-serde-final.jsonl'), '--baseline', str(root / 'xtask/clippy-baseline.toml')])
print('Failed final commands: ' + ', '.join(failed), flush=True)
sys.exit(bool(failed))
