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

## Measure a candidate with a live model (explicit, bounded)

Live workflow inference is available only for `workflow compare`. Set
`MAOS_INFERENCE_MODE=live` on
`maos workflow compare <candidate-plan.json> <candidate-digest> <cases.json> [--repeat <k>]`.
No cassette is used. `run`, `eval`, and `apply` refuse live mode. Both plans must
still be authorized by the operator's current catalog, and the candidate's
parent must match the active revision. Comparison never changes the deployment;
applying a candidate is a separate operation.

Register the provider explicitly, for example with `MAOS_OLLAMA_URL` for
Ollama or a vendor key through the [normal secret backend](../../docs/maos.dev/run-maos.md).
The catalog must pin the exact provider and model for each inference agent.
An unpinned or unregistered provider is refused; there is no fallback to
another provider or model. Live calls use the same governed inference port
with scoped capability tokens, policy checks, and cost attribution checked
against those pins.

`--repeat` takes an integer from 1 to 20 and defaults to 1. Values above 1
are refused outside live mode before any work. The comparison runs each case
`k` times in sequence for the active baseline, then does the same for the
candidate, in case-file order. Live mode requires the positive integer
`MAOS_WORKFLOW_MAX_INFERENCE_CALLS`. Before running, MAOS checks
`case count × k × 2 × max(baseline inference steps, candidate inference steps)`,
counting steps that use an inference agent regardless of routing. A missing,
invalid, or too-low cap is refused before any inference, and a runtime counter
prevents calls beyond the cap.

The example below assumes a running local Ollama server with `llama3.1:8b`
available, a built `maos`, and `jq`. Run it from the repository root; set
`MAOS_OLLAMA_URL` if the server is not on `localhost:11434`. It pins Ollama in an
isolated copy of the catalog, applies only the temporary baseline under strict
replay (admission only, no inference call), then compares the unapplied
candidate in live mode.

The copy also replaces the extractor prompt. The committed catalog prompt asks
for "only JSON" but is part of the replay cassettes' prompt hashes, so it is not
edited in place. `llama3.1:8b` answers it with prose, a code fence, and wrong
keys, and every run then abstains as `malformed evidence JSON`. The stricter
prompt below spells out the exact object. The parser stays strict: a model that
cannot honor the output contract abstains rather than being rescued.

`--repeat 1` is enough here. The workflow hook requests temperature 0, and in
the first measurement below three repeats produced identical tokens. Use
`--repeat` above 1 only with a provider or settings that vary between calls.

```sh
(
set -eu
maos="${MAOS_BIN:-target/debug/maos}"
examples="$PWD/examples/workflows"
export HOME="$(mktemp -d)" MAOS_SECRETS_BACKEND=env
export MAOS_HOME="$HOME/maos"
export XDG_CONFIG_HOME="$HOME/config" XDG_DATA_HOME="$HOME/data"
export XDG_STATE_HOME="$HOME/state" XDG_CACHE_HOME="$HOME/cache"
mkdir "$MAOS_HOME"
printf 'Measurement home (kept for audit): %s\n' "$MAOS_HOME" >&2
unset MAOS_INFERENCE_MODE MAOS_REPLAY_CASSETTE MAOS_REPLAY_STRICT
prompt='Read the invoice excerpt. Reply with exactly one JSON object and nothing else: no prose, no markdown, no code fence. The object must have exactly two keys: "outcome", whose value is the string "finding_ready", and "artifact", an object with "purchase_order_present" (true only if the excerpt states a purchase order was issued or assigned; otherwise false) and "quote" (a verbatim substring copied from the excerpt that supports the answer). Example: {"outcome":"finding_ready","artifact":{"purchase_order_present":false,"quote":"copied text"}}'
awk -v p="$prompt" '
  BEGIN { q = sprintf("%c", 39) }
  /^provider = "anthropic"$/ { $0 = "provider = \"ollama\"" }
  /^model = "claude-haiku-4-5-20251001"$/ { $0 = "model = \"llama3.1:8b\"" }
  /^prompt = "Read the invoice excerpt\./ { $0 = "prompt = " q p q }
  { print }
' "$examples/catalog.toml" > "$MAOS_HOME/workflow-catalog.toml"
baseline=$("$maos" workflow plan "$examples/invoice-evidence.toml" \
  "$HOME/baseline.json" | jq -er 'select(.digest) | .digest')
MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 \
  MAOS_REPLAY_CASSETTE="$examples/invoice-evidence-missing.cassette.json" \
  "$maos" workflow apply "$HOME/baseline.json" "$baseline" >/dev/null
candidate=$("$maos" workflow plan "$examples/invoice-evidence-reroute.toml" \
  "$HOME/candidate.json" | jq -er 'select(.digest) | .digest')
MAOS_INFERENCE_MODE=live MAOS_OLLAMA_URL="${MAOS_OLLAMA_URL:-http://localhost:11434}" \
  MAOS_WORKFLOW_MAX_INFERENCE_CALLS=50 \
  "$maos" workflow compare "$HOME/candidate.json" "$candidate" \
  "$examples/invoice-evidence-holdout-v1.json" > "$HOME/report.json"
jq -e 'select(.event == "workflow_comparison" and .format == 2 and .mode == "live") |
  {repeat, max_inference_calls, cases_sha256, plans}' "$HOME/report.json"
)
```

Live comparison writes one JSON line with `event: "workflow_comparison"`,
`format: 2`, and `mode: "live"`. It identifies `workflow`,
`baseline_revision`, `candidate_revision`, `cases_sha256`, `repeat`, and
`max_inference_calls`. Replay and rules-mode output remain format 1.

- `plans.baseline` and `plans.candidate` each summarize `runs` (case count
  times `repeat`), `ran` (runs that completed), and `passed` (completed runs
  that matched every label). `pass_rate` is `passed / runs`, while `ran_rate`
  is `ran / runs`; both use all scheduled runs as the denominator.
- `passed` combines two checks that are also reported apart. `decision_passed`
  counts runs whose final outcome, path and step outcomes match the case
  labels (the workflow's decision); `decision_rate` is over all runs.
  `evidence_passed` counts runs whose labeled `step_artifacts` fields all
  match, out of `evidence_runs` (runs of cases that label artifacts), with
  `evidence_rate` null when no case labels any. Abstained runs count as misses
  in both. `decision_without_evidence` counts runs that decided as labeled
  while a labeled artifact field did not match, the cases a bare `passed` or
  final outcome would hide.
- `abstained` counts runs that did not complete (`ran: false`), such as a
  provider transport error, malformed JSON, an incomplete or oversized
  response, or an unknown outcome. `failed` counts completed runs that missed
  a label. These are different failure modes.
- Each plan also reports `calls`, `input_tokens`, `output_tokens`,
  `latency_ms_total`, and `latency_ms_max`. Tokens come from provider-reported
  response usage. Latency is wall time around each inference port call, not
  total workflow runtime.
- Each `cases` entry has an `id` and `baseline`/`candidate` summaries with
  `runs`, `ran`, `passed`, `decision_passed`, `evidence_passed`, and ordered
  `details` using the existing per-run verdict detail format. A completed run's
  detail carries `decision` and, for cases with artifact labels, `evidence`
  next to `artifact_checks`, which names the mismatched fields. A case's
  `evidence_passed` of 0 also means "no artifact labels"; use the plan's
  `evidence_runs` to tell them apart. Replay and rules-mode per-run details
  carry the same added keys; their format 1 report is otherwise unchanged.

Live comparison exits zero when it produces the report without a governance
failure, even if some runs abstain or no run passes. Capability or policy denial
(`workflow inference denied`), attribution mismatch, an unregistered pinned
provider, or a call-bound violation aborts the comparison with a nonzero exit
and no report.

The report line and the single `workflow:evaluation` event at the end of a
completed comparison record counts and digests, not prompts, case inputs,
or model text. The report also records evaluation metadata such as case IDs
and scoring details. It uses only these fixed error labels: `provider rejected (status N)`,
`provider transport error`, `inference timeout`, `provider response malformed`,
`rate limited`, `incomplete response`, `malformed evidence JSON`, `unknown outcome`,
and `hook did not complete`. The evaluation event records only revisions,
the case hash, repeat count, calls, and tokens.

Every real call still goes through the standard kernel inference audit path.
Its `infer:` audit rows may contain the prompt text, including the case excerpt.
Use only synthetic or already-shareable data for live runs.

The comparison report does not compute monetary cost. Reconcile costs for
the measurement month against the same MAOS home with
`maosctl audit cost-reconcile --month YYYY-MM`. The default price book
(`xtask/provider-pricing.toml`) has no `ollama/llama3.1:8b` entry. To price
this example, pass `--pricing <file>` with an operator-supplied price book
containing that exact provider/model pair.

Limitations:

- Each inference hook has the dispatcher's default 30-second deadline. A
  slower provider answer becomes an abstention (`hook did not complete`).
- Ollama attributes the response to the model named in the request, ignoring
  a different response `model`. It also treats a missing `done_reason` as
  complete. The Ollama model pin therefore guarantees only the request-side
  model, not which model served the response.
- The committed [12-case holdout](invoice-evidence-holdout-v1.json) is a public
  development benchmark. The repeat count (`k`) and case count (`N`) here are
  for wiring and cost checks, not for setting an operational pass threshold.
  Use a separately kept, unseen set for release-quality estimates. The
  synthetic cassette control below checks scoring and routing only, not
  live model quality.

### First measurement (baseline, not a threshold)

Real runs of the example above, on 2026-11-21 and 2026-11-22: `llama3.1:8b`
through a local Ollama 0.40.17 on CPU, the 12-case holdout. They are a
reference point for later changes, not a quality claim. The first run used
`--repeat 3` and gave `passed` 12 / 36 for the baseline and 18 / 36 for the
candidate; the example's `--repeat 1` run below gives the same rates and was
taken after the decision and evidence fields existed.

| Plan | `ran` | `passed` | `decision_passed` | `evidence_passed` | `decision_without_evidence` |
|---|---|---|---|---|---|
| Baseline | 12 / 12 | 4 | 6 | 6 | 2 |
| Candidate | 12 / 12 | 6 | 12 | 6 | 6 |

- With the unedited catalog prompt, all 72 runs abstained as `malformed evidence
  JSON` (`ran` 0). The tighter prompt, tuned on the three
  [live cases](invoice-evidence-live-cases.json) only, brought `ran` to 36 / 36.
- The extractor's `purchase_order_present` answer was correct for all 12
  cases. The baseline scores low because its missing-order route finishes as
  `request_order`, not the labeled `hold_for_procurement`; that label is
  exactly what the candidate changes.
- The candidate's 6 misses are all `quote`: the model returned a shorter span
  that is verbatim in the excerpt, while the label asks for the full sentence.
  Some shorter spans are enough (`issued PO-711`); one drops the key fact
  (`PO-756 was voided` omits "no replacement"). Reading `passed` alone hides
  this; `decision_passed` and `evidence_passed` separate the two.
- Both plans share the same extractor, so token counts matched exactly
  (5,556 in, 972 out per plan). The run measures whether the candidate's
  routing works with this model's output, not a difference between models.
- 12 of 12 correct decisions bound the true rate only to roughly 74% or higher
  at 95% confidence, and repeats of one case are not independent samples. Use a
  separately kept unseen set before setting any threshold.

### Run the holdout scoring control

The [v1 holdout](invoice-evidence-holdout-v1.json) contains 12 synthetic
excerpts not used in the pilot prompt: six with an issued buyer PO for this
invoice and six without one. Cases cover unrelated references, voided or
draft orders, and multiline fields. The label is about what the excerpt
supports, not whether a PO exists in an external system. `quote` is graded by
exact string equality: a different but valid supporting span can score as
wrong. The set has no abstention label and is too small to establish a
production quality floor.

The [seed cassette](invoice-evidence-holdout-v1.cassette.json) has 12 pinned,
synthetic completions written from the labels. Replay it twice in case
order to check scoring and routing, **not** model quality. From the repo root:

```sh
(
set -eu
maos="${MAOS_BIN:-target/debug/maos}"
examples="$PWD/examples/workflows"
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
export HOME="$scratch"
export MAOS_HOME="$HOME/maos" MAOS_SECRETS_BACKEND=env
export XDG_CONFIG_HOME="$HOME/config" XDG_DATA_HOME="$HOME/data"
export XDG_STATE_HOME="$HOME/state" XDG_CACHE_HOME="$HOME/cache"
mkdir "$MAOS_HOME"
cp "$examples/catalog.toml" "$MAOS_HOME/workflow-catalog.toml"
export MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1
export MAOS_REPLAY_CASSETTE="$HOME/holdout.cassette.json"
jq '.entries += .entries | .entries |= to_entries | .entries |= map(.value + {sequence: .key})' \
  "$examples/invoice-evidence-holdout-v1.cassette.json" > "$MAOS_REPLAY_CASSETTE"
baseline=$("$maos" workflow plan "$examples/invoice-evidence.toml" \
  "$HOME/baseline.json" | jq -er 'select(.digest) | .digest')
"$maos" workflow apply "$HOME/baseline.json" "$baseline" >/dev/null
candidate=$("$maos" workflow plan "$examples/invoice-evidence-reroute.toml" \
  "$HOME/candidate.json" | jq -er 'select(.digest) | .digest')
"$maos" workflow compare "$HOME/candidate.json" "$candidate" \
  "$examples/invoice-evidence-holdout-v1.json" > "$HOME/report.json"
jq -e 'select(.event == "workflow_comparison") |
  select(.passed and .baseline_passed == 6 and .candidate_passed == 12 and .total == 12) |
  {mode, baseline_passed, candidate_passed, total, cases_sha256}' "$HOME/report.json"
)
```

Only the temporary baseline is applied. To measure live quality later,
freeze the corpus hash, exclude these excerpts and labels from prompt tuning,
and collect fresh repeated responses per plan using the
[Spirit evaluation protocol](../../docs/research/spirit-evaluation-protocol.md).
Report extraction, supporting evidence, and routing separately from malformed
responses, latency, and cost. This public synthetic set is a development
benchmark; use a separately kept, unseen set for a release-quality estimate.
`workflow compare` does not currently collect live responses.

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
