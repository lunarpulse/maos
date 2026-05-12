# maos-corpus-gen

MAOS parameterized corpus generators — deterministic secret-redaction and red-team
corpus frameworks.  Part of the MAOS quality substrate (Epic 0).

## Generators

- **secret-redaction** — 10⁴-item per-commit secret-leakage corpus from 200 SHA-pinned
  seed patterns across 11 named secret classes.
- **red-team** — ≥640-item adversarial-Spirit red-team corpus from 80 canonical
  scenarios across 8 §8.1 attack classes.

## Usage

```sh
# Generate a corpus
cargo run -p maos-corpus-gen -- generate --corpus secret-redaction-1e4 \
  --mode per-commit --out tests/corpora/secret-redaction-1e4.jsonl

# Run coverage report (text)
cargo run -p maos-corpus-gen -- coverage --corpus secret-redaction-1e4

# Run coverage report (JSON)
cargo run -p maos-corpus-gen -- coverage --corpus secret-redaction-1e4 --json
```

## Regenerate-seed workflow

If you edit a seed TOML file:

1. Edit `seeds/<name>.toml`
2. Compute new SHA-256: `sha256sum seeds/<name>.toml`
3. Update `SEED_FILE_SHA256` constant in `src/<generator>/mod.rs`
4. Update the SHA in `build.rs`
5. Regenerate JSONL: `cargo run -p maos-corpus-gen -- generate --corpus <name> --mode per-commit --out tests/corpora/<name>.jsonl`
6. Register new SHA: `cargo run -p xtask -- check-corpus --register <name>`
7. Paste the printed TOML snippet into `tests/corpora/MANIFEST.toml`
8. Update `EXPECTED_SHA_*` constant in `tests/determinism_integration.rs`
9. Verify: `cargo test -p maos-corpus-gen`

## Determinism contract

All generators produce **byte-identical** output on every host, every run, given the
same `(seed_sha256, rule_version)`.  No RNG, no system clock, no env reads, no PID
reads.  The `build.rs` enforces seed-file SHA at compile time; a mismatch is a build
failure.
