#!/usr/bin/env bash
# Hosted-only package-manager witnesses; static wiring is not execution proof.
set -euo pipefail
route=$1
dist=$(realpath "$2")
root=$(cd "$(dirname "$0")/.." && pwd)
if [[ ${3:-} != --inside ]]; then
  case "$route" in
    rpm)
      docker run --rm -v "$root:$root" -w "$root" fedora:43@sha256:f01112b9a1f1d0504ad962d9adfa401425033f8ab067e0a821904ebdde5b8ca5 bash -euc '
        dnf install -y rpm-build python3-cryptography
        packaging/witness-recipe.sh rpm "$1" --inside
      ' bash "$dist"
      exit ;;
    arch)
      docker run --rm -v "$root:$root" -w "$root" archlinux:base-devel@sha256:767de6eaa40dcfeda5263a48737ea03a782caabdf3a403b6e6a203c7fd333681 bash -euc '
        pacman -Syu --noconfirm python-cryptography sudo
        useradd -m builder
        echo "builder ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/builder
        packaging/witness-recipe.sh arch "$1" --inside
      ' bash "$dist"
      exit ;;
    brew)
      eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
      export GIT_AUTHOR_NAME=maos-witness GIT_AUTHOR_EMAIL=witness@invalid
      export GIT_COMMITTER_NAME=maos-witness GIT_COMMITTER_EMAIL=witness@invalid
      export HOMEBREW_GIT_NAME=maos-witness HOMEBREW_GIT_EMAIL=witness@invalid
      export HOMEBREW_GIT_COMMITTER_NAME=maos-witness HOMEBREW_GIT_COMMITTER_EMAIL=witness@invalid
      ;;
    *) echo "unknown route: $route" >&2; exit 2 ;;
  esac
fi
scratch=$(mktemp -d)
files=(SHA256SUMS.sig recipes/PKGBUILD recipes/maos.rb recipes/maos.spec)
for file in "${files[@]}"; do mkdir -p "$scratch/$(dirname "$file")"; cp "$dist/$file" "$scratch/$file"; done
restore() {
  for file in "${files[@]}"; do cp "$scratch/$file" "$dist/$file"; cmp "$scratch/$file" "$dist/$file"; touch "$dist/$file"; done
}
cleanup() {
  restore
  if [[ $route == brew ]]; then
    brew list --formula maos >/dev/null 2>&1 && brew uninstall maos || true
    brew untap maos/verify >/dev/null 2>&1 || true
  fi
  rm -rf "$scratch"
}
trap cleanup EXIT
python3 "$root/packaging/forge-recipe-signature.py" "$dist"
case "$route" in
  rpm)
    if "$root/packaging/build-packages.sh" "$dist" rpm "$scratch/bad" > "$scratch/negative.log" 2>&1; then
      echo 'accepted forged signature: RPM recipe' >&2; exit 1
    fi
    cat "$scratch/negative.log"
    grep -F InvalidSignature "$scratch/negative.log"
    ! rpm -q maos
    for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "/usr/bin/$bin"; done
    restore
    "$root/packaging/build-packages.sh" "$dist" rpm "$scratch/good"
    rpm -i "$scratch/good"/*.rpm
    /usr/bin/maos --version
    python3 "$root/packaging/check-installed.py" "$dist" /usr/bin linux-amd64
    rpm -e maos
    for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "/usr/bin/$bin"; done
    rpm --eval "%{lua:print(rpm.vercmp(\"1.0.0~rc.1\",\"1.0.0\"))}" | python3 -c 'import sys; assert int(sys.stdin.read()) < 0'
    ;;
  arch)
    chmod 755 "$scratch"
    mkdir "$scratch/bad" "$scratch/good"
    cp "$dist/recipes/PKGBUILD" "$scratch/bad/"
    chown -R builder:builder "$scratch/bad" "$scratch/good"
    if (cd "$scratch/bad" && sudo -u builder makepkg -si --noconfirm) > "$scratch/negative.log" 2>&1; then
      echo 'accepted forged signature: AUR recipe' >&2; exit 1
    fi
    cat "$scratch/negative.log"
    grep -F InvalidSignature "$scratch/negative.log"
    ! pacman -Q maos
    for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "/usr/bin/$bin"; done
    restore
    cp "$dist/recipes/PKGBUILD" "$scratch/good/"
    (cd "$scratch/good" && sudo -u builder makepkg -si --noconfirm)
    /usr/bin/maos --version
    python3 "$root/packaging/check-installed.py" "$dist" /usr/bin linux-amd64
    # Use the real rendered values, not hand-written comparator examples.
    actual=$(bash -c 'source "$1"; printf "%s" "$pkgver"' bash "$dist/recipes/PKGBUILD")
    if [[ $actual == *[a-zA-Z]* ]]; then
      test "$(vercmp "$actual" "${actual%%[a-zA-Z]*}")" -lt 0
    else
      test "$(vercmp "$actual" "$actual")" -eq 0
    fi
    pacman -R --noconfirm maos
    for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "/usr/bin/$bin"; done
    ;;
  brew)
    brew tap-new maos/verify
    tap=$(brew --repository maos/verify)
    cp "$dist/recipes/maos.rb" "$tap/Formula/maos.rb"
    if HOMEBREW_NO_INSTALL_CLEANUP=1 brew install --build-from-source maos/verify/maos > "$scratch/negative.log" 2>&1; then
      echo 'accepted forged signature: Homebrew recipe' >&2; exit 1
    fi
    cat "$scratch/negative.log"
    grep -F InvalidSignature "$scratch/negative.log"
    ! brew list --formula maos >/dev/null 2>&1
    prefix=$(brew --prefix)/opt/maos
    for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "$prefix/bin/$bin"; done
    restore
    cp "$dist/recipes/maos.rb" "$tap/Formula/maos.rb"
    HOMEBREW_NO_INSTALL_CLEANUP=1 brew install --build-from-source maos/verify/maos
    if brew audit --strict maos/verify/maos > "$scratch/audit.log" 2>&1; then
      audit_status=0
    else
      audit_status=$?
    fi
    cat "$scratch/audit.log"
    brew test maos/verify/maos
    python3 "$root/packaging/check-installed.py" "$dist" "$(brew --prefix maos)/bin" linux-amd64
    prefix=$(brew --prefix maos)
    brew uninstall maos
    for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "$prefix/bin/$bin"; done
    brew untap maos/verify
    test "$audit_status" -eq 0
    ;;
esac
echo "PASS: $route independently rejected wrong-key signature, then installed exact signed bytes and removed them"
