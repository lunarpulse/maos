# usage (in run dir): fetch_logs.py — per-job logs into logs/<i>_<name>.txt (run log zip download truncates: curl 92/18)
import json, subprocess, concurrent.futures as cf, os
tok = open(os.path.expanduser("~/.config/maos-agent/gh-ci-read.token")).read().strip()
jobs = [json.loads(l) for l in open("jobs.jsonl")]
os.makedirs("logs", exist_ok=True)
def get(ij):
    i, j = ij
    if not j.get("started_at"):
        return None
    path = f"logs/{i}_{j['name'].replace('/', '_')}.txt"
    for _ in range(4):
        r = subprocess.run(["curl", "--http1.1", "-sSfL", "--max-time", "300", "-H", f"Authorization: Bearer {tok}", "-o", path,
                            f"https://api.github.com/repos/lunarpulse/maos/actions/jobs/{j['id']}/logs"], capture_output=True, text=True)
        if r.returncode == 0:
            return None
    return (j["name"], r.stderr.strip())
with cf.ThreadPoolExecutor(8) as ex:
    errs = [e for e in ex.map(get, enumerate(jobs)) if e]
print("fetched logs; errors:", errs)
open("logs.SOURCE", "w").write("logs/ = per-job logs via /actions/jobs/{id}/logs (run log zip download repeatedly truncated: curl 92/18)\n")
