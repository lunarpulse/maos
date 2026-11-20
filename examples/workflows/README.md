# Business workflows

`maos workflow` runs a business-authored graph without recompiling MAOS. The single-host runtime admits catalog-backed Spirits through the standard MAOS gates and invokes their lifecycle hooks. It supports deterministic rules and governed inference from a strict local cassette. It does not call a live model, launch remote agents, or perform external business actions. Run it as a local operator command, not through the long-running daemon's HTTP door.

## Try the invoice workflow

From the repository root, with a network-feature build (`cargo build -p maos-bin`):

```sh
(
set -eu
export HOME="$(mktemp -d)"
export MAOS_HOME="$HOME/maos"
export XDG_CONFIG_HOME="$HOME/config" XDG_DATA_HOME="$HOME/data"
export XDG_STATE_HOME="$HOME/state" XDG_CACHE_HOME="$HOME/cache"
mkdir "$MAOS_HOME"
cp examples/workflows/catalog.toml "$MAOS_HOME/workflow-catalog.toml"
maos=target/debug/maos
examples=examples/workflows

"$maos" workflow plan "$examples/invoice-baseline.toml" "$MAOS_HOME/plan-a.json"
# Review the printed steps, packages, and digest, then paste the digest.
printf 'Approved digest for A: '; read -r digest_a
"$maos" workflow apply "$MAOS_HOME/plan-a.json" "$digest_a"
"$maos" workflow run invoice-review "$examples/invoice-input.json"

"$maos" workflow plan "$examples/invoice-revised.toml" "$MAOS_HOME/plan-b.json"
printf 'Approved digest for B: '; read -r digest_b
"$maos" workflow apply "$MAOS_HOME/plan-b.json" "$digest_b"
"$maos" workflow run invoice-review "$examples/invoice-input.json"
"$maos" workflow eval invoice-review "$examples/invoice-revised-cases.json"
)
```

The baseline sends the supplied invoice evidence to the reviewer. The revision routes a duplicate flag to `held` without invoking review. The duplicate flag is **supplied evidence**; the example does not consult an invoice database or recommend using the baseline for real approvals. `book-return.toml` and its inputs exercise the same runtime with a different domain schema.

The evidence workflow instead starts with a document excerpt. Its specialist
produces a typed finding and a verbatim quote from the excerpt. A second Spirit
reads the finding and chooses an outcome; the approved TOML decides where that
outcome goes. A real quote proves the evidence was present in the input, not
that the model interpreted it correctly. Use synthetic excerpts here: the
existing inference audit path can log the prompt, including the excerpt.

### Run the governed evidence handoff

From the repository root, point `maos` at your built executable if you used a
separate Cargo target directory:

```sh
(
set -eu
export HOME="$(mktemp -d)" MAOS_SECRETS_BACKEND=env
export MAOS_HOME="$HOME/maos"
export XDG_CONFIG_HOME="$HOME/config" XDG_DATA_HOME="$HOME/data"
export XDG_STATE_HOME="$HOME/state" XDG_CACHE_HOME="$HOME/cache"
mkdir "$MAOS_HOME"
maos="${MAOS_BIN:-target/debug/maos}"
examples=examples/workflows
cp "$examples/catalog.toml" "$MAOS_HOME/workflow-catalog.toml"
export MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1
export MAOS_REPLAY_CASSETTE="$PWD/$examples/invoice-evidence-missing.cassette.json"
"$maos" workflow plan "$examples/invoice-evidence.toml" "$MAOS_HOME/evidence.json"
printf 'Approved evidence plan digest: '; read -r digest
"$maos" workflow apply "$MAOS_HOME/evidence.json" "$digest"
"$maos" workflow run invoice-evidence "$examples/invoice-evidence-missing.json"
)
```

The expected outcome is `request_order`. Plan and apply
[`invoice-evidence-reroute.toml`](invoice-evidence-reroute.toml) to change that
outcome with the same excerpt and cassette. The cassettes are synthetic,
keyless test inputs, not evidence of live model quality.

The pilot uses synthetic data and local, read-only Spirits. Its KLOC grant is
recorded in [`xtask/kloc.toml`](../../xtask/kloc.toml); it changes neither the
kernel nor the Spirit ABI.

## Compare a candidate before applying it

To try the reroute without applying it, insert this block before the closing
`)` in the previous example. The comparison uses the same isolated MAOS home
with `invoice-evidence` still active:

```sh
"$maos" workflow plan "$examples/invoice-evidence-reroute.toml" "$MAOS_HOME/candidate.json"
printf 'Candidate digest: '; read -r candidate_digest
jq '.entries += .entries | .entries[1].sequence = 1' \
  "$examples/invoice-evidence-missing.cassette.json" > "$MAOS_HOME/compare.cassette.json"
MAOS_REPLAY_CASSETTE="$MAOS_HOME/compare.cassette.json" "$maos" workflow compare \
  "$MAOS_HOME/candidate.json" "$candidate_digest" \
  "$examples/invoice-evidence-compare-cases.json"
```

The [labeled cases](invoice-evidence-compare-cases.json) set the target
business outcome and require at least one `step_artifacts` entry per case.
An entry names a producing step and the exact typed values of the fields to
grade. A skipped step, missing field, or different value fails the label.
For the example, the baseline does not meet the new target outcome
(`request_order`), while the candidate does (`hold_for_procurement`). A
candidate that reaches the right outcome with the wrong quote still fails.

The active revision must match the candidate's parent. Both plans must
remain authorized by the current catalog; if testing a new package version,
keep the baseline entry and register the candidate under a separate key.
The command runs all cases in file order against the baseline first and then
against the candidate. A replay cassette must follow that execution order.
The report identifies both plan digests, the case file hash, the execution
mode and the cassette hash when applicable. It reports mismatched field
names, not the input, generated evidence, or gold values. Any candidate
failure or baseline execution error makes the command exit nonzero. A
baseline that executes but misses a business label does not block an
improved candidate. Comparison never changes the active revision and is not
an approval; `apply` remains a separate step.

## Author and operate

- **Business author:** Choose a stable workflow ID, name a starting step, select catalog agents, and write a destination for each named outcome: `{ step = "next-role" }` or `{ finish = "business_result" }`. Steps run by declared routes, not declaration order. Cycles, unreachable steps, missing routes, and mismatched schemas are rejected. Each step selects one successor or ends the run; there are no parallel joins.
- **Operator:** Own `$MAOS_HOME/workflow-catalog.toml` and the MAOS home. Use a distinct catalog `environment` for each deployment target. Only catalog-listed agents can be used. An agent declares a semantic package version and exact `name@revision` input/output schemas. Fields are bounded `string`, `bool`, or `i64`. `emit` and `equals` can only project existing input fields. `infer` pins a provider, model, prompt, source field, citation field, and outcome; it can produce new fields but can only run through the governed port with `MAOS_INFERENCE_MODE=replay` and `MAOS_REPLAY_STRICT=1`. The cassette must contain the exact prompt hash and length; wildcard hashes are rejected.
- **Reviewer:** `plan` writes a new plan file and reports the resolved graph, pinned packages, deployment target, and delta against the active revision. `apply` requires its digest, checks present-day catalog authorization, stages admissions without dispatching business work, then changes the active revision. A stale active revision must be replanned. A previous source topology can be replanned and approved to restore its routing; there is no external work to undo. `run` and `eval` bind the active revision when they start and recheck current catalog authorization.
- **Evaluator:** A JSON case specifies an input and expected terminal outcome, optionally the exact ordered `path` and the corresponding `step_outcomes`. `eval` uses the live executor and exits nonzero on disagreement. Receipts report the producing role and package version, input/output schema and digest, observed outcome, selected route, and elapsed time. The Transparency Log gets `workflow:*` activation, dispatch, handoff, and completion events without raw input artifacts. Runs return their final artifact to the caller; treat it as business data.

The persisted deployment database contains plans, not run inputs. `apply` uses the local operator's filesystem authority; its digest argument is an explicit content check, not a signature or a separate business approval for an effect.

To inspect a trace, run `maos audit query --intent-contains workflow: --format ndjson`
against the same home. Audit rows encode SHA-256 fingerprints as `sha256:` followed
by four dot-separated hex groups; the CLI run report uses plain hex. Compare
fingerprints after removing the prefix and dots.
