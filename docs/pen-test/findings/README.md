# Pen-Test Findings

This directory will contain pen-test engagement results.

- `summary-schema.toml` — Schema definition for the findings summary
- `summary.toml` — (committed after engagement) Aggregated finding counts for the CI gate
- Individual finding writeups will be added as separate files

The `check-pentest-gate` CI job validates `summary.toml` against this schema.
When `summary.toml` is absent or unreadable, the gate reports `evidence_state: "ABSENT"`,
`passed: false`, and `advisory: true`; it exits 0 at `v1_5` because the registry
disposition is blocking-when-present. This is not evidence of a successful engagement.
