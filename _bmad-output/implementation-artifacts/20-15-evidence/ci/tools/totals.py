import json, sys, datetime as dt, collections
t = lambda s: dt.datetime.fromisoformat(s.replace("Z", "+00:00"))
J = [json.loads(l) for l in open(sys.argv[1])]
D = [j for j in J if j.get("started_at") and j.get("completed_at") and j["conclusion"] in ("success", "failure")]
sec = lambda j: max(0.0, (t(j["completed_at"]) - t(j["started_at"])).total_seconds())
print("jobs", len(J), "ran", len(D), "conclusions", dict(collections.Counter(j["conclusion"] for j in J)))
print("job-h", round(sum(map(sec, D)) / 3600, 2))
start = min(t(j["started_at"]) for j in D)
print("wall_min", round((max(t(j["completed_at"]) for j in D) - start).total_seconds() / 60, 1))
fam = collections.defaultdict(list)
for j in D:
    fam[j["name"].split(" (")[0]].append(sec(j))
for k in ["ci-test-producer", "wasm-host-tests", "cgroup-ceiling-delegated", "cgroup-ceiling-undelegated", "epic-17-exit", "epic-18-exit", "workspace-test-suite"]:
    v = fam.get(k, [])
    print(k, "legs", len(v), "sum_s", round(sum(v)), "max_s", round(max(v)) if v else "-")
p = [j for j in D if j["name"].startswith("ci-test-producer")]
w = [j for j in D if j["name"].startswith("wasm-host-tests")]
if p:
    for j in p:
        print("producer", j["name"], j["conclusion"], "dur_s", round(sec(j)), "start+%ds" % (t(j["started_at"]) - start).total_seconds(), "done+%ds" % (t(j["completed_at"]) - start).total_seconds())
    print("producer_critical_path_min (first job start -> last producer done)", round((max(t(j["completed_at"]) for j in p) - start).total_seconds() / 60, 1))
if p and w:
    print("producer_done_to_last_wasm_done_s", round((max(t(j["completed_at"]) for j in w) - max(t(j["completed_at"]) for j in p)).total_seconds()))
last = max(D, key=lambda j: t(j["completed_at"]))
print("last job to finish", last["name"], "done+%.1fmin" % ((t(last["completed_at"]) - start).total_seconds() / 60))
print("--- top 12 families by job-s")
for k, v in sorted(fam.items(), key=lambda kv: -sum(kv[1]))[:12]:
    print(" ", k, "legs", len(v), "sum_s", round(sum(v)))
