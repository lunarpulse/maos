# LTS policy — what NFR-Maint-6 promises, and when the clock starts

> Source requirement: **NFR-Maint-6** (`_bmad-output/planning-artifacts/prd/non-functional-requirements.md`):
> *1-year LTS commitment at v1.0; 2-year LTS commitment at v1.5 once support load is
> known; security-only patches after year 1. Don't write a check the v0.8 team can't cash.*
> The published, generated statement is the **LTS Policy** section of
> [`STABILITY.md`](../../STABILITY.md) (`cargo run --locked -p xtask -- stability-matrix --check`
> keeps it byte-identical to this rule). This page is the procedure behind it.

## The rule

| Tag | LTS commitment | Clock |
|---|---|---|
| `v0.1.0-alpha.N`, any other pre-release | none | never starts |
| `v1.0.0-rc.N` (release candidate) | **none** — a release candidate is evidence, not a promise | **not started** |
| `v1.0.0` (GA) | **1-year LTS** | starts at the `v1.0.0` tag commit |
| after year 1 of the `v1.0.0` clock | **security-only patches** | — |
| v1.5 | a 2-year commitment is a v1.5 decision, taken once support load is known | — |

"**Security-only patches after year 1**" means: once the first year from the
`v1.0.0` clock-start has elapsed, the v1.0 line receives fixes for security
vulnerabilities only (no feature, performance or compatibility backports). **Where
that period ends is not set here**: the 2-year term and any end-of-support date are
the v1.5 decision named above; until the operator rules it, the line has no stated
end date and nothing in this repository claims one.

What the commitment is **not**: a check. NFR-Maint-6 is a published rule, not a
gate; no CI job can enforce a support promise. The only things this repository
verifies are that the published text equals this rule (`stability-matrix --check`)
and that the clock-start field is a real tag or an honest placeholder.

## When the clock starts

Only `v1.0.0`, and only after both RELEASE-HOLDS holds clear
([`RELEASE-HOLDS.md`](../../RELEASE-HOLDS.md): Hold 1 external pen-test, Hold 2 export
classification). Cutting it is `ops-ga-tag-after-holds`, an operator row. A release
candidate never starts it: `STABILITY.md` renders the release channel
(`kernel_version`'s pre-release suffix — `-rc.N` is "release candidate — pre-release;
LTS clock not started", no suffix is GA) so a reader cannot mistake an rc for the
commitment.

## How the clock record is written at GA

The clock-start is a **committed record**, never a `git` lookup at render time (a
`git archive` copy or a shallow clone would otherwise render a different
`STABILITY.md` than the tagged commit, and `--check` could not be byte-for-byte for
both). Until GA the record is **absent** and `STABILITY.md` renders the placeholder.

`ops-ga-tag-after-holds`, in a commit **after** the `v1.0.0` tag exists (the record
names the tag commit, so it cannot be inside it), writes `xtask/release-channel.toml`:

```toml
[lts]
clock_start_tag = "v1.0.0"
clock_start_sha = "<40-hex SHA of the v1.0.0 tag commit>"
```

then regenerates and verifies:

```bash
cargo run --locked -p xtask -- stability-matrix
cargo run --locked -p xtask -- stability-matrix --check
```

A malformed record (a non-hex SHA, a missing key) is a loud `--check` error — never
the placeholder. The record is written by the operator lane; an agent never invents a
SHA or a tag, and no part of this repository writes the file before the tag exists.

## Operator-lane status (unverified here)

The tag, its signature, the T3 fuzz chain and the GA decision are operator-owned
(`docs/release/tag-procedure.md`). This page and the generator describe the rule;
they do not start, assert or schedule any commitment.
