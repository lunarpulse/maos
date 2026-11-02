#!/usr/bin/env python3
"""Assigned final gates and prepush, serial, retaining every command and exit."""
import os
import pathlib
import subprocess
import sys
here = pathlib.Path(__file__).resolve().parent
root = here.parents[3]
target = pathlib.Path(os.environ['CARGO_TARGET_DIR'])
xtask = str(target / 'debug/xtask')
os.environ['CARGO_MANIFEST_DIR'] = str(root / 'xtask')
failed = []
def run(label, args):
    result = subprocess.run([sys.executable, str(here / 'capture.py'), label, *args], cwd=root)
    if result.returncode:
        failed.append(label)
for gate in [
    'kloc-check', 'check-kernel-baseline', 'check-service-boundary',
    'check-env-contract', 'check-serde-error-handling',
    'check-composition-root-completeness', 'check-j1-loopback-delegation',
    'check-dev-record-completeness', 'check-epic-close-coherence',
    'check-review-findings-resolved', 'check-test-cost', 'invariant-lock',
    'coverage-matrix', 'check-coverage-matrix-completeness',
    'check-perf-runner-policy', 'check-dependency-closure', 'check-corpus',
    'check-ship-gate-completeness',
]:
    run('final-' + gate, [xtask, gate])
run('test-budget-serial', [xtask, 'check-test-budget', '--junit', str(here / 'serial-repaired-junit.xml')])
run('cargo-deny', ['/home/lunarpulse/.cargo/bin/cargo', 'deny', '--manifest-path', str(root / 'Cargo.toml'), 'check'])
os.environ['MAOS_17_2_CGROUP_MODE'] = 'undelegated'
run('prepush', [xtask, 'prepush', '--json'])
print('Failed final verification commands: ' + ', '.join(failed), flush=True)
sys.exit(bool(failed))
