#!/usr/bin/env python3
"""
Standalone MAOS trajectory bundle verifier (Story 9.2b).

Verifies:
  1. The Ed25519 signature on a maos.trajectory.v1 bundle.
  2. Schema-level invariants (applied_redaction, redaction_policy, redaction
     fields on entries).

Uses ONLY the Python standard library + an Ed25519 backend
(PyNaCl / ed25519 / cryptography), same as verify-audit-bundle and
verify-erasure.

Exit codes:
  0 — bundle valid
  1 — bundle invalid or error
  2 — usage error or missing Ed25519 backend
Usage:
  python verify.py <trajectory.json> <64-char-hex-pubkey>
  python verify.py <trajectory.json> --pubkey-file <pubkey.hex>
  python verify.py <trajectory.json> <pubkey> --tamper-test
"""

import hashlib
import json
import sys
import os

# ─── Ed25519 verification backend ──────────────────────────────────────────

def _verify_ed25519(pubkey_bytes: bytes, message: bytes, signature_bytes: bytes) -> bool:
    """Verify an Ed25519 signature. Tries multiple backends."""
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

    return None


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
    Deterministic canonical serialization: sorted keys, no whitespace.
    Excludes the `signature_block` field.
    """
    unsigned = {k: v for k, v in bundle.items() if k != "signature_block"}
    sorted_bundle = _sort_value(unsigned)
    return json.dumps(sorted_bundle, separators=(",", ":"), sort_keys=True).encode("utf-8")


# ─── Validation helpers ───────────────────────────────────────────────────

def _validate_schema(bundle: dict) -> list:
    """Validate trajectory-specific schema invariants. Returns list of errors."""
    errors = []

    sv = bundle.get("schema_version")
    if sv != "maos.trajectory.v1":
        errors.append(f"unexpected schema_version: {sv!r} (expected 'maos.trajectory.v1')")

    if "signature_block" not in bundle:
        errors.append("missing signature_block")

    # applied_redaction must be a boolean
    ar = bundle.get("applied_redaction")
    if not isinstance(ar, bool):
        errors.append(f"applied_redaction must be a boolean, got {type(ar).__name__}")

    # redaction_policy must be a string
    rp = bundle.get("redaction_policy")
    if not isinstance(rp, str):
        errors.append(f"redaction_policy must be a string, got {type(rp).__name__}")

    # If applied_redaction is true, at least one entry must have a non-null redaction
    if ar is True:
        entries = bundle.get("entries", [])
        has_redaction = any(
            e.get("redaction") is not None
            for e in entries
            if isinstance(e, dict)
        )
        if not has_redaction:
            errors.append(
                "applied_redaction is true but no entry has a non-null redaction field"
            )

    return errors


# ─── Main ───────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 3:
        print(
            "Usage: verify.py <trajectory.json> <pubkey.hex | --pubkey-file <path>> [--tamper-test]",
            file=sys.stderr,
        )
        sys.exit(2)

    bundle_path = sys.argv[1]
    pubkey_arg = sys.argv[2]
    tamper_test = "--tamper-test" in sys.argv

    try:
        with open(bundle_path, "r") as f:
            bundle = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(f"Error reading bundle: {e}", file=sys.stderr)
        sys.exit(1)

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

    try:
        pubkey_bytes = bytes.fromhex(pubkey_hex)
    except ValueError:
        print(f"Error: invalid hex pubkey: {pubkey_hex[:32]}...", file=sys.stderr)
        sys.exit(1)

    if len(pubkey_bytes) != 32:
        print(f"Error: pubkey must be 32 bytes, got {len(pubkey_bytes)}", file=sys.stderr)
        sys.exit(1)

    # ── Schema validation ──────────────────────────────────────────────────
    schema_errors = _validate_schema(bundle)
    if schema_errors:
        for err in schema_errors:
            print(f"Schema error: {err}", file=sys.stderr)
        sys.exit(1)

    # ── Signature verification ─────────────────────────────────────────────
    sig_block = bundle["signature_block"]
    try:
        signature_bytes = bytes.fromhex(sig_block["signature"])
    except (KeyError, ValueError) as e:
        print(f"Error reading signature: {e}", file=sys.stderr)
        sys.exit(1)

    algorithm = sig_block.get("algorithm", "")
    if algorithm != "Ed25519":
        print(f"Error: unsupported algorithm: {algorithm}", file=sys.stderr)
        sys.exit(1)

    attester_pubkey = sig_block.get("attester_pubkey", "")
    if attester_pubkey and attester_pubkey != pubkey_hex:
        print(
            f"Warning: attester_pubkey in bundle ({attester_pubkey[:16]}...) "
            f"differs from supplied pubkey ({pubkey_hex[:16]}...)",
            file=sys.stderr,
        )
    canonical_bytes = _canonicalize(bundle)
    digest = hashlib.sha256(canonical_bytes).digest()

    backend_result = _verify_ed25519(pubkey_bytes, digest, signature_bytes)
    if backend_result is None:
        print("Error: no Ed25519 library found. Install one of:", file=sys.stderr)
        print("  pip install PyNaCl", file=sys.stderr)
        print("  pip install ed25519", file=sys.stderr)
        print("  pip install cryptography", file=sys.stderr)
        sys.exit(2)
    if not backend_result:
        print("FAIL: Ed25519 signature verification failed.", file=sys.stderr)
        sys.exit(1)

    print("OK: trajectory bundle signature is valid.")

    # ── Tamper test (anti-tautology) ───────────────────────────────────────
    if tamper_test:
        tampered = bytearray(canonical_bytes)
        # Flip one byte
        tampered[0] ^= 0xFF
        tampered_digest = hashlib.sha256(bytes(tampered)).digest()
        tamper_result = _verify_ed25519(pubkey_bytes, tampered_digest, signature_bytes)
        if tamper_result is None:
            print("Error: no Ed25519 library found for tamper test.", file=sys.stderr)
            sys.exit(2)
        if tamper_result:
            print("FAIL: tamper test — tampered data still verifies!", file=sys.stderr)
            sys.exit(1)
        print("OK: tamper test passed (flipped byte correctly rejected).")

    sys.exit(0)


if __name__ == "__main__":
    main()
