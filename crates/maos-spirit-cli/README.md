# maos-spirit-cli

Spirit-author CLI for validating, publishing, vetting and yanking Spirit packages (Story 7.2, FR35).

## 30-Minute Publish Path

```bash
# 1. Scaffold a Spirit (Story 7.1)
cargo generate maos-spirit --lang rust --name my-spirit

# 2. Build the Spirit
cd my-spirit && cargo build --release

# 3. Generate an Ed25519 signing key (one-time setup)
openssl genpkey -algorithm Ed25519 > ~/.config/maos/spirit-signing.key

# 4. Publish to the local registry
maos-spirit publish \
  --tier local \
  --manifest manifest.toml \
  --artifact target/release/my-spirit \
  --registry-uri http://localhost:6789/mcp
```

## Trust Tiers

| Tier | Use case |
|------|----------|
| `local` | Personal / development Spirits |
| `org_internal` | Organization-internal sharing |
| `public_untrusted` | Public sharing (self-attested) |
| `public_vetted` | Vetted promotion via a signed **vetting attestation** (FR37/ADR-056); the tier is a declared aspiration until an attestation promotes it at admission |

## Subcommands

- `publish` — Sign and publish a Spirit package to a registry (v1.0)
- `validate` — Check a package locally, before `publish`: the install gate, the Host's boot-gate manifest checks
  and the component shape (Story 22-7). `maos-spirit validate --manifest manifest.toml --artifact dist/spirit.wasm [--json]`
  exits 0 when the package would pass, 1 with one `[check] code: detail` finding per failed check, 2 on an
  unreadable input. It needs no registry, daemon or signing key (an ephemeral key signs the in-memory package)
  and writes nothing.
- `vet`, `yank` — vetting attestations and withdrawing a release.

## Signing Key Precedence

1. `--signing-key <path>` (explicit file path)
2. `--signing-key-env <VAR>` (env var holding key content)
3. `~/.config/maos/spirit-signing.key` (default fallback)

## Compliance Claims

If `--compliance-claim <path>` is omitted, the CLI auto-populates a self-attested `ComplianceClaimEnvelope` from the manifest fields. For third-party attestation, provide a pre-signed CBOR envelope explicitly.

## Dry Run

```bash
maos-spirit publish --tier local --manifest manifest.toml --artifact my-spirit --dry-run
```

Prints the would-be `SignedPackage` JSON to stdout without dispatching to the registry.
