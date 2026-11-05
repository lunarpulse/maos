#!/usr/bin/env python3
"""Witness-only wrong-key plant: update download hashes, NEVER the trusted key.

Caller owns backup/copy-back/cmp/touch of the signature and rendered recipes.
"""
import hashlib
import pathlib
import sys
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def forge(directory):
    dist = pathlib.Path(directory)
    signature = dist / 'SHA256SUMS.sig'
    old_hash = hashlib.sha256(signature.read_bytes()).hexdigest()
    wrong = Ed25519PrivateKey.from_private_bytes(bytes(32)).sign(
        hashlib.sha256((dist / 'SHA256SUMS').read_bytes()).digest())
    new_hash = hashlib.sha256(wrong).hexdigest()
    if old_hash == new_hash:
        raise ValueError('fixture already uses wrong control key')
    for name in ['PKGBUILD', 'maos.rb']:
        recipe = dist / 'recipes' / name
        text = recipe.read_text()
        if text.count(old_hash) != 1:
            raise ValueError(f'{name}: expected exactly one outer signature hash')
        recipe.write_text(text.replace(old_hash, new_hash, 1))
    signature.write_bytes(wrong)


if __name__ == '__main__':
    forge(sys.argv[1])
