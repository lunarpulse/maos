import tomllib,re,sys
path=sys.argv[1] if len(sys.argv)>1 else '/home/lunarpulse/dev_ws'
lines=open(path+'/tests/coverage-matrix.yaml').read().split('\n')
rows={};cur=None;field=None;hdr={}
for l in lines:
    m=re.match(r'^([a-z_]+): ?(.*)$',l)
    if m: hdr[m.group(1)]=m.group(2); cur=None; continue
    m=re.match(r'^  ([^ #][^:]*):$',l)
    if m: cur=m.group(1); rows[cur]={'gates':[],'corpora':[]}; field=None; continue
    if cur is None: continue
    m=re.match(r'^    ([a-z_]+):(.*)$',l)
    if m:
        field=m.group(1); v=m.group(2).strip()
        if field in('gates','corpora') and v.startswith('['):
            rows[cur][field]=[x.strip().strip("'\"") for x in v.strip('[]').split(',') if x.strip()]
        elif v and v!='[]': rows[cur][field]=v.strip("'\"")
        continue
    m=re.match(r"^ {4,6}- (.*)$",l)
    if m and field in('gates','corpora'): rows[cur][field].append(m.group(1).strip("'\""))
order=[l[2:] for l in lines[lines.index('phase_order:')+1:] if l.startswith('- ')]
dp=hdr['delivered_phase']
reg=tomllib.load(open(path+'/xtask/gate-registry.toml','rb'));gates=set(reg['gates'])
man=tomllib.load(open(path+'/tests/corpora/MANIFEST.toml','rb')).get('corpus',{})
print('mode',hdr['mode'],'delivered',dp,'rows',len(rows),'registry gates',len(gates))
unk=[(k,g) for k,r in rows.items() for g in r['gates'] if g not in gates]
print('unknown-gate refs',len(unk),'in rows',len({k for k,_ in unk}))
print(sorted({g for _,g in unk}))
unc=[(k,c) for k,r in rows.items() for c in r['corpora'] if c not in man]
print('unknown-corpus refs',len(unc), sorted({c for _,c in unc}))
print('bad phase',[(k,rows[k].get('phase')) for k,r in rows.items() if r.get('phase') not in order])
zero=[k for k,r in rows.items() if r.get('phase') in order and order.index(r['phase'])<=order.index(dp) and not r['gates'] and not r['corpora']]
print('delivered zero coverage',zero)
print('v1.0 empty gates',len([k for k,r in rows.items() if r.get('phase')=='v1.0' and not r['gates']]))
print('total violations (unknown gate+corpus+badphase+zero)',len(unk)+len(unc)+len([1 for k,r in rows.items() if r.get('phase') not in order])+len(zero))
