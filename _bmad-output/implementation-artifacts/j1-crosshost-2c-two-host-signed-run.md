---
baseline_commit: "5a921c0c, measured in a CLEAN `git archive` tree. `j1-crosshost-2a` was landing in the working tree while this story was scouted, so every `crates/maos-bin/src/main.rs` number reads +2 in a 2a-carrying tree. Re-measure against `2b`'s merge — not HEAD, not this file."
depends_on: "`j1-crosshost-2b` must reach `done`. Hard: three of this story's five ACs judge a mechanism 2b builds — there is no cross-host execution to interrupt (AC3), no second log to reconcile (AC2), and no host-B row to scan (AC4) until 2b lands."
blocks: "NONE — this is the closer of the J1 cross-host lane."
split_from: j1-crosshost-2-cross-host-signed-run (three-way split RATIFIED by Lunarpulse 2026-08-15; that file is the shared preflight for 2a/2b/2c)
kernel_grant: "NONE, and the correct answer for the one kernel-core surface in scope is **do not touch it**. `check-kernel-baseline` GREEN at **24472** (`xtask/kernel-core-baseline.toml:472`; 23679 is stale). The tempting edit is `spawn_and_bridge` (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:449-473`) to add `env_clear()` — **that would be a REGRESSION, not a hardening** (see F3). Every other seam this story needs lives in `maos-a2a-tcp`, `maos-audit`, `maos-cli`, `maos-cohort`, `xtask` or `tests/`. Do NOT cite `abi-diff` (FLAG-E4, `crates/maos-spirit-abi` only)."
kloc_grant: "REQUIRED, and **the wall moved AGAINST this story when `j1-crosshost-2a` landed at `0769869d`. `maos-cli` is now at ZERO headroom — AC1 has nowhere to land without a grant.** Re-measured at `0769869d`: **`maos-cli` 4642/4642 = ZERO** (was +25; 2a took an exact-measured review grant 4626→4642 for the Tier-2 capture surface), **`maos-audit` 6643/6665 = +22** (unchanged — AC2's host field still fits), `maos-a2a-tcp` **1085/1500 = +415** (AC3), `xtask` **38386/38609 = +223** (was +534; AC5's gate is tighter than planned), `_aggregate` **147942/147057 = RED −885**. **AC1 — both P12 sites plus the stdout arm plus the verify-side derivation — is entirely inside `maos-cli/src/subcommands.rs` and therefore needs a named measured grant before the first line.** `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/` and all of `spirits/` cost ZERO, so every test and proven-red vector is still free. Cite `kloc.toml:60-65` with the measurement attached; `kloc.toml:85-88`'s correctness-repair sentence is PROSE, not a machine carve-out (`2a` F11) — permission to ask, not an exemption. **P12 is a correctness repair on a signing path, which is the strongest possible case for the ask — make it explicitly, with numbers.**"
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (this story decides what a signed two-host artifact is allowed to assert)
---

# j1-crosshost-2c — the two-host signed run

Status: **backlog** — story authored, preflight complete, **not** `ready-for-dev`.

> `backlog` is this lane's convention for "authored with blockers stated", not a downgrade —
> `blocked` is not a status in `sprint-status.yaml`. See **§ Blocking conditions**.

> **What this story is.** `2a` made one host able to tell the truth about its worker. `2b` made a
> second host actually do the work. `2c` is **the judge**: it breaks the wire on purpose, scans what
> was stored rather than what was sent, journals the refusals nobody was recording, and produces one
> signed artifact from two independent Transparency Logs that a third party can verify.
>
> **And its first job is to stop a bug that would burn the paid run.** `sealed-export` prints one
> public key and signs with a different one whenever a region is configured — in **two** places, not
> the one the shared preflight filed. `demo-j1` already scrapes that printed key and feeds it to
> `verify-bundle`. Set `MAOS_REGION_HOME` and the existing Tier-2 leg fails **after** the agent has
> been billed. AC1 lands first for that reason.

---

## ⚠ Read this block before the ACs — the ratified scope is wrong or imprecise in fourteen places

Every line number was re-derived at clean `5a921c0c` by five parallel scouts. **Inherit nothing from
the shared preflight, from `sprint-status.yaml`, or from memory** — three of the corrections below
overturn statements that are currently recorded as facts in project memory.

### The six findings that change what you build

**F1 — P12 is real, and it is in TWO places. Fixing the filed one leaves the bug live.**
`crates/maos-cli/src/subcommands.rs:2242` computes `derive_pubkey(&seed)` and prints it raw
(`:2243-2248`), while `sign_bundle` signs with `derive_region_signing_seed(seed, region)` whenever the
bundle carries a region (`crates/maos-audit/src/sealed_export.rs:253-261`). Region comes from
`resolve_region_home()` (`subcommands.rs:2205`, `:3620-3624`) → `Region::resolve_home`
(`crates/maos-domain/src/region.rs:84-104`, env read at `:97`), precedence `MAOS_REGION_HOME` →
`~/.config/maos/operator.toml [region].home_region` (`subcommands.rs:3628-3640`).
**`MAOS_REGION_HOME` is read at SIGN time only** — `audit_verify_bundle`
(`subcommands.rs:2557-2639`) never calls `resolve_region_home()` and uses the supplied `--pubkey`
verbatim (`sealed_export.rs:283-320`). So with a region configured, the key the tool tells the
operator to publish is **not** the key that signed, and `verify-bundle` fails.
*The second site the filing missed:* the trajectory export has the identical defect —
`subcommands.rs:2988-2995` region-pins, `:2997` signs with the derived seed, and **`:3025` prints
`derive_pubkey(&seed)` raw**. Fix only `:2242` and the bug is still live on `maosctl audit export`.
*Blast radius today:* `xtask/src/demo_j1.rs:1104-1109` scrapes the pubkey out of `sealed-export`'s
stderr (`pubkey_hex` `:1133-1138`) and feeds it to `verify-bundle` (`:1110-1126`). demo-j1 neither
sets nor clears `MAOS_REGION_HOME`, so it inherits the defect. Confirmed unhit *on this box only*:
the variable is unset and `~/.config/maos/operator.toml` does not exist.
*The helper already exists:* `derive_region_pubkey` (`sealed_export.rs:41-43`).

**F2 — Two-TL reconciliation is NOT greenfield. The exact two-sided shape ships today and is
`PROVEN_LIVE_SIGNED`.** `crates/maos-loom-lite/src/replication/bundle.rs` carries:
- `CrossRegionReplicationBundle { schema_version, source_region, root, leaves, source_team:
  Option<TeamId>, region_sig }` (`:67-81`) — a **source-identified** signed envelope whose
  `source_team` was added **additively** with `serde(default, skip_serializing_if)` (`:73-79`) to
  preserve wire compatibility, and whose verify **derives the pubkey from the CLAIMED identity**
  (`:74-76`) rather than reading it out of the artifact.
- `ReAttestationReceipt { schema_version, source_region, dest_region, source_root, timestamp_ns,
  signature }` (`:104-112`) — literally *"source X's bundle landed at dest Y"*.
- Verbs: `build_replication_bundle` `:306`, `_v2` `:345`, `verify_replication_bundle` `:536`,
  `build_reattestation_receipt` `:982`, `verify_reattestation_receipt` `:1011`.
Its legs are **4× `PROVEN_LIVE_SIGNED`** in `tests/reports/evidence-ledger-check-cross-region-consensus.json`.
**Port this design.** The additive-`Option`-with-`skip_serializing_if` pattern is also how `host` goes
into `AuditBundle` without disturbing the 9.2b HARD byte-identity replay — `region: None` is the
in-repo precedent (`sealed_export.rs:537-563`).

**F3 — INVERTED: the missing `env_clear` is LOAD-BEARING. An AC that adds it is a regression AND a
kernel breach.** The shared preflight files *"`env_clear` appears ZERO times so the child inherits the
full maos env"* as a defect. Measured: 23 occurrences exist, **all** in `crates/maos-cli/tests/*.rs`;
production count is **0**; and `spawn_and_bridge`
(`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:449-473`) adds env only
(`for (k,v) in &spec.env { cmd.env(k,v); }` at `:467-469`). **That is deliberate and documented**:
`CodexCli::nonsecret_env` (`crates/maos-bin/src/worker_cli.rs:302-310`) says the credential
*"is **inherited host-side from the maos process env**, NEVER set here (so MAOS never holds the
value)"*. Adding `env_clear()` breaks the paid worker path and breaches the 24472 pin besides.
**AC4's credential deliverable is therefore a NEGATIVE TEST asserting the current posture, not a code
change.** And note the caveat that actually matters: the 11 payload variants
(`crates/maos-domain/src/frame.rs:63-80`) carry no credential *by schema*, but
`TaskAssignPayload.goal` and `.success_criteria` are free-form `String` (`:93-96`) and redaction runs
on the **TL write path, not the A2A wire** — so a negative that plants `sk-ant-…` in a `goal` is
testing something the type system does not guarantee. Say which one you are asserting.

**F4 — Two statements this project currently records as fact are false, and code written against
either will not compile or will mis-scope the story.**
- **`MAOS_AUDIT_KEY_SEED` does not exist.** Repo-wide it appears exactly once, inside an
  error-message string literal at `crates/maos-bin/src/main.rs:8873`. The real mechanism is
  `maos_domain::audit_key::load_audit_key_seed` (`crates/maos-domain/src/audit_key.rs:31`), env var
  **`MAOS_AUDIT_KEY`** (`:92`) holding a **filesystem path**, precedence explicit → env →
  `~/.config/maos/audit-signing.key`. The evidence harness signs with
  `maos_audit::release_verify::sign_sha256sums` (`tests/harness/evidence_record.rs:100`) — **not**
  `sealed_export::sign_bundle` — emitting a `MAOS-EVIDENCE-V1` record (`:35`) bound to
  `MAOS_EVIDENCE_COMMIT`/`MAOS_EVIDENCE_NONCE` (`:30-31`, `:72`), verified by
  `verify_release_signature` (`xtask/src/evidence_ledger.rs:49`, `:647`) which requires
  `outcome == "PASSED"` (`:622`), `payload.commit == binding.commit` (`:628`) and
  `payload.nonce == binding.nonce` (`:634`).
- **`PROVEN_LIVE_SIGNED` HAS been reached — 27 legs.** Four Reza ledgers at commit `55f8bf228c`
  (`check-cross-region-consensus` 4, `check-multi-region-slo` 3, `check-multi-tenant-loom` **16**,
  `check-reza-production-path` 4), each with `operator_key_available: true` and reason
  *"operator audit key loaded (MAOS_AUDIT_KEY precedence)"*; `demo_reza.rs:379-392` already gates a
  product claim on two of them. The true statement is narrower: **CI has never reached it** (no
  operator key by ratified design — `evidence_ledger.rs:567-574` downgrades on `NotFound` with a
  written reason, no dev-key fallback), and **J1 has never reached it**. The four ledger files are
  **gitignored** (`.gitignore:36`), so a fresh clone sees none — do not conclude from their absence.
Do not write an AC that says the state is unreachable. Copy Reza's posture instead.

**F5 — The beat cannot be flipped by a ledger. That route is structurally dead, twice.**
`xtask/src/demo_j1.rs:797-801` is verbatim:
```rust
Beat::absent(
    "two-host-signed-run",
    "two real hosts over mTLS/TOFU, heterogeneous worker, one reconciled signed bundle",
    "j1-crosshost-2",
),
```
The owner string names **a story key that no longer exists** post-split — stale at birth. `2c` must
re-point it, and `demo_j1.rs:283` prints the same dead name.
`Beat::absent` (`:98-107`) sets `executed: false`, and `Beat::failed()` (`:110-112`) is
`self.executed && !self.state.is_proven()` — so **an unlanded beat can never fail a run** (`:315-329`,
and `:331-332` says so out loud). The two flip routes:
1. **An executed leg mutates it in-process**, exactly as `--live-codex` does for `TIER2_BEAT`
   (`:240-258`: find by name, set `state`/`detail`/`executed = true`/`owner = None`). **This is the
   viable path.**
2. **A published ledger** (`apply_published_ledgers` `:813-872`) — dead twice: it filters
   `l.gate == DELEGATION_GATE` = `"check-j1-loopback-delegation"` (`:828-831`), and that gate
   **writes no ledger file at all** (`check_j1_loopback_delegation.rs:299-353` only `println!`s under
   `--json`); and even if it did, `PublishedLedger::validate` rejects an unregistered gate because
   `ledger_gates()` (`evidence_ledger.rs:148-150`) **is** `check_loom_substrate_drift::contract_jobs()`
   — the four Postgres substrate gates — so J1 would need a `Contract` row demanding
   `MAOS_TEST_POSTGRES*`, the wrong substrate entirely.
**And `demo-j1` has ZERO CI invocation** — no `.github/workflows/` file references any demo. The gate
is enforced; the scene is not.

**F6 — "Partition semantics made real" is TWO defects, and the cheaper one is worse. Also, two of the
shared preflight's P3 clauses are false.**
- **Two unbounded operations, not one.** `TcpStream::connect` is unbounded — `transport.rs:465-467`
  (**not `:464-466`**), bare `.await`, so a blackholed peer hangs on the OS ~130s. But
  **`framed.send(request)` at `transport.rs:484-487` is ALSO unbounded** — a peer that accepts the
  connection and then stops reading hangs `route_outbound` forever, with no OS backstop. That is a
  *cheaper* real partition to produce and it has no timeout at all. Bound both.
  For reference, `TcpTimeouts::production()` (`transport.rs:66-72`) is `handshake = h` (30s at both
  real sites, `main.rs:10153`, `:10508`), `intake = 30s`, `idle = 60s`; `test_profile()` (`:75-81`) is
  250ms across the board.
- **`A2AError::PartitionTimeout` does NOT have zero match sites.** One real arm exists:
  `crates/maos-a2a-core/src/router.rs:1710-1716`, producing
  `IacBusError::CrossHostPartitionTimeout` (`crates/maos-domain/src/iac_bus_types.rs:56`), on the
  production bridge for **both** transports.
- **`TcpA2ATransport::route_outbound` does read `peer_cfg`** (`:773`, `:776`); the true, narrower
  claim is that it never reads `partition_timeout_secs`. That field
  (`crates/maos-a2a-core/src/config.rs:65-66`, default 30 at `:83-85`) has exactly one production
  consumer — `LoopbackA2ARouter::route_outbound` (`crates/maos-a2a/src/adapter.rs:81`) — and zero in
  `maos-a2a-tcp`, which is the part that holds.

### Eight corrections you would otherwise carry forward as facts

**F7 — Pin-mismatch journaling: the verifier provably cannot reach a Transparency Log, and the trace
the shared preflight cites is unreachable code.** Three independent walls: **(i) dependency** —
`crates/maos-a2a-tcp/Cargo.toml:16-33` has no `maos-iac`/`maos-cohort`/`maos-kernel-core` in
production deps (they are dev-deps at `:36-46`), and adding `maos-kernel-core` reds a blocking test in
the 50× loop (`t12a_kernel_zero_auto_retry_dep_absent`, `t11_t12_chaos_absence.rs:170-176`);
**(ii) ownership** — `TofuPinningVerifier` holds only `pins/posture/direction/expected_peer/
validation_time/sig_algs` (`verifier.rs:62-80`) and both constructors receive nothing else
(`transport.rs:719-725`, `:747-753`); **(iii) signature** — `verify_server_cert` (`verifier.rs:226-243`)
and `verify_client_cert` (`:275-288`) are **synchronous** rustls trait methods, so no `.await` is
possible inside a handshake.
*And it does not need to.* Two seams already exist: **dial side**, the mismatch is already a typed
value at the caller — `route_outbound` returns `A2AError::HandshakeFailed { class: PinMismatch }`
(`transport.rs:795`) and the composition root in `maos-bin` owns the TL; **listen side**, replace the
discarding `_ => return` at `transport.rs:579` with a match on `Ok(Err(e))` — `core: Arc<A2ARouterCore>`
is already in scope at `serve_connection` (`:565`) and the core already carries an installed
**synchronous** `ConsentRuptureSink` (`fn append(&self, frame: &IacFrame) -> Result<(), String>`,
`crates/maos-a2a-core/src/cohort.rs:41-42`; installed `router.rs:352-355`; prod impl
`CohortRuptureLogSink` wired to the primary TL at `main.rs:10146-10147`). That is the 12.4a template.
*Correction to P7:* the cited `tracing::warn!` at `transport.rs:593` is **structurally unreachable** —
`resolve_verified_peer` (`:679-688`) consults the same `find_active_pin_by_fingerprint` oracle over
the same `Arc` (`pins.clone()` at `:330` and `:359`), so if the verifier passed, the resolver cannot
return `None`. **The listen side leaves ZERO trace of any kind**, which is worse than "only a warn".

**F8 — All three pin-mismatch tests are DIAL-side, and the untested side is the weaker one.**
`t3_tofu_pin_mismatch_rejected` (`crates/maos-a2a-tcp/tests/t3_t6_security.rs:82`, assert `:103-106`),
`t4b_pin_only_unpinned_leaf_rejected_at_pin` (`:157`, assert `:177-180`),
`t6_mitm_cert_swap_after_pin_rejected` (`:299`, assert `:320-323`) — all drive `mira_dials_nash`
(`:24-50`), so the *client's* verifier rejects the *server's* leaf. **No test exercises the listen side
rejecting a client cert**, and that is the side where `find_active_pin_by_fingerprint` accepts **any**
active pin, discarding which peer it belonged to (`verifier.rs:178-187`) — per-peer scoping exists
only on the dial side (`verifier.rs:171-174`, `scoped_client_config` `transport.rs:441-453`).
*And under TLS 1.3 the dialer may not even see it:* `connector.connect()` (`transport.rs:470-479`) can
complete before the server evaluates the client cert, so the rejection arrives as an alert on the
response read and maps to `Io` (`transport.rs:494`), **not** `HandshakeFailed{PinMismatch}`. **A
listen-side negative must assert on the SERVER's journal, never on the dialer's error class.**

**F9 — TOFU is thinner than its own docs claim, in four ways.** `InMemoryTofuPinStore` is the **only**
`impl TofuPinStore` in the workspace (`crates/maos-a2a-core/src/tofu.rs:233`); the comments claiming a
persistence-backed impl in `maos-persistence` (`tofu.rs:66-68`, `:129-131`) are **false** — that crate
contains zero `tofu` references, so **pins are rebuilt from config at every boot**; `pin_first_contact`
is a boot-time config loader (`crates/maos-a2a-tcp/src/config.rs:119-138`, called with
`observed == declared == pin.fingerprint`), never an observed cert; and the listen side accepts any
active pin (F8). Context for AC3's honesty, not a defect for `2c` to fix.

**F10 — Two of the three sketched fault windows have no target, and the third is mis-named.**
"Disconnect during execution" needs a receive-side executor, which **`2b` builds** — at HEAD
`build_cohort_a2a_daemon_runtime` never installs an intake sink, so an accepted frame is validated,
ACKed and dropped. And "after-completion-before-ACK" is not a window on this wire: the ACK is
`AckBody { delivered: true, receiver_logical_clock }` (`router.rs:1459-1465`) and means **delivered**,
not **executed**. The three honest windows are: **(a) before the delivery ACK**, **(b) during host-B
worker execution**, **(c) on the reverse `TaskComplete` delivery**.
*The levers to produce them already exist, all kloc-free:* `silent_endpoint()` — listener accepts TCP,
never completes TLS, yielding `TransportFailed("timeout: client handshake")`
(`crates/maos-a2a-tcp/tests/t_12_3_cohort_halt_receipt.rs:279`, used `:515-522`); `drop(transport)` via
`ServeGuard::drop` (`transport.rs:91-100`, used `t_12_3…:452`, `:529`); `set_peer_endpoint` to a dead
address (`t_11_3_scale_churn.rs:124-126`, asserted `:845`, `:869`); and `raw_client_stream` /
`raw_client_connect` for full byte control over a genuine authenticated mTLS session
(`crates/maos-a2a-tcp/tests/support/mod.rs:157-201`). **There is no `a2a-fault-inject` feature and you
should not add one** — `maos-a2a-tcp`'s only feature is `churn-fault-inject`, whose own comment says
it blinds the harness tally and never `verifier.rs`/the router ("the 11.2b P2 sin").

**F11 — `2c` owns the READ-path scan and nothing else on redaction; `2a` explicitly claimed the rest.**
Write-path redaction is real: `static RULES` in `crates/maos-iac/src/adapter/redaction.rs:67-133` has
**16** prefix rules (`:69,73,77,81,85,89,93,97,101,105,109,113,117,121,125,129`) plus a **17th
non-prefix heuristic** — a hex run ≥ `TOKEN_HEX_MIN_LEN = 32` (`:140`, `contains_hex_token` `:317`) —
which the filter scrubs but which `detect_credential` (`:309-313`) deliberately does not report
(`:397-401`). It runs at five call sites, **all pre-write** (`transparency_log.rs:786`, `:1281`,
`:1770`, `:1934`, `:1951`). **No read-path scan exists anywhere** — the only non-write consumer is
`subcommands.rs:2398` (`detect_credential` on the capture doc), also a write-path guard.
*Ownership:* `j1-crosshost-2a` AC2.5 claims demo-j1's provider-aware scan verbatim — *"Do not defer
this to `2c`: `2c` owns the read-path TL scan, and the claim being signed is `2a`'s"* — and AC2.1
claims `ClaudeCli::ambient_auth_path`. **`2c`'s redaction deliverable is the thing that exists
nowhere: a scan over STORED rows.**

**F12 — The `AuditBundle` has a `boot_nonce` the filing omitted, and `region` cannot discriminate
hosts.** Fields (`crates/maos-audit/src/sealed_export.rs:94-113`): `schema_version`, `entries`,
`i12_digest_refs`, `i11_distilled_content`, `freshness` (`:121-132`), `applied_redaction`,
`redaction_policy`, `region` (`:110-111`), `signature_block` (`:134-139`).
`AuditEntry` (`crates/maos-audit/src/lib.rs:91-118`): `frame_id_hex`, `timestamp_ns`, `spirit_pid`,
**`boot_nonce`**, `capability_token_hex`, `kind`, `intent`, `payload`, `redaction`.
So there **is** a per-boot discriminator — but P11 already showed one bundle sweeping **8 distinct
boot nonces** because `--range 1d` swept eight boots. And `region` is jurisdiction, not host: two hosts
in the same region derive the **same** key (`derive_region_signing_seed`, `sealed_export.rs:27-36`), so
region cannot discriminate them. `attester_pubkey` is the only signer identity and it is
bundle-supplied, which R-RG1 forbids trusting (`:84-90`). **`verify-bundle` takes one bundle and one
`--pubkey`** (`crates/maos-cli/src/cli.rs:440-446`, impl `subcommands.rs:2557-2639`), and so does the
standalone `tools/verify-audit-bundle/verify.py` — though that one is field-agnostic (drops
`signature_block`, sorts the rest at `verify.py:91-93`), so a new field flows through it untouched.

**F13 — CORRECTED at the 2026-08-16 round-table. The scout finding below is TRUE but its conclusion was FALSE, and this file inherited the error before it was caught.**
Even after `2b` wires `correlation_id` end-to-end, **`maos_audit::query` does not SELECT it** —
`crates/maos-audit/src/lib.rs:194-196` selects 8 columns and drops `correlation_id`, `from_spirit_id`,
`to_spirit_id` and `origin`. The column, index and filter all exist in the TL
(`crates/maos-iac/src/adapter/transparency_log.rs:268`, `:497-500`, `:1387-1389`). **But `correlation_id` is not the join key.** `maos_audit::query` selects **`frame_id` FIRST**
(same lines), `deliver_typed` writes the *received* frame's id (`crates/maos-iac/src/adapter.rs:562`),
and J1 ids are deterministic `seq ‖ run_nonce` — so once `2b` installs the intake sink, both hosts'
bundles already carry the same `frame_id_hex`. **The reconciliation verb needs no projection and no
`maos-audit` lines.** Budget preserved: `maos-audit`'s +22 stays available for AC2.1's host field.

**F14 — Two null controls sit directly under this story's deliverables.**
- **`schemas/audit-bundle.schema.json` is enforced by nothing** — zero references in
  `.github/workflows/` and zero in `xtask/src/`. The 9.1 "CI schema-gate" never landed. Worse, it is
  **already drifted**: `additionalProperties: false` with properties
  `[schema_version, entries, i12_digest_refs, i11_distilled_content, freshness, signature_block]` — it
  does not list `region`, `applied_redaction` or `redaction_policy`, all of which the Rust struct
  emits. Adding `host` makes it a **fourth** drift against a schema no machine reads. Either wire the
  gate or state plainly that the schema is documentation.
- **`demo_j1::apply_published_ledgers` would silently ignore this story's own gate** — it filters
  `l.gate == DELEGATION_GATE` (`demo_j1.rs:826-831`), so a new
  `evidence-ledger-check-j1-two-host.json` is never read and the beat stays `ABSENT` **even after the
  gate proves it**. See F5: use the executed-leg route.

### What is already true — verify, do not rebuild

| Claim | State at clean HEAD |
|---|---|
| A two-sided signed reconciliation design exists and is proven | TRUE — `maos-loom-lite/src/replication/bundle.rs:67-112`, verbs `:306/:345/:536/:982/:1011`, 4× `PROVEN_LIVE_SIGNED` |
| An additive bundle field can preserve byte-identity | TRUE — `source_team` (`bundle.rs:73-79`) and `region: None` (`sealed_export.rs:537-563`) are both in-repo precedents |
| A staged key-derivation weld is precedented | TRUE — region (`sealed_export.rs:27-36`) → team over the region seed (`:72-82`), each with a frozen salt, an `ascii-v1` info prefix, a **frozen-constants tripwire test** (`:383-390`) and four negatives (`:415-449`). A host weld is stage 3 of the same template |
| `verify-bundle`'s Python twin is field-agnostic | TRUE — `tools/verify-audit-bundle/verify.py:91-93`; a new field needs no porting there |
| The operator key is present on this box | TRUE — `~/.config/maos/audit-signing.key`, 64 bytes, mode 0600 (stat only; contents never read) |
| Fault-injection levers exist and are kloc-free | TRUE — `silent_endpoint`, `drop(transport)`, `set_peer_endpoint`, `raw_client_stream` (F10) |
| A sync sink reachable from `serve_connection` exists | TRUE — `ConsentRuptureSink::append`, `cohort.rs:41-42`, prod impl wired `main.rs:10146-10147` |
| Pin-mismatch refusal is real and blocking-tested | TRUE — three tests, `t3_t6_security.rs:82/:157/:299`, in `a2a-tcp-tests-8-6` (`discipline.yml:1522`) **and** run 50× more by the determinism loop (`:1538-1544`) |
| Pin-mismatch is journaled | **FALSE — zero trace on the listen side** (F7) |
| Hostnames work | **FALSE** — bare `IP:port`, no DNS (`transport.rs:434`, `:468`); `A2APeerConfig::validate` accepts a hostname anyway (`config.rs:115-136`) and fails at first dial |
| The J1 lane has traceability rows | **FALSE — zero.** `_bmad-output/test-artifacts/traceability-matrix.md` carries only a J3 GAP row (`:84`). The closer owns this |
| `check-kernel-baseline` | GREEN, 24472 = 24472 |
| `kloc-check` at committed HEAD | RED on four keys — D13/D14/D15/D17, **none of them yours** |

---

## Blocking conditions

1. **`j1-crosshost-2b` reaches `done`.** AC2, AC3(b), AC4 and AC5 all judge a mechanism 2b builds.
2. **`2b` has stated its boundaries** — in particular the **boot-nonce gap** (2b G4: a release-build
   two-host handshake NACKs and permanently invalidates the pin, because the test override is
   `debug_assertions`-gated). `2c` runs a **paid agent** against this. If 2b ships the gap as a stated
   boundary rather than a fix, AC5 must decide what the signed artifact may claim before the run,
   not after.
3. **AC1 lands before any paid run is scheduled** — it is the only AC with a money consequence
   attached to *not* doing it first.

---

## Story

**As** the founder who has to hand a third party evidence that the developer-remote loop is real,
**I want** the wire broken on purpose and the refusals recorded, the stored rows scanned rather than
the sent ones, and one signed artifact reconciled from two independent Transparency Logs whose signer
identity actually verifies —
**so that** "two hosts did this" is a claim a stranger can check, and every gap it does not cover is
named in the artifact rather than in a story file nobody reads.

---

## Acceptance Criteria (5)

### AC1 — Fix the signing-identity bug first, in both places, or nothing downstream verifies

1. **`sealed-export` prints the key that actually signed.** When a region is resolved, print
   `derive_region_pubkey(&seed, &region)` (`sealed_export.rs:41-43`), not `derive_pubkey(&seed)`
   (`subcommands.rs:2242`). **Keep the stderr line's exact shape** — `"…({N} entries, pubkey {hex})"`
   (`:2243-2248`) is a de-facto ABI with three consumers: `pubkey_hex` (`demo_j1.rs:1133-1138`),
   `entry_count` (`:1141-1154`, also used by `sealed_export_parity` at `:497`), and the pin at
   `demo_j1_tests.rs:350-362`.
2. **Fix the second site the filing missed.** `subcommands.rs:3025` (trajectory export) prints the
   raw pubkey while `:2997` signs with the derived seed. Fixing only AC1.1 leaves the bug live on
   `maosctl audit export` (F1).
3. **Cover the `--output`-less arm.** `subcommands.rs:2250-2257` writes the bundle to stdout and
   emits **no pubkey line at all**, so a stdout-mode export is unverifiable. Decide and implement:
   print to stderr, or refuse, or document. Silence is not an outcome.
4. **Give `verify-bundle` a derivation path.** It never calls `resolve_region_home()` and demands a
   pre-derived key (F1). Accept a base seed plus a claimed region (mirroring
   `verify_replication_bundle`'s *derive-from-claimed-identity* rule, `bundle.rs:74-76`), or state
   explicitly in `--help` that `--pubkey` must be the region-derived key.
5. **Two negative tests.** One asserting `printed_pubkey == signing_pubkey` with
   `MAOS_REGION_HOME` set — **which reds at HEAD** — and one asserting they match with it unset.
   Tests are kloc-free; the fix is not (`maos-cli` has **+25**).

### AC2 — Two-TL reconciliation, ported not invented

1. **Add a host discriminator to `AuditBundle` additively — and understand that it is an ANTI-FORGERY
   control, not a label.** *(Reframed at the 2026-08-16 round-table.)* `2b` proves the crossing by
   writing **the same `frame_id`** into both hosts' logs (2b G16). That is what makes reconciliation
   free — and it means the two bundle halves are, on their own, **indistinguishable**: nothing else in
   the artifact separates them. `region` cannot (two hosts in one region derive the **same** signing
   key, `sealed_export.rs:27-36`); `boot_nonce` is per-boot and T6 proved one export swept eight;
   `attester_pubkey` is bundle-supplied, which R-RG1 forbids trusting (`:84-90`). **So without this
   field, one host can produce both halves of a "two-host" bundle.** The field is the only thing
   making each half attributable.
   Implement as `Option<String>` (or a `HostId` newtype) with
   `serde(default, skip_serializing_if = "Option::is_none")`, exactly as `source_team` was added to
   `CrossRegionReplicationBundle` (`bundle.rs:73-79`) and as `region` already behaves
   (`sealed_export.rs:537-563`). The 9.2b **HARD byte-identity replay must stay byte-identical** for
   bundles that omit the field — assert it, do not assume it. **And the field must be bound by the
   signature, not merely present in the JSON** — an unsigned host label is a decoration a forger fills
   in. Add the negative: a bundle whose host field is altered post-signing must fail verification.
2. **A two-bundle verb.** Port `verify_replication_bundle` / `build_reattestation_receipt` /
   `verify_reattestation_receipt` (`bundle.rs:536`, `:982`, `:1011`) rather than designing a new
   protocol. The receipt shape — *"source X's bundle landed at dest Y"* — is exactly the two-host
   claim. **Derive each side's pubkey from its CLAIMED identity; never read `attester_pubkey` out of
   the artifact** (R-RG1, `sealed_export.rs:84-90`).
3. **Reconcile on `frame_id`. It is already in both bundles — do NOT project `correlation_id`.**
   *(F13 was wrong and this AC previously inherited the error; corrected at the round-table.)*
   `maos_audit::query` selects **`frame_id` as its FIRST column**
   (`crates/maos-audit/src/lib.rs:194-196`), so it is already `AuditEntry.frame_id_hex` in every
   bundle; and `deliver_typed` writes the **received** frame's id
   (`crates/maos-iac/src/adapter.rs:562`), so once `2b` installs the intake sink both hosts' rows carry
   the *same* 16 bytes. The join costs **zero** `maos-audit` lines — which matters, because that crate
   has **+22**. `correlation_id` is indeed dropped from that SELECT, but it is **not the join key**;
   projecting it would spend scarce budget on a second path to a result `frame_id` already gives.
   *Note for the reconciliation logic:* J1 frame ids are **deterministic** — `seq ‖ run_nonce`
   (`spirits/orchestrator/src/lib.rs:357`, `crates/maos-bin/src/delegation.rs:240-242`), no ULID
   entropy — so `2c` can compute the expected id rather than discover it. That is a strength for
   reconciliation and a hazard everywhere else (2b AC3.2).
4. **If a per-host key weld is used, it is stage 3 of the existing template.** region →
   team (`sealed_export.rs:27-36`, `:72-82`) → host, same frozen-salt + `ascii-v1` info-prefix shape,
   **with the frozen-constants tripwire test** (`:383-390`) extended and the four negatives
   (`:415-449`) mirrored.
5. **Say what the bundle can and cannot discriminate.** `region` cannot separate two hosts in one
   region; `boot_nonce` is per-boot, and P11 proved a single `--range 1d` export swept **eight** of
   them. Whatever the host field is, state its scope in the artifact.
6. **Decide the schema's status.** `schemas/audit-bundle.schema.json` is read by no machine and is
   already three fields behind the struct (F14). Either wire a gate that validates emitted bundles
   against it, or mark it documentation in the file itself. Adding `host` silently makes it a fourth
   drift.

### AC3 — Break the wire on purpose, and record what happens

1. **Bound BOTH unbounded operations.** `TcpStream::connect` (`transport.rs:465-467`) and
   **`framed.send`** (`:484-487`). The second is the cheaper real partition — a peer that accepts and
   stops reading hangs `route_outbound` forever with no OS backstop (F6). Both fixes land in
   `maos-a2a-tcp` (**+415**), never in `maos-a2a-core` (**0**, D10).
2. **Wire `partition_timeout_secs` to the TCP path, or delete the claim.** It has one production
   consumer and it is the loopback router (`crates/maos-a2a/src/adapter.rs:81`); `maos-a2a-tcp` reads
   it zero times. The wire is bounded by hardcoded `TcpTimeouts::production()` instead. Do not restate
   "in-flight frames are NACKed after a configurable 30s partition timeout" — and note
   `A2AError::PartitionTimeout` **does** have a match arm (`router.rs:1710-1716`), so the
   "zero match sites" claim must not be repeated either.
3. **Three fault windows, named correctly.** (a) before the delivery ACK, (b) during host-B worker
   execution, (c) on the reverse `TaskComplete` delivery (F10). Do **not** write
   "after-completion-before-ACK" — the ACK means *delivered*, not *executed* (`router.rs:1459-1465`).
   Use the existing levers (`silent_endpoint`, `drop(transport)`, `set_peer_endpoint`,
   `raw_client_stream`); **do not add an `a2a-fault-inject` feature.**
4. **Journal the pin mismatch on BOTH sides, without touching the verifier.** It provably cannot reach
   a TL (F7). Dial side: the typed `HandshakeFailed{PinMismatch}` already surfaces at the composition
   root (`transport.rs:795`). Listen side: replace the discarding `_ => return` (`transport.rs:579`)
   with an `Ok(Err(e))` arm using the already-installed synchronous `ConsentRuptureSink` — `core` is in
   scope at `:565`. **Do not add `maos-kernel-core` to `maos-a2a-tcp`'s deps**; a blocking test greps
   for exactly that (`t11_t12_chaos_absence.rs:170-176`).
5. **The listen-side negative asserts on the SERVER's journal.** All three existing pin tests are
   dial-side (F8), the listen side accepts any active pin, and under TLS 1.3 the dialer may see `Io`
   rather than `PinMismatch`. A negative that only proves the dial side proves the strong half.
6. **Keep it out of the 50× loop.** Every test in `crates/maos-a2a-tcp/tests/` runs **51× per push**
   (50 from `discipline.yml:1538-1544` inside `timeout-minutes: 10` at `:1524`, plus once scoped). A
   test that waits out the 60s idle timeout or the ~130s unbounded connect cannot live there — that is
   what `TcpTimeouts::test_profile()` (250ms, `:75-81`) exists for.

### AC4 — Scan what was stored, and assert the credential posture rather than changing it

1. **Build the read-path scan — it exists nowhere.** All 16 prefix rules plus the hex heuristic run
   pre-write only (F11). `2c` owns a scan over **stored rows**. Reuse
   `redaction::detect_credential` (`redaction.rs:309-313`) rather than re-deriving rules, and note it
   is prefix-only by design — the ≥32-hex-run heuristic (`:140`, `:317`) scrubs but does not report,
   so decide which one the scan asserts.
2. **The credential-isolation deliverable is a NEGATIVE TEST asserting the current posture.**
   `env_clear` is absent **deliberately** — the worker credential is inherited host-side so MAOS never
   holds it (`worker_cli.rs:302-310`), and `spawn_and_bridge` is kernel-core (F3). Assert that the 11
   payload variants carry no credential *by schema*, and state the caveat honestly: `goal` and
   `success_criteria` are free-form `String` (`frame.rs:93-96`) and redaction runs on the TL write
   path, **not** on the A2A wire. A negative that plants a token in `goal` is testing content, not
   construction — say which.
3. **Do not touch what `2a` owns.** demo-j1's provider-aware write-path scan and
   `ClaudeCli::ambient_auth_path` are `2a` AC2.5 and AC2.1 verbatim (F11). If `2a` shipped them,
   verify; do not re-implement.

### AC5 — The judge: a gate that binds, a beat that flips, and a claim bounded by what was proven

1. **A new gate, registered in all FIVE places or it is an empty box.** `EXPECTED_GATES`
   (`xtask/src/check_ship_gate_completeness.rs:20+` — hand-maintained, **nothing forces a gate into
   it**), `gates = [...]` (`xtask/gate-registry.toml:5+`), a `[[ship_gate]]` disposition block, the job
   in `discipline.yml`, and the job name in the `v1-0-ship-gate` `needs:` array (`:3153+`). Only the
   last two are machine-checked *given* the first. **No `services:` block** —
   `check_loom_substrate_drift`'s leg 2 rejects an unregistered service-bearing gate job and is itself
   blocking and in ship-gate needs (`:3193`). Copy `check-live-bilateral-consent`
   (`discipline.yml:2437-2449`).
2. **Binding class, chosen honestly.** The mechanism legs a hermetic CI can run are
   `BindingClass::Blocking` from the day they land — 1a's stated precedent
   (`gate-registry.toml:274-279`, *"Blocking from the day it lands, not advisory-now-blocking-later"*).
   The paid two-host run genuinely has a substrate CI cannot provision, so that leg is
   `AdvisorySubstrate` with a WOULD-HAVE-BLOCKED banner (`gate_common.rs:85-93`).
3. **Flip the beat by an executed leg, and re-point its dead owner.** Mirror
   `demo_j1.rs:240-258` (find by name, set `state`/`detail`/`executed = true`/`owner = None`). The
   ledger route is structurally dead twice (F5, F14) — do not attempt it. Correct the owner string at
   `demo_j1.rs:800` and the printed name at `:283`, both of which still say `"j1-crosshost-2"`, a story
   key that no longer exists.
4. **`PROVEN_LIVE_SIGNED` follows Reza's posture, and the vocabulary is real.** Use `MAOS_AUDIT_KEY`
   (a **path**), `release_verify::sign_sha256sums`, a `MAOS-EVIDENCE-V1` record bound to
   `MAOS_EVIDENCE_COMMIT`/`MAOS_EVIDENCE_NONCE`, verified by `verify_release_signature` — **not**
   `MAOS_AUDIT_KEY_SEED`, which does not exist and will not compile (F4). Do **not** write "no leg has
   ever reached this state": 27 have, on the operator lane. The honest sentence is *"CI holds no
   operator key by ratified design, so in CI this leg is `INDETERMINATE`; the operator lane produces
   the signed claim."*
5. **Bound the claim by what was actually proven.** Name, in the artifact and not only in this file:
   whether the two hosts were two processes or two machines; that peers are addressed by bare
   `IP:port` with **no DNS** (F6/`transport.rs:434`); `2b`'s boot-nonce boundary if it shipped as a
   boundary; that pins are rebuilt from config at every boot and no durable TOFU store exists (F9);
   and that the listen side accepts any active pin. Use `2a`'s established shape — a stated posture a
   capture **cannot overclaim**, with a negative test refusing the overclaim direction
   (`CaptureDoc::validate`, `crates/maos-cli/src/subcommands.rs:2285-2360`, negative at `:3935`).
6. **Close the lane's record.** Add the J1 rows to
   `_bmad-output/test-artifacts/traceability-matrix.md` — the lane has **zero** today, only a J3 GAP
   row at `:84`. Extend `runbook-j1-tier-2-signed-live-run.md` (287 lines, 5 phases, one host, codex
   only, **mentions claude zero times**) to the two-host + heterogeneous-adapter run. Disclose the
   seven blind story-file gates (**D19**) and populate the model/§A6 fields anyway.

---

## Traps

1. **Do not add `env_clear`** (F3). It is a regression and a kernel breach.
2. **Do not write code against `MAOS_AUDIT_KEY_SEED`** (F4). It does not exist.
3. **Do not claim `PROVEN_LIVE_SIGNED` is unreached** (F4). 27 legs, operator lane. The ledger files
   are gitignored (`.gitignore:36`) — their absence in a fresh clone proves nothing.
4. **Do not flip the beat via a published ledger** (F5, F14). Dead twice.
5. **Do not restate P3's false clauses** (F6): `PartitionTimeout` has a match arm
   (`router.rs:1710-1716`), and `route_outbound` **does** read `peer_cfg` — just not
   `partition_timeout_secs`.
6. **Do not add `maos-kernel-core` to `crates/maos-a2a-tcp`** — `t12a_kernel_zero_auto_retry_dep_absent`
   (`t11_t12_chaos_absence.rs:170-176`) greps the manifest and reds inside the 50× loop.
7. **Do not put a slow test in `crates/maos-a2a-tcp/tests/`** — 51× per push, 10-minute cap (AC3.6).
8. **Do not add a `services:` block to a new job** (AC5.1).
9. **Do not touch `2a`'s redaction or ambient-auth work** (F11). Verify it; do not re-own it.
10. **Do not read `attester_pubkey` out of the artifact to verify it** (`sealed_export.rs:84-90`).
    Derive from the claimed identity.
11. **Do not change the `sealed-export` stderr line shape** — three consumers parse it (AC1.1).
12. **Any new gate leg must read via `root.join(rel)`, never a hardcoded path.** The proven-red harness
    runs with `current_dir(tempdir)`; a leg using `Path::new("…")` either reds the baseline control or
    passes every vector **vacuously**. Known-dangerous callees: `gate_common::read_disposition`
    (`:63-65`), `check_ship_gate_completeness` (`:174`), `evidence_ledger::REPORT_DIR` (`:73`), and
    anything shelling `cargo`/`tokei` (no `Cargo.toml` in the tempdir).
13. **`maos-cli` +25 and `maos-audit` +22.** Measure before writing. `kloc.toml:85-88`'s
    correctness-repair sentence is **prose** — `kloc_check.rs` has no exemption token and the compare
    is unconditional (`2a` F11). It is permission to ask for a grant, not a carve-out.
14. **`maos-a2a-core` is 4654/4654, frozen by D10.** One production line hard-fails.
15. **`check-env-contract` cannot see anything this story writes.** It walks **only**
    `crates/maos-bin/src/` (`check_env_contract.rs:119-121`). `MAOS_AUDIT_KEY`,
    `MAOS_REGION_HOME`, `MAOS_EVIDENCE_*` and `CODEX_API_KEY` are all unregistered because they are
    read elsewhere. Do not read that as permission.
16. **`cargo test -p maos-bin` is RED under default parallel flags** (D16). Run scoped,
    `--test-threads=1`.
17. **FR21's 60s window** bites repeated runs on one data home (`orchestrator_dispatch.rs:40`,
    `:63-146`); `MAOS_HOME` outranks `XDG_DATA_HOME`. The advertised
    `MAOS_ORCHESTRATOR_DISPATCH_WINDOW_NS` escape hatch has **zero code readers** — it is prose.
18. **There is no unscoped `cargo test -p maos-bin` in CI.** A test file that is not `--test`-enrolled
    is a suggestion.
19. **`A2AProfile` is dead config and defaults to `Loopback`** (`config.rs:79-81`). Never derive a
    "cross-host" claim from it; derive it from the transport type or the endpoint.
20. **`demo-j1` has zero CI invocation** and `Beat::absent` never fails a run (F5). The new gate is the
    only thing that will bind.

---

## Tasks

- [ ] **T1 (AC1)** — Both P12 sites (`subcommands.rs:2242`, `:3025`), the stdout-mode arm, the
      verify-side derivation path, and the two negatives. **First, own commit, before any paid run is
      scheduled.** Measure `maos-cli` before and after.
- [ ] **T2 (AC2.1, AC2.5, AC2.6)** — Additive host field on `AuditBundle` with the byte-identity
      assertion; state its discrimination scope; decide the schema's status.
- [ ] **T3 (AC2.2, AC2.3)** — Port the two-bundle verb and receipt from
      `maos-loom-lite/src/replication/bundle.rs`; reconcile the two bundles on **`frame_id_hex`**,
      which `maos_audit::query` already projects. **No `correlation_id` work** — it is not the join key
      (F13, corrected).
- [ ] **T4 (AC2.4)** — If a per-host weld is used: stage 3 of the region→team template, frozen-constants
      tripwire extended, four negatives mirrored.
- [ ] **T5 (AC3.1, AC3.2)** — Bound `connect` **and** `framed.send`; wire `partition_timeout_secs` to
      the TCP path or delete the claim.
- [ ] **T6 (AC3.3, AC3.6)** — The three correctly-named fault windows using existing levers; keep them
      out of the 50× loop.
- [ ] **T7 (AC3.4, AC3.5)** — Pin-mismatch journaling on both sides via the composition root and the
      `ConsentRuptureSink`; the listen-side negative asserting on the server's journal.
- [ ] **T8 (AC4)** — The read-path stored-row scan; the credential-posture negative; verify (do not
      re-own) `2a`'s work.
- [ ] **T9 (AC5.1, AC5.2)** — The new gate, registered in all five places, no `services:` block,
      correct binding classes, plus a proven-red file copying
      `xtask/tests/j1_crosshost_1a_proven_red.rs`.
- [ ] **T10 (AC5.3, AC5.4)** — Executed-leg beat flip; re-point the two dead `"j1-crosshost-2"` owner
      strings; Reza-posture signing with the real vocabulary.
- [ ] **T11 (AC5.5)** — The bounded claim as a capture that cannot overclaim, with the
      overclaim-refusing negative.
- [ ] **T12 (AC5.6)** — J1 traceability rows; two-host runbook; D19 disclosure; Dev Agent Record;
      budget attributed by key in a clean tree.

### Review Findings

_(populate at review; §A6 net is non-degradable.)_

---

## Dev Notes

### Measured at clean HEAD `5a921c0c` — `git archive`, not the working tree

| Instrument | Ceiling | Measured | Verdict |
|---|---|---|---|
| kloc `maos-cli` | **4642** | **4642** | **ZERO — 2a took a review grant at `0769869d`. AC1 lives here and needs a named grant** |
| kloc `maos-audit` | 6665 | **6643** | **+22 — AC2's host field and verb live here** |
| kloc `maos-a2a-tcp` | 1500 | **1085** | +415 — AC3's timeouts and journaling |
| kloc `xtask` | 38609 | **38386** | +223 — AC5's gate (was +534 pre-2a) |
| kloc `maos-a2a-core` | 4654 | **4654** | **ZERO — D10 wall, stay out** |
| kloc `maos-domain` | 8644 | **8694** | RED −50 — D14, not yours |
| kloc `maos-kernel-core` | 18248 | **18933** | RED −685 — D13, not yours |
| kloc `_aggregate_hardfail` | 147057 | **147942** | RED −885 — D17, standing, not yours |
| `check-kernel-baseline` | 24472 | **24472** | GREEN |
| Zero-cost surfaces | — | `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/`, all of `spirits/` | `kloc_check.rs:167-193` |

> **All of this story's test weight is free. All of its budget risk is now in `maos-cli`, which has ZERO headroom** — re-measured at `0769869d` after 2a. AC1 needs a named grant before the first line.
> Measure in a clean `git archive` tree — two scouts on this preflight reached a false conclusion
> about a ceiling record by measuring a tree that `2a` was mutating.

### Signing and evidence — the real vocabulary

| Concern | Mechanism | file:line |
|---|---|---|
| Key material | `MAOS_AUDIT_KEY` = a **path**; explicit → env → `~/.config/maos/audit-signing.key` | `crates/maos-domain/src/audit_key.rs:31`, `:92` |
| Bundle signing | `sealed_export::sign_bundle`, region-welded seed | `crates/maos-audit/src/sealed_export.rs:253-261` |
| Region seed | `derive_region_signing_seed` | `sealed_export.rs:27-36` |
| Team seed (stage 2 over region) | `derive_team_signing_seed` | `sealed_export.rs:72-82` |
| Frozen-constants tripwire | test | `sealed_export.rs:383-390`, negatives `:415-449` |
| Evidence record signing | `release_verify::sign_sha256sums`, `MAOS-EVIDENCE-V1` | `tests/harness/evidence_record.rs:100`, `:35` |
| Build binding | `MAOS_EVIDENCE_COMMIT` / `MAOS_EVIDENCE_NONCE` | `evidence_record.rs:30-31`, `:72` |
| Verification | `verify_release_signature`, requires `outcome == "PASSED"` + commit + nonce | `xtask/src/evidence_ledger.rs:49`, `:622`, `:628`, `:634`, `:647` |
| State projection (derived, never annotated) | `EvidenceVerdict::project`; inner field module-private | `xtask/src/gate_common.rs:186`, `:197-211`, truth table `:206-208` |

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| P12 site 1 | `crates/maos-cli/src/subcommands.rs` | `:2242`, print `:2243-2248`, stdout arm `:2250-2257` |
| P12 site 2 | `crates/maos-cli/src/subcommands.rs` | region `:2988-2995`, sign `:2997`, **raw print `:3025`** |
| verify-bundle | `crates/maos-cli/src/subcommands.rs` | `:2557-2639`; CLI surface `crates/maos-cli/src/cli.rs:440-446` |
| Bundle type | `crates/maos-audit/src/sealed_export.rs` | `AuditBundle` `:94-113`, `SignatureBlock` `:134-139`, byte-identity precedent `:537-563` |
| Entry type | `crates/maos-audit/src/lib.rs` | `AuditEntry` `:91-118`; **query drops `correlation_id`** `:194-196` |
| Design to PORT | `crates/maos-loom-lite/src/replication/bundle.rs` | types `:67-81`, `:104-112`; verbs `:306`, `:345`, `:536`, `:982`, `:1011` |
| Read-path scan | `crates/maos-iac/src/adapter/redaction.rs` | `RULES` `:67-133`, `detect_credential` `:309-313`, hex heuristic `:140`, `:317` |
| Timeouts | `crates/maos-a2a-tcp/src/transport.rs` | `connect` `:465-467`, `framed.send` `:484-487`, `TcpTimeouts` `:64-82` |
| Pin journaling seams | `crates/maos-a2a-tcp/src/transport.rs` | dial `:795`; listen `_ => return` `:579`, `core` in scope `:565` |
| Rupture sink (sync) | `crates/maos-a2a-core/src/cohort.rs` | `append` `:41-42`; install `router.rs:352-355`; prod wire `main.rs:10146-10147` |
| Fault levers (free) | `crates/maos-a2a-tcp/tests/` | `silent_endpoint` `t_12_3…:279`, `drop` `:452`/`:529`, `set_peer_endpoint` `t_11_3…:124-126`, raw `support/mod.rs:157-201` |
| Beat + demo coupling | `xtask/src/demo_j1.rs` | beat `:797-801`, dead name `:283`, flip template `:240-258`, ledger filter `:826-831`, pubkey scrape `:1104-1109`/`:1133-1138` |
| Gate registration | `xtask/src/check_ship_gate_completeness.rs`, `xtask/gate-registry.toml`, `.github/workflows/discipline.yml` | `EXPECTED_GATES` `:20+`; registry `:5+`; job shape `:2437-2449`; needs `:3153+` |
| Proven-red template | `xtask/tests/j1_crosshost_1a_proven_red.rs` | `lay_green` `:72`, `assert_red` `:106-124`, baseline `:130-139` |
| Overclaim precedent | `crates/maos-cli/src/subcommands.rs` | `CaptureDoc::validate` `:2285-2360`, negative `:3935` |

### References

- Shared preflight: `j1-crosshost-2-cross-host-signed-run.md` (§2 P1-P14 — **P3, P7 and the
  `env_clear`/`MAOS_AUDIT_KEY_SEED`/`PROVEN_LIVE_SIGNED` claims are corrected here by F6, F7, F3 and F4**)
- Predecessor: `j1-crosshost-2b-cross-host-delegation-mechanism.md` — AC3's correlation and AC4.1's
  three bounded gaps are `2c`'s inputs
- Runbook to extend: `_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` (287 lines,
  5 phases, one host, codex only, **claude zero times**)
- T6 evidence: `_bmad-output/test-artifacts/j1-tier2-evidence/{j1-tier2-capture.json, j1-tier2-bundle.json}`
- Decision register: `epic-14-preflight-decisions.md` (D10, D13, D14, D15, D17, D18, **D19**)

---

## Dev Agent Record

### Agent Model Used

_(record `vendor/model` + harness + date. **Required by policy even though seven story-file gates skip
this filename — D19.**)_

### Debug Log References

### Completion Notes List

### File List

---

## Open Questions

**Q1 — RESOLVED at the 2026-08-16 round-table: two processes for the mechanism, the artifact states
which for the run.** In MAOS's own vocabulary a Host is a process with its own Transparency Log,
identity and mTLS cert, so "two hosts, one box" is architecturally honest — but a reader will hear
"two machines", so the **artifact** carries the distinction, not the story file. `2b` proves the
mechanism with two processes (its harness is hermetic and blocking); `2c`'s paid run may be two
machines and the capture records which. Already AC5.5; the only remaining operator call is whether to
fund a two-machine run. Original framing kept below.
**Q1 (original) — What is the second host, physically?** The shared preflight left this open and said it changes
AC shape. Measured, the constraint is real but narrow: peers are bare `IP:port` with no DNS
(`transport.rs:434`, `:468`), CI cannot hold two machines, and the two-real-process precedent exists.
Recommendation: **`2b` proves the mechanism with two processes on one box (its harness is hermetic and
blocking); `2c`'s paid run may be two machines, and the artifact carries a field saying which.** In
MAOS's own vocabulary a Host is a process with its own TL, identity and mTLS cert — so "two hosts, one
box" is architecturally honest — but a reader will assume machines, so the artifact must say it rather
than the story. That is AC5.5 either way; the operator's call is only whether to fund the two-machine
run.

**Q2 — Does `2c` fix the boot-nonce pairing, or inherit `2b`'s stated boundary?** `2b` Q2 recommends
shipping the mechanism debug-provable with the gap asserted. If that is what lands, `2c`'s paid run
needs out-of-band nonce pairing on a **release** build, which the `debug_assertions` gate forbids
(`main.rs:2585`). Either `2c` closes it — first-contact nonce learning on the listen side, or a
nonce-agnostic pin, both landing in `maos-a2a-core` at **zero headroom** — or the paid run is performed
with a documented manual pairing step and the artifact says so. Recommendation: **decide this before
the run is scheduled, not at the run.** This is the one open question with money attached.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-16 | **Preflight round-table** (same room). **One AC corrected, one AC's meaning changed by a question nobody had asked.** **(F13 corrected — this file had inherited a scout's false conclusion.)** AC2.3 said to project `correlation_id` into `maos_audit::query` and budget it against `maos-audit`'s +22. `correlation_id` *is* dropped from that SELECT, but it is **not the join key**: `frame_id` is selected **first** (`crates/maos-audit/src/lib.rs:194-196`) and `deliver_typed` writes the *received* frame's id (`crates/maos-iac/src/adapter.rs:562`), so once `2b` installs the intake sink both bundles already carry the same `frame_id_hex`. Reconciliation costs **zero** `maos-audit` lines, and that +22 stays available for AC2.1. J1 ids are also **deterministic** `seq ‖ run_nonce`, so `2c` can *compute* the expected id rather than discover it. **(AC2.1 reframed.)** Sally asked how you tell the two logs apart when the join key is identical on both sides. Nothing else in the artifact does: `region` cannot (two hosts in one region derive the **same** signing key), `boot_nonce` is per-boot and T6 swept eight, `attester_pubkey` is bundle-supplied and R-RG1 forbids trusting it. **So without the host discriminator one host can produce both halves of a "two-host" bundle.** It is an anti-forgery control, not a label — and it must be **bound by the signature**, with a negative test proving a post-signing alteration fails verification. **(Q1 resolved)** Two processes prove the mechanism; the artifact — not the story — records whether the paid run was two processes or two machines. A MAOS Host is a process with its own TL, identity and cert, so "two hosts, one box" is honest; the reader who hears "two machines" is who the field is for. 5 ACs unchanged. |
| 2026-08-16 | **Created** at clean `5a921c0c` from a five-scout preflight (faults/partition/pin-journaling · two-TL reconciliation & signing · correlation/outcome · composition root · gates/CI/budget), following the 2026-08-15 ratification of the `2a/2b/2c` split. Status **`backlog`** with three blocking conditions. **Fourteen premises disproved or corrected, three of which overturn statements currently recorded as project fact.** Headline: **(F1) P12 is in TWO places, not one** — `subcommands.rs:2242` *and* `:3025` print a pubkey that is not the signing key whenever a region resolves, `verify-bundle` never derives, the `--output`-less arm prints no key at all, and `demo-j1` already scrapes that key and feeds it to `verify-bundle` — so the existing Tier-2 leg fails **after** the agent is billed; AC1 lands first for that reason. **(F2)** two-TL reconciliation is **not greenfield** — `CrossRegionReplicationBundle`/`ReAttestationReceipt` (`maos-loom-lite/src/replication/bundle.rs:67-112`) is the same two-sided shape, is 4× `PROVEN_LIVE_SIGNED`, and its additive-`Option` field pattern is how `host` enters `AuditBundle` without breaking the 9.2b byte-identity replay. **(F3) INVERTED — the missing `env_clear` is LOAD-BEARING**: the worker credential is inherited host-side by design (`worker_cli.rs:302-310`) so MAOS never holds it, and `spawn_and_bridge` is kernel-core; an AC that adds `env_clear` is a regression *and* a pin breach, so the deliverable is a negative test asserting the posture. **(F4) two recorded facts are false** — `MAOS_AUDIT_KEY_SEED` **does not exist** (the real var is `MAOS_AUDIT_KEY`, a path; the harness is `release_verify::sign_sha256sums` + `MAOS-EVIDENCE-V1` bound to commit+nonce), and `PROVEN_LIVE_SIGNED` **has** been reached — 27 legs across four Reza ledgers on the operator lane; the true statement is that *CI* and *J1* have not. **(F5)** the beat cannot be flipped by a ledger — dead twice (the demo filters on a gate that writes no ledger, and `ledger_gates()` would reject an unregistered gate) — and its owner string names a story key that no longer exists. **(F6)** "partition" is **two** unbounded operations (`connect` `:465-467` **and** `framed.send` `:484-487`, the latter cheaper and with no OS backstop), and two of P3's clauses are false (`PartitionTimeout` has a match arm at `router.rs:1710-1716`; `route_outbound` does read `peer_cfg`). Corrections: **(F7)** a rustls verifier provably cannot reach a TL (dependency + ownership + sync signature) and the cited `transport.rs:593` warn is **structurally unreachable**, so the listen side leaves *zero* trace — journal via the composition root and the already-installed sync `ConsentRuptureSink` instead; **(F8)** all three pin tests are dial-side while the listen side accepts *any* active pin, and TLS 1.3 may hide the class from the dialer; **(F9)** no durable TOFU store exists and two doc comments claim one does; **(F10)** two of the three sketched fault windows have no target and the third is mis-named — the ACK means *delivered*, not *executed* — while every lever needed already exists kloc-free; **(F11)** `2c` owns only the read-path stored-row scan, `2a` claimed the rest verbatim; **(F12)** `AuditBundle` has a `boot_nonce` the filing omitted and `region` cannot discriminate two hosts in one region; **(F13)** `maos_audit::query` drops `correlation_id`, so `2b`'s join key is invisible to the bundle until `2c` projects it; **(F14)** two null controls sit under this story — an unenforced, already-thrice-drifted bundle schema, and a demo ledger filter that would silently ignore this story's own gate. 5 ACs, 20 traps, 12 tasks; ZERO kernel-Δ @24472 where the correct action on the one kernel surface is **do not touch it**. Budget: all test weight free, **all risk in 47 lines across `maos-cli` (+25) and `maos-audit` (+22)**. |
