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
Kernel-internal T3 admission verifies the image lock through the selected provider
instance as well (20-13 L2); the kernel constructs no provider of its own.


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
`MAOS_SECRETS_BACKEND=encrypted-file` with `MAOS_KMS_MASTER_KEY` (or, for a KMS or
HSM you run, `MAOS_KMS_ENDPOINT` pointing at a `maos.kms.v1` sidecar and the
optional `MAOS_KMS_BEARER`; Story 23-8) for the sealed
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

`primary.id` binds both token issuance and inference routing. `endpoint_url` is an
equality assertion against the boot configuration, not reconfiguration; `model_id`,
when present, must name a model the root admits for that provider (below). Non-empty
`fallback` is refused as `fallback_unsupported`: the primary token cannot authorise
another provider.

Endpoint pinning is **opt-in**: declaring `[providers]` selects/asserts configuration,
but only an explicit `provider_endpoint_pin` field enables the SHA-256 check.
Manifests without `[providers]`, or with a primary id but no pin, are not
endpoint-pinned. Deployment policy must require pins where endpoint locking is
mandatory. Model ids on the Gemini wire must use only ASCII letters, digits, `.`,
`_`, `:`, or `-`.

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

**20-4 landing boundary:** The Gemini/Kimi shell cassettes are explicitly `seed`;
they prove selection and routing (`infer:gemini->replay:…`, `infer:kimi->replay:…`),
not a vendor wire exchange. Fixture tests prove the wire shape. L2 (2026-11-11)
widened the kernel's rate-limit id set to `gemini` and `kimi`, so live, record and
deterministic calls pass the limiter; any other id still fails closed as
`Unconfigured`. No vendor round trip is claimed until the operator records one
(`ops-live-gemini-leg`, paid keys). The kernel IO adapter no longer echoes a
rejected request header (a credential) and keeps a non-2xx status when the
body exceeds 10 MiB.

### Manifest model pins

A `provider.complete` entry of the form `<provider>.<model>` is a **model pin**: the
model the Spirit is served by on that provider. Precedence is *manifest pin over the
configured provider default* (operator ruling Q6, 2026-11-13): the Researcher's
`anthropic.claude-sonnet-5-5` reaches the wire as `claude-sonnet-5-5` even though the
Anthropic driver is configured with `claude-haiku-5-5`. A bare `<provider>` entry is
provider-only and is served the provider's configured default. The bound provider is
the Spirit's first declared provider the root has registered (or `[providers].primary`,
whose `model_id`, when set, is the pin); the model is that provider's pin.

The root admits, per provider, its configured default plus every id
`xtask/model-currency.toml` lists as `allowed`, so a Haiku workflow default, a Sonnet
pinned Spirit and the Sonnet eval judge coexist on one configured provider. A pin
outside that set is refused as `model_not_admitted` on every path that admits or stages
a manifest (`maos run`, `maosctl load`, the shell and one-shot hello, and every
hot- or cold-swap successor; a refused hot swap leaves the predecessor serving) and, if it
reaches a driver anyway, before any transport call. The Gemini exception is deliberate (20-4
review F1: the model id is a URL path segment): its admitted set is only its
configured model, so a Gemini pin must equal that model.

### MCP server bearers

Each live MCP server (`MAOS_MCP_<SERVER>_URI` for calendar, slack, linear,
figma, web, arxiv, github and citation-graph) takes its bearer from the same
secret store, under `MAOS_MCP_<SERVER>_BEARER` (keyring entry
`mcp-<server>-bearer`). The bearer is sent only as `Authorization: Bearer` to
that server; with none stored, no `Authorization` header is sent. The client
speaks MCP 2026-07-28 first and falls back to the `initialize` handshake and
`Mcp-Session-Id` of a 2025-11-25 server. `maos purge` removes the bearer entries
with the provider keys.

### Enterprise identity (SSO) key sources

The SSO key source and its tuning are set with these variables
(`docs/runbooks/ent-1-enterprise-daemon-posture.md` has the full table and every typed reason):

| Variable | Required | Meaning |
|---|---|---|
| `MAOS_SSO_JWKS` | yes, unless `MAOS_SSO_DISCOVERY_ISSUER` | a static JWKS document: the air-gap key source |
| `MAOS_SSO_DISCOVERY_ISSUER` | no (`online-adapters` build) | an https OIDC issuer whose rotating keys the Host follows instead of `MAOS_SSO_JWKS`; setting both leaves SSO configured-down |
| `MAOS_SSO_JWKS_TTL_SECS` | no, default 3600 (1 to 604800) | how long a discovered key set is trusted; past it assertions fail closed |
| `MAOS_SSO_JWKS_MIN_REFRESH_SECS` | no, default 30, at most the TTL | minimum time between two refreshes |
| `MAOS_SSO_ISSUERS` | yes | trusted `iss` values; with discovery it must hold exactly the discovery issuer |
| `MAOS_SSO_AUDIENCE` | yes | expected `aud` |
| `MAOS_SSO_ALGS` | no, default `RS256,ES256` | allowed algorithms |
| `MAOS_SSO_ASSERTION` | yes | the assertion the daemon presents at issuance; a task that carries its own (`maosctl ask --assertion-file`) overrides it for its turn |

Setting both key sources leaves SSO configured-down, which denies issuance and never
falls open. A key is revoked only by a successful refresh whose published set omits it:
a fetch can obtain a set that contains the key, the issuer can then withdraw it, and that
in-flight fetch still completes and renews the key's TTL. A withdrawn key therefore
verifies until the first successful refresh that returns a set without it, and never past
the TTL counted from the completion of the last successful fetch whose set contained it
(at most 7 days at the maximum TTL, in monotonic time, plus any time the host is
suspended). Half the TTL plus the idle tick is a scheduling target, not a completion
bound: it needs a refresh to start and succeed and a minimum refresh of at most half the
TTL, and when the minimum refresh equals the TTL the idle-only path can expire the keys
(fail closed) before the next fetch completes. Anyone who can block the fetch (without
breaking TLS) can keep a withdrawn key valid for the full TTL. A key set that is
malformed (as opposed to validly empty) keeps the old keys. A fetch that does not answer
within 30 s is abandoned; an issuer that never answers at all leaves discovery down until
it does. A cohort digest read is verified inside the cohort lock without ever waiting for
the identity provider, so the first digest request after an IdP key rotation is refused.
A retry usually recovers only after the refresh limiter has reopened (a new key id that
arrives within the minimum refresh interval of the previous refresh is refused without
scheduling a refresh) and a refresh has started and published, so repeated refusals are
possible, and every retry is made by the caller; the daemon does not retry. The sequence
can end sooner: if an already-running refresh publishes the new key before the limiter
reopens, a retry can succeed while the limiter is still closed, and if the idle refresh
published it before the first request, the first request can succeed. If the issuer reuses
a key id with new key material, the refusal does not itself ask for a refresh: recovery is
on the next successful publication, normally the half-TTL idle refresh (about half
`MAOS_SSO_JWKS_TTL_SECS` plus the idle tick, a scheduling target gated by the limiter, not
a completion bound). A token's key is rechecked just before it is minted; a key withdrawn
by a publication after that check and before the token is stored (the duration of the
mint, with no fixed bound) is not detected, which is small against the TTL-long
revocation window above and is an operator-accepted residual. A `MAOS_SSO_*`
variable that is not valid UTF-8 leaves SSO configured-down
(`sso_env_not_unicode: <NAME>`). The principal is asserted, not authenticated, at the
door. See the [enterprise daemon runbook](../runbooks/ent-1-enterprise-daemon-posture.md)
for every variable and typed reason.

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

## Serve a standing Host

`maos serve --config <host.toml>` starts a Host from one file. The file declares the Host's settings and the
standing cast; every boot brings the same cast back, because the file names it. `maos run <manifest>` is
unchanged and reads no Host file.

```toml
[host]
home = "/srv/maos"                 # MAOS_HOME (absolute)
door_bind = "127.0.0.1:7600"       # MAOS_OPERATOR_HTTP_BIND (loopback only)
region = "eu-west"                 # MAOS_REGION_HOME
crypto_provider = "ring"           # MAOS_CRYPTO_PROVIDER
inference_mode = "replay"          # MAOS_INFERENCE_MODE: live, record or replay
audit_rotate_bytes = 268435456     # MAOS_AUDIT_ROTATE_BYTES (default 256 MiB)
audit_rotate_days = 30             # MAOS_AUDIT_ROTATE_DAYS (default 30)
# audit_keep_days = 365            # MAOS_AUDIT_KEEP_DAYS (default unset; at least 90)

[[host.spirits]]
manifest = "spirits/butler/manifest.toml"   # relative to this file; `path` is an alias

[[host.spirits]]
installed = "researcher"                    # a registry-installed Spirit
```

| Key | Environment name | Rule |
|---|---|---|
| `home` | `MAOS_HOME` | Absolute path. |
| `door_bind` | `MAOS_OPERATOR_HTTP_BIND` | Loopback socket address; the door still needs `MAOS_OPERATOR_BEARER_TOKEN` (both or neither). |
| `region` | `MAOS_REGION_HOME` | An `ascii-v1` region tag. |
| `crypto_provider` | `MAOS_CRYPTO_PROVIDER` | Must equal the provider recorded in `<home>/config.toml` (default `ring`). |
| `inference_mode` | `MAOS_INFERENCE_MODE` | `live`, `record` or `replay`. |
| `audit_rotate_bytes` | `MAOS_AUDIT_ROTATE_BYTES` | Positive whole number of bytes; default 268435456 (256 MiB). When the live Transparency Log holds this many bytes, its oldest rows are sealed into a signed segment. |
| `audit_rotate_days` | `MAOS_AUDIT_ROTATE_DAYS` | Positive whole number of days; default 30. Live rows older than this are sealed into a signed segment. |
| `audit_keep_days` | `MAOS_AUDIT_KEEP_DAYS` | Positive whole number of days, **at least 90** (a lower value refuses the boot with `host_config_setting`, exit 78). Default unset: MAOS deletes no audit record. |

A set environment variable overrides its key for the process; a key fills only a name that is unset. The
environment never stands in for a missing or invalid file. The cast is exactly the `[[host.spirits]]` list:
installed Spirits the file does not name, and Spirits added with `maosctl load`, do not boot. An entry takes
`manifest` (or `path`) or `installed`, never `host`. `maosctl agents` reads the running cast back.

A missing file, a TOML error, an unknown key, an invalid setting, a cast entry that cannot be admitted, an
invalid `[[host.a2a_peers]]` table, or an
older file that disagrees with a key the Host file sets (`operator.toml` `[region]`, the provider in
`<home>/config.toml`) refuses the boot with exit 78 and one of the kinds `host_config_missing`,
`host_config_parse`, `host_config_cast_entry`, `host_config_cast_host_key`, `host_config_setting`,
`host_config_older_file_disagrees`, `host_config_installed_missing`, `host_config_installed_refused` or
`host_config_a2a_peer`.

### Let cast members hand each other evidence

`[[host.a2a_peers]]` declares the consent between cast members that the Host's in-process A2A router
enforces. It is a table, like `[[host.spirits]]`, so it has no environment name.

```toml
[[host.a2a_peers]]
spirit = "mira"                                            # a loaded member's [class].name
send_allowlist = ["diagnosis-handoff:read-only-evidence"]

[[host.a2a_peers]]
spirit = "nash"
accept_allowlist = ["diagnosis-handoff:read-only-evidence"]
```

A frame from one member to another is admitted only if the sender's `send_allowlist` and the recipient's
`accept_allowlist` both name its typed intent; a missing or empty list admits nothing. Unknown keys, a spirit
named twice and an empty intent refuse the file. With no `[[host.a2a_peers]]` no router is built. Mira hands
Nash her read-only advisory under `diagnosis-handoff:read-only-evidence`: Nash journals the admitted frame, then
proposes a fix. If Nash's list does not admit it, the router refuses the frame and journals a typed
`ConsentRupture` row (`maosctl audit query --kind consent.rupture`); Mira's answer then says
`advisory: "denied:intent_denied"`.

**A halt survives a restart.** Every Spirit termination writes a halt receipt, and the pending-halt list is in
memory. At each boot, once the Host file's cast is admitted, the Host re-admits the halts a cast member raised
and nobody resolved: one `halt.carried` row names the old boot and pid and the new pid, the halt id is the same,
and `maosctl halt resolve` works on it once. A halt you resolved is never carried; a Spirit that is not in the
file (or was added with `maosctl load`) carries nothing.

**Region is an integrity setting.** `MAOS_REGION_HOME` stamps every memory write. On a cohort host the daemon
refuses to boot when this tag disagrees with the `region` the signed cohort manifest declares for the host's
team, or when a region is set and the manifest declares no team for the host. The check compares what is
deployed with what the signed manifest says; it does not stop an operator who controls both the process and the
manifest from choosing a tag. Unset the tag to turn region pinning off.

### Bound the audit store

The Transparency Log no longer grows without limit. By policy the oldest rows move out of the live log into a
sealed, signed segment, `<audit dir>/segments/<seq>.sqlite` with a `<seq>.manifest.json` beside it
(`maos.audit-segment.v1`). The Host checks the policy at boot and every 60 seconds; `maosctl audit rotate`
asks for one now. Queries, recall and citation read across the live log and every segment as if nothing had
moved. Rotation needs the operator audit key (`MAOS_AUDIT_KEY`, then `~/.config/maos/audit-signing.key`), the
same key `sealed-export` uses, and is region-derived when the Host has a region. With no key, rotation is
refused with one `maos: audit rotation refused` line and the live log keeps growing: logging never fails
because of sealing.

A rotation never seals a row that something still needs: not the newest row, not an open forget intent, an
outstanding re-distillation request or the completed-forget marker the door re-checks, and not a row the
escape-anomaly tail or the Worker progress stamper has yet to read. A segment is deleted from the live log
only after it is sealed and every copied row has been re-verified against its live twin. Nothing else is
deleted unless you set `audit_keep_days`: then a whole segment older than that many days (never fewer than
90) is removed, and never one that holds a frame of a principal under legal hold or with an open erasure.
Each expiry is journaled as an `audit.segment.expired` row before the file is removed. A forget scrubs the
principal's frames in the segments too; each affected segment is re-sealed with a manifest that names the
digest it supersedes and the frames it scrubbed.

| Command | What it does | Exit codes |
|---|---|---|
| `maosctl audit rotate` | Seal the oldest prefix now, over the operator door; with no Host answering, offline. Prints the outcome as JSON. | 0 sealed or nothing to seal; 1 refused (no audit key, bad policy); 75 the door is busy |
| `maosctl audit verify-bundle <manifest> --seed <key>` (or `--pubkey <hex>`) | Verify one segment from its manifest and the segment file beside it: the signature, the file digest, the content digest, the Merkle root and the claimed shape. Works on one segment copied alone. | 0 verified; 1 verification failed; 2 input or key error |
| `maosctl audit verify-segments <dir> [--seed <key>] [--log <db>]` | Check the chain of a segment directory: a missing, reordered or forked segment, each file digest and, with a key, each signature. A gap is explained only by an `audit.segment.expired` row in the log (`--log`, default `transparency.sqlite` beside the directory). | 0 intact; 1 broken; 2 input error |

Back up the segment directory with the log: `maosctl backup` and `maosctl migrate sqlite-to-postgres` cover the
live log only, and `maos purge` removes the segment directory it resolves at run time (`--keep-log` keeps it
with the log and discloses the segment count and bytes).

### Where data lives

The private memory root, the registry store, the registry yank cursor and the migration plans share one
precedence: an explicitly set `MAOS_HOME`, then `$XDG_DATA_HOME/maos`, then the legacy
`$HOME/.local/share/maos`. A store lives under the highest root that already holds it, else under the highest
root, so data under a legacy location stays readable in place and a migration plan written under another root
is still consumed. `MAOS_MEMORY_ROOT` and `MAOS_REGISTRY_YANK_CURSOR_PATH` override all three. `maos purge`
names every location.

### A component with resource caps needs a delegated cgroup

A component whose manifest declares `cpu_max_pct` or `pids_max` (the `templates/spirit-ts` scaffold declares
`cpu_max_pct = 100`) is launched into a writable, delegated cgroup v2 subtree, and the launch fails closed without
one: the Spirit never reaches `Running` and the Host's stderr says why. Memory alone falls back to an address-space
limit with a warning. Start the Host under a delegated scope:

```sh
systemd-run --user --scope -p Delegate=yes -- maos serve --config host.toml
```

The packaged `maos.service` unit does not set `Delegate=yes`, so a capped component in that unit's cast is refused
at launch until the unit changes (`RELEASE-HOLDS.md`, K4). The authoring kit (`kit/spirit-author/AGENTS.md`) is the
whole path from a scaffold to a Spirit answering `maosctl ask` in such a Host.

## Hand a Spirit a task

`maosctl ask <spirit> "<text>"` hands a running, directable Spirit one task through the authenticated door and
prints its task id at once. `maosctl task show <id>` reads the Spirit's answer, or the reason it gave none.
The Host must be a `maos serve` (or `maos shell`, or `maos run <topology>`) root whose cast includes an
installed component, or Mira:

```sh
id=$(maosctl ask researcher "Summarise yesterday's incident log")   # exit 0, prints a 26-character task id
maosctl task show "$id"
maosctl ask researcher "Summarise it again" --wait                  # waits (default 120 s) and prints the task object
maosctl ask researcher "Summarise it" --key nightly-2026-11-23      # a retry with this key returns the same task
```

**Mira** is directable too. Her task text is an incident: an `AnomalySignal` JSON
(`{"subject","metric","observed","baseline"}`), or free text when the Host binds an inference provider
(`inference_mode` set; with it unset only JSON is accepted). She diagnoses it deterministically, keeps one record
per diagnosed subject in her private memory, raises her diagnostic-confidence scalar at her halt boundary (the
task then ends `halted`), hands the advisory to the cast's declared peers and answers one JSON object:

```sh
maosctl ask mira '{"subject":"payments-api","metric":"error_rate","observed":0.42,"baseline":0.02}' --wait
# result: {"subject":"payments-api","finding":"...","severity":1.0,"confidence":0.85,
#          "advisory":"delivered","prior_diagnoses":0}
```

`advisory` is `delivered`, `denied:<code>` (the router refused the frame, e.g. `denied:intent_denied`) or
`not_routed` (the Host declares no `[[host.a2a_peers]]`). `prior_diagnoses` counts her earlier diagnoses of the
subject, across restarts. Nash receives the advisory over A2A and is not directable.

`task show` prints one JSON object, `{task_id, spirit_id, origin, state, result?, halt?, reason?, accepted_ns,
ended_ns?}`. The task id is also the `TaskAssign` frame id, so `maosctl audit query` finds the task's rows by it:
the accept row `task.<id>:accepted`, the `TaskAssign` row, the reply row (correlation `maos.task-reply/<hex>`)
and, for a task that gave no answer, the terminal row `task.<id>:<state>`.

| `state` | Meaning |
|---|---|
| `queued`, `running` | Waiting in the Spirit's queue, or its turn is in progress. |
| `completed` | The Spirit answered. `result` is the answer as journaled (redacted), the same bytes after a restart. |
| `halted` | The kernel recorded a halt on the Spirit while the task ran; `halt` is `{halt_id, state}`. |
| `failed` | No answer; `reason` is one of `deadline_exceeded`, `session_ended`, `session_changed`, `no_answer`, `dispatch_refused`, `spirit_unloaded`, `spirit_quarantined`, or, for Mira, `invalid_task` (the text is not a signal and no provider is bound), `inference_refused`, `memory_refused` or `halt_refused`. |
| `orphaned` | The Spirit crashed during the task; the kernel's `task.orphaned` row names it. |
| `interrupted` | The Host stopped (SIGTERM or SIGKILL) while the task was queued or running. It is never delivered again. |

**Refusals mint no task id.** `ask` exits 1 for each of: `spirit_not_loaded` (404), `spirit_not_running` (409; the
Spirit is Loaded, Paused or quarantined), `spirit_not_directable` (409; Nash or another native class, a `cli_wrapper` Worker or
a `maos run` single session, the form named), `invalid_task` (400; empty text), `host_stopping` (409) and a body
over 64 KiB (413). `task_queue_full` (409) exits **75**, as does a `--wait` that runs out: nothing was accepted,
retry. Other exits are the door's: 2 for a local id check, 69 for no daemon, 77 for a rejected bearer and 78 for
an uninitialised home.

**Bounds.** Each Spirit holds one running task and up to 32 queued; the Host holds at most 512 queued and running.
The door never waits on a Spirit: a full queue is refused at once. `task show` keeps the last 1024 finished
tasks in memory and reads older ones from the audit trail.

**Limits you should know.** Only the bearer holder can ask, from this machine (loopback); the task text reaches
the Spirit raw while the audit trail keeps a redacted copy; delivery is at most once and queued work is lost,
not re-run, when the Host stops; an idempotency key is not crash-safe between the accept row and delivery; and
`task show` reads only the live audit segment. The release's claim boundaries B1 to B7 are in `RELEASE-HOLDS.md`.

## Agents and sessions

`maosctl agents` lists every Spirit the running Host holds; `maosctl agents show <id>` reads one Spirit's session
from what the Host already journaled. Both are authenticated reads through the loopback door (`GET /v1/agents`,
`GET /v1/spirits/<id>/session`). They write nothing, and polling them never grows the audit log.

```sh
maosctl agents                       # one line per Spirit, sorted by id, no header
maosctl agents show researcher       # the newest 50 rows of this boot, oldest first
maosctl agents show researcher --limit 500 --follow --interval 1000
```

`agents` prints eight tab-separated columns, `-` for an empty value; the id is always the first:

| Column | Source |
|---|---|
| `id`, `pid` | The scheduler's control block (the pid the audit trail journals). |
| `state` | The Spirit's lifecycle state. |
| `form` | `in-process`, `wasm`, `native` or `worker`, recorded when the Spirit was admitted; `unknown` for a pid no admission recorded. |
| `originator` | The Spirit that handed over its current task; `-` when idle. For a Worker, the Orchestrator that assigned it. |
| `task` | The first in-flight task id, `+N` when more are in flight; `-` when none. |
| `age` | Whole seconds (`41s`) since the Spirit's first audit row of this boot; `-` before it has one. |
| `spend` | `in/out` tokens summed from this boot's cost frames. Tokens only, no money. |

`agents show` prints the Spirit's current-boot rows, oldest first: the frames it emitted, the frames it received
(for example its `task.assign`), and the halt resolutions recorded against it. A row is `<rowid>`
(`a<n>` for an approval row), time as `seconds.millis`, kind, intent (at most 64 characters), the 8-character
frame id, `from->to`, origin and a typed summary (halt: `record`, `halt_id`, `tag`; cost: the token counts;
approval: the decision and the resolution kind). **Never printed:** payload bytes, a halt's Spirit-supplied
`derived_from`, and the text an operator typed when resolving a halt. `maosctl audit query --spirit <id>` prints
full rows when you need them, and reads the store directly.

`--follow` polls with a cursor of what it printed (`--interval` is in milliseconds, default 1000, floor 100). It
exits 0, saying why on stderr, when the id stops resolving or its boot or pid changes (the Spirit was unloaded or
reloaded, or the Host restarted); it exits 69 when the daemon becomes unreachable. Other exits are the door's: 1
for an unknown id, 2 for a local id or `--limit` check, 75 for an unreadable audit store, 77 for a rejected
bearer and 78 for an uninitialised home. Rows of an earlier boot of the same id are not shown; use
`maosctl audit query --spirit <id> --all-boots`.

## Run as a service

The deb and the rpm install `/usr/lib/systemd/system/maos.service` and a `maos` system account (home
`/var/lib/maos`, created by `sysusers.d` on the deb and a `%pre` scriptlet on the rpm). Neither package enables
or starts the unit. They ship an example Host file at `/usr/share/doc/maos/examples/host.toml`, not at
`/etc/maos/host.toml`; you write that file:

```sh
sudo install -d /etc/maos
sudo cp /usr/share/doc/maos/examples/host.toml /etc/maos/host.toml   # then declare your cast
sudo systemctl enable --now maos
```

The unit runs as `maos` with `MAOS_HOME=/var/lib/maos`. Before each start it runs `maos init --plain`, the one
minter of the operator door config (`control.json`); `init` is idempotent. It then runs `maos serve --config
/etc/maos/host.toml`. A missing or invalid file is the typed refusal of the previous section (exit 78),
visible in `systemctl status maos`; the unit does not restart on 78, because restarting cannot fix the file,
and restarts on any other failure. `systemctl stop maos` sends SIGTERM and the Host drains; systemd waits up
to `TimeoutStopSec=30`.

**Known limit.** A local Worker that never exits is not joined with a bound at shutdown
(`RELEASE-HOLDS.md` row 11, open). systemd kills the Host at `TimeoutStopSec` and the unit ends `failed`.

### `maosctl status`

`maosctl status` is one authenticated read of `GET /v1/daemon` with the `control.json` bearer, and prints
one line: `pid=<pid> version=<version> spirits=<n> audit_store_writable=<true|false>`. It must run as the
Host's user, because `control.json` is refused when another uid owns it:

```sh
sudo -u maos env MAOS_HOME=/var/lib/maos maosctl status
```

| Exit | Meaning |
|---|---|
| 0 | The daemon answers and its audit store is writable. |
| 74 | The daemon answers and its audit store is not writable. |
| 69 | The daemon is unreachable. |
| 77 | The operator bearer was rejected. |
| 78 | The home is not initialised. |

Writable means a separate connection to the live audit store takes a write lock (`BEGIN IMMEDIATE`, then
rollback) within 3 seconds, and the capability audit channel is open. The probe writes no row. A held lock
reads as not writable, so a liveness probe should tolerate one 74 before restarting the Host. No route is
added and none is unauthenticated: `/v1/daemon` gains the key `audit_store_writable` and keeps every other key.

## Run in a container

The image holds `maos`, `maosctl`, `maos-wasm-runner`, the in-tree Spirit manifests and a default Host file
(`/usr/share/maos/host.toml`: nash and observer, no provider key needed). Pull a tagged image from the GitHub
registry and verify its signature first (see [Install](/install/#container)):

```sh
cosign verify --certificate-identity <release-workflow-identity> \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com ghcr.io/<owner>/maos@sha256:<digest>
```

**Compose.** `packaging/compose/compose.yaml` runs a one-shot `init` (`maos init --plain`) and then `maos serve
--config /etc/maos/host.toml` on the named volume `maos-home` at `/var/lib/maos`, with `restart:
unless-stopped`, a 30 s stop grace period and the healthcheck `maosctl status`:

```sh
MAOS_IMAGE=ghcr.io/<owner>/maos:<tag> docker compose -f packaging/compose/compose.yaml up -d
docker compose -f packaging/compose/compose.yaml exec maos /usr/local/bin/maosctl agents
```

After `docker compose restart maos` the same cast is read back, and the home survives `docker compose down`
(do not pass `-v`). Edit `packaging/compose/host.toml` to change the cast. Provider keys pass through from the
shell (`MAOS_ANTHROPIC_API_KEY`, ...) only when set.

**Helm.** `packaging/helm/maos` deploys a one-replica StatefulSet with a PersistentVolumeClaim at
`/var/lib/maos`, the Host file from `.Values.hostToml` (a ConfigMap mounted read-only at `/etc/maos`) and
secrets from `.Values.secrets` or `.Values.existingSecret` as `MAOS_*` environment variables. An init
container runs `maos init --plain`; readiness and liveness are `exec: maosctl status`:

```sh
helm install maos packaging/helm/maos --set image.tag=<tag>
kubectl exec maos-0 -c maos -- /usr/local/bin/maosctl status
```

The root filesystem is read-only, the pod runs as uid/gid 65532 with every capability dropped, and
`HOME=/var/lib/maos` puts every data root on the volume.

**The door never leaves loopback.** Neither deployment publishes a port or renders a Service. A `door_bind`
that is not loopback is the typed refusal (exit 78) of the previous section, and an HTTP probe cannot reach
the door, which is why the probe is an exec of `maosctl status`.

**Footprint (NFR-Ops-14, proposed bounds, operator unconfirmed).** The per-PR `container-build` job asserts the
amd64 image size <= 140,022,361 B and the default cast's idle memory <= 50,000,000 B (container cgroup
`memory.current`, 60 s after `maosctl status` = 0). Each run prints what it measured; the bounds are the
measurement plus 10 % and live in `tests/integration/image-size-limit` and `tests/integration/idle-memory-limit`.

**T3 Workers are not supported in a container or pod.** They need a rootless container runtime (the detector
probes `/usr/bin/podman`), which neither provides by default; neither deployment mounts a container socket or
requests privilege to work around it. Run T3 Workers on a host install.

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
The root binds one task per shell turn (`shell.turn:<id>`), per Butler,
Orchestrator or Researcher seam pass (`<spirit>.pass:<id>`) and for the one-shot
hello run (`hello-spirit.run:<id>`), and, when `MAOS_SESSION_PRINCIPAL` is set at
boot, that session principal; such frames are schema 2 with `task_ref` and
`attribution_source: session_principal`. A malformed value (a `:`, a control,
invisible or bidirectional-format character, or more than 128 bytes) refuses the boot;
unset or blank binds none and the write-target proxy attributes, tagged
`write_target_proxy`. Identity text the Transparency Log redactor would rewrite (a
32-hex run or a credential-shaped prefix) or that needs a JSON escape is journaled as
`x:` plus colon-delimited hex; the report and `forget` read it by its decoded value.
Frames written before 20-14 stay schema 1.

In the JSON report, `per_spirit_attributable_fraction` has one entry per
boot-local Spirit, keyed `<boot_nonce>:<pid>` (both unsigned decimal, for example
`"12:7"`); entries from different boots never merge.

`maosctl forget <principal>` erases the principal from cost frames too: it
replaces matching principal, task, session-principal, provider and model values
with `[REDACTED]` (whole-token matches; keys, schema tags and token counts stay),
so a redacted model or provider is priced as unpriced afterwards. A principal id
equal to a real provider or model name therefore erases that value in every frame
that carries it. Kernel governance receipts that quote the principal are
value-scrubbed the same way, while governance event tags, ratification statuses
and manifest fingerprints are kept. Known limits: matching is exact and
case-sensitive (case variants such as `ALICE@x`, zero-width infixes and homoglyphs
of a principal id are not found, and the forget is still reported complete), and
Spirit-name columns (`from_spirit_id`, `to_spirit_id`, recipients) are not erased.

NFR-Cost-1's per-Spirit, task and principal dimensions concern token spend;
CPU time and storage I/O are planned for v1.1 (ADR-046 R6). The ≥98% aggregate
invoice reconciliation is an operational check, not a synthetic test or proof
of per-principal billing accuracy.

## Air-gapped install

For network-isolated environments, see the [Air-Gap Deployment Runbook](/deploy/air-gap-deployment) and the `maosctl install --source` offline import path.
