# Standalone Audit Bundle Verifier

Verifies the Ed25519 signature on a `maos.audit-bundle.v1` JSON file.

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
python verify.py bundle.json <64-char-hex-pubkey>

# Verify with pubkey in a file
python verify.py bundle.json --pubkey-file pubkey.hex

# The pubkey file should contain a single line: 64 hex characters (32 bytes)
```

## How It Works

1. Reads the bundle JSON
2. Extracts `signature_block` (algorithm, attester_pubkey, signature)
3. Rebuilds the unsigned bundle (all fields except `signature_block`)
4. Canonicalizes: sorted keys, no whitespace → deterministic byte sequence
5. Computes SHA-256 of the canonical bytes
6. Verifies Ed25519 signature over the SHA-256 digest
7. Exit 0 = valid, exit 1 = invalid, exit 2 = usage error

## OpenSSL Alternative

If no Python Ed25519 library is available, use OpenSSL. **Every command below was executed against the
real T6 bundle on 2026-08-22 and ends in `Signature Verified Successfully`.**

> ⚠ **The block previously published here could never have worked, in three independent ways**
> (`j1-crosshost-2e` AC1.2). It is corrected in place rather than quietly replaced, because a
> *documented* fallback that fails is worse than none: an operator hitting it concludes the signature is
> bad.
> 1. `json.dump(..., sort_keys=True)` without `ensure_ascii=False` — the F5 defect. Rust emits non-ASCII
>    as raw UTF-8, Python escaped it to `\uXXXX`, so any bundle with one non-ASCII byte produced
>    different canonical bytes and failed. The real T6 bundle carries **12** such bytes.
> 2. `-pkeyopt digest:SHA256` — **not supported for Ed25519.** OpenSSL refuses outright:
>    `pkeyutl: Can't set parameter "digest:SHA256" … command not supported`. Ed25519 is not a prehashed
>    scheme; the SHA-256 digest is the *message* here, and `-rawin` already says so.
> 3. `echo <hex> | xxd -r -p > pubkey.bin` then `-inkey pubkey.bin` — **OpenSSL cannot read a raw
>    32-byte Ed25519 key.** It reports `No supported data to decode`. The key must be wrapped in an
>    SPKI DER/PEM structure first. (`xxd` is also not always installed; `python3` already is.)

```bash
BUNDLE=bundle.json
PUBKEY=<64-hex-pubkey>          # from `sealed-export`'s stderr, NOT from `keygen` (that prints a
                                # truncated fingerprint and is rejected by verify.py)

# §A6 review P8 — every file below lives in a PRIVATE temp dir that is removed on
# exit, and `set -e` aborts on the first failed step. The previous block wrote
# fixed /tmp paths with no fail-fast: two verifications running concurrently over
# the same bundle body could cross-read each other's sig.bin, letting an INVALID
# signature verify against the other invocation's valid one.
set -e
WORK=$(mktemp -d /tmp/verify-audit-bundle.XXXXXX)
trap 'rm -rf "$WORK"' EXIT

# 1. Extract the signature.
python3 -c "
import json
b = json.load(open('$BUNDLE'))
open('$WORK/sig.bin','wb').write(bytes.fromhex(b['signature_block']['signature']))
"

# 2. Canonical unsigned bundle: drop signature_block, sort every key, no whitespace, RAW UTF-8.
#    ensure_ascii=False is load-bearing (see defect 1 above); stdout.buffer keeps the encoding
#    UTF-8 regardless of the operator's locale.
python3 -c "
import json, sys
b = json.load(open('$BUNDLE'))
del b['signature_block']
sys.stdout.buffer.write(
    json.dumps(b, ensure_ascii=False, separators=(',',':'), sort_keys=True).encode('utf-8')
)
" > "$WORK/canonical.json"

# 3. The signed message is sha256(canonical).
python3 -c "
import hashlib
open('$WORK/digest.bin','wb').write(hashlib.sha256(open('$WORK/canonical.json','rb').read()).digest())
"

# 4. Wrap the raw pubkey in Ed25519 SPKI DER, then convert to PEM (see defect 3 above).
python3 -c "
import sys
open('$WORK/pubkey.der','wb').write(bytes.fromhex('302a300506032b6570032100') + bytes.fromhex('$PUBKEY'))
"
openssl pkey -pubin -inform DER -in "$WORK/pubkey.der" -out "$WORK/pubkey.pem"

# 5. Verify. NO -pkeyopt (see defect 2 above).
openssl pkeyutl -verify -pubin -inkey "$WORK/pubkey.pem" \
  -rawin -in "$WORK/digest.bin" -sigfile "$WORK/sig.bin"
#   → Signature Verified Successfully
```
