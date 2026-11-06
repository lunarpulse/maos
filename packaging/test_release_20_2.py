#!/usr/bin/env python3
"""Real signatures and adversarial exact-name/trace vectors; no cargo subprocess."""
import ast
import hashlib
import importlib.util
import pathlib
import tempfile
import shutil
import shlex
import re
import subprocess
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
assembly = load('assembly', ROOT/'packaging/assemble-release-packaging.py')
release_run = load('release_run', ROOT/'packaging/release-run.py')
forger = load('forger', ROOT/'packaging/forge-recipe-signature.py')


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

    def test_review_f6_sendmmsg_cannot_hide_an_outbound_attempt(self):
        inet, _ = trace.outbound('12 sendmmsg(5<UDPv6:[3]>, [{msg_hdr={msg_name=NULL}}], 1, 0) = 1')
        self.assertEqual(len(inet), 1)

    def test_review_f6_io_uring_is_uninspectable_not_a_clean_witness(self):
        _, other = trace.outbound('12 io_uring_enter(7<anon_inode:[io_uring]>, 1, 1, 0, NULL, 8) = 1')
        self.assertEqual(other.get('io_uring_enter'), 1)

    def test_review_f1_packaging_manifest_covers_executable_verifier_and_every_recipe(self):
        packages = self.dist/'packages'
        packages.mkdir()
        for name in ['amd64.deb', 'arm64.deb', 'amd64.rpm', 'arm64.rpm']:
            (packages/name).write_bytes(b'opaque manifest-unit-test package fixture')
        recipes = self.dist/'recipes'
        shutil.copytree(ROOT/'packaging/deb', recipes/'debian')
        for source, target in [('homebrew/maos.rb', 'maos.rb'), ('aur/PKGBUILD', 'PKGBUILD'),
                               ('rpm/maos.spec', 'maos.spec'), ('verify-release.py', 'verify-release.py')]:
            shutil.copyfile(ROOT/'packaging'/source, recipes/target)
        assembly.assemble(self.dist)
        manifest = (self.dist/'SHA256SUMS-packages').read_bytes()
        self.sign(manifest)
        names = [line.split('  ')[1] for line in manifest.decode().splitlines()]
        verifier.verify(self.dist, self.pubkey, names)
        for name in ['verify-release.py', 'maos.rb', 'PKGBUILD', 'maos.spec', 'debian-recipes.tar.gz']:
            with self.subTest(asset=name):
                original = (self.dist/name).read_bytes()
                (self.dist/name).write_bytes(original+b'changed asset')
                with self.assertRaisesRegex(ValueError, 'SHA256 mismatch'):
                    verifier.verify(self.dist, self.pubkey, names)
                (self.dist/name).write_bytes(original)

    def test_review_p5_wrong_key_control_updates_outer_hash_not_trusted_key(self):
        recipes = self.dist/'recipes'
        recipes.mkdir()
        old_hash = hashlib.sha256((self.dist/'SHA256SUMS.sig').read_bytes()).hexdigest()
        for source, target in [('homebrew/maos.rb', 'maos.rb'), ('aur/PKGBUILD', 'PKGBUILD')]:
            text = (ROOT/'packaging'/source).read_text()
            (recipes/target).write_text(text.replace('@SIGNATURE_HASH@', old_hash).replace('@PUBKEY@', self.pubkey))
        forger.forge(self.dist)
        for name in ['maos.rb', 'PKGBUILD']:
            self.assertIn(self.pubkey, (recipes/name).read_text())
            self.assertNotIn(old_hash, (recipes/name).read_text())
        with self.assertRaises(Exception):
            self.verify()

    def test_review_f9_exact_tag_and_sha_excludes_another_successful_run(self):
        good = dict(id=10, run_attempt=2, head_sha='abc', head_branch='v1', event='push')
        unrelated = dict(good, id=20, head_branch='v2')
        self.assertEqual(release_run.select([good, unrelated], 'abc', 'v1'), (10, 2))
        self.assertIsNone(release_run.select([unrelated], 'abc', 'v1'))

    def test_review_f9_stale_attempt_and_failed_current_attempt_are_refused(self):
        stale = dict(id=10, run_attempt=1, head_sha='abc', head_branch='v1', event='push',
                     status='completed', conclusion='success')
        with self.assertRaisesRegex(ValueError, 'identity/attempt'):
            release_run.successful(stale, 10, 2, 'abc', 'v1')
        current = dict(stale, run_attempt=2, conclusion='failure')
        with self.assertRaisesRegex(ValueError, 'failed'):
            release_run.successful(current, 10, 2, 'abc', 'v1')
        current['conclusion'] = 'success'
        self.assertTrue(release_run.successful(current, 10, 2, 'abc', 'v1'))

    def test_hosted_1_arch_raw_ed25519_signature_is_not_openpgp(self):
        recipe = (ROOT/'packaging/aur/PKGBUILD').read_text()
        sources = next(line for line in recipe.splitlines() if line.startswith('source=('))
        names = [source.split('::', 1)[0] if '::' in source else source.rsplit('/', 1)[-1]
                 for source in shlex.split(sources[len('source=('):-1])]
        self.assertFalse(any(name.endswith(('.sig', '.asc')) for name in names), names)
        self.assertIn('SHA256SUMS.ed25519::@BASE_URL@/SHA256SUMS.sig', recipe)
        self.assertLess(recipe.index('cp SHA256SUMS.ed25519 SHA256SUMS.sig'),
                        recipe.index('python3 verify-release.py'))

    def test_hosted_1_brew_plant_removes_exact_actual_two_line_verifier(self):
        workflow = ROOT/'_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml'
        assignment = next(line.strip() for line in workflow.read_text().splitlines()
                          if line.strip().startswith('pattern = '))
        context = {'route': 'brew'}
        exec(compile(ast.parse(assignment), str(workflow), 'exec'), context)
        recipe = (ROOT/'packaging/homebrew/maos.rb').read_text()
        changed, count = re.subn(context['pattern'], '', recipe, flags=re.M)
        self.assertEqual(count, 1)
        self.assertNotIn('system Formula["python@3.14"]', changed)
        self.assertNotIn('"--pubkey"', changed)
        self.assertIn('names.each { |name| bin.install', changed)

    def test_hosted_1_arch_verifier_bypass_reaches_package_instead_of_false_exit(self):
        workflow = ROOT/'_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml'
        statements = [line.strip() for line in workflow.read_text().splitlines()
                      if line.strip().startswith(('pattern = ', 'text, count=re.subn('))]
        self.assertEqual(len(statements), 2)
        recipe = (ROOT/'packaging/aur/PKGBUILD').read_text().replace('@ARCH_SOURCES@', '')
        context = {'route': 'arch', 'text': recipe, 're': re}
        exec(compile(ast.parse('\n'.join(statements)), str(workflow), 'exec'), context)
        self.assertEqual(context['count'], 1)
        self.assertNotIn('python3 verify-release.py', context['text'])
        (self.dist/'PKGBUILD').write_text(context['text'])
        (self.dist/'SHA256SUMS.ed25519').write_bytes((self.dist/'SHA256SUMS.sig').read_bytes())
        result = subprocess.run(['bash', '-euc', 'source "$1"; CARCH=x86_64; prepare',
                                 'bash', str(self.dist/'PKGBUILD')],
                                cwd=self.dist, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout+result.stderr)


if __name__ == '__main__':
    unittest.main()
