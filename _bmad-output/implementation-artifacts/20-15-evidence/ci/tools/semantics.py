# usage (in run dir): semantics.py — 20-15 corrected-semantics checklist over consumer logs -> stdout
import json, re, glob, os
ANSI = re.compile(r"\x1b\[[0-9;]*m")
cons = [l.strip() for l in open("consumer-logs.list") if l.strip()]
jobs = {j["name"]: j for j in map(json.loads, open("jobs.jsonl"))}
def jobname(path):
    return os.path.basename(path).split("_", 1)[1][:-4]
bad = 0
print("== 1. nextest profile per run (consumers)")
for f in cons:
    lines = [ANSI.sub("", l.rstrip("\n")) for l in open(f, errors="replace")]
    runs = []
    for i, l in enumerate(lines):
        m = re.search(r"Nextest run ID \S+ with nextest profile: (\S+)", l)
        if m:
            nxt = next((lines[k] for k in range(i + 1, min(i + 6, len(lines))) if re.search(r"(START|PASS|FAIL|SKIP) +\[", lines[k])), "")
            tm = re.search(r"\)\s+(\S+)\s", nxt)
            runs.append((i + 1, m[1], tm[1] if tm else "?"))
    wrong = [r for r in runs if (r[1] != "default") if r[2].startswith("equiv-native-twin")] + \
            [r for r in runs if not r[2].startswith("equiv-native-twin") and r[1] != "archive-consumer"]
    prof = {}
    for _, p, b in runs:
        prof[(p, b.split("::")[0])] = prof.get((p, b.split("::")[0]), 0) + 1
    # epic-18-exit runs the restored debug binaries directly (no nextest invocation).
    bad += len(wrong) + (not runs and "epic-18-exit" not in f)
    print(f"{f[5:]}: {len(runs)} runs; " + ", ".join(f"{p}<-{b} x{n}" for (p, b), n in sorted(prof.items())) + (f"  WRONG={wrong}" if wrong else ""))
    for ln, p, b in runs:
        print(f"    L{ln} profile={p} first={b}")
print("== 2. 'Cancelling due to test failure' in consumers")
for f in cons:
    n = sum("Cancelling due to test failure" in l for l in open(f, errors="replace"))
    bad += n
    print(f"{f[5:]}: {n}")
print("== 3. skipped-test guard errors (SKIP, ignored, or zero tests) in consumers")
for f in cons:
    hits = [f"L{i}: {l.strip()[:200]}" for i, l in enumerate(open(f, errors="replace"), 1) if "SKIP, ignored, or zero tests" in l and "##[error]" in l]
    bad += len(hits)
    print(f"{f[5:]}: {len(hits)}", *hits, sep="\n    " if hits else "")
print("== 4. restore step (producer.json == expected incl. nextest) per consumer")
for f in cons:
    j = jobs[jobname(f)]
    st = [(s["name"], s["conclusion"]) for s in j["steps"] if s["name"].startswith("Restore exact producer")]
    ok = st and all(c == "success" for _, c in st)
    bad += not ok
    print(f"{jobname(f)}: {st}")
print("== 5. producer pack step + nextest version")
for n, j in sorted(jobs.items()):
    if n.startswith("ci-test-producer"):
        st = [(s["name"], s["conclusion"]) for s in j["steps"] if s["name"].startswith("Pack exact producer")]
        bad += not (st and st[0][1] == "success")
        print(n, st)
print("TOTAL VIOLATIONS:", bad)
