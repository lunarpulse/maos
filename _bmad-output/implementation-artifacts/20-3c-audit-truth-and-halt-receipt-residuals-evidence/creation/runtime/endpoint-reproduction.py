import os,re,subprocess,sys,tempfile,socket,json,time
B='/mnt/build/cargo-maos/debug/maos'
root=tempfile.mkdtemp(prefix='maos-endpoint-20-3c-',dir='/tmp/c203c/ep')
env={'PATH':'/usr/bin:/bin','HOME':root+'/home','MAOS_HOME':root+'/maos-home','XDG_DATA_HOME':root+'/xdg-data','XDG_CONFIG_HOME':root+'/xdg-config','MAOS_MEMORY_ROOT':root+'/memory','MAOS_ARCHIVE_DIR':root+'/archives','MAOS_CRL_PATH':root+'/crl','MAOS_ERASURE_PROOFS_DIR':root+'/proofs','MAOS_REGISTRY_YANK_CURSOR_PATH':root+'/yank.json','MAOS_NOTIFY_DISABLE':'1'}
for k in ('home','xdg-data','xdg-config'): os.makedirs(root+'/'+k,exist_ok=True)
rec={}
p=subprocess.run([B,'init','--plain'],env=env,capture_output=True,text=True,timeout=60)
rec['init_rc']=p.returncode
m=re.search(r'tcp://([0-9.]+):(\d+)',p.stdout); host,port=m.group(1),int(m.group(2)); rec['endpoint']=f'{host}:{port}'
# does anything listen on the recorded port after init returns?
s=socket.socket()
try:
  s.bind((host,port)); s.listen(1); rec['foreign_bind_after_init']='succeeded (init holds nothing)'
except OSError as e:
  rec['foreign_bind_after_init']=f'failed: {e}'
rec['holder_pid']=os.getpid()
p=subprocess.run([B,'shell','--plain'],env=env,capture_output=True,text=True,timeout=120,stdin=subprocess.DEVNULL)
rec['held_rc']=p.returncode
rec['held_line']=[l for l in p.stderr.splitlines() if 'EndpointInUse' in l]
s.close()
p=subprocess.run([B,'shell','--plain'],env=env,capture_output=True,text=True,timeout=120,stdin=subprocess.DEVNULL)
rec['released_rc']=p.returncode
rec['released_listen_line']=[l for l in p.stderr.splitlines() if 'operator HTTP listening' in l]
print(json.dumps(rec,indent=1))
