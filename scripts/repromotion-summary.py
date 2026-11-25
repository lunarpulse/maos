#!/usr/bin/env python3
"""Counting rule of .github/workflows/repromotion.yml (WP-15, decision 8).

Input (JSON file argument): {"run_id": "...", "sha": "...", "attempt": 1,
"jobs": [{"name": "expensive-tests (3)", "conclusion": "success", "head_sha": "..."}, ...]}

Green means exactly: both jobs ran all ten iterations, 20 of 20 concluded `success`, the
dispatch is attempt 1 (a re-run voids it), and every job ran at the one dispatch SHA.
Any other state exits 1 and names the reason. Jobs not named below (the summary job itself)
are ignored. Output on success: one record line, also written to --record when given.

usage: repromotion-summary.py <input.json> [--record <file>] | --self-test
"""
import json
import re
import sys

JOBS = ("nfr-rel-11-halt-receipt-999pct", "expensive-tests")
ITERATIONS = range(1, 11)


def judge(data):
    """Return (ok, reasons)."""
    reasons = []
    if data.get("attempt") != 1:
        reasons.append(f"attempt {data.get('attempt')!r} is not 1 (a re-run voids the dispatch)")
    sha = data.get("sha")
    if not sha:
        reasons.append("no dispatch SHA recorded")
    seen = {}
    for job in data.get("jobs", []):
        match = re.fullmatch(r"(.+) \((\d+)\)", job.get("name", ""))
        if not match or match.group(1) not in JOBS:
            continue
        key = (match.group(1), int(match.group(2)))
        if key in seen:
            reasons.append(f"{key[0]} iteration {key[1]} appears twice")
        seen[key] = job
        if job.get("head_sha", sha) != sha:
            reasons.append(f"{key[0]} iteration {key[1]} ran at {job.get('head_sha')}, not {sha}")
    green = 0
    for name in JOBS:
        for iteration in ITERATIONS:
            job = seen.get((name, iteration))
            if job is None:
                reasons.append(f"{name} iteration {iteration} is missing")
            elif job.get("conclusion") != "success":
                reasons.append(f"{name} iteration {iteration} concluded {job.get('conclusion')!r}")
            else:
                green += 1
    if green != 20:
        reasons.append(f"{green} of 20 iterations green")
    return (not reasons, reasons)


def record(data):
    return f"repromotion run_id={data.get('run_id')} sha={data.get('sha')} attempt={data.get('attempt')} green=20/20"


def self_test():
    def good(**over):
        data = {
            "run_id": "1",
            "sha": "a" * 40,
            "attempt": 1,
            "jobs": [
                {"name": f"{name} ({i})", "conclusion": "success", "head_sha": "a" * 40}
                for name in JOBS
                for i in ITERATIONS
            ]
            + [{"name": "summary", "conclusion": None}],
        }
        data.update(over)
        return data

    assert judge(good())[0], "20 of 20 must be green"
    red19 = good()
    red19["jobs"][3]["conclusion"] = "failure"
    ok, reasons = judge(red19)
    assert not ok and any("19 of 20" in r for r in reasons), reasons
    assert not judge(good(attempt=2))[0], "attempt 2 must be refused"
    short = good()
    del short["jobs"][0]
    assert not judge(short)[0], "a missing iteration must be refused"
    other_sha = good()
    other_sha["jobs"][5]["head_sha"] = "b" * 40
    assert not judge(other_sha)[0], "a second SHA must be refused"
    print("repromotion-summary self-test ok")


def main(argv):
    if argv == ["--self-test"]:
        self_test()
        return 0
    if not argv or argv[0].startswith("-"):
        print(__doc__, file=sys.stderr)
        return 2
    record_path = None
    if len(argv) == 3 and argv[1] == "--record":
        record_path = argv[2]
    elif len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    with open(argv[0], encoding="utf-8") as handle:
        data = json.load(handle)
    ok, reasons = judge(data)
    if not ok:
        for reason in reasons:
            print(f"::error::repromotion not green: {reason}", file=sys.stderr)
        return 1
    line = record(data)
    print(line)
    if record_path:
        with open(record_path, "w", encoding="utf-8") as out:
            out.write(line + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
