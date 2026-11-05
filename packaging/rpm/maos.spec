# Render with xtask render-packages before building.
Name: maos
Version: @DEB_VERSION@
Release: 1%{?dist}
Summary: Minimal Agentic Operating System
License: Apache-2.0 OR MIT
URL: https://github.com/lunarpulse/maos
ExclusiveArch: x86_64 aarch64
BuildRequires: python3-cryptography
%global debug_package %{nil}
%global __os_install_post %{nil}
%ifarch x86_64
%global suffix linux-amd64
%else
%global suffix linux-arm64
%endif
Source0: @BASE_URL@/maos-%{suffix}
Source1: @BASE_URL@/maosctl-%{suffix}
Source2: @BASE_URL@/maos-spirit-%{suffix}
Source3: @BASE_URL@/maos-registry-server-%{suffix}
Source4: @BASE_URL@/SHA256SUMS
Source5: @BASE_URL@/SHA256SUMS.sig
Source6: @BASE_URL@/verify-release.py

%description
MAOS multi-Spirit runtime with append-only audit logging and capability tokens.

%prep
cd %{_sourcedir}
python3 verify-release.py --pubkey @PUBKEY@ maos-%{suffix} maosctl-%{suffix} maos-spirit-%{suffix} maos-registry-server-%{suffix}

%build

%install
for bin in maos maosctl maos-spirit maos-registry-server; do
    install -Dm755 %{_sourcedir}/$bin-%{suffix} %{buildroot}%{_bindir}/$bin
done

%files
%{_bindir}/maos
%{_bindir}/maosctl
%{_bindir}/maos-spirit
%{_bindir}/maos-registry-server
