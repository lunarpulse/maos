import json, re, subprocess, sys, datetime, collections

RUN = sys.argv[1]
REPO = "lunarpulse/maos"
BUCKETS = ["wasm-host-tests", "cgroup-ceiling-undelegated", "cgroup-ceiling-delegated", "epic-17-exit", "workspace-test-suite"]
SETUP = re.compile(r"^(Set up job|Complete job|Post |Run actions/checkout|Run dtolnay|Run Swatinem|Run taiki-e|Run actions/(upload|download)-artifact|.*[Ii]nstall|.*toolchain|.*[Cc]ache)")
FIN = re.compile(r"Finished\s*(?:\x1b\[[0-9;]*m|\^\[\[[0-9;]*m)*\s*`[^`]+` profile .*? in (?:(\d+)m )?([\d.]+)s")


def ts(s):
    return datetime.datetime.fromisoformat(s.replace("Z", "+00:00"))


jobs = []
for page in json.loads(subprocess.check_output(["gh", "api", f"repos/{REPO}/actions/runs/{RUN}/jobs?per_page=100", "--paginate", "--slurp"])):
    jobs += page["jobs"]
out = {
    "run_id": int(RUN),
    "method": "18-0 AC10 (creation/e17-a2-method-probe.txt): compile_s = sum of cargo `Finished ... in <t>` lines attributed (by timestamp, the steps API windows) to non-setup steps; setup_s = wall of set-up/checkout/toolchain/cache/install/artifact/post steps (their own cargo-install builds included); test_s = job wall - compile_s - setup_s (also absorbs fixture fetch, scripts and runner overhead)",
    "legs": [],
}
for job in jobs:
    bucket = next((b for b in BUCKETS if job["name"] == b or job["name"].startswith(b + " (")), None)
    if not bucket or not job.get("started_at") or not job.get("completed_at"):
        continue
    wall = (ts(job["completed_at"]) - ts(job["started_at"])).total_seconds()
    setup_s = 0.0
    windows = []
    for step in job.get("steps", []):
        if step.get("started_at") and step.get("completed_at"):
            start, end = ts(step["started_at"]), ts(step["completed_at"])
            windows.append((start, end, step["name"]))
            if SETUP.match(step["name"]):
                setup_s += (end - start).total_seconds()
    log = subprocess.run(["gh", "run", "view", RUN, "--repo", REPO, "--job", str(job["id"]), "--log"], capture_output=True, text=True, errors="replace").stdout
    compile_s = 0.0
    finished = 0
    per_step = collections.Counter()
    for line in log.splitlines():
        parts = line.split("\t", 2)
        if len(parts) < 3:
            continue
        m = FIN.search(parts[2])
        if not m:
            continue
        # `gh run view --log` names every step "UNKNOWN STEP": attribute by timestamp.
        stamp = ts(parts[2].split(" ", 1)[0][:19] + "Z")
        step = next((name for start, end, name in windows if start <= stamp <= end), "unattributed")
        if SETUP.match(step):
            continue
        t = int(m.group(1) or 0) * 60 + float(m.group(2))
        compile_s += t
        finished += 1
        per_step[step] += t
    out["legs"].append({
        "bucket": bucket, "job": job["name"], "job_id": job["id"], "conclusion": job["conclusion"],
        "wall_s": round(wall), "compile_s": round(compile_s), "setup_s": round(setup_s),
        "test_s": round(wall - compile_s - setup_s), "finished_lines": finished,
        "compile_share": round(compile_s / wall, 3) if wall else None,
        "compile_by_step_s": {k: round(v) for k, v in per_step.most_common()},
    })
summary = {}
for b in BUCKETS:
    legs = [l for l in out["legs"] if l["bucket"] == b]
    w = sum(l["wall_s"] for l in legs); c = sum(l["compile_s"] for l in legs); s = sum(l["setup_s"] for l in legs)
    summary[b] = {"legs": len(legs), "wall_s": w, "compile_s": c, "setup_s": s, "test_s": w - c - s, "compile_share": round(c / w, 3) if w else None}
out["summary"] = summary
tw = sum(v["wall_s"] for v in summary.values()); tc = sum(v["compile_s"] for v in summary.values()); tsu = sum(v["setup_s"] for v in summary.values())
out["total"] = {"wall_s": tw, "compile_s": tc, "setup_s": tsu, "test_s": tw - tc - tsu, "compile_share": round(tc / tw, 3) if tw else None}
print(json.dumps(out, indent=2))
