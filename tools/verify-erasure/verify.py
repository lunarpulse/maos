#!/usr/bin/env python3
"""Verify the unreleased maos.erasure-proof.v2 signed inventory contract.

The declared Ed25519 backend is tools/verify-erasure/requirements.txt (CI installs
it with hashes). No optional-backend fallback or missing-dependency skip exists.
A valid partial inventory is not a claim of complete erasure. A frame-sweep
completion covers matching rows committed through its durable transactional
marker, never rows committed after that marker or a handler-return cutoff.

Usage: verify.py <bundle> <public-key-hex>
       verify.py <bundle> --pubkey-file <path>
Exit codes: 0 valid signed inventory; 1 invalid/error; 2 usage error.
"""
import bisect
import hashlib
import json
import os
import sys

MAX_BUNDLE_BYTES = 128 * 1024 * 1024
U64_MAX = (1 << 64) - 1
SCHEME = "hkdf-sha256-signer-keyed-v1"
BOUNDARY = "durable_transactional_marker"
CATEGORIES = {
    "memory_namespace", "principal_index", "capability_tokens", "shared", "private",
    "principal_frames", "intent_lineage", "pending_halts", "scheduled_invocations",
    "manifest_archive", "skill_queue", "lifecycle_journal", "worker_workspaces",
    "installed_package", "legal_hold", "region_signing_key",
}
TOP_FIELDS = {
    "schema_version", "spirit_id", "spirit_pid", "uninstalled_at_ns", "pre_root", "post_root",
    "pre_leaves", "post_leaves", "erased_frame_proofs", "redacted_principal_frame_proofs",
    "subject_exclusion_proofs", "subject_count", "subject_commitment_scheme",
    "completion_boundary", "frame_sweep_completion", "categories", "category_evidence", "signature_block",
}
HEX = frozenset("0123456789abcdefABCDEF")


def _object(value, fields, label):
    if type(value) is not dict or set(value) != set(fields):
        raise ValueError(f"{label}: exact fields required: {sorted(fields)}")
    return value


def _array(value, label):
    if type(value) is not list:
        raise ValueError(f"{label}: array required")
    return value


def _text(value, label):
    if type(value) is not str:
        raise ValueError(f"{label}: string required")
    return value


def _uint(value, bits, label):
    if type(value) is not int or not 0 <= value < (1 << bits):
        raise ValueError(f"{label}: unsigned {bits}-bit integer required")
    return value


def _hash(value):
    values = _array(value, "hash")
    if len(values) != 32:
        raise ValueError("hash: exactly 32 bytes required")
    return bytes(_uint(byte, 8, "hash byte") for byte in values)


def _frame_id(value):
    value = _text(value, "frame id")
    if len(value) == 39:
        groups = value.split(":")
        if len(groups) != 8 or any(len(group) != 4 for group in groups):
            raise ValueError("frame id: eight four-hex-digit groups required")
        value = "".join(groups)
    if len(value) != 32 or any(character not in HEX for character in value):
        raise ValueError("frame id: plain or canonical colon hex required")
    return bytes.fromhex(value)


def _hex_bytes(value, length, label):
    value = _text(value, label)
    if len(value) != length * 2 or any(character not in HEX for character in value):
        raise ValueError(f"{label}: exactly {length} hex bytes required")
    return bytes.fromhex(value)


def _leaf(frame_id):
    return hashlib.sha256(frame_id).digest()


def _empty_root():
    return hashlib.sha256(b"maos.erasure.empty-tree").digest()


def _root(leaves):
    if not leaves:
        return _empty_root()
    layer = list(leaves)
    while len(layer) > 1:
        layer = [hashlib.sha256(layer[index] + layer[min(index + 1, len(layer) - 1)]).digest()
                 for index in range(0, len(layer), 2)]
    return layer[0]


def _merkle(value):
    _object(value, {"leaf", "adjacent_leaf", "leaf_index", "siblings", "directions"}, "Merkle proof")
    leaf = _hash(value["leaf"])
    adjacent = None if value["adjacent_leaf"] is None else _hash(value["adjacent_leaf"])
    index = _uint(value["leaf_index"], 64, "leaf index")
    siblings = [_hash(sibling) for sibling in _array(value["siblings"], "siblings")]
    directions = _array(value["directions"], "directions")
    if len(siblings) != len(directions) or any(type(direction) is not bool for direction in directions):
        raise ValueError("Merkle proof: equal sibling/direction lengths and Boolean directions required")
    return leaf, adjacent, index, siblings, directions


def _path(root, base, index, siblings, directions, leaf_count):
    if not leaf_count or index >= leaf_count or len(siblings) != (leaf_count - 1).bit_length():
        return False
    current = base
    for sibling, right in zip(siblings, directions):
        if right != bool(index & 1):
            return False
        current = hashlib.sha256(sibling + current if right else current + sibling).digest()
        index >>= 1
    return index == 0 and current == root


def _inclusion(root, leaf, value, leaves):
    stored, adjacent, index, siblings, directions = _merkle(value)
    return (stored == leaf and adjacent is None and index < len(leaves) and leaves[index] == leaf
            and _path(root, leaf, index, siblings, directions, len(leaves)))


def _exclusion(root, value, leaves):
    leaf, adjacent, index, siblings, directions = _merkle(value)
    if not leaves:
        return root == _empty_root() and adjacent is None and index == 0 and not siblings and not directions
    position = bisect.bisect_left(leaves, leaf)
    if position < len(leaves) and leaves[position] == leaf:
        return False
    expected = max(position - 1, 0)
    return (index == expected and adjacent == leaves[expected]
            and _path(root, adjacent, index, siblings, directions, len(leaves)))


def _frame_proofs(values, root, leaves, frame_ids):
    found = set()
    for value in _array(values, "frame proofs"):
        _object(value, {"frame_id", "pre_inclusion"}, "frame proof")
        frame = _frame_id(value["frame_id"])
        if frame in found or frame not in frame_ids or not _inclusion(root, _leaf(frame), value["pre_inclusion"], leaves):
            raise ValueError("duplicate, absent, or invalid pre-inclusion frame")
        found.add(frame)
    return found


def _removals(values, category, spirit, post_ids, erased, principal_frames):
    total, seen = 0, set()
    allowed = {
        "memory_namespace": "memory_namespace_batch", "principal_index": "principal_index_batch",
        "capability_tokens": "capability_batch", "principal_frames": "frame", "intent_lineage": "frame",
        "installed_package": "package", "scheduled_invocations": "schedule",
    }.get(category)
    for item in _array(values, "removal evidence"):
        if type(item) is not dict or item.get("kind") != allowed or allowed is None:
            raise ValueError(f"{category}: removal evidence kind mismatch")
        if allowed == "frame":
            _object(item, {"kind", "frame_id"}, "frame removal")
            frame = _frame_id(item["frame_id"])
            if frame not in (erased if category == "intent_lineage" else principal_frames):
                raise ValueError("removed frame lacks its matching pre-inclusion evidence")
            identity, count = frame, 1
        elif allowed == "package":
            _object(item, {"kind", "spirit_id", "generation"}, "package removal")
            generation = _text(item["generation"], "generation")
            if _text(item["spirit_id"], "package spirit") != spirit or not generation:
                raise ValueError("package removal identity mismatch")
            identity, count = generation, 1
        elif allowed == "schedule":
            _object(item, {"kind", "schedule_id"}, "schedule removal")
            identity = _text(item["schedule_id"], "schedule id")
            if not identity:
                raise ValueError("empty schedule id")
            count = 1
        elif allowed == "capability_batch":
            _object(item, {"kind", "boot_nonce", "spirit_pid", "revoked_tokens"}, "capability batch")
            identity = (_uint(item["boot_nonce"], 64, "boot nonce"), _uint(item["spirit_pid"], 32, "pid"))
            count = _uint(item["revoked_tokens"], 64, "revoked tokens")
        else:
            field = "deleted_entries" if allowed == "memory_namespace_batch" else "deleted_index_rows"
            _object(item, {"kind", "receipt_frame_id", field}, "memory batch")
            identity = _frame_id(item["receipt_frame_id"])
            if identity not in post_ids:
                raise ValueError("memory batch receipt absent from post-tree")
            count = _uint(item[field], 64, field)
        if identity in seen or total + count > U64_MAX:
            raise ValueError("duplicate removal evidence or count overflow")
        seen.add(identity)
        total += count
    if allowed == "frame" and seen != (erased if category == "intent_lineage" else principal_frames):
        raise ValueError("frame removal evidence omits a declared redaction")
    return total


def verify(bundle, public_key):
    _object(bundle, TOP_FIELDS, "bundle")
    if bundle["schema_version"] != "maos.erasure-proof.v2" or bundle["subject_commitment_scheme"] != SCHEME or bundle["completion_boundary"] != BOUNDARY:
        raise ValueError("unsupported signed schema, commitment scheme, or completion boundary")
    spirit = _text(bundle["spirit_id"], "spirit id")
    _uint(bundle["spirit_pid"], 32, "spirit pid")
    _uint(bundle["uninstalled_at_ns"], 64, "uninstall timestamp")
    subject_count = _uint(bundle["subject_count"], 64, "subject count")
    pre_root, post_root = _hash(bundle["pre_root"]), _hash(bundle["post_root"])
    pre = [_frame_id(value) for value in _array(bundle["pre_leaves"], "pre leaves")]
    post = [_frame_id(value) for value in _array(bundle["post_leaves"], "post leaves")]
    pre_ids, post_ids = set(pre), set(post)
    if len(pre_ids) != len(pre) or len(post_ids) != len(post) or not pre_ids <= post_ids:
        raise ValueError("duplicate leaves or append-only frame-set violation")
    pre_hashes, post_hashes = sorted(map(_leaf, pre)), sorted(map(_leaf, post))
    if _root(pre_hashes) != pre_root or _root(post_hashes) != post_root:
        raise ValueError("root does not match the complete declared leaf set")
    erased = _frame_proofs(bundle["erased_frame_proofs"], pre_root, pre_hashes, pre_ids)
    principal_frames = _frame_proofs(bundle["redacted_principal_frame_proofs"], pre_root, pre_hashes, pre_ids)
    if not erased.isdisjoint(principal_frames):
        raise ValueError("one frame cannot be counted as two different removal kinds")
    subjects = _array(bundle["subject_exclusion_proofs"], "subject proofs")
    commitments = set()
    for subject in subjects:
        leaf = _hash(subject["leaf"]) if type(subject) is dict and "leaf" in subject else None
        if leaf is None or leaf in commitments or not _exclusion(post_root, subject, post_hashes):
            raise ValueError("duplicate or invalid subject exclusion")
        commitments.add(leaf)
    if len(subjects) != subject_count or (principal_frames and subject_count == 0):
        raise ValueError("declared subject count differs from unique exclusion evidence")
    categories = _array(bundle["categories"], "categories")
    evidence = _array(bundle["category_evidence"], "category evidence")
    if len(categories) != len(CATEGORIES) or len(evidence) != len(CATEGORIES):
        raise ValueError("the complete mandatory inventory is required")
    names, evidence_ids, gaps = set(), set(), set()
    for category, observed in zip(categories, evidence):
        _object(category, {"name", "status"}, "category")
        name = _text(category["name"], "category name")
        if name not in CATEGORIES or name in names:
            raise ValueError("unknown or duplicate category")
        names.add(name)
        status = category["status"]
        if type(status) is not dict or type(status.get("status")) is not str:
            raise ValueError("typed category status required")
        tag = status["status"]
        if tag == "Removed":
            _object(status, {"status", "count"}, "Removed")
            count = _uint(status["count"], 64, "removed count")
        elif tag == "VerifiedEmpty":
            _object(status, {"status"}, "VerifiedEmpty")
            count = 0
        elif tag == "CoverageGap":
            _object(status, {"status", "reason"}, "CoverageGap")
            _text(status["reason"], "gap reason")
            gaps.add(name)
            count = None
        else:
            raise ValueError("unknown category status")
        _object(observed, {"spirit_id", "name", "frame_id", "post_inclusion", "removals"}, "category evidence")
        frame = _frame_id(observed["frame_id"])
        if (_text(observed["spirit_id"], "evidence spirit") != spirit or _text(observed["name"], "evidence name") != name
                or frame in evidence_ids or frame not in post_ids
                or not _inclusion(post_root, _leaf(frame), observed["post_inclusion"], post_hashes)):
            raise ValueError("invalid category observation binding")
        evidence_ids.add(frame)
        total = _removals(observed["removals"], name, spirit, post_ids, erased, principal_frames)
        if count is not None and total != count:
            raise ValueError("Removed/VerifiedEmpty count differs from matching typed evidence")
        if tag == "VerifiedEmpty" and observed["removals"]:
            raise ValueError("VerifiedEmpty cannot carry removal claims")
    completion = bundle["frame_sweep_completion"]
    frame_gap = bool(gaps & {"principal_frames", "intent_lineage"})
    if completion is None:
        if not frame_gap:
            raise ValueError("uncertified frame sweep must be an explicit coverage gap")
    else:
        _object(completion, {"frame_id", "through_rowid", "post_inclusion"}, "frame-sweep completion")
        frame = _frame_id(completion["frame_id"])
        _uint(completion["through_rowid"], 64, "completion high-water")
        if (frame_gap or frame in evidence_ids or frame in pre_ids or frame not in post_ids
                or not _inclusion(post_root, _leaf(frame), completion["post_inclusion"], post_hashes)):
            raise ValueError("invalid durable completion marker binding")
    signature = _object(bundle["signature_block"], {"algorithm", "attester_pubkey", "signature"}, "signature block")
    if signature["algorithm"] != "Ed25519" or _hex_bytes(signature["attester_pubkey"], 32, "attester key") != public_key:
        raise ValueError("signature algorithm or pinned attester mismatch")
    raw_signature = _hex_bytes(signature["signature"], 64, "signature")
    unsigned = {key: value for key, value in bundle.items() if key != "signature_block"}
    canonical = json.dumps(unsigned, separators=(",", ":"), sort_keys=True, ensure_ascii=False, allow_nan=False).encode("utf-8")
    if len(canonical) > MAX_BUNDLE_BYTES:
        raise ValueError("canonical bundle exceeds the size limit")
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
    except ImportError as error:
        raise ValueError("declared cryptography backend missing; install tools/verify-erasure/requirements.txt with --require-hashes") from error
    Ed25519PublicKey.from_public_bytes(public_key).verify(raw_signature, hashlib.sha256(canonical).digest())
    return gaps, completion


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _integer(text):
    value = int(text)
    if text.startswith("-") and value == 0:
        raise ValueError("negative zero is not an unsigned integer")
    return value


def _nonfinite(text):
    raise ValueError(f"non-finite JSON number: {text}")


def main():
    if len(sys.argv) not in (3, 4) or (len(sys.argv) == 4 and sys.argv[2] != "--pubkey-file"):
        print("Usage: verify.py <bundle> <public-key-hex | --pubkey-file path>", file=sys.stderr)
        return 2
    try:
        with open(sys.argv[1], "rb") as stream:
            raw = stream.read(MAX_BUNDLE_BYTES + 1)
        if len(raw) > MAX_BUNDLE_BYTES:
            raise ValueError("bundle exceeds 128 MiB")
        bundle = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object,
                            parse_int=_integer, parse_constant=_nonfinite)
        key = sys.argv[2]
        if key == "--pubkey-file" or (len(key) != 64 and os.path.isfile(key)):
            with open(sys.argv[3] if key == "--pubkey-file" else key, encoding="ascii") as stream:
                key = stream.read(129).strip()
        gaps, completion = verify(bundle, _hex_bytes(key, 32, "pinned public key"))
    except Exception as error:
        print(f"FAIL — {type(error).__name__}: {error}", file=sys.stderr)
        return 1
    if gaps:
        print("OK — valid signed partial inventory; coverage gaps remain: " + ", ".join(sorted(gaps)))
    else:
        print("OK — signed inventory and frame-sweep marker verified")
    if completion is not None:
        print(f"Frame sweep covers commits through rowid {completion['through_rowid']} at its durable marker; later commits are not claimed erased.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
