#!/usr/bin/env python3
"""
Standalone MAOS proof-of-erasure verifier (Story 9.2).

Verifies:
  1. The Ed25519 signature on the bundle.
  2. Each Merkle inclusion proof against the claimed post_root.
  3. Each Merkle exclusion proof against the claimed post_root, and that the
     claimed excluded leaf is absent from the post-tree leaf set.

Uses ONLY the Python standard library + an Ed25519 backend
(PyNaCl / ed25519 / cryptography), same as verify-audit-bundle.

Exit codes:
  0 — proof valid
  1 — proof invalid or error
  2 — usage error

Usage:
  python verify.py <spirit-id>-<timestamp>.bundle <pubkey.hex>
  python verify.py <bundle> --pubkey-file <pubkey.hex>
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

    print("Error: no Ed25519 library found. Install one of:", file=sys.stderr)
    print("  pip install PyNaCl", file=sys.stderr)
    print("  pip install ed25519", file=sys.stderr)
    print("  pip install cryptography", file=sys.stderr)
    return False


# ─── Merkle helpers ────────────────────────────────────────────────────────

def _empty_root() -> bytes:
    return hashlib.sha256(b"maos.erasure.empty-tree").digest()


def _hash_leaf(frame_id: bytes) -> bytes:
    return hashlib.sha256(frame_id).digest()


def _hash_pair(left: bytes, right: bytes) -> bytes:
    return hashlib.sha256(left + right).digest()


def _parse_frame_id_hex(s: str) -> bytes:
    """Parse colon-separated or plain hex frame_id into 16 bytes."""
    clean = "".join(c for c in s if c in "0123456789abcdefABCDEF")
    raw = bytes.fromhex(clean)
    if len(raw) != 16:
        raise ValueError(f"frame_id must be 16 bytes, got {len(raw)}")
    return raw


def _bytes_from_int_list(value) -> bytes:
    """Rust serializes [u8; N] as a JSON array of integers."""
    if isinstance(value, list):
        return bytes(value)
    if isinstance(value, str):
        return bytes.fromhex(value)
    raise ValueError(f"cannot convert to bytes: {value!r}")


def _verify_merkle_proof(root: bytes, leaf: bytes, proof: dict) -> bool:
    siblings = [_bytes_from_int_list(s) for s in proof.get("siblings", [])]
    directions = proof.get("directions", [])
    adjacent = proof.get("adjacent_leaf")

    if adjacent is not None:
        base_leaf = _bytes_from_int_list(adjacent)
        if base_leaf == leaf:
            return False
    else:
        base_leaf = leaf

    if not siblings:
        if adjacent is not None:
            # Zero-height exclusion proof on a single-leaf tree: the one real
            # (adjacent) leaf IS the root.
            return base_leaf == root
        # Single-leaf inclusion (leaf == root) or the empty-tree sentinel.
        if leaf == root:
            return True
        return root == _empty_root() and leaf != _empty_root()

    current = base_leaf
    for sibling, is_right in zip(siblings, directions):
        if is_right:
            current = _hash_pair(sibling, current)
        else:
            current = _hash_pair(current, sibling)
    return current == root


def _build_root(leaf_hashes) -> bytes:
    """Recompute the Merkle root from a leaf-hash set (mirrors the Rust
    `build_tree`: sort + dedup, then pairwise hash duplicating an odd last
    leaf)."""
    leaves = sorted(set(leaf_hashes))
    if not leaves:
        return _empty_root()
    layer = list(leaves)
    while len(layer) > 1:
        nxt = []
        for i in range(0, len(layer), 2):
            left = layer[i]
            right = layer[i + 1] if i + 1 < len(layer) else left
            nxt.append(_hash_pair(left, right))
        layer = nxt
    return layer[0]


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


# ─── Main ───────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 3:
        print("Usage: verify.py <bundle.json> <pubkey.hex | --pubkey-file <path>>",
              file=sys.stderr)
        sys.exit(2)

    bundle_path = sys.argv[1]
    pubkey_arg = sys.argv[2]

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

    try:
        pre_root = _bytes_from_int_list(bundle.get("pre_root"))
        post_root = _bytes_from_int_list(bundle.get("post_root"))
        pre_leaves = bundle.get("pre_leaves", [])
        post_leaves = bundle.get("post_leaves", [])
        pre_leaf_hashes = [_hash_leaf(_parse_frame_id_hex(s)) for s in pre_leaves]
        post_leaf_hashes = [_hash_leaf(_parse_frame_id_hex(s)) for s in post_leaves]
        erased_frame_proofs = bundle.get("erased_frame_proofs", [])
        subject_exclusion_proofs = bundle.get("subject_exclusion_proofs", [])
    except (ValueError, TypeError) as e:
        print(f"Error: malformed bundle field: {e}", file=sys.stderr)
        sys.exit(1)

    # 1. Signature
    canonical = _canonicalize(bundle)
    digest = hashlib.sha256(canonical).digest()
    if not _verify_ed25519(pubkey_bytes, digest, sig_bytes):
        print("FAIL — signature verification failed", file=sys.stderr)
        sys.exit(1)

    # 2. Recompute both roots from the leaf sets — root↔leaves tamper check.
    recomputed_pre_root = _build_root(pre_leaf_hashes)
    if recomputed_pre_root != pre_root:
        print("FAIL — pre_root does not match the recomputed root of pre_leaves",
              file=sys.stderr)
        sys.exit(1)
    recomputed_post_root = _build_root(post_leaf_hashes)
    if recomputed_post_root != post_root:
        print("FAIL — post_root does not match the recomputed root of post_leaves",
              file=sys.stderr)
        sys.exit(1)

    # 3. Erased-frame pre-inclusion proofs (each scrubbed frame WAS in pre-tree).
    for efp in erased_frame_proofs:
        try:
            fid = _parse_frame_id_hex(efp["frame_id"])
            proof = efp["pre_inclusion"]
        except (KeyError, ValueError, TypeError) as e:
            print(f"FAIL — malformed erased-frame proof: {e}", file=sys.stderr)
            sys.exit(1)
        leaf = _hash_leaf(fid)
        if not _verify_merkle_proof(pre_root, leaf, proof):
            print("FAIL — erased-frame pre-inclusion proof invalid", file=sys.stderr)
            sys.exit(1)

    # 4. Subject exclusion proofs (canonical subject leaf absent from post-tree).
    for sp in subject_exclusion_proofs:
        try:
            leaf = _bytes_from_int_list(sp["leaf"])
        except (ValueError, KeyError, TypeError) as e:
            print(f"FAIL — malformed subject exclusion proof: {e}", file=sys.stderr)
            sys.exit(1)
        if leaf in post_leaf_hashes:
            print("FAIL — subject exclusion leaf is present in the post-tree",
                  file=sys.stderr)
            sys.exit(1)
        if not _verify_merkle_proof(post_root, leaf, sp):
            print("FAIL — subject exclusion proof invalid", file=sys.stderr)
            sys.exit(1)

    # 5. Reject empty proof sets when the bundle claims erasure.
    claims_removal = any(
        c.get("status", {}).get("status") == "Removed" and c["status"].get("count", 0) > 0
        for c in bundle.get("categories", [])
    )
    if claims_removal and not erased_frame_proofs and not subject_exclusion_proofs:
        print("FAIL — bundle claims erasure but carries no inclusion or exclusion proofs",
              file=sys.stderr)
        sys.exit(1)

    print("OK — proof-of-erasure signature and Merkle proofs verified")
    sys.exit(0)


if __name__ == "__main__":
    main()
