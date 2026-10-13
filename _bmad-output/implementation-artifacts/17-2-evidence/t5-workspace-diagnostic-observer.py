import json
import pathlib
import time
import urllib.error
import urllib.request

BASE='https://api.github.com/repos/lunarpulse/maos'
RUN_ID=38210028700
OUTPUT=pathlib.Path('/tmp/17-2-workspace-diagnostic-proof.json')
evidence={'run_id':RUN_ID,'head_sha':'5f4757bc73def045c2e97025cb17f61a67984cdf','observations':[],'read_failures':[]}
deadline=time.monotonic()+3480

def save():
    OUTPUT.write_text(json.dumps(evidence,indent=2)+'\n')

def request(path):
    while True:
        req=urllib.request.Request(BASE+path,headers={'Accept':'application/vnd.github+json','User-Agent':'maos-17-2-disposable-proof'})
        try:
            with urllib.request.urlopen(req,timeout=30) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if error.code!=403 or error.headers.get('x-ratelimit-remaining')!='0':
                raise
            reset=int(error.headers['x-ratelimit-reset'])
            delay=max(1,reset+5-time.time())
            evidence['read_failures'].append({'error':str(error),'reset_unix':reset,'retry_delay_seconds':delay})
            save()
            if delay>=deadline-time.monotonic():
                raise SystemExit('Public quota reset exceeds bounded observation deadline; no result inferred.')
            print(json.dumps(evidence['read_failures'][-1]),flush=True)
            time.sleep(delay)

while time.monotonic()<deadline:
    run=request(f'/actions/runs/{RUN_ID}')
    assert run['head_sha']==evidence['head_sha']
    evidence['run']=run
    evidence['observations'].append({'observed_unix_time':time.time(),'status':run['status'],'conclusion':run['conclusion']})
    save()
    print(json.dumps(evidence['observations'][-1]),flush=True)
    if run['status']=='completed':
        evidence['jobs']=request(f'/actions/runs/{RUN_ID}/jobs?per_page=100')['jobs']
        evidence['annotations']={}
        save()
        for job in evidence['jobs']:
            check_id=job['check_run_url'].rsplit('/',1)[-1]
            evidence['annotations'][str(job['id'])]=request(f'/check-runs/{check_id}/annotations?per_page=100')
            save()
        print(json.dumps({'conclusion':run['conclusion'],'jobs':[{'id':j['id'],'name':j['name'],'conclusion':j['conclusion']} for j in evidence['jobs']],'evidence':str(OUTPUT)}),flush=True)
        raise SystemExit(0 if run['conclusion']=='success' else 1)
    time.sleep(300)
save()
raise SystemExit('Observation deadline reached; running CI is not an acceptance result.')
