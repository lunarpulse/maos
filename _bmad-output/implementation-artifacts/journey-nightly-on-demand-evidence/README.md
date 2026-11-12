# journey-nightly: paid re-record is on-demand only (evidence)

Operator ruling 2026-11-12: "I do not want to spend tokens like that ... not every
day. it could be weekly or fortnightly, or on demand manual tests". `MAOS_ANTHROPIC_API_KEY`
was provisioned 2026-11-12T14:40Z, so `tier-2-rerecord` had to stop being reachable
from the `0 3 * * *` cron.

## Shape (`.github/workflows/journey-nightly.yml`)

- `workflow_dispatch.inputs`: `rerecord` and `include_researcher`, both `boolean`, default `false`.
- `secret-gate`: `if: github.event_name == 'workflow_dispatch' && inputs.rerecord == true` (the cron never even reads the secret).
- `tier-2-rerecord`: `if: github.event_name == 'workflow_dispatch' && inputs.rerecord == true && needs.secret-gate.outputs.has_anthropic_key == 'true'`.
- Researcher + Sonnet judge step: `if: ${{ !cancelled() && inputs.include_researcher == true }}`.
- `tier-2-journey` and `j6-real-measurement`: unchanged, still on the daily cron.

## Commands and results

| Command | Result |
|---|---|
| `cargo nextest run -p xtask --locked --test ci_shape_18_0` | 22 passed, 0 failed (includes new `journey_nightly_paid_rerecord_is_on_demand_only`) |
| `cargo nextest run -p xtask --locked --test decision_adrs_and_provisioning --test rc_readiness_20_5` | 15 passed (checklist row now `present`) |
| `rustfmt --check --edition 2021 xtask/tests/ci_shape_18_0.rs` | clean |
| `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=$PWD/xtask /mnt/build/cargo-maos/debug/xtask check-env-contract --json` | exit 0, `{"passed":true,"registered_count":124,"violations":[]}` |
| YAML parse, throwaway venv `pyyaml` (`actionlint` not installed) | parses; `on` has `schedule` + `workflow_dispatch.inputs{rerecord,include_researcher}`; jobs `tier-2-journey, secret-gate, tier-2-rerecord, j6-real-measurement` |

## Mutation checks (real workflow temporarily edited, then restored; `cmp` identical to the pre-edit copy)

| Mutation | Result |
|---|---|
| A: remove `github.event_name == 'workflow_dispatch' && inputs.rerecord == true &&` from `tier-2-rerecord` `if:` | `journey_nightly_paid_rerecord_is_on_demand_only` FAILED (1 failed / 22) |
| B: drop `&& inputs.include_researcher == true` from the Researcher step | `journey_nightly_paid_rerecord_is_on_demand_only` and `epic_18_exit_runs_line_2_verbatim_and_its_drift_blind_twin` FAILED (2 failed / 22) |

The new test also carries eight in-memory plants, each of which must produce a violation: the
`tier-2-rerecord` guard reduced to the secret check; the guard replaced by a `schedule` path; the
`secret-gate` guard removed; the researcher gate removed; the `include_researcher` input removed;
`rerecord` default flipped to true; a second cron added; `j6-real-measurement` gated off.
