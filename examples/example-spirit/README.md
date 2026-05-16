# example-spirit

This is the baked output of `cargo generate --path templates/spirit-rust --name example-spirit`.

It is committed as a workspace member so the discipline-suite job
`example-spirit-tests` continuously proves the template produces compiling
code as the SDK + ABI evolve.

## Regeneration

To re-render this directory from the template after the template changes:

```
cargo run -p xtask -- example-spirit-regen
```

To verify in CI that the baked output has not drifted from the template:

```
cargo run -p xtask -- example-spirit-regen --check
```

The `example-spirit-drift` discipline job runs this on every PR.

## Status

This is the Story 2.3 v0.3 NFR-Onb-1 PREREQUISITE proof artifact. It is
NOT the Butler reference Spirit (that ships in Story 8.1) and NOT the
NFR-Onb-1 gate itself (that runs at Story 7.5b). It exists to validate
the template + local runner end-to-end.
