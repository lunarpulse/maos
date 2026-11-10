---
title: Run MAOS
sidebar_position: 2
description: Stand up and operate the MAOS kernel.
---

# Run MAOS

For operators standing up and running the MAOS kernel.

## Build the kernel

```bash
cargo build -p maos-bin --release
```

## Start the daemon

```bash
./target/release/maos init    # first-time setup (creates $MAOS_HOME)
./target/release/maos run     # start the kernel daemon
```

`maos init` is also the one writer of the operator door's `control.json`:
it records the loopback endpoint and bearer token the mutating verbs
authenticate with. Rotation is `rm $MAOS_HOME/control.json && maos init`.

### Crypto provider

`maos init --crypto-provider ring` records `[crypto] provider = "ring"` in
`$MAOS_HOME/config.toml`. A fresh init defaults to `ring`; an older home without
the section also defaults to `ring`. Re-running init with the flag changes only
the crypto selection, effective on the next boot. Unknown or malformed
selections refuse boot with exit 78 and list the providers available in that
build. Each boot journals `crypto.provider.selected` with the provider, version
and whether the selection came from config or the default. Inspect it with
`maosctl audit query --frame-kind TelemetryEvent --intent-contains crypto.provider`.
Trust only rows whose origin is `Kernel` and pid is `0`; a Spirit can emit the
same intent and its rows are not evidence of the selected provider.

v1.0 ships `ring` only: this is a pluggability seam, not FIPS validation. A
FIPS 140-3 validated module is planned for v1.5. Daemon boot re-verifies
installed publisher signatures and ComplianceClaims using the selected
provider. `maosctl spirit install` and `maosctl import` remain ring-direct
install-time pre-checks. An explicit `maos run <installed-manifest>` likewise
uses ring-direct publisher verification and does not evaluate the ComplianceClaim;
it is not covered by the daemon boot re-verification guarantee. Vetting, yank
and registry publish checks also remain ring-direct. Audit-artifact signatures,
publisher tooling and rustls TLS are outside this provider boundary.
Until 20-13 L2, kernel-internal T3 admission still uses `ring`.


### The evaluator shell

```bash
./target/release/maos shell
```

The shell loads the reference Spirit (`hello-spirit`) into the scheduler
under a real pid, then renders a REPL. Directives are typed as
`@hello-spirit <message>`; `Ctrl-D` exits. A deliberately ambiguous
directive — for example `@hello-spirit refactor src/main.rs to be more
idiomatic` — halts the Spirit on `task.acceptance_criterion.ambiguous`:

```
[HALT task.acceptance_criterion.ambiguous] 'more idiomatic' is undefined; …
halt 01M2GJ3GJMJRBB7PWQXPKWJT3K — type a clarification, or:
  maosctl halt resolve 01M2GJ3GJMJRBB7PWQXPKWJT3K --spirit hello-spirit \
    --kind provided-context --text "…"
```

Resolve it on **either surface** — type the clarification directly into the
REPL, or from a second terminal:

```bash
maosctl halt list --spirit hello-spirit          # shows the halt_id
maosctl halt resolve <halt_id> --spirit hello-spirit \
  --kind provided-context --text "idiomatic = clippy-clean, no unwrap"
```

Both surfaces share one mechanism (the operator door): the REPL renders the
resolution the moment the registry flips, and the Spirit **proceeds with
your clarification**. `maos audit query --spirit hello-spirit` then shows
the halt row and the `operator.halt-resolve` completion row — including
whether it completed.

### Inference mode (ADR-064)

The kernel's inference can run from a cassette (hermetic), record into one,
or call a live provider:

```bash
MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=<cassette.json> maos shell
MAOS_INFERENCE_MODE=record MAOS_REPLAY_CASSETTE=<out.json> maos shell
MAOS_INFERENCE_MODE=live    maos shell
```

### Provider keys (today)

Live inference resolves `MAOS_ANTHROPIC_API_KEY` and `MAOS_OPENAI_API_KEY`
through the OS keyring by default. If the bounded keyring attempt is unavailable
or the named entry is absent, MAOS falls back to the composition-root-captured
environment value and records a `secret.source.fallback` audit event without key
material. Set `MAOS_SECRETS_BACKEND=env` for headless CI, or
`MAOS_SECRETS_BACKEND=encrypted-file` with `MAOS_KMS_MASTER_KEY` for the sealed
file vault. Keys are resolved when providers are constructed; just-in-time
materialization and bounded in-memory lifetime remain open.

When the keyring is healthy and the named entry is absent, MAOS promotes the
captured environment credential into the keyring at boot, and later boots
resolve from the keyring; the promotion is journaled without key material. A
later `maos purge` removes keyring entries under MAOS's namespace, including
entries created by this promotion.

The keyring is safer for secrets at rest and keeps them out of process
environment listings. It does not protect a key from another process running as
your user on the same host.

### MCP server bearers

Each live MCP server (`MAOS_MCP_<SERVER>_URI` for calendar, slack, linear,
figma, web, arxiv, github and citation-graph) takes its bearer from the same
secret store, under `MAOS_MCP_<SERVER>_BEARER` (keyring entry
`mcp-<server>-bearer`). The bearer is sent only as `Authorization: Bearer` to
that server; with none stored, no `Authorization` header is sent. The client
speaks MCP 2026-07-28 first and falls back to the `initialize` handshake and
`Mcp-Session-Id` of a 2025-11-25 server. `maos purge` removes the bearer entries
with the provider keys.

### Remove MAOS state

`maos purge` is a dry run unless `--yes` is present. It names each MAOS-owned
root and credential before removal, leaves operator-authored signing and
configuration files intact, refuses while a live daemon or offline durable
operation holds a store, and writes a JSON receipt outside the deleted roots.
After a successful purge, remove the Cargo-owned binary with
`cargo uninstall maos`.

On Linux, confirmed purge first removes this home's T3 containers: both MAOS
identity labels, their matching name, and a `/workspace` mount under this
home's workspace roots must agree. Other-home or unattributable labelled
containers are only reported with a removal command; label-less containers
are ignored. Empty dead-creator `maos.slice/spirit-*` leaves are removed, never
populated or live-creator leaves and never by killing. Private T3 custody
requires a valid CID captured before removal and freshly confirmed absent,
with the same private, owned directory identity rechecked. Podman may delete
its cidfile during removal; a CID missing at inventory never authorizes cleanup.
Empty process roots additionally require a positively absent creator PID.
Live/anonymous process roots, shared state
parents and `maos.slice` are retained; unknown/live spawn custody is retained.
Dry-run spawn-custody lines are conditional on CID absence. Enumeration uses
read-only `ps`/`inspect`, with bounded `container exists` only after failed
inspect: confirmed absence skips a vanished candidate; present/unknown stays
fatal with inspect status/stderr. Receipts add
`podman://<CID>`, `cgroup://<path>` and custody-directory rows, without changing
the schema: `left (<residue>)` can coexist with status `complete`, as with
Worker workspace residue. Read every row. User-namespace mappings and podman
storage/configuration are never edited.

Matching metadata is not unforgeable proof: another container creator under the
same rootless UID can forge the labels, name and mount. Same-UID runtime control
is not a privilege boundary. Bare `podman` is resolved on trusted operator PATH.
Surviving MAOS-written receipt rows produce `complete with residue: N left` in
the existing `next_step` string and final CLI line, with exit 1; receipt status
vocabulary and schema remain unchanged. HOME must be set and absolute; HOME or
ancestor purge roots and `..` paths are refused. Legacy ancestors are checked
without following symlinks; purge never uses the shared HOME-unset `/tmp` fallback.

`maos purge --keep-log --yes` checkpoints and keeps the Transparency Log. This
also keeps shared-memory rows and the principal namespace index when they share
that database, including capability-token rows and legacy `shell.turn` text.
The dry run reports those retained categories with counts before deletion.
When tenant mode is active the whole `teams/` subtree is retained — not only
the resolved team's log — and the dry run and receipt name every sibling team
log with its row counts. This retention is an operator-chosen, documented
exception to FR2's clean-uninstall guarantee; deletion is the default.

### Forget a principal

`maosctl forget --principal <id>` erases the principal's Private namespace and
index, scrubs Transparency Log frames naming the token-bounded id across all
writers, and redacts distillates derived from those frames. Its JSON result
includes scrubbed frame ids, redacted distillate ids, re-distillation requests
with surviving source references, and each request's outcome.

In a running daemon, live compiled-in Spirits receive a forget notice even if
ordinary hook delivery was quarantined. The Researcher clears cached claims
and its last output, rechecks cached ids against the log, and regenerates only
from current surviving log payloads through governed recall. The root validates
and re-scans the result, then performs a bounded tail sweep. Frame completion is
the **durable transactional TL marker**, not handler-return time: no matching
erasure-eligible pre-marker frame body survives. Scope/forget/hold/redaction audit
metadata, IAC lineage arrays and structured numerical facts are retained and
named; rows committed after the marker are post-completion data and are not
claimed erased. A busy, held or unverifiably attributed scope remains explicitly
pending/retained. An owned malformed kind-11 body can be erased only with verified
boot/pid/Spirit attribution; the proof names its real id, and only a final marker
clears its earlier malformed observation. Ambiguous malformed bytes stay retained.
Retained numerical facts remain citable. A partial cost/governance redaction is
not a full-body tombstone and does not cause an unrelated forget to erase its
neutral derivatives.
An HTTP503
`handler_still_running` keeps its unchanged deadline and does not cancel the
in-flight handler; inspect its eventual report or repeat the identical request.
Do not read a pending result as erased or complete. Regeneration-only pending
is separate: unavailable writers can remain pending after frame erasure.
Once the operator's completed receipt is durable, the same decoded principal and
optional reason return its stored report with the original `operation_id`,
component counts and frame marker, plus `replayed: true`. This is historical
marker-scoped replay, not a new claim about current stores. An indexed lookup
validates that marker and checks at most 4096 later TL rows: new matching bodies,
any new distillate, an active hold or reaching the limit start a fresh forget.
Retained control metadata is not a new application body.
The transport request still has a fresh handle; its completion audit row links
`replayed_operation_id` to the original without extending the route deadline.
Other stores are not inventoried again during replay. For new data outside the
observable TL tail, submit a fresh request with a distinct `--reason`.
Only journaled requests with no recorded outcome are recovered on a later
forget/pump pass. A recorded Pending outcome is not automatically retried;
requests do not automatically resume across boots.

`maosctl forget --principal <id> --reason legal-hold` records a durable hold and
exits 3 without scrubbing or regeneration. A prior hold also blocks erasure.
Frames also naming another principal under legal hold, and their derived
distillates, are preserved and reported as typed skips.
Shared and Loom retain non-principal coordination data by partition. Unicode
letters/digits are part of ids; trailing prose punctuation is a boundary, but
an id glued directly to CJK text without spaces may not match. Detection is
id-based, not semantic: copied content without the id, transformed/escaped ids,
and later collection from an external source are outside this guarantee.

### Uninstall a Spirit and retain its inventory proof

`maosctl uninstall <id>` uses the live door when that Spirit has an SCB.
For a refused installed package with no SCB, stop the serving root and use the
locked offline path. Discovery uses stable owner identity and admission-qualified
lineage, not recycled pids. Legacy numeric owners are a named coverage gap.
Held/Failed results retain the complete installed generation and expose the
available signed partial proof path; explicit recovery errors require operator
attention. Already-performed erasures are not undone by a package rollback.

A signed provisional gap receipt precedes irreversible changes. Normal finalization
retains it beside the terminal inventory; a publication failure may leave only that
earlier partial receipt. Use the returned JSON `proof_path`, not the first directory
entry or filename order. A provisional receipt never certifies completion.
Unreadable held values remain retained; their hold cannot cover a separate
unindexed private namespace, which keeps the uninstall Failed.

Proof verification checks an exact sixteen-category inventory and typed evidence
totals, not just a signature. A valid partial inventory is **not** proof that all
substrates were erased: L1 retains named AC3/KS/O4, watchdog and
`kernel_private_index_hold_lease` gaps. Only the frame marker certifies the
pre-marker TL scope. Same-UID independent writers have no barrier against
post-marker re-introduction, and audit scope/lifecycle metadata is retained.

**Final-tail regeneration limitation.** The uninstall tail records real requests
as `Pending/Refused("late-frame")`, with count and original ids in its durable
`redistillation` report. It does not dispatch them to another live Spirit.
The signed frame proof's body-removal count is not a regeneration-complete claim.
Neither repeating uninstall nor an ordinary forget automatically redrives a
recorded Pending request. There is no operator retry command in L1.
An authorized API embedding may explicitly retry outside uninstall: for a known
subject, obtain a fresh real `ForgetOutcome::Erased` using
`memory.forget_with_reason`, append the unchanged durable request to its
`redistill` list, and invoke `HookPump::finish_forget(tl, &outcome, Some(live_pump))`.
Preserve the real receipt/counts and verify the writer incarnation; do not
fabricate a subject, receipt or ids, or override a Suspended hold. Unknown
subjects and unavailable/ambiguous incarnations stay pending. An operator
redistill-retry surface remains explicitly ownerless/open, trigger-owned by the
first story adding that surface; no automatic recovery is claimed.

The standalone verifier has a declared, hash-pinned Ed25519 backend. In a checkout
on a supported Linux CPython runner, install it in an isolated environment:

```sh
python3 -m venv .venv-erasure-verify
.venv-erasure-verify/bin/python -m pip install --require-hashes -r tools/verify-erasure/requirements.txt
.venv-erasure-verify/bin/python tools/verify-erasure/verify.py <bundle> --pubkey-file <pinned-public-key>
```

CI provisions the same backend for both consuming jobs. Missing dependencies are
failures, not skipped vectors. Bundles are limited to 128 MiB, owner-only and
no-replace; subject commitments are signer-secret, but identity/timing/frame-id
correlation remains visible. Keep the returned proof securely: product purge can
remove retained proofs until the separate retention-policy work is ruled.

## Operate

The kernel composition root lives in `crates/maos-bin`. Operational surfaces — the transparency log, capability mediation, sandbox tiers, ComplianceClaim admission, the registry, and yank propagation — are documented in the reference guides:

- [Deployment Topology](/deploy/) — air-gap, backup/restore, release signing
- [Troubleshooting](/troubleshoot/) — every error code with cause and remediation
- [ABI Stability](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — the v1.0 ABI compatibility matrix

## Air-gapped install

For network-isolated environments, see the [Air-Gap Deployment Runbook](/deploy/air-gap-deployment) and the `maosctl install --source` offline import path.
