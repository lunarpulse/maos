# Install and remove MAOS

Choose a release tag from <https://github.com/lunarpulse/maos/releases>.
The binary set is `maos`, `maosctl`, `maos-spirit`, `maos-registry-server`.
Linux release binaries require **glibc 2.39 or newer** (Ubuntu 24.04 / Debian 13
class systems); older distributions need a compatible build or the container.
Linux amd64/arm64 and macOS arm64 are the release artifact targets. macOS formula
execution is not part of the Linux discipline matrix.

## Signed binary

Download all four binaries for one suffix (`linux-amd64`, `linux-arm64`, or
`darwin-arm64`), `SHA256SUMS` and `SHA256SUMS.sig`. Obtain the release public key
through the trusted release-key channel. Bootstrap verification from an
**already trusted source checkout**, not a verifier downloaded beside the
untrusted release. With Python 3 and `cryptography` installed:

```bash
python3 /path/to/trusted-maos-checkout/packaging/verify-release.py --directory ./dist --pubkey "$RELEASE_PUBKEY" \
  maos-linux-amd64 maosctl-linux-amd64 maos-spirit-linux-amd64 maos-registry-server-linux-amd64
chmod +x dist/*-linux-amd64
./dist/maosctl-linux-amd64 install --from-local ./dist --verify-only
for bin in maos maosctl maos-spirit maos-registry-server; do
  install -Dm755 "dist/$bin-linux-amd64" "$HOME/.local/bin/$bin"
done
```

Do not execute any downloaded verifier until its bytes have been authenticated
by trusted tooling. The release's `verify-release.py` and rendered recipes are
covered by the second signed manifest below; it cannot authenticate itself.
Signatures are Ed25519 over SHA256(SHA256SUMS). The trusted Python verifier
rejects malformed/duplicate entries, missing exact names, changed bytes and
incorrect signatures.

## Debian and RPM

Download **every asset listed in `SHA256SUMS-packages`**, together with
`SHA256SUMS-packages.sig`: both architectures' deb/RPM packages, `maos.rb`,
`PKGBUILD`, `maos.spec`, `verify-release.py` and `debian-recipes.tar.gz`.
Verify that second signed manifest **before installing or executing recipes**.
Use trusted locally built `xtask release-verify --verify --sha256sums
SHA256SUMS-packages --sig SHA256SUMS-packages.sig --artifacts-dir .`, with the
release public key compiled into that trusted tool. This authenticates package
metadata/payloads and the exact attached verifier/recipe bytes.
Then `sudo dpkg -i <native.deb>` or `sudo rpm -i <native.rpm>`.
Deb/RPM recipes verify the original binary manifest **at package build time**.
There are no install-time signature-verifying maintainer scripts; native
package installation does not independently reverify that original manifest.
End-user pre-install verification of the signed packaging manifest is required.
The recipes disable stripping/debug rewriting to preserve signed bytes.

## Homebrew and AUR

Use **rendered recipes attached to that release**, never the committed templates,
and authenticate them with the second signed manifest before execution.
Tap/AUR publication is operator-owned and remains unavailable until
`ops-brew-tap-and-aur-publication` provisions and publishes those accounts.
A local Homebrew tap can place `maos.rb` in `Formula/`, then
`brew install --build-from-source <owner>/<tap>/maos`; AUR's rendered `PKGBUILD`
runs with `makepkg -si` as a non-root user. Both verify the signed original
manifest using Python `cryptography` before installing all four exact binaries.
The formula depends on actual `python@3.14` and `cryptography` formulae; the
PKGBUILD needs `python-cryptography`, not a Rust compiler. Homebrew byte
preservation is **claimed, not verified**: targeted `skip_clean` is configured,
but no hosted install/relocation/installed-hash proof has run. The authored
Linux witness compares all four installed binaries; it remains unverified.

## Cargo source route

Cargo builds the selected source tag rather than installing signed binary bytes.
Trust the repository/tag and its locked dependency graph; this is a distinct
source-distribution trust route, not Ed25519 binary verification:

```bash
cargo install --locked --git https://github.com/lunarpulse/maos --tag <tag> maos-bin maos-cli maos-spirit-cli maos-registry
```

The packages are **not** named `maos`. The hosted witness installs `maos-bin`
into a temporary root, checks its workspace version and uninstalls it again.

## Container

The image's verify stage checks the original signed binary. The runtime is
`gcr.io/distroless/cc-debian13:nonroot`, with `maos` as its entrypoint and the
source SHA label. Discipline builds and runs `--version`/`--help` without
registry secrets. Tag builds always run; **all publication and keyless cosign
signing wait for both Docker Hub credentials**. If absent, the job says
`SKIPPED: DOCKERHUB_* absent (ops-provisioning-secrets-and-accounts)` after build.
No published image or signing result is claimed until the operator lane runs.
Publication jobs reference the `release` environment, but a reference does not
prove protection is configured. An administrator must provision required
reviewers and tag/deployment protections per the provisioning checklist first.
After publication, verify its digest with `cosign verify --certificate-identity
<release-workflow-identity> --certificate-oidc-issuer https://token.actions.githubusercontent.com
<registry-image>@sha256:<digest>` before running it. Use the real tag workflow
identity, never a wildcard issuer/identity. Do not infer a registry image from a
successful local build.

## Remove data, then binaries

First stop MAOS and inspect `maos purge`; confirm with `maos purge --yes`.
It removes MAOS-owned data and writes a receipt, retaining operator-authored
configuration/signing material and reporting unavailable credential backends.
Then use the installer that owns the binaries:

| Route | Removal |
|---|---|
| Debian | `sudo dpkg -r maos` |
| RPM | `sudo rpm -e maos` |
| AUR / Arch | `sudo pacman -R maos` |
| Homebrew | `brew uninstall maos` |
| Cargo | `cargo uninstall maos-bin maos-cli maos-spirit-cli maos-registry` |
| Direct binary | remove the four files from the prefix you chose |
| Container | remove the image/container and explicitly manage its MAOS data volume |

`maos purge` never removes package-owned executables. See
[air-gap deployment](../runbooks/ag-1-air-gap-deployment.md) for the shipped-build
network-isolation model and the distinction from the compile-shape-only build.
