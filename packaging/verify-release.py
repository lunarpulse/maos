#!/usr/bin/env python3
"""Verify Ed25519(SHA256(SHA256SUMS)) and exact, unique artifact names.

No downloaded executable is run by this verifier. A caller must supply the
release public key obtained from the trusted, built release tooling.
"""
import argparse
import hashlib
import pathlib
import re
import sys

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey


def verify(directory, pubkey, names):
    directory = pathlib.Path(directory)
    manifest = (directory / "SHA256SUMS").read_bytes()
    Ed25519PublicKey.from_public_bytes(bytes.fromhex(pubkey)).verify(
        (directory / "SHA256SUMS.sig").read_bytes(), hashlib.sha256(manifest).digest()
    )
    entries = {}
    for line in manifest.decode("utf-8").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)", line)
        if match is None or match[2] in entries:
            raise ValueError("malformed or duplicate SHA256SUMS entry")
        entries[match[2]] = match[1]
    if not names:
        raise ValueError("at least one artifact is required")
    for name in names:
        if name not in entries or pathlib.Path(name).name != name:
            raise ValueError(f"missing exact manifest filename: {name}")
        path = directory / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"not a regular artifact: {name}")
        with path.open("rb") as artifact:
            actual = hashlib.file_digest(artifact, "sha256").hexdigest()
        if actual != entries[name]:
            raise ValueError(f"SHA256 mismatch: {name}")
    print("verified signature and exact artifact hashes: " + ", ".join(names))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", default=".")
    parser.add_argument("--pubkey", required=True)
    parser.add_argument("names", nargs="+")
    args = parser.parse_args()
    try:
        verify(args.directory, args.pubkey, args.names)
    except Exception as error:
        print(f"release verification refused: {type(error).__name__}: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
