#!/usr/bin/env bash
# Hosted-only package-manager lanes. No static recipe check substitutes for these.
set -euo pipefail
route=$1
dist=$(realpath "$2")
case "$route" in
  rpm)
    docker run --rm -v "$PWD:$PWD" -w "$PWD" fedora:43 bash -euc '
      dnf install -y rpm-build python3-cryptography
      packaging/build-packages.sh "$1" rpm "$1/rpms"
      rpm -i "$1"/rpms/*.rpm
      maos --version
      python3 packaging/check-installed.py "$1" /usr/bin linux-amd64
      rpm -e maos
      for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "/usr/bin/$bin"; done
      rpm --eval "%{lua:print(rpm.vercmp(\"1.0.0~rc.1\",\"1.0.0\"))}" | python3 -c "import sys; assert int(sys.stdin.read()) < 0"
      rpm --eval "%{lua:print(rpm.vercmp(\"0.1.0~alpha.1\",\"0.1.0\"))}" | python3 -c "import sys; assert int(sys.stdin.read()) < 0"
    ' bash "$dist"
    ;;
  arch)
    docker run --rm -v "$PWD:$PWD" -w "$PWD" archlinux:base-devel bash -euc '
      pacman -Syu --noconfirm python-cryptography sudo
      useradd -m builder
      echo "builder ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/builder
      work=$(mktemp -d); chmod 755 "$work"
      cp "$1/recipes/PKGBUILD" "$work/"
      chown -R builder:builder "$work"
      cd "$work"
      sudo -u builder makepkg -si --noconfirm
      /usr/bin/maos --version
      python3 "$2/packaging/check-installed.py" "$1" /usr/bin linux-amd64
      test "$(vercmp 1.0.0rc1 1.0.0)" -lt 0
      test "$(vercmp 0.1.0alpha1 0.1.0)" -lt 0
      pacman -R --noconfirm maos
      for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "/usr/bin/$bin"; done
    ' bash "$dist" "$PWD"
    ;;
  brew)
    eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
    brew tap-new maos/verify
    tap=$(brew --repository maos/verify)
    cp "$dist/recipes/maos.rb" "$tap/Formula/maos.rb"
    HOMEBREW_NO_INSTALL_CLEANUP=1 brew install --build-from-source maos/verify/maos
    brew test maos/verify/maos
    python3 packaging/check-installed.py "$dist" "$(brew --prefix maos)/bin" linux-amd64
    brew audit --strict maos/verify/maos
    prefix=$(brew --prefix maos)
    brew uninstall maos
    for bin in maos maosctl maos-spirit maos-registry-server; do test ! -e "$prefix/bin/$bin"; done
    brew untap maos/verify
    ;;
  *) echo "unknown route: $route" >&2; exit 2 ;;
esac
