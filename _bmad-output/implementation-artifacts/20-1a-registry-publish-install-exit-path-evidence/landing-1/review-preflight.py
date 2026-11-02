#!/usr/bin/env python3
"""Run after both local mutations have been restored, compared and touched."""
import json
import os
import pathlib
import subprocess
import sys
import time
here=pathlib.Path(__file__).resolve().parent
root=here.parents[3]
cargo='/home/lunarpulse/.cargo/bin/cargo'
xtask=str(pathlib.Path(os.environ['CARGO_TARGET_DIR'])/'debug/xtask')
os.environ['CARGO_MANIFEST_DIR']=str(root/'xtask')
os.environ['MAOS_17_2_CGROUP_MODE']='undelegated'
failed=[]
def run(label,command,required=False):
    result=subprocess.run([sys.executable,str(here/'capture.py'),label,*command],cwd=root)
    if result.returncode:
        failed.append(label)
        if required: sys.exit(result.returncode)
    return result.returncode
run('review-final-fmt',[cargo,'fmt','--all','--manifest-path',str(root/'Cargo.toml')],True)
run('review-final-measurement',[xtask,'kloc-check','--json'],True)
run('review-final-build',[cargo,'build','--locked','--manifest-path',str(root/'Cargo.toml'),'--workspace','--all-targets'],True)
run('review-restored-fixtures',[xtask,'build-test-fixtures'],True)
run('review-p12-restored-green',[cargo,'test','--locked','--manifest-path',str(root/'Cargo.toml'),'-p','maos-registry','--lib','http::tests::later_resolved_address_is_tried_after_connection_refusal'],True)
base=[cargo,'nextest','run','--locked','--manifest-path',str(root/'Cargo.toml'),'--no-fail-fast','--test-threads','1']
run('review-p9-restored-green',[*base,'-p','maos-cli','--test','spirit_install_20_1','independently_keyed_server_is_verified_and_wrong_key_never_persists'],True)
run('review-wasm-boot-integrity',[*base,'-p','maos-bin','--features','wasm-host','--test','installed_spirit_dispatch_20_1'],True)
for gate in ['check-kernel-baseline','check-service-boundary','check-env-contract','check-serde-error-handling','check-composition-root-completeness','check-j1-loopback-delegation','check-dev-record-completeness','check-epic-close-coherence','check-review-findings-resolved','check-test-cost','invariant-lock','coverage-matrix','check-coverage-matrix-completeness','check-perf-runner-policy','check-dependency-closure','check-corpus','check-ship-gate-completeness','check-exit-commands','check-decision-register']:
    run('review-'+gate,[xtask,gate])
command=[cargo,'clippy','--workspace','--all-targets','--locked','--manifest-path',str(root/'Cargo.toml'),'--message-format=json']
start=time.monotonic()
with (here/'review-clippy.jsonl').open('w') as output,(here/'review-clippy.stderr').open('w') as errors:
    result=subprocess.run(command,cwd=root,stdout=output,stderr=errors)
record={'command':['env','-u','CI','-u','GITHUB_ACTIONS',*command],'exit_code':result.returncode,'seconds':round(time.monotonic()-start,3),'log':'review-clippy.jsonl','stderr':'review-clippy.stderr','environment':{k:os.environ[k] for k in ['CARGO_TARGET_DIR','CARGO_MANIFEST_DIR','MAOS_17_2_CGROUP_MODE']}}
with (here/'captured-commands.jsonl').open('a') as output: output.write(json.dumps(record)+'\n')
print(json.dumps(record),flush=True)
if result.returncode: failed.append('review-clippy')
else: run('review-clippy-ratchet',[xtask,'clippy-ratchet','--input',str(here/'review-clippy.jsonl'),'--baseline',str(root/'xtask/clippy-baseline.toml')])
run('review-cargo-deny',[cargo,'deny','--manifest-path',str(root/'Cargo.toml'),'check'])
print('Failed review preflight commands: '+', '.join(failed),flush=True)
sys.exit(bool(failed))
