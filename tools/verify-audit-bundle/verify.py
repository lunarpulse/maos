#!/usr/bin/env python3
"""
Standalone MAOS audit-bundle verifier.

Verifies the Ed25519 signature on a maos.audit-bundle.v1 JSON file.
Uses ONLY the Python standard library + the `ed25519` package (or falls back
to `nacl` / `cryptography`).

Exit codes:
  0 — signature valid
  1 — signature invalid or error
  2 — usage error

Usage:
  python verify.py bundle.json pubkey.hex
  python verify.py bundle.json --pubkey-file pubkey.hex
"""

import hashlib
import json
import sys
import os

# ─── Ed25519 verification backend ──────────────────────────────────────────

def _verify_ed25519(pubkey_bytes: bytes, message: bytes, signature_bytes: bytes) -> bool:
    """Verify an Ed25519 signature. Tries multiple backends."""
    # Try PyNaCl first (wraps libsodium)
    try:
        from nacl.signing import VerifyKey
        from nacl.exceptions import BadSignatureError
        vk = VerifyKey(pubkey_bytes)
        try:
            vk.verify(message, signature_bytes)
            return True
        except BadSignatureError:
            return False
    except ImportError:
        pass

    # Try the `ed25519` package
    try:
        from ed25519 import VerifyingKey as Ed25519VerifyKey
        from ed25519 import BadSignatureError as Ed25519BadSig
        vk = Ed25519VerifyKey(pubkey_bytes)
        try:
            vk.verify(signature_bytes, message)
            return True
        except Ed25519BadSig:
            return False
    except ImportError:
        pass

    # Try the `cryptography` package
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
        from cryptography.exceptions import InvalidSignature
        pk = Ed25519PublicKey.from_public_bytes(pubkey_bytes)
        try:
            pk.verify(signature_bytes, message)
            return True
        except InvalidSignature:
            return False
    except ImportError:
        pass

    print("Error: no Ed25519 library found. Install one of:", file=sys.stderr)
    print("  pip install PyNaCl", file=sys.stderr)
    print("  pip install ed25519", file=sys.stderr)
    print("  pip install cryptography", file=sys.stderr)
    return False


# ─── Canonical serialization ───────────────────────────────────────────────

def _sort_value(value):
    """Recursively sort all JSON object keys for deterministic serialization."""
    if isinstance(value, dict):
        return {k: _sort_value(v) for k, v in sorted(value.items())}
    if isinstance(value, list):
        return [_sort_value(item) for item in value]
    return value


def _canonicalize(bundle: dict) -> bytes:
    """
    Deterministic canonical serialization: sorted keys, no whitespace, raw UTF-8.
    Excludes the `signature_block` field.

    `ensure_ascii=False` is LOAD-BEARING, not style. Rust's `canonicalize_value`
    (`crates/maos-audit/src/sealed_export.rs:632-639`) ends in
    `serde_json::to_string`, which emits non-ASCII as raw UTF-8. Python's
    `json.dumps` defaults to `ensure_ascii=True` and would escape the same
    characters to `\\uXXXX`, producing different bytes for an identical document
    — so every bundle containing a single non-ASCII byte failed verification
    here despite carrying a valid signature (`j1-crosshost-2e` F5, reproduced
    against the real T6 artifact). Key ordering was never the defect: both sides
    already sort. Do not "also" add `sort_keys=True` elsewhere to compensate.
    """
    # Build unsigned bundle (everything except signature_block)
    unsigned = {k: v for k, v in bundle.items() if k != "signature_block"}
    sorted_bundle = _sort_value(unsigned)
    return json.dumps(
        sorted_bundle, ensure_ascii=False, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


# ─── Main ───────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 3:
        print("Usage: verify.py <bundle.json> <pubkey.hex | --pubkey-file <path>>",
              file=sys.stderr)
        sys.exit(2)

    bundle_path = sys.argv[1]
    pubkey_arg = sys.argv[2]

    # Read bundle
    try:
        with open(bundle_path, "r") as f:
            bundle = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(f"Error reading bundle: {e}", file=sys.stderr)
        sys.exit(1)

    # Read public key
    if pubkey_arg == "--pubkey-file":
        if len(sys.argv) < 4:
            print("Error: --pubkey-file requires a path argument", file=sys.stderr)
            sys.exit(2)
        try:
            with open(sys.argv[3], "r") as f:
                pubkey_hex = f.read().strip()
        except OSError as e:
            print(f"Error reading pubkey file: {e}", file=sys.stderr)
            sys.exit(1)
    else:
        pubkey_hex = pubkey_arg.strip()

    # Also support reading pubkey from a file if the arg looks like a path
    if len(pubkey_hex) != 64 and os.path.exists(pubkey_hex):
        with open(pubkey_hex, "r") as f:
            pubkey_hex = f.read().strip()

    try:
        pubkey_bytes = bytes.fromhex(pubkey_hex)
    except ValueError:
        print("Error: public key must be 64-char hex (32 bytes)", file=sys.stderr)
        sys.exit(1)

    if len(pubkey_bytes) != 32:
        print(f"Error: public key must be 32 bytes, got {len(pubkey_bytes)}", file=sys.stderr)
        sys.exit(1)

    # Extract signature_block
    sig_block = bundle.get("signature_block")
    if not sig_block:
        print("Error: bundle missing 'signature_block'", file=sys.stderr)
        sys.exit(1)

    algorithm = sig_block.get("algorithm", "")
    if algorithm != "Ed25519":
        print(f"Error: unsupported algorithm: {algorithm}", file=sys.stderr)
        sys.exit(1)

    sig_hex = sig_block.get("signature", "")
    try:
        sig_bytes = bytes.fromhex(sig_hex)
    except ValueError:
        print("Error: signature must be hex-encoded", file=sys.stderr)
        sys.exit(1)

    if len(sig_bytes) != 64:
        print(f"Error: signature must be 64 bytes, got {len(sig_bytes)}", file=sys.stderr)
        sys.exit(1)

    # Canonicalize unsigned bundle and hash
    canonical = _canonicalize(bundle)
    digest = hashlib.sha256(canonical).digest()

    # Verify
    if _verify_ed25519(pubkey_bytes, digest, sig_bytes):
        print("OK — signature verified")
        sys.exit(0)
    else:
        print("FAIL — signature verification failed", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
