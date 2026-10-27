# 20-15 creation (2026-10-27): per-job compile classification of one discipline run.
# usage: gh api repos/lunarpulse/maos/actions/runs/<RUN>/jobs --paginate -q '.jobs[]' > jobs.jsonl
#        gh api repos/lunarpulse/maos/actions/runs/<RUN>/logs > logs.zip && unzip -q logs.zip -d logs
#        python3 job-pricing.method.py jobs.jsonl logs discipline.json   (discipline.json = the workflow YAML as JSON)
import collections, datetime as dt, glob, json, os, re, sys

jobs = [json.loads(l) for l in open(sys.argv[1])]
logs = {os.path.basename(f).split("_", 1)[1][:-4]: f for f in glob.glob(os.path.join(sys.argv[2], "*.txt"))}
wf = json.load(open(sys.argv[3]))["jobs"]
SETUP = re.compile(r"^(Set up job|Complete job|Post |Run actions/checkout|Run dtolnay|Run Swatinem|Run taiki-e|Run actions/(upload|download)-artifact|.*[Ii]nstall|.*toolchain|.*[Cc]ache)")
FIN = re.compile(r"Finished\s*(?:\x1b\[[0-9;]*m)*\s*`([^`]+)` profile .*? in (?:(\d+)m )?([\d.]+)s")
t = lambda s: dt.datetime.fromisoformat(s.replace("Z", "+00:00"))
name2key = {j.get("name", k): k for k, j in wf.items()}
steptext = {(k, s.get("name")): s.get("run") or "" for k, j in wf.items() for s in j.get("steps", [])}
classes, per_job = collections.Counter(), collections.defaultdict(collections.Counter)
for job in jobs:
    key = name2key.get(job["name"]) or job["name"].split(" (")[0]
    windows = [(t(s["started_at"]), t(s["completed_at"]), s["name"]) for s in job["steps"] if s.get("started_at") and s.get("completed_at")]
    for line in open(logs[job["name"]], errors="replace"):
        m = FIN.search(line)
        if not m or not line[:4].isdigit():
            continue
        stamp = t(line[:19] + "Z")
        step = next((n for a, b, n in windows if a <= stamp <= b), "unattributed")
        if SETUP.match(step):
            continue
        sec, prof, txt = int(m.group(2) or 0) * 60 + float(m.group(3)), m.group(1), steptext.get((key, step), "")
        if "build-test-fixtures" in txt or "build-test-fixtures" in step:
            c = f"fixtures({prof})"
        elif prof in ("test", "bench"):
            c = prof
        elif prof == "dev" and re.search(r"-p\s+xtask", txt):
            c = "xtask-dev"
        else:
            c = f"{'release' if prof == 'release' else 'dev'}-other"
        classes[c] += sec
        per_job[job["name"]][c] += sec
print(json.dumps({"compile_by_class_h": {k: round(v / 3600, 2) for k, v in classes.most_common()},
                  "per_job_s": {j: {k: round(v) for k, v in c.items()} for j, c in per_job.items()}}, indent=1))
