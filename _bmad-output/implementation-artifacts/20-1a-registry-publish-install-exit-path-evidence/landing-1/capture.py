#!/usr/bin/env python3
"""Capture an assigned verification command without changing its inherited target dir."""
import json
import os
import pathlib
import subprocess
import sys
import time

here = pathlib.Path(__file__).resolve().parent
root = here.parents[3]
label, *command = sys.argv[1:]
start = time.monotonic()
log = here / (label + '.log')
with log.open('w') as output:
    result = subprocess.run(command, cwd=root, stdout=output, stderr=subprocess.STDOUT)
record = {'command': ['env', '-u', 'CI', '-u', 'GITHUB_ACTIONS', *command],
          'exit_code': result.returncode, 'seconds': round(time.monotonic() - start, 3),
          'log': log.name}
record['environment'] = {key: os.environ[key] for key in
    ['CARGO_TARGET_DIR', 'CARGO_MANIFEST_DIR', 'MAOS_17_2_CGROUP_MODE']
    if key in os.environ}
ledger = here / 'captured-commands.jsonl'
with ledger.open('a') as output:
    output.write(json.dumps(record) + '\n')
print(json.dumps(record), flush=True)
sys.exit(result.returncode)
