# Story 9.4 AC-2 — RPM spec for MAOS.
#
# Build: rpmbuild -bb maos.spec
# NOTE: At v0.5 this is a SCAFFOLD. The RPM must be built against the first
# tagged release. The embedded RELEASE_PUBKEY must be replaced with the
# production key before the first tagged release.

Name:           maos
Version:        0.5.0
Release:        1%{?dist}
Summary:        Minimal Agentic Operating System — multi-Spirit runtime

License:        Apache-2.0 OR MIT
URL:            https://github.com/lunarpulse/maos
Source1:        https://github.com/lunarpulse/maos/releases/download/v%{version}/SHA256SUMS
Source2:        https://github.com/lunarpulse/maos/releases/download/v%{version}/SHA256SUMS.sig

# Placeholder: replace with the production Ed25519 release public key
# (64 lowercase hex chars) before the first tagged release.
%global release_pubkey bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7

ExclusiveArch:  x86_64 aarch64

BuildRequires:  python3-cryptography

# Architecture-specific binary source.
%ifarch x86_64
%global maos_binary maos-linux-amd64
Source0:        https://github.com/lunarpulse/maos/releases/download/v%{version}/maos-linux-amd64
%endif
%ifarch aarch64
%global maos_binary maos-linux-arm64
Source0:        https://github.com/lunarpulse/maos/releases/download/v%{version}/maos-linux-arm64
%endif

%description
MAOS provides a minimal, verifiable substrate for running autonomous AI
agents (Spirits) with append-only audit logging, capability-token
authorization, and configurable sandbox tiers. Pre-built release binaries
are Ed25519-signed with SHA256 integrity verification.

%prep
# Verify the Ed25519 signature over SHA256SUMS using the bundled pubkey.
# The signature convention is Ed25519(SHA256(SHA256SUMS)) as implemented in
# `crates/maos-audit/src/release_verify.rs`.
python3 - <<'PY'
import hashlib, sys
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
pubkey = Ed25519PublicKey.from_public_bytes(bytes.fromhex('%{release_pubkey}'))
with open('%{SOURCE1}', 'rb') as f:
    message = hashlib.sha256(f.read()).digest()
with open('%{SOURCE2}', 'rb') as f:
    signature = f.read()
try:
    pubkey.verify(signature, message)
except Exception as e:
    print(f'Ed25519 signature verification failed: {e}', file=sys.stderr)
    sys.exit(1)
PY

# Verify that the downloaded binary matches the signed manifest.
expected_hash=$(awk -v fname="%{maos_binary}" '$0 ~ "  " fname {print $1}' %{SOURCE1})
actual_hash=$(sha256sum %{SOURCE0} | awk '{print $1}')
if [ "$expected_hash" != "$actual_hash" ]; then
    echo "SHA256 mismatch for %{SOURCE0}" >&2
    exit 1
fi

%install
mkdir -p %{buildroot}%{_bindir}
install -m 755 %{SOURCE0} %{buildroot}%{_bindir}/maos

%files
%{_bindir}/maos
