# Render with xtask render-packages before building.
Name: maos
Version: @DEB_VERSION@
Release: 1%{?dist}
Summary: Minimal Agentic Operating System
License: Apache-2.0 OR MIT
URL: https://github.com/lunarpulse/maos
ExclusiveArch: x86_64 aarch64
BuildRequires: python3-cryptography
Requires(pre): shadow-utils
%global debug_package %{nil}
%global __os_install_post %{nil}
%ifarch x86_64
%global maos_artifact_suffix linux-amd64
%else
%global maos_artifact_suffix linux-arm64
%endif
Source0: @BASE_URL@/maos-%{maos_artifact_suffix}
Source1: @BASE_URL@/maosctl-%{maos_artifact_suffix}
Source2: @BASE_URL@/maos-spirit-%{maos_artifact_suffix}
Source3: @BASE_URL@/maos-registry-server-%{maos_artifact_suffix}
Source4: @BASE_URL@/maos-wasm-runner-%{maos_artifact_suffix}
Source5: @BASE_URL@/SHA256SUMS
Source6: @BASE_URL@/SHA256SUMS.sig
Source7: @BASE_URL@/verify-release.py

%description
MAOS multi-Spirit runtime with append-only audit logging and capability tokens.

%prep
cd %{_sourcedir}
python3 verify-release.py --pubkey @PUBKEY@ maos-%{maos_artifact_suffix} maosctl-%{maos_artifact_suffix} maos-spirit-%{maos_artifact_suffix} maos-registry-server-%{maos_artifact_suffix} maos-wasm-runner-%{maos_artifact_suffix}

%pre
getent group maos >/dev/null || groupadd -r maos
getent passwd maos >/dev/null || useradd -r -g maos -d /var/lib/maos -s /sbin/nologin -c "MAOS Host" maos
exit 0

%build

%install
for bin in maos maosctl maos-spirit maos-registry-server maos-wasm-runner; do
    install -Dm755 %{_sourcedir}/$bin-%{maos_artifact_suffix} %{buildroot}%{_bindir}/$bin
done
# Story 22-2: the service unit, account file and example Host file, rendered from the same
# packaging/systemd sources as the deb. Literal paths: %{_unitdir} is not assumed on rpm-ubuntu.
install -d %{buildroot}/usr/lib/systemd/system %{buildroot}/usr/lib/sysusers.d %{buildroot}/usr/share/doc/maos/examples
cat > %{buildroot}/usr/lib/systemd/system/maos.service <<'MAOS_RENDERED_EOF'
@SERVICE_UNIT@MAOS_RENDERED_EOF
cat > %{buildroot}/usr/lib/sysusers.d/maos.conf <<'MAOS_RENDERED_EOF'
@SYSUSERS_CONF@MAOS_RENDERED_EOF
cat > %{buildroot}/usr/share/doc/maos/examples/host.toml <<'MAOS_RENDERED_EOF'
@HOST_EXAMPLE@MAOS_RENDERED_EOF

%files
%{_bindir}/maos
%{_bindir}/maosctl
%{_bindir}/maos-spirit
%{_bindir}/maos-registry-server
%{_bindir}/maos-wasm-runner
/usr/lib/systemd/system/maos.service
/usr/lib/sysusers.d/maos.conf
%dir /usr/share/doc/maos
%dir /usr/share/doc/maos/examples
/usr/share/doc/maos/examples/host.toml
