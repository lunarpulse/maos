---
title: Third-Party WASM Component Spirit
sidebar_position: 2.5
description: Build a third-party TypeScript Spirit as a WASM component with the T2 sandbox boundary.
---

# Third-Party WASM Component Spirit

## Problem

You want to author a third-party Spirit in TypeScript without placing third-party code in the MAOS process. The supported form is a self-contained WASM component built from the TypeScript scaffold and declared with the T2 sandbox boundary.

## Solution

Scaffold a project from `templates/spirit-ts`, then install its public npm dependencies and build the component:

```bash
cargo generate --git https://github.com/lunarpulse/maos \
  templates/spirit-ts --name my-wasm-spirit
cd my-wasm-spirit
npm install
npm run build && npm run componentize
```

The scaffold directly pins `@bytecodealliance/componentize-js` at `0.23.0`; its supported Node range is `^22.20 || ^24.12 || >=25`. `componentize-js` writes the component to `dist/spirit.wasm`.

Declare the component with this complete schema v5 manifest:

```toml
[class]
name = "my-wasm-spirit"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 5
min_substrate_version = "0.1.0"
forms = ["wasm-component"]
trust_tier = "public-untrusted"
description = "A third-party TypeScript WASM component Spirit."
artifact = "dist/spirit.wasm"

[author]
name = "you"

[sandbox]
tier = "T2"

[resources]
cpu_max_pct = 25
memory_max_mb = 64
fd_max = 64
pids_max = 64

[capabilities.required.iac]
send = ["spirit:peer"]

[capabilities.required.log]
recall = true
fetch = true

[posture]
default = "assistive"
allowed_max = "assistive"

[output_shape]
required_fields = ["response"]

[budget]
context_window_size = 4096
time_cap_seconds = 60
```

Declare `[capabilities.required.iac]` for bus emission: the kernel mediates each delivered frame with an `iac.send` capability token (`"spirit:peer"` for frames addressed to Spirits, `"broadcast"` for frames with no recipient), so the Spirit's deliveries appear in `maosctl audit query --spirit`. The schema v5 `[capabilities.required.log]` table independently grants recall and fetch; both booleans default to `false`, and unknown keys are refused. Below schema 5, the table is dropped with a degradation warning, like `[capabilities.required.iac]`. Do not add `[capabilities.required.provider]`: the WASM world imports no provider capability. Inside `[class]`, trust tiers use hyphenated values such as `org-internal`, `public-untrusted`, and `public-vetted`.

Schema 7 adds the [FR8 declarations](/manifest/v7). `explanation_shape` is
declared and journaled, **not enforced** because no decision frame has an
explanation body. `swap_invariants` is enforced at hot swap, migrator and cold
swap: a successor that does not declare every invariant its predecessor
declared is refused (`409 swap_invariant_dropped`) before the swap or unload,
and the predecessor keeps serving. The check is structural and does not prove
that state survived.

The additive `maos:spirit/skills@2.4.0` import offers `author(document)` and
`propose(target-id, target-version, diff)` only inside `handle-frame`.
`capability-denied` means authority was refused; `refused(code)` is a stable
port refusal; `unavailable` covers calls outside the export or absent host
service. The host binds pid, class and telemetry; guests supply none of them.
No skill-content import exists and admission remains operator-only.

## Recall frames you participated in

The `maos:spirit@2.4.0` world imports `maos:spirit/recall@2.4.0`, which provides two functions:

```wit
recall: func(filter: recall-filter) -> result<recall-page, recall-error>;
fetch: func(frame-id: list<u8>) -> result<fetch-response, recall-error>;
```

`recall` returns metadata, not payloads. Its filter accepts an optional kind, time bounds, cursor, and exact intent, plus a `limit` from 0 through 1024. Kind strings use the exact `FrameKindLabel` spelling, such as `TaskAssign`. Pass the returned opaque `next-cursor` to resume; a page may be shorter than the requested limit to fit the reply budget. `fetch` takes a 16-byte frame id and returns the stored redacted payload without truncation, metadata, and frame origin.

Both functions are available **only inside `handle-frame`** (`handleFrame` in TypeScript) and, for a component that exports `lifecycle`, inside `idle`, `consolidate`, `telemetry` and `schedule` (below). Calls from `on-start`, `on-shutdown`, `pause`, `resume`, `snapshot`, `swap-in`, `migrate` or elsewhere return `unavailable`. Recall uses the runner's existing stdio channel; it adds no descriptor, socket, or syscall.

Local scope is **participant-only within this daemon run**: a Spirit can recall disclosable bus frames (raw kinds 0–6) and MCP invocations (kind 18) that it sent **or for which it is a recorded recipient**. Routed delivery records only admitted local addressees (`host_id = None`) accepted by the consent gate, if installed, whose Spirit id resolves through the admitted-incarnation map. Resolution is snapshotted before journaling or enqueueing; a role never confers participation. Audit rows, non-participant rows, and earlier daemon runs are omitted; fetching them returns `scope-violation`. Earlier incarnations are not recallable by design; use memory tiers and distillates across restarts. Native and WASM Spirits use the same kernel-mediated `MediatedLogRecall`. Cross-wall recall stays emitter-scoped, not participant-scoped.

Consent constrains a recorded recipient, not the sender. An expired envelope, a matching later grant-scoped rupture, or a `Withheld` policy omits the frame from `recall`; `fetch` returns `consent-withheld`. A `HeaderOnly` frame remains listed with `payload-available = false`, but fetching it also returns `consent-withheld`, never its payload. The daemon-side `ConsentEnvelope.recall` policy defaults to `Full`; unknown values fail closed as `Withheld`. It is **not in WIT**: guests see only its effect, and guest-asserted envelopes confer no authority. This deliberate exception to lossless frame transport is ratified as `17-8-consent-recall-policy`.

**Production limits (Story 17-8, AC3.6 / rule 11):** at HEAD no production granter sets `HeaderOnly` or `Withheld`, and no `ConsentGate` is installed in production. Envelope-bearing frames reach admitted local recipients only through A2A delegation intake; peer-identity ruptures carry no envelope and withhold nothing. Native compiled-in Spirits' bus sends journal at pid 0; attribution is a non-goal of this story. Consent cases are exercised by suites delivering frames through `deliver_typed`, not claimed as active production policy.

### Errors and audit trail

| `recall-error` case | When it occurs |
|---|---|
| `scope-violation` | Fetch names an existing row for a non-participant, another daemon run, or a non-disclosable kind such as an audit row. |
| `frame-not-found` | No row exists for the requested frame id. |
| `invalid-request(string)` | Malformed cursor or id (ids must be 16 bytes), unknown kind label, or timestamp above `i64::MAX`. A valid but non-disclosable kind filter returns an empty page instead. |
| `limit-exceeded(u32)` | Requested limit exceeds 1024; the error carries the maximum. |
| `capability-denied` | The grant is absent, or per-call issuance is refused by policy, operator denial, exhausted self-reported context, or the inclusive 60,000-issuance/60 s monotonic tumbling quota for this admitted scheduler pid/incarnation. Recall and fetch require separate grants; context exhaustion clears on a report at or below the window, while issuance exhaustion can recover next window. |
| `consent-withheld` | A recipient's fetch is blocked by expiry, a matching grant-scoped rupture, `Withheld` (including unknown policy), or `HeaderOnly`. |
| `reply-too-large(u64)` | Even one page entry or the complete fetched payload cannot fit the frame-byte and codec-expansion budgets; the error carries the reply byte bound. Payloads are never truncated. |
| `unavailable` | Called outside `handle-frame`, storage is unavailable, or the recall channel has closed or failed. Internal storage messages are not exposed. |

Issuance follows **policy → context → quota → tokens**; policy and context
refusals spend no issuance quota. Failed signing refunds, but the per-call token
revoke does not. There is no Spirit-facing issuance-quota pre-warning.
Operator-only kind-7 `cap.quota:pressure` / `cap.quota:limit` /
`cap.quota:exhausted` rows are never Spirit-recallable and have qualified,
non-blocking delivery, not guaranteed completeness. See the
[issuance-quota reference](/manifest/v5#capability-issuance-quota) for thresholds,
path-specific errors, delivery semantics and labelled limits (per-incarnation
scope, fresh Worker capacity, non-configurable budget, string-only callers and
audit-only schedule tokens). These are not ADR-016 context-token signals:
those arrive as kernel `context-pressure` / `context-limit` frames through
`handle-frame` after the guest reports its own count with
`maos:spirit/context@2.4.0` `report(used-tokens)`; the daemon binds the pid and
the admitted window (`18-5-context-token-budget-accounting`).

The 2.4 runner retains bindings for existing 2.0–2.3 components; the local
compatibility witnesses exercise 2.0 and frozen 2.2 and 2.3 builds, while a
genuine 2.1 guest remains unverified. It selects their original export bindings
before instantiation and shares the same frame pump. A lifecycle request to a
guest without the `lifecycle` export is answered `unsupported` without calling
any export. A context kind or payload cannot be lowered into the closed 2.0/2.1
schema, including a context kind nested in
`retract.original-kind` or `consent-rupture.original-kind`; the runner refuses
it with the typed unrepresentable-frame exit (5), rather than renumbering it
or dropping the signal. Use a 2.2 or newer component to receive the new context frames.

The daemon binds the caller's admitted identity and boot nonce; the guest supplies neither identity nor token. Each validated, admitted call receives a freshly minted capability, writes a token-bearing invocation audit row before data is disclosed, and revokes the token afterward. Every refusal writes exactly one governance receipt, `log.recall.refused:<code>` or `log.fetch.refused:<code>`, where `<code>` is the WIT error case name. Validation and mint refusals write a receipt without an invocation row. Successful calls, including empty pages, add no refusal receipt. These audit rows are not themselves recallable.

A consent refusal writes exactly one `log.fetch.refused:consent-withheld` receipt at the caller pid, without an owner pid. Omitting withheld rows from a successful recall page is not a refusal and adds no refusal receipt.

An in-band protocol violation writes `recall.channel.closed:protocol-violation` and ends the session. An invalid reply makes the runner return `unavailable`, stop further recall I/O, finish the current export, and exit with a typed `recall protocol desync` error; EOF instead ends normally after the export.

## Lifecycle triggers and hot swap (`maos:spirit@2.4.0`)

The `spirit-lifecycle` world adds one optional export, `maos:spirit/lifecycle`, to the frame exports above. Implement it to receive the kernel's budgeted triggers (`idle`, `consolidate`, `telemetry`, `schedule`, `pause`, `resume`) and to be swappable in place: `maosctl spirit upgrade <name> --to <manifest>` keeps the pid, the capability tokens the pid holds and its working memory, and hands the state your `snapshot` returns to the successor's `swap-in` (`migrate` first under `--policy migrator`). A guest built against world `spirit`, or any 2.0–2.3 component, runs unchanged and is never called; it is then refused a swap as `spawned_swap_unsupported` before anything is replaced. The generated scaffold exports `lifecycle` (`idle`, `snapshot`, `swap-in`) so a published version can be upgraded the same way.

Every lifecycle export returns `result<_, lifecycle-error>` and emits no frames. `idle`, `consolidate`, `telemetry` and `schedule` may use the `recall`, `context` and `skills` imports; `pause`, `resume`, `snapshot`, `swap-in` and `migrate` may not. Each runs under `[budget].time_cap_seconds`. See [Lifecycle Hooks](./lifecycle-hooks#spawned-wasm-component-spirits) for the trigger table, the typed swap refusals, cold swap and the post-swap revert behaviour.

## Discussion

The manifest validates every section before its form-specific result. A published/default MAOS build refuses this form as `wasm_engine_off` while Hold 2 keeps the `wasm-host` engine disabled. A build made with `--features wasm-host` launches the component through `maos run` inside the contained T2 runner (`maos-wasm-runner`, installed next to `maos`). In-process surfaces the contained runner does not provide — `maosctl load` of a component and topology — are refused as `spawned_surface_unsupported`; a component is swapped, upgraded and cold-swapped through `maosctl spirit upgrade` (below). These are intentional typed refusals, not an unknown class or unknown form.

The generated TypeScript guest implements the world's `onStart`, `handleFrame`, and `onShutdown` exports. To halt from guest code, throw the value `throw { tag: "fault", val: "<reason>" }`; the host maps that shape to `Halt::Fault`. Guest WASI stdout/stderr (including `console.log`) is routed to the runner's stderr and journaled as `spirit.diagnostic` rows. Treat them as untrusted guest text: they are bounded per session (256 rows / 64 KiB, overflow counted in `dropped_diagnostics`), over-long lines are truncated with a marker, and they never become frames.

Use [Hello-World Spirit](./hello-world-spirit) for in-tree first-party Rust authoring. Its `rust-inproc` form and T0 tier are deliberately different from this third-party component route.
