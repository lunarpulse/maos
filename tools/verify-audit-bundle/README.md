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

If no Python Ed25519 library is available, use OpenSSL:

```bash
# Extract the public key from hex
echo "<hex-pubkey>" | xxd -r -p > /tmp/pubkey.bin

# Extract the signature from hex
python3 -c "
import json, sys
b = json.load(open('bundle.json'))
open('/tmp/sig.bin','wb').write(bytes.fromhex(b['signature_block']['signature']))
"

# Create the canonical unsigned bundle (Python one-liner)
python3 -c "
import json, sys
b = json.load(open('bundle.json'))
del b['signature_block']
json.dump(b, sys.stdout, separators=(',',':'), sort_keys=True)
" > /tmp/canonical.json

# Hash and verify
python3 -c "import hashlib; open('/tmp/digest.bin','wb').write(hashlib.sha256(open('/tmp/canonical.json','rb').read()).digest())"
openssl pkeyutl -verify -pubin -inkey /tmp/pubkey.bin -rawin -in /tmp/digest.bin -sigfile /tmp/sig.bin -pkeyopt digest:SHA256
```
