# Render with xtask render-packages before building.
Name: maos
Version: 0.1.0~alpha.1
Release: 1%{?dist}
Summary: Minimal Agentic Operating System
License: Apache-2.0 OR MIT
URL: https://github.com/lunarpulse/maos
ExclusiveArch: x86_64 aarch64
BuildRequires: python3-cryptography
%global debug_package %{nil}
%global __os_install_post %{nil}
%ifarch x86_64
%global maos_artifact_suffix linux-amd64
%else
%global maos_artifact_suffix linux-arm64
%endif
Source0: file:///tmp/maos-20-2-hosted1-dev202/dist/maos-%{maos_artifact_suffix}
Source1: file:///tmp/maos-20-2-hosted1-dev202/dist/maosctl-%{maos_artifact_suffix}
Source2: file:///tmp/maos-20-2-hosted1-dev202/dist/maos-spirit-%{maos_artifact_suffix}
Source3: file:///tmp/maos-20-2-hosted1-dev202/dist/maos-registry-server-%{maos_artifact_suffix}
Source4: file:///tmp/maos-20-2-hosted1-dev202/dist/SHA256SUMS
Source5: file:///tmp/maos-20-2-hosted1-dev202/dist/SHA256SUMS.sig
Source6: file:///tmp/maos-20-2-hosted1-dev202/dist/verify-release.py

%description
MAOS multi-Spirit runtime with append-only audit logging and capability tokens.

%prep
cd %{_sourcedir}
python3 verify-release.py --pubkey bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7 maos-%{maos_artifact_suffix} maosctl-%{maos_artifact_suffix} maos-spirit-%{maos_artifact_suffix} maos-registry-server-%{maos_artifact_suffix}

%build

%install
for bin in maos maosctl maos-spirit maos-registry-server; do
    install -Dm755 %{_sourcedir}/$bin-%{maos_artifact_suffix} %{buildroot}%{_bindir}/$bin
done

%files
%{_bindir}/maos
%{_bindir}/maosctl
%{_bindir}/maos-spirit
%{_bindir}/maos-registry-server
