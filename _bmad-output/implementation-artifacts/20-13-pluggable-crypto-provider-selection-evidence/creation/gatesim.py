import os,sys
root=sys.argv[1]
allowed=["maos-bin/src/crypto_provider.rs","maos-kernel-core/src/security/crypto.rs","maos-bench/src/"]
for dp,dn,fn in os.walk(root+'/crates'):
  for f in fn:
    if not f.endswith('.rs'): continue
    p=os.path.join(dp,f); rel=p.split('crates/',1)[1]
    if '/src/' not in rel or any(rel.startswith(a) for a in allowed): continue
    t=open(p,errors='replace').read()
    import re
    m=re.search(r'(?m)^\s*#\[cfg\((?:all\()?test[^\]]*\]\s*\n\s*(?:pub(?:\(crate\))? )?mod ',t)
    prod=t[:m.start()] if m else t
    for n,l in enumerate(prod.split('\n')):
      s=l.strip()
      if 'RingCryptoProvider' in l and not s.startswith('//') and not s.startswith('use ') and not s.startswith('pub use ') and not s.endswith(','):
        print(f"{rel}:{n+1}: {s[:110]}")
