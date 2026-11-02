#!/usr/bin/env python3
"""Compile, establish fixture provenance, and execute the review regression vectors."""
import os
import pathlib
import subprocess
import sys
here=pathlib.Path(__file__).resolve().parent
root=here.parents[3]
cargo='/home/lunarpulse/.cargo/bin/cargo'
xtask=str(pathlib.Path(os.environ['CARGO_TARGET_DIR'])/'debug/xtask')
def run(label, command):
    result=subprocess.run([sys.executable,str(here/'capture.py'),label,*command],cwd=root)
    if result.returncode: sys.exit(result.returncode)
run('review-exact-pins',[xtask,'kloc-check','--json'])
run('review-build-repaired',[cargo,'build','--locked','--manifest-path',str(root/'Cargo.toml'),'--workspace','--all-targets'])
run('review-fixture-provenance',[xtask,'build-test-fixtures'])
base=[cargo,'nextest','run','--locked','--manifest-path',str(root/'Cargo.toml'),'--no-fail-fast','--test-threads','1']
for label,args in [('review-green-registry',['-p','maos-registry','--test','registry_security_20_1a','--test','registry_server_bin_20_1']),('review-green-install',['-p','maos-cli','--test','spirit_install_20_1']),('review-green-boot',['-p','maos-bin','--test','installed_spirit_dispatch_20_1']),('review-green-release',['-p','xtask','--test','release_build_integrity_20_1a','--test','registry_exit_20_1','--test','release_dry_run_gate'])]:
    run(label,[*base,*args])
