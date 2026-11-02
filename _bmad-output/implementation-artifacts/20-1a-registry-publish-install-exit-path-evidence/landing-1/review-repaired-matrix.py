#!/usr/bin/env python3
"""Final review matrix; retain each result and never retry the default run."""
import os
import pathlib
import subprocess
import sys
here=pathlib.Path(__file__).resolve().parent
root=here.parents[3]
cargo='/home/lunarpulse/.cargo/bin/cargo'
xtask=str(pathlib.Path(os.environ['CARGO_TARGET_DIR'])/'debug/xtask')
os.environ['CARGO_MANIFEST_DIR']=str(root/'xtask')
os.environ['MAOS_17_2_CGROUP_MODE']='undelegated'
failed=[]
def run(label,command):
    result=subprocess.run([sys.executable,str(here/'capture.py'),label,*command],cwd=root)
    if result.returncode: failed.append(label)
    return result.returncode
run('review-repaired-host-load',['/usr/bin/uptime'])
processes=subprocess.run([sys.executable,str(here/'capture.py'),'review-repaired-host-compilers','/usr/bin/ps','-C','cargo,rustc,clippy-driver','-o','pid,etime,args'],cwd=root)
if processes.returncode!=1: sys.exit('STOP: compiler idleness not established')
base=[cargo,'nextest','run','--locked','--manifest-path',str(root/'Cargo.toml'),'--workspace','--no-fail-fast']
run('review-repaired-workspace-serial',[*base,'--profile','ci','--test-threads','1'])
run('review-repaired-serial-junit-copy',['/usr/bin/cp',str(pathlib.Path(os.environ['CARGO_TARGET_DIR'])/'nextest/ci/junit.xml'),str(here/'review-repaired-serial-junit.xml')])
run('review-repaired-serial-test-budget',[xtask,'check-test-budget','--junit',str(here/'review-repaired-serial-junit.xml'),'--json'])
run('review-repaired-workspace-default',base)
run('review-repaired-fixture-replay',[cargo,'nextest','run','--locked','--manifest-path',str(root/'Cargo.toml'),'-p','maos-registry','-p','maos-spirit-cli','--features','fixture_replay','--no-fail-fast','--test-threads','1'])
run('review-repaired-kernel-lib',[cargo,'test','--locked','--manifest-path',str(root/'Cargo.toml'),'-p','maos-kernel-core','--lib'])
run('review-repaired-spirit-boundary',[cargo,'test','--locked','--manifest-path',str(root/'Cargo.toml'),'-p','maos-kernel-core','--test','spirit_boundary_invariants'])
run('review-repaired-prepush',[xtask,'prepush','--json'])
print('Failed review matrix commands: '+', '.join(failed),flush=True)
sys.exit(bool(failed))
