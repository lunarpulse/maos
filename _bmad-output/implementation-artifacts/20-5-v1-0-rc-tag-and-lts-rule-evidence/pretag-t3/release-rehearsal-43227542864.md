# Early `release.yml` rehearsal (tag-procedure step 6b shape), 2026-11-12

- Dispatched by the orchestrator: `gh workflow run release.yml --repo lunarpulse/maos --ref main`.
- Run 43227542864, event `workflow_dispatch`, head `23b2200b` (the remote `main` tip at dispatch; first commit carrying workspace version `1.0.0-rc.1`).
- Result (read with `gh run view 43227542864 --repo lunarpulse/maos --json status,conclusion,jobs`): `completed` / `success`.

| Job | Conclusion |
|---|---|
| `build (x86_64-unknown-linux-gnu, ubuntu-latest, linux-amd64)` | success |
| `build (aarch64-unknown-linux-gnu, ubuntu-latest, linux-arm64)` | success |
| `build (aarch64-apple-darwin, macos-latest, darwin-arm64)` | success |
| `sign-and-publish` | skipped (guarded on `push`; a rehearsal never publishes) |

Scope: this is the first `release.yml` build on any event that compiled the three targets (earlier runs were label-gated
`pull_request` runs that skipped `build`). It is NOT the candidate-SHA rehearsal that operator ruling 13 condition (e)
requires; that one is re-run at the final candidate SHA. Signing, packaging publication and `release.yml`'s rc-only
steps (notes guard, perf reconciliation, readiness) did not run here (they live in `sign-and-publish`).
