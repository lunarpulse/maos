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

Live inference resolves `MAOS_ANTHROPIC_API_KEY`, `MAOS_OPENAI_API_KEY`,
`MAOS_GEMINI_API_KEY` and `MAOS_KIMI_API_KEY` through the OS keyring by default.
If the bounded keyring attempt is unavailable
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

The keyring names for the new keys are `gemini-api-key` and `kimi-api-key`.
Provider construction refuses non-visible-ASCII credential bytes (only 0x21–0x7E
is accepted), names the key but never its value, and refuses boot with exit 78.
Gemini sends its key only in `x-goog-api-key`, never a URL; Kimi uses Bearer auth
through the reusable OpenAI-compatible driver at `https://api.moonshot.ai`.

`MAOS_DEFAULT_PROVIDER=anthropic|openai|gemini|kimi|ollama` selects the router
default; empty is unset. An unregistered id refuses boot and names the registered
ids, rather than silently selecting another provider. Existing registration order
is preserved, with Gemini and Kimi between OpenAI and Ollama, only with their keys.
Explicit replay registers all five ids without credentials or network traffic.

### Per-Spirit provider selection and endpoint pins

A Spirit can select a registered provider it declares in `provider.complete`:

```toml
[providers.primary]
id = "ollama"
endpoint_url = "http://127.0.0.1:11434"
# provider_endpoint_pin = "<64 hex characters: SHA-256 of the exact base URL>"
```

`primary.id` binds both token issuance and inference routing. `endpoint_url` and
`model_id`, when present, are equality assertions against the boot configuration,
not reconfiguration. Non-empty `fallback` is refused as `fallback_unsupported`:
the primary token cannot authorise another provider.

Endpoint pinning is **opt-in**: declaring `[providers]` selects/asserts configuration,
but only an explicit `provider_endpoint_pin` field enables the SHA-256 check.
Manifests without `[providers]`, or with a primary id but no pin, are not
endpoint-pinned. Deployment policy must require pins where endpoint locking is
mandatory. Gemini also refuses per-request model ids that differ from its
configured/admitted model; model ids must use only ASCII letters, digits, `.`, `_`,
`:`, or `-`.

Gemini and OpenAI-compatible responses must include valid, nonnegative `u32`
input/output usage counts; absent or overflowing accounting data is refused
rather than reported as zero. Gemini also requires a nonempty finish reason;
an explicit blocked candidate may have no generated content and keeps its vendor
stop reason. Vendor stop labels are capped at 64 UTF-8 bytes; provider error text
is limited to a 512-byte source prefix with control characters removed.

Compute a pin with `printf %s 'http://127.0.0.1:11434' | sha256sum` and use the
digest as `provider_endpoint_pin`. Admission checks SHA-256 of the **exact configured
base-URL bytes**, without normalising scheme, case, port, path or trailing slash.
`MAOS_OLLAMA_URL` is captured once at boot; a different URL refuses admission with
`endpoint_pin_mismatch`, before token issuance. Pins have the same meaning in
keyless replay as live mode. This locks configuration, **not network identity**:
DNS/TLS authentication remains the transport's responsibility.

The Gemini default is `gemini-3.5-flash-lite`; Kimi is `kimi-k3.1-pro` (D-20-4-12 B).
K2 ids are vendor-deprecated and refused by `check-model-currency`. Kimi's successor
requires a successful $1 top-up and has account-tier limits; see the
[vendor limits](https://platform.kimi.ai/docs/pricing/limits).

**20-4 landing boundary:** L1's Gemini/Kimi shell cassettes are explicitly `seed`;
they prove selection and routing (`infer:gemini->replay:…`, `infer:kimi->replay:…`),
not a vendor wire exchange. Fixture tests prove the wire shape. Until L2 widens
the kernel's rate-limit id set, live/record/deterministic Gemini/Kimi calls fail
closed as `Unconfigured`; no release tag belongs between these landings.

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
After purging data, let the installer remove its binaries: `dpkg -r maos`,
`rpm -e maos`, `pacman -R maos`, `brew uninstall maos`, or
`cargo uninstall maos-bin maos-cli maos-spirit-cli maos-registry`.
See [install and remove MAOS](./install.md); purge never deletes package-owned binaries.

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

## Operate

The kernel composition root lives in `crates/maos-bin`. Operational surfaces — the transparency log, capability mediation, sandbox tiers, ComplianceClaim admission, the registry, and yank propagation — are documented in the reference guides:

- [Deployment Topology](/deploy/) — air-gap, backup/restore, release signing
- [Troubleshooting](/troubleshoot/) — every error code with cause and remediation
- [ABI Stability](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — the v1.0 ABI compatibility matrix

### Cost attribution

`maosctl audit cost-reconcile --month YYYY-MM --format ndjson --pricing xtask/provider-pricing.toml`
groups Inference Port token usage by task, principal, attribution source, Spirit,
provider and model. Plain output is also available. The report is observability,
not an invoice. Worker CLI spend through the egress proxy is not included in
cost frames.

`session_principal` means an operator-bound boot session principal, not an
authenticated end user; `write_target_proxy` is a pid-keyed, lifetime-cumulative
reverse lookup influenced by the Spirit's writes, not a session authority.
Ambiguous and unattributed results remain `host-unallocated`. An absent `task: none`
is displayed as `none (schema 1)` (legacy frame) or `none (schema 2)` (field
absent); the latter alone cannot prove whether a scope was bound. Plain output
escapes backslashes, control and bidirectional-format characters and truncates
each field to its first 256 input characters (escapes can make the printed field
longer); NDJSON is not escaped. A provider/model without a pricing entry is
priced at 0 with a Warnings line.
Root task and session binding is pending the final kernel landing, so existing
writers still emit schema 1 and proxy attribution.

In the JSON report, `per_spirit_attributable_fraction` has one entry per
boot-local Spirit, keyed `<boot_nonce>:<pid>` (both unsigned decimal, for example
`"12:7"`); entries from different boots never merge.

`maosctl forget <principal>` erases the principal from cost frames too: it
replaces matching principal, task, session-principal, provider and model values
with `[REDACTED]` (whole-token matches; keys, schema tags and token counts stay),
so a redacted model or provider is priced as unpriced afterwards. A principal id
equal to a real provider or model name therefore erases that value in every frame
that carries it. Kernel governance receipts that quote the principal are
value-scrubbed the same way.

NFR-Cost-1's per-Spirit, task and principal dimensions concern token spend;
CPU time and storage I/O are planned for v1.1 (ADR-046 R6). The ≥98% aggregate
invoice reconciliation is an operational check, not a synthetic test or proof
of per-principal billing accuracy.

## Air-gapped install

For network-isolated environments, see the [Air-Gap Deployment Runbook](/deploy/air-gap-deployment) and the `maosctl install --source` offline import path.
