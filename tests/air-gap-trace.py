#!/usr/bin/env python3
"""Story 20-2: count outbound attempts, never socket creation or AF_UNIX IPC."""
import collections
import pathlib
import re
import sqlite3
import sys


def outbound(text):
    attempts = [line for line in text.splitlines()
                if re.search(r"\b(connect|sendto|sendmsg)\(", line)]
    inet = [line for line in attempts
            if re.search(r"\bAF_INET6?\b|<(?:TCP|UDP)(?:v6)?:", line)]
    other = collections.Counter()
    for line in attempts:
        if line not in inet:
            match = re.search(r"AF_[A-Z0-9_]+", line)
            other[match[0] if match else "other/connected-fd"] += 1
    return inet, dict(other)


def main():
    mode, filename = sys.argv[1:]
    path = pathlib.Path(filename)
    if mode == "rows":
        if not path.exists():
            print(0)
            return
        with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as db:
            exists = db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='transparency_log'").fetchone()
            print(db.execute("SELECT COUNT(*) FROM transparency_log").fetchone()[0] if exists else 0)
        return
    text = path.read_text()
    if not text.strip():
        raise SystemExit("empty trace: no observation")
    inet, other = outbound(text)
    print(f"{mode}: AF_INET/AF_INET6 outbound attempts={len(inet)}; other={other}")
    for line in inet:
        print(line)
    if mode == "canary":
        if not inet:
            raise SystemExit("canary not detected")
    elif mode == "workload":
        if inet:
            raise SystemExit("STOP: NFR-Ops-12 product defect; no outbound allowlist permitted")
    else:
        raise SystemExit("unknown trace mode")


if __name__ == "__main__":
    main()
