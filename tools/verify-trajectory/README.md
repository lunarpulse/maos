# Standalone Trajectory Bundle Verifier

Verifies the Ed25519 signature on a `maos.trajectory.v1` JSON file.

**Zero MAOS workspace dependencies** — this tool uses only Python stdlib + a
lightweight Ed25519 library.

## Install

```bash
pip install PyNaCl   # recommended (wraps libsodium)
# alternatively: pip install ed25519
# alternatively: pip install cryptography
```

## Usage

```bash
# Verify with hex-encoded public key
python verify.py trajectory.json <64-char-hex-pubkey>

# Verify with pubkey in a file
python verify.py trajectory.json --pubkey-file pubkey.hex

# Run with tamper-detection anti-tautology check
python verify.py trajectory.json <pubkey> --tamper-test

# The pubkey file should contain a single line: 64 hex characters (32 bytes)
```

## How It Works

1. Reads the trajectory bundle JSON
2. Validates `schema_version == "maos.trajectory.v1"`
3. Validates `applied_redaction` (boolean) and `redaction_policy` (string)
4. If `applied_redaction` is true, checks at least one entry has a non-null `redaction` field
5. Extracts `signature_block` (algorithm, attester_pubkey, signature)
6. Rebuilds the unsigned bundle (all fields except `signature_block`)
7. Canonicalizes: sorted keys, no whitespace → deterministic byte sequence
8. Computes SHA-256 of the canonical bytes
9. Verifies Ed25519 signature over the SHA-256 digest
10. Exit 0 = valid, exit 1 = invalid/error, exit 2 = usage error or missing Ed25519 backend

## Tamper Test

The `--tamper-test` flag flips one byte in the canonical representation and
verifies that the signature check correctly rejects the tampered data. This
guards against tautological verification (e.g. a no-op stub always returning
true).
