import subprocess, shutil, os, time, hashlib, json, re
ROOT='/home/lunarpulse/dev_ws'
EV=ROOT+'/_bmad-output/implementation-artifacts/20-3c-audit-truth-and-halt-receipt-residuals-evidence/landing-2'
K='crates/maos-kernel-core/src/'
files=[K+'halt/mod.rs',K+'hot_swap/precheck.rs',K+'hot_swap/coordinator.rs',K+'hot_swap/post_swap_monitor.rs',K+'inference/mod.rs']
def sha(p): return hashlib.sha256(open(os.path.join(ROOT,p),'rb').read()).hexdigest()
before={f:sha(f) for f in files}
for f in files: shutil.copyfile(os.path.join(ROOT,f),'/tmp/plants/t0-'+f.replace('/','_'))
diff=''
for f in files:
    old=subprocess.run(['git','show','HEAD:'+f],cwd=ROOT,capture_output=True,text=True).stdout
    open(os.path.join(ROOT,f),'w').write(old)
cmd='cargo test --locked --no-fail-fast -p maos-kernel-core --test routed_halt_swap_20_3b --test halt_carry_20_3c --test hot_swap_halt_continuity_corpus_integration --test context_budget_gate_18_5 --test audit_sink_truth_16_5'
t=time.time()
r=subprocess.run('env -u CI -u GITHUB_ACTIONS '+cmd,shell=True,cwd=ROOT,capture_output=True,text=True)
secs=time.time()-t
out=r.stdout+r.stderr
open(EV+'/plant-logs/P0-t0-pre-change-kernel-tree.log','w').write('$ '+cmd+'\n(the five kernel src files below restored to their committed HEAD 3ea4bbb9 bytes for this run)\n'+'\n'.join(files)+'\n'+out+'\nEXIT %d (%.1fs)\n'%(r.returncode,secs))
ok=True
for f in files:
    shutil.copyfile('/tmp/plants/t0-'+f.replace('/','_'),os.path.join(ROOT,f)); os.utime(os.path.join(ROOT,f),None)
    ok&= sha(f)==before[f]
fails=re.findall(r'^test (\S+) \.\.\. FAILED',out,re.M)
passes=re.findall(r'^test (\S+) \.\.\. ok',out,re.M)
print(json.dumps(dict(rc=r.returncode,secs=round(secs,1),failed=fails,passed=passes,restored=ok)))
