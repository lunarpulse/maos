import subprocess, shutil, os, time, hashlib, json, sys, difflib, re
ROOT='/home/lunarpulse/dev_ws'
EV=ROOT+'/_bmad-output/implementation-artifacts/20-3c-audit-truth-and-halt-receipt-residuals-evidence/landing-2'
os.makedirs(EV+'/plant-logs',exist_ok=True)
os.makedirs(EV+'/plant-cuts',exist_ok=True)
K='crates/maos-kernel-core/src/'
def rd(p): return open(os.path.join(ROOT,p)).read()
def sha(p): return hashlib.sha256(open(os.path.join(ROOT,p),'rb').read()).hexdigest()

def p1(s):
    old=") -> Result<SwapVerdict, HaltContinuityError> {\n    let carried = registry.carried_for_spirit(predecessor_spirit_pid);"
    assert s.count(old)==1
    return s.replace(old,") -> Result<SwapVerdict, HaltContinuityError> {\n    let _drained = registry.drain_for_spirit(predecessor_spirit_pid);\n    let carried = registry.carried_for_spirit(predecessor_spirit_pid);")
def p2(s):
    old="halt_registry.carried_for_spirit(predecessor_pid)"
    assert s.count(old)==1
    return s.replace(old,"halt_registry.pending_halt_ids()")
def p3(s):
    i=s.index("                let current_halt_ids: std::collections::BTreeSet<String> = snapshot")
    j=s.index("                    .collect();\n",i)+len("                    .collect();\n")
    new="                let current_halt_ids: std::collections::BTreeSet<String> = registry\n                    .pending_halt_ids()\n                    .iter()\n                    .map(|h| h.as_str().to_string())\n                    .collect();\n"
    return s[:i]+new+s[j:]
def p4(s):
    i=s.index("            Err(_) => match serde_json::to_vec(&frame.payload) {")
    j=s.index("            },\n        }\n    }\n",i)
    return s[:i]+"            Err(_) => {}\n        }\n    }\n"+s[j+len("            },\n        }\n    }\n"):]
def p5(s):
    old="                    if iac.deliver_typed(frame, 0, None).await.is_err() {\n                        record_drop(drop_site, AuditDropReason::Full);\n                    }\n"
    assert s.count(old)==1
    return s.replace(old,"                    let _ = iac.deliver_typed(frame, 0, None).await;\n")

CT='cargo test --locked --no-fail-fast -p maos-kernel-core --test routed_halt_swap_20_3b --test halt_carry_20_3c --test hot_swap_halt_continuity_corpus_integration'
CB='cargo test --locked -p maos-kernel-core --test context_budget_gate_18_5'
plants=[
 ('P1-ac2-restore-drain', 'AC2.5', K+'halt/mod.rs', p1, CT),
 ('P2-ac2-precheck-global-set', 'AC2.1 (precheck/swap same set)', K+'hot_swap/precheck.rs', p2, 'cargo test --locked -p maos-kernel-core --test halt_carry_20_3c'),
 ('P3-ac2-monitor-pending-set', 'AC2 (post-swap monitor, resolved != lost)', K+'hot_swap/post_swap_monitor.rs', p3, 'cargo test --locked -p maos-kernel-core --test halt_carry_20_3c'),
 ('P4-ac3-no-runtime-noop', 'AC3.4', K+'inference/mod.rs', p4, CB),
 ('P5-ac3-discard-delivery-error', 'AC3.2 second vector', K+'inference/mod.rs', p5, CB),
]
only=sys.argv[1:]
rows=[]
for name,ac,path,fn,cmd in plants:
    if only and name not in only: continue
    full=os.path.join(ROOT,path)
    orig=open(full).read()
    bak='/tmp/plants/'+name+'.orig'
    shutil.copyfile(full,bak)
    before=sha(path)
    new=fn(orig)
    assert new!=orig
    diff=''.join(difflib.unified_diff(orig.splitlines(True),new.splitlines(True),'a/'+path,'b/'+path))
    open(EV+'/plant-cuts/'+name+'.diff','w').write(diff)
    open(full,'w').write(new)
    t=time.time()
    r=subprocess.run('env -u CI -u GITHUB_ACTIONS '+cmd,shell=True,cwd=ROOT,capture_output=True,text=True)
    secs=time.time()-t
    out=r.stdout+r.stderr
    open(EV+'/plant-logs/'+name+'.log','w').write('$ '+cmd+'\n'+out+'\nEXIT %d (%.1fs)\n'%(r.returncode,secs))
    shutil.copyfile(bak,full)
    os.utime(full,None)
    same=subprocess.run(['cmp',bak,full]).returncode==0 and sha(path)==before
    fails=re.findall(r'^test (\S+) \.\.\. FAILED',out,re.M)
    pan=re.findall(r"panicked at [^\n]*\n([^\n]*)",out)
    rows.append(dict(name=name,ac=ac,path=path,cmd=cmd,rc=r.returncode,secs=round(secs,1),failed=fails,panics=pan[:6],restored=same))
    print(json.dumps(rows[-1]),flush=True)
json.dump(rows,open('/tmp/plants/results-'+('-'.join(only) if only else 'all')+'.json','w'),indent=1)
