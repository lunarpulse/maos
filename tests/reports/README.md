# Bench Results

## Retention Policy (per arch §13.1)

- Daily results: live 90 days hot in `tests/reports/section-13-1-<sha>.json` (one per bench-run).
- Weekly summaries: `tests/reports/weekly/{year}-W{wk}.json` retained 1 year.
- Tagged-release benchmarks: `tests/reports/release/{semver}.json` retained indefinitely under git LFS.
- Smoke-mode reports: `tests/reports/section-13-1-smoke.json` (single file, overwritten each run; for observability only — NOT a measurement record).

## Pruning Automation

Pruning runs in CI on the 1st of each month (per arch §13.1). The prune job opens a PR (not a force-merge) so an operator can audit what's leaving hot storage.

**v0.5-α status:** Pruning automation is NOT YET WIRED. The directory is appendable until Story 9.4 (operator-surface productionization) ships the prune `xtask`.

## Trend Dashboards

Grafana reads weekly summaries for >90-day windows; daily JSON for <90-day windows. Dashboard wiring lands at Story 9.4.
