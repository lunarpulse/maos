#!/usr/bin/env python3
"""Assert the complete installer-owned denominator matches its signed bytes."""
import hashlib
import pathlib
import sys

BINS = ("maos", "maosctl", "maos-spirit", "maos-registry-server")
dist, prefix, suffix = map(pathlib.Path, sys.argv[1:])
entries = {}
for line in (dist / "SHA256SUMS").read_text().splitlines():
    digest, name = line.split("  ")
    if name in entries:
        raise SystemExit("duplicate manifest filename")
    entries[name] = digest
for binary in BINS:
    path = prefix / binary
    with path.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    name = f"{binary}-{suffix}"
    assert entries[name] == actual, f"installed bytes differ: {name}"
    print(f"identity: {path} = {actual}")
