#!/usr/bin/env python3
"""Generate per-run Ed25519 T3 pins; provision the immutable glibc/shell image."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

# Docker Hub's Linux/amd64 manifest, observed 2026-10-09, not its mutable tag
# or multi-platform index. Ubuntu 26.04 supplies glibc and /bin/sh for both CI
# images' native worker-cli-fixture and the existing shell adapter fixtures.
IMAGE = 'docker.io/library/ubuntu@sha256:88a381d5b5eeb2b35d3ad70925a362c37ce569daf43ede89ff818ec20e4d3794'


def run(arguments, **kwargs):
    environment = {key: os.environ[key] for key in ('PATH', 'HOME', 'XDG_RUNTIME_DIR', 'DBUS_SESSION_BUS_ADDRESS') if key in os.environ}
    return subprocess.run(arguments, env=environment, check=True, **kwargs)


def storage_configuration(store):
    """Reuse only OCI storage, never the provisioner's home or credentials."""
    lines = ['[storage]']
    for key, field in (('driver', 'graphDriverName'), ('graphroot', 'graphRoot'), ('runroot', 'runRoot')):
        lines.append(f'{key} = {json.dumps(store[field])}')
    tables = {}
    for qualified, value in store.get('graphOptions', {}).items():
        driver, option = qualified.split('.', 1)
        if isinstance(value, dict):
            value = value['Executable']
        tables.setdefault(driver, []).append(f'{json.dumps(option)} = {json.dumps(value)}')
    for driver, options in tables.items():
        lines.extend([f'[storage.options.{json.dumps(driver)}]', *options])
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output_dir', type=Path)
    parser.add_argument('--runtime', choices=('podman', 'docker'), default='podman')
    parser.add_argument('--generate-only', action='store_true', help='exercise signing without claiming a usable OCI substrate')
    arguments = parser.parse_args()
    directory = arguments.output_dir.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    runtime = f'/usr/bin/{arguments.runtime}'
    if not arguments.generate_only:
        run([runtime, 'pull', IMAGE])
        observed = json.loads(run([runtime, 'image', 'inspect', '--format', '{{json .RepoDigests}}', IMAGE], stdout=subprocess.PIPE, text=True).stdout)
        if IMAGE not in observed:
            raise RuntimeError(f'pulled image lacks the exact attested manifest digest: {observed}')
        run([runtime, 'run', '--rm', '--network=none', '--http-proxy=false', '--read-only', '--user=65534:65534', IMAGE, '/bin/sh', '-c', 'test -r /lib/x86_64-linux-gnu/libc.so.6 && printf "T3 image glibc and shell available\\n"'])
        if arguments.runtime == 'podman':
            store = json.loads(run([runtime, 'info', '--format', '{{json .Store}}'], stdout=subprocess.PIPE, text=True).stdout)
            (directory / 'storage.conf').write_text(storage_configuration(store))
    entries = [{
        'image_uri': IMAGE,
        'image_sha256': list(bytes.fromhex(IMAGE.split('@sha256:')[1])),
        'description': '17-1 hermetic Ubuntu 26.04 glibc and /bin/sh test image',
        'default_for_v05': True,
    }]
    # Rust serde_json serializes T3ImageEntry in this declaration order, compact.
    canonical = json.dumps(entries, separators=(',', ':'), ensure_ascii=False).encode()
    with tempfile.TemporaryDirectory(prefix='maos-t3-test-key-') as scratch:
        scratch = Path(scratch)
        private = scratch / 'private.pem'
        public = scratch / 'public.der'
        payload = scratch / 'entries.json'
        signature = scratch / 'signature'
        payload.write_bytes(canonical)
        run(['openssl', 'genpkey', '-algorithm', 'ED25519', '-out', str(private)], stdout=subprocess.DEVNULL)
        private.chmod(0o600)
        run(['openssl', 'pkey', '-in', str(private), '-pubout', '-outform', 'DER', '-out', str(public)], stdout=subprocess.DEVNULL)
        der = public.read_bytes()
        if len(der) != 44 or der[:12] != bytes.fromhex('302a300506032b6570032100'):
            raise RuntimeError('unexpected Ed25519 public-key encoding')
        anchor = der[12:]
        run(['openssl', 'pkeyutl', '-sign', '-rawin', '-inkey', str(private), '-in', str(payload), '-out', str(signature)])
        run(['openssl', 'pkeyutl', '-verify', '-rawin', '-pubin', '-keyform', 'DER', '-inkey', str(public), '-in', str(payload), '-sigfile', str(signature)], stdout=subprocess.DEVNULL)
        signed = signature.read_bytes()
        if len(signed) != 64:
            raise RuntimeError('unexpected Ed25519 signature length')
        lock = [{
            'id': list(os.urandom(32)),
            'schema_version': 1,
            'signed_at_ns': time.time_ns(),
            'entries': entries,
            'signature': list(signed),
            'signer_pub_key': list(anchor),
        }]
    lock_path = directory / 't3-image.lock'
    lock_path.write_text(json.dumps(lock, separators=(',', ':')) + '\n')
    (directory / 'trust-anchor.hex').write_text(anchor.hex() + '\n')
    exports = {
        'MAOS_T3_RUNTIME': arguments.runtime,
        'MAOS_T3_IMAGE_LOCK_PATH': str(lock_path),
        'MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX': anchor.hex(),
    }
    env_file = directory / 't3.env'
    env_file.write_text(''.join(f'export {name}={json.dumps(value)}\n' for name, value in exports.items()))
    if not arguments.generate_only and os.environ.get('GITHUB_ENV'):
        with open(os.environ['GITHUB_ENV'], 'a') as github_env:
            github_env.write(''.join(f'{name}={value}\n' for name, value in exports.items()))
    capability = 'signing-only; OCI unexercised' if arguments.generate_only else 'immutable image pulled; rootless network-none shell exercised'
    print(f'::notice title=T3 test substrate::{capability}; image={IMAGE}; lock={lock_path}; Ed25519 verified')
    print(f'Source {env_file} for the test runtime, lock and public trust anchor; private key removed.')


if __name__ == '__main__':
    main()
