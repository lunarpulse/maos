#!/usr/bin/env python3
"""Refuse evidence unless every independent libFuzzer worker reports its runtime."""
import argparse
import datetime
import json
import pathlib
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--target", required=True)
parser.add_argument("--logs", type=pathlib.Path, required=True)
parser.add_argument("--corpus", type=pathlib.Path, required=True)
parser.add_argument("--workers", type=int, required=True)
parser.add_argument("--tier", choices=["T2", "T3"], required=True)
parser.add_argument("--commit", required=True)
parser.add_argument("--run-id", required=True)
parser.add_argument("--output", type=pathlib.Path, required=True)
args = parser.parse_args()
if args.workers < 1:
    parser.error("workers must be positive")
seconds = 0
for worker in range(args.workers):
    path = args.logs / f"worker-{worker}.log"
    try:
        text = path.read_text()
    except OSError as error:
        parser.error(f"refusing record: worker {worker} log unavailable: {error}")
    counters = re.findall(r"^Done \d+ runs in (\d+) second(?:\(s\)|s)?\s*$", text, re.MULTILINE)
    if len(counters) != 1:
        parser.error(f"refusing record: worker {worker} missing or ambiguous libFuzzer Done counter")
    seconds += int(counters[0])
if not args.corpus.is_dir():
    parser.error("refusing record: corpus directory unavailable")
record = {
    "target": args.target,
    "cpu_seconds": seconds,
    "corpus_size": sum(1 for path in args.corpus.rglob("*") if path.is_file()),
    "timestamp": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "tier": args.tier,
    "commit": args.commit,
    "workers": args.workers,
    "run_id": args.run_id,
}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(record, sort_keys=True) + "\n")
print(json.dumps(record, sort_keys=True))
