#!/usr/bin/env python3
"""Assemble the second signed denominator: packages AND verifier/recipe bytes.

Run as the runner user after native package builds. The caller signs and verifies
SHA256SUMS-packages with trusted locally built xtask, never this downloaded file.
"""
import hashlib
import pathlib
import shutil
import sys
import tarfile


def assemble(directory):
    dist = pathlib.Path(directory)
    packages = sorted((dist / 'packages').glob('*.deb')) + sorted((dist / 'packages').glob('*.rpm'))
    if len(packages) != 4:
        raise ValueError(f'expected two deb and two RPM packages: {packages}')
    for source in packages:
        source.replace(dist / source.name)
    recipes = ['maos.rb', 'PKGBUILD', 'maos.spec', 'verify-release.py']
    for name in recipes:
        shutil.copyfile(dist / 'recipes' / name, dist / name)
    with tarfile.open(dist / 'debian-recipes.tar.gz', 'w:gz') as archive:
        archive.add(dist / 'recipes' / 'debian', arcname='debian')
    names = [source.name for source in packages] + recipes + ['debian-recipes.tar.gz']
    lines = []
    for name in sorted(names):
        with (dist / name).open('rb') as artifact:
            lines.append(f'{hashlib.file_digest(artifact, "sha256").hexdigest()}  {name}\n')
    (dist / 'SHA256SUMS-packages').write_text(''.join(lines))


if __name__ == '__main__':
    assemble(sys.argv[1])
