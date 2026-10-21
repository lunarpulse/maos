#!/usr/bin/env python3
"""Epic-17 exit line 2b (Story 17-8 AC1, re-stating 17-3d AC1.3), run after exit line 2 on the same store.

    maos audit query --spirit example-spirit-ts --format plain

`--spirit` already resolves the Spirit's latest `(boot_nonce, spirit_pid)` and
selects the FR4 renderer, so a tokenless recall row exits non-zero there. Under
17-8 participant scope the `--once` seed (from `operator`, addressed to the TS
Spirit) is a RECEIVED frame, so the scaffold's `fetch(seed.frameId)` succeeds.
This checker reads that output and requires, at that one pid, exactly the
recall-operation rows R1 log.recall -> R2 log.fetch as an ordered subsequence
before the turn's 17-3c rows, and no refusal or channel row. The issuer's
`identity.asserted` rows may interleave.
"""

import argparse
import sys

R_ROWS = [
    ("capability.invocation", "log.recall"),
    ("capability.invocation", "log.fetch"),
]
AUTHORIZATION = ("governance.event", "spirit.frame.authorization")
TASK_ASSIGN = ("task.assign", "standard")


def recall_row(intent):
    return (intent in ("log.recall", "log.fetch")
            or (intent.startswith("log.") and ".refused:" in intent)
            or intent.startswith("recall.channel.closed:"))


def check(lines):
    rows = []
    for line in lines:
        fields = line.split(None, 5)
        if len(fields) == 6 and fields[2].isdigit():
            rows.append((int(fields[2]), fields[3], fields[5].strip()))
    errors = []
    pids = {pid for pid, _, _ in rows}
    if len(pids) != 1:
        return [f"expected rows at exactly one spirit_pid, got {sorted(pids)}"]
    pairs = [(call_type, intent) for _, call_type, intent in rows]
    recalls = [pair for pair in pairs if recall_row(pair[1])]
    if recalls != R_ROWS:
        errors.append(f"recall-operation rows {recalls}, want exactly {R_ROWS}")
    if AUTHORIZATION not in pairs or TASK_ASSIGN not in pairs:
        errors.append(f"17-3c rows {AUTHORIZATION} and {TASK_ASSIGN} must stay present")
    elif not errors:
        last_r = max(pairs.index(pair) for pair in R_ROWS)
        if last_r > pairs.index(AUTHORIZATION):
            errors.append("R1-R2 must precede the turn's spirit.frame.authorization row")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("plain_output", help="captured `maos audit query ... --format plain` stdout")
    args = parser.parse_args()
    with open(args.plain_output, encoding="utf-8") as handle:
        errors = check(handle)
    for error in errors:
        print(f"LINE-2B FAIL {error}")
    if not errors:
        print("LINE-2B OK R1 log.recall -> R2 log.fetch (participant scope: seed fetched, no refusal)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
