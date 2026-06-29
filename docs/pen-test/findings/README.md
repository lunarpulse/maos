# Pen-Test Findings

This directory will contain pen-test engagement results.

- `summary-schema.toml` — Schema definition for the findings summary
- `summary.toml` — (committed after engagement) Aggregated finding counts for the CI gate
- Individual finding writeups will be added as separate files

The `check-pentest-gate` CI job validates `summary.toml` against this schema.
When `summary.toml` is absent, the gate passes with an advisory annotation.
