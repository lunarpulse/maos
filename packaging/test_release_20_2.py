#!/usr/bin/env python3
"""Real signatures and adversarial exact-name/trace vectors; no cargo subprocess."""
import hashlib
import importlib.util
import pathlib
import tempfile
import unittest

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

ROOT = pathlib.Path(__file__).resolve().parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


verifier = load('verifier', ROOT/'packaging/verify-release.py')
trace = load('trace', ROOT/'tests/air-gap-trace.py')


class ReleaseControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.dist = pathlib.Path(self.temp.name)
        self.key = Ed25519PrivateKey.from_private_bytes(bytes(range(32)))
        self.pubkey = self.key.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw).hex()
        (self.dist/'maos-linux-amd64').write_bytes(b'actual signed artifact bytes')
        digest = hashlib.sha256((self.dist/'maos-linux-amd64').read_bytes()).hexdigest()
        self.manifest = f'{digest}  maos-linux-amd64\n'.encode()
        self.sign(self.manifest)

    def sign(self, manifest):
        (self.dist/'SHA256SUMS').write_bytes(manifest)
        (self.dist/'SHA256SUMS.sig').write_bytes(self.key.sign(hashlib.sha256(manifest).digest()))

    def verify(self):
        verifier.verify(self.dist, self.pubkey, ['maos-linux-amd64'])

    def test_ac1_ac2_signed_bytes_pass(self):
        self.verify()

    def test_ac1_forged_signature_refused(self):
        zero = Ed25519PrivateKey.from_private_bytes(bytes(32))
        (self.dist/'SHA256SUMS.sig').write_bytes(zero.sign(hashlib.sha256(self.manifest).digest()))
        with self.assertRaises(Exception):
            self.verify()

    def test_ac5_changed_byte_refused(self):
        (self.dist/'maos-linux-amd64').write_bytes(b'tampered signed artifact bytes')
        with self.assertRaisesRegex(ValueError, 'SHA256 mismatch'):
            self.verify()

    def test_ac1_exact_filename_not_regex_substring(self):
        self.sign(self.manifest.replace(b'maos-linux-amd64', b'prefix-maos-linux-amd64'))
        with self.assertRaisesRegex(ValueError, 'missing exact'):
            self.verify()

    def test_ac2_duplicate_manifest_entry_refused(self):
        self.sign(self.manifest+self.manifest)
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.verify()

    def test_ac3_outbound_canary_and_unix_exclusion(self):
        text = '\n'.join([
            '10 socket(AF_INET, SOCK_STREAM, IPPROTO_TCP) = 3',
            '10 connect(3<TCP:[1]>, {sa_family=AF_INET, sin_port=htons(443)}, 16) = -1 ENETUNREACH',
            '11 connect(4<UNIX-STREAM:[2]>, {sa_family=AF_UNIX, sun_path="/tmp/maos"}, 16) = 0',
            '12 sendmsg(5<UDPv6:[3]>, {msg_name=NULL}, 0) = 8',
        ])
        inet, other = trace.outbound(text)
        self.assertEqual(len(inet), 2)
        self.assertEqual(other, {'AF_UNIX': 1})
        self.assertEqual(trace.outbound('socket(AF_INET, SOCK_STREAM, 0) = 3')[0], [])


if __name__ == '__main__':
    unittest.main()
