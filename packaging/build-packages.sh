#!/usr/bin/env bash
# Build pre-rendered recipes without compiling or transforming release binaries.
set -euo pipefail
dist=$(realpath "$1")
format=$2
out=$(realpath -m "$3")
mkdir -p "$out"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
case "$format" in
  deb)
    mkdir -p "$work/maos"
    cp -a "$dist/recipes/debian" "$work/maos/debian"
    cp "$dist/recipes/verify-release.py" "$dist"/SHA256SUMS* "$dist"/*-linux-* "$work/maos/"
    chmod +x "$work/maos/debian/rules"
    (cd "$work/maos" && dpkg-buildpackage -us -uc -b)
    cp "$work"/*.deb "$out/"
    ;;
  rpm)
    mkdir -p "$work"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
    cp "$dist/recipes/verify-release.py" "$dist"/SHA256SUMS* "$dist"/*-linux-* "$work/SOURCES/"
    rpmbuild -bb --define "_topdir $work" "$dist/recipes/maos.spec"
    find "$work/RPMS" -name '*.rpm' -exec cp '{}' "$out/" \;
    ;;
  *) echo "unsupported package format: $format" >&2; exit 2 ;;
esac
