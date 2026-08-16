---
baseline_commit: 5a921c0c
depends_on: j1-crosshost-1b-consent-proofs-and-gate (ready-for-dev, NOT done) + j1-crosshost-1a (done, 6827dc87)
blocks_on: "j1-crosshost-1b must reach `done` with rung-1 evidence reading PROVEN_BLOCKING; D18 must be decided (its deadline is literally *before this story writes its first line*); and the SPLIT below must be ratified."
split_from: j1-crosshost-1-loopback-developer-remote-delegation (SCP 2026-07-16 §4.2)
kernel_grant: "NONE EXPECTED, high confidence, measured. `check-kernel-baseline` GREEN at 24472. The install seam is transport-agnostic: `Mailbox::deliver` Phase 3 only calls `route_outbound` on `Arc<dyn A2ARouter>` (`maos-iac/src/adapter/mailbox.rs:519-541`), `TcpA2ATransport` already implements that port (`maos-a2a-tcp/src/transport.rs:825-840`) and is ALREADY constructed as `Arc<dyn A2ARouter>` in shipped code (`main.rs:10678`), and `install_a2a_router` is already classified at `xtask/kernel-api-classes.toml:290`. Do NOT cite `abi-diff` as evidence — FLAG-E4, it scopes to `crates/maos-spirit-abi` only."
kloc_grant: "REQUIRED, size unknown until scoped. **The wall is `maos-a2a-core` at 4654/4654 — ZERO headroom, frozen by D10 (no third unscoped grant).** `maos-bin` is also at 16219/16219 zero (D15, deliberate). `maos-a2a-tcp` has ~415 and `xtask` ~534. Any `router.rs` edit — including D18's — hits the wall. Measure before code, grant after."
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (cross-host security + audit surface)
---

# j1-crosshost-2 — cross-host signed run

Status: **backlog** — preflight COMPLETE; **not** `ready-for-dev`.

*(`backlog` is deliberate, not a downgrade. This lane's flow is `backlog → ready-for-dev → review →
done`; `blocked` is not a status in `sprint-status.yaml` — `j1-crosshost-1b`'s file was corrected
from `blocked` to `backlog` on 2026-08-14 for exactly this reason. The blocking conditions are
stated in §5 and must be cleared before this moves.)*

**Three things gate the move to `ready-for-dev`:** `j1-crosshost-1b` reaching `done` with rung-1
evidence at `PROVEN_BLOCKING`; **D18** decided (its deadline precedes this story's first line, and
its fix currently has no budget — §5.2); and the **split in §4 ratified**.

This document is the rung-2 preflight. It was scouted at `5a921c0c` by four parallel agents, one of
which **executed a live `claude` worker end-to-end** rather than reasoning about whether it could.
Nothing below is inherited from the ratified card without being re-measured; several of the card's
own clauses are disproved in §2.

---

## 1. Preflight verdict — the two questions the card said this story owed

The `sprint-status` row states rung 2's preflight owes (a) proof a live non-Codex adapter is viable,
else insert an enablement story, and (b) the peer-authentication non-coverage inherited from rung 1,
written in explicitly. Both are answered.

### (a) Adapter viability — **VIABLE. No enablement story. The escape hatch does not trigger.**

Not asserted — **run**. At HEAD, with a manifest and a `MAOS_HOST_GRANTS` file and **zero repo
changes**, a live `claude` worker completed through `maos run`: host grant admitted
(`attested_image=claude`, T3), the `MAOS_LIVE_AGENT` gate refused without the flag and permitted with
it, the `--version` liveness probe admitted the binary, the bridge spawned a real child with stdin
closed and no deadlock, one `CliSubprocessOutput` row was journaled, and the oracle emitted
`{"worker_cli":"claude","completed":true,...}`.

`ClaudeCli` (`crates/maos-bin/src/worker_cli.rs:339-358`) is real and live-reachable: one construction
site (`:373`), reached from the single production dispatch `select_worker_cli` (`:363-376`) ←
`main.rs:1033`. It is **not** the "type nobody constructs" category error. Adapter selection is
manifest-declared by binary basename, fail-closed for anything unlisted — so **two hosts can run
different adapters under the same protocol today with no code change.** The codex half is T6; the
claude half is now proven.

> **But the same run found a ship-blocker, and it is upstream of everything else.**
> Second task: write `hello.txt`. `claude -p` refused for lack of permission, **exited 0**, and
> `final_stdout_message_oracle` (`worker_cli.rs:211-228` — "clean exit + non-empty final stdout
> line") scored the refusal as **`completed: true`** with a citable `completion_tl_ref`. The file was
> never created.
> This is latent for `CodexCli` too; T6 simply never tripped it. **A false completion exists on one
> host, with no faults injected, before any cross-host work begins.** The card's "never a false
> completion" is violated by the *existing* mechanism. Nothing signed can be built on it.

### (b) Inherited non-coverage — written in, and **narrower than feared**

Rung 1 proves the wire with peer authentication stubbed out: `LoopbackA2ARouter` calls
`handle_intake` directly (`crates/maos-a2a/src/adapter.rs:82`, `:97`) because there is no wire
identity to bind, so `frame.from.host_id` — the field that *selects which `accept_allowlist` judges
the frame* — is sender-asserted. **Rung 1's consent refusals do not transfer.**

What is **not** true is that rung 2 must build the fix. `handle_intake_verified`
(`crates/maos-a2a-core/src/router.rs:1494`) has **exactly one production caller, and it is on the TCP
path** (`crates/maos-a2a-tcp/src/transport.rs:638`). The binding is already exercised by
`crates/maos-a2a-tcp/tests/t_10_4b_live_bilateral.rs` — **7 tests, ZERO `#[ignore]`, run 50× per push**
(`discipline.yml:1522`, any single flake fails the job) — over real sockets with rcgen-minted mTLS:
a 50-scenario corpus all passing with `binding_passed == true`, a forged `from.host_id` rejected
`CODE_PEER_IDENTITY_MISMATCH` *before* intake, and `IntentDeniedAtPeer` on the live wire.

**So the peer-auth gap is already closed at the transport layer. It was simply never wired to the J1
delegation path.** That is rung 2's work, and it is composition, not construction.

Also inherited: **D18** (`map_a2a_error_to_iac_bus` flattens the deny vocabulary,
`router.rs:1671-1783`), filed 2026-08-15 with the deadline *"before `j1-crosshost-2` writes its first
line"* — see §5.

---

## 2. Disproved — card clauses and shipped documents that are wrong at HEAD

**P1 — "Rung 2 builds a two-host substrate from scratch." FALSE.**
`crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1642` already boots **two real `maos` OS
processes** (`Command::new(env!("CARGO_BIN_EXE_maos"))` `:1608`, `MAOS_ONE_SHOT=cohort-a2a-daemon`
`:1611`), scrapes the real ephemeral port, dials `tls://127.0.0.1:{port_b}`, and asserts the row
landed physically in the other team's database. A three-process variant exists at `:2355`. Both run
under `check-multi-tenant-loom`, which is a `needs` of `v1-0-ship-gate`. **Rung 2 re-targets an
existing substrate.** Caveat that defines the work: that substrate carries *cohort crossing* frames
and has **never carried a Mailbox `TaskAssign`**.

**P2 — Rung 1's boundary tripwire can never fire, so "rung 2 flips it" is unachievable as written.**
`leg_loopback_from_host_unverified` (`xtask/src/check_j1_loopback_delegation.rs:266-293`) tests
`contains("frame.from.host_id") && contains("pub async fn handle_intake_verified")` over one shared
file. Both needles are permanent. It publishes `true` forever. Three documents assert otherwise (the
gate doc `:30-32`, `j1-crosshost-1a-…md:288`, the `j1-crosshost-2` sprint-status row).
**Repair is assigned to `j1-crosshost-1b` (its AC2.2a), not here** — but rung 2 must verify the
repaired leg actually flips when this story lands, and must not inherit the false claim.

**P3 — "In-flight frames are NACKed after a configurable 30s partition timeout." DOC-ONLY on the live
wire.** `partition_timeout_secs` (`maos-a2a-core/src/config.rs:65-66`) has exactly **one** production
consumer — `LoopbackA2ARouter::route_outbound` (`maos-a2a/src/adapter.rs:81-91`) — and **zero** in
`maos-a2a-tcp`. `TcpA2ATransport::route_outbound` destructures `peer_cfg` and never reads it; the
wire is bounded by hardcoded `TcpTimeouts::production()` (`transport.rs:64-71`), not operator config.
**`TcpStream::connect` is unbounded** (`transport.rs:464-466`) — the canonical partition (blackholed
peer) hangs on the OS TCP timeout, ~130s. `A2AError::PartitionTimeout` has **zero match sites
repo-wide**; only `Display`. And the loopback timeout is structurally unreachable anyway — it wraps
an in-process `async fn` whose only sink is a non-blocking `UnboundedSender`.

**P4 — "The kernel does NOT auto-retry." TRUE for the kernel, FALSE for the stack.**
`mailbox.rs:533` calls `route_outbound` exactly once. But `transport.rs:779-806` loops up to
`max_attempts` (default **4**, `mtls.rs:22-28`) with backoff, **re-sending the same request**, gated
to cert-class handshake failures. That is an **at-least-once redelivery path below the kernel** —
and the intake path never reads `frame.frame_id`. Duplicate execution is reachable today, not a fault
that must be injected. (Separately, multi-peer fan-out keeps only `first_err` and silently discards
later peers' failures, `mailbox.rs:531-540`.)

**P5 — "There is a `completed | halted | indeterminate` vocabulary." There is no task-outcome
vocabulary at all.** `TaskCompletePayload` is `{result: String}` (`maos-domain/src/frame.rs:189-191`)
with **no `impl` block anywhere** and no back-reference to the assign it answers.
`delegation.rs:243` hardcodes `"completed"`; a shipped subcommand emits `result: "worker-a done"`
(`main.rs:9203-9207`), proving the field is arbitrary prose. `"halted"`/`"aborted"` appear **zero**
times as outcome values. Worst: abnormal endings **bypass the typed payload entirely** —
`halt/resolver.rs:222-229` and `supervision/crash_detector.rs:163-170` write rows tagged
`FrameKind::TaskComplete` with topic `"task.orphaned"` and **no payload**. A reader of the typed
frame stream can never see a crash.

**P6 — "Halt/indeterminate semantics are reusable." `HaltPresence::Indeterminate` is dead in
production.** `HaltReceiptDistributor` — the shipper *and* the prober that produces it — is
constructed **only in tests**. Zero `src/` instantiations. Only the receive half is production-wired,
inside `build_cohort_a2a_daemon_runtime`, i.e. the cohort daemon, **not** `maos run`.

**P7 — "Pin mismatch refused **and journaled**." Half false.** Refusal is real and proven by three
blocking tests. **Journaling does not exist**: the verifier returns a rustls error and the connection
dies (`verifier.rs:180`); on the listen side the only trace is a `tracing::warn!`
(`transport.rs:593`). No audit row, no TL entry, no wire code.

**P8 — TOFU is thinner than its own docs claim.** `InMemoryTofuPinStore` is the **only** `impl
TofuPinStore` in the workspace; the comment claiming a persistence-backed impl lives in
`maos-persistence` is false (zero `tofu` references there). **No durable pin store exists** — pins
are rebuilt from config at every boot. `pin_first_contact` is a **boot-time config loader**
(`maos-a2a-tcp/src/config.rs:119-138`), not first-contact. And on the **listen** side there is no
per-peer scoping at all: `find_active_pin_by_fingerprint` accepts *any* active pin
(`verifier.rs:178-187`); per-peer scoping exists only on the dial side.

**P9 — Hostnames do not work.** `A2APeerConfig::validate` accepts a hostname endpoint, but
`dial_addr` does `rest.parse::<SocketAddr>()` (`transport.rs:434`) and SNI is `ServerName::IpAddress`
(`:468`). **Two real hosts must be addressed by bare `IP:port`. No DNS.**

**P10 — `A2AProfile` is dead config.** `{Loopback, CrossHost}` is never read to select behavior in
any production path — written and unit-asserted only. A peer declared `Loopback` on a TCP transport
behaves identically to `CrossHost`.

**P11 — T6's "one signed bundle" is looser than the doc implies.**
`_bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json`: 247 entries of which **159
(64%) render `kind: "unknown"`**, `i12_digest_refs` and `i11_distilled_content` both empty, no
`region`, and **8 distinct `boot_nonce` values** — `--range 1d` swept eight boots, not the one paid
run. One host, explicitly.

**P12 — Latent bug rung 2 walks straight into.** `maos-cli/src/subcommands.rs:2240` prints
`derive_pubkey(&seed)` **raw**, while `sign_bundle` signs with `derive_region_signing_seed(seed,
region)` when `MAOS_REGION_HOME` is set (`sealed_export.rs:253-264`). With a region configured, **the
pubkey `sealed-export` tells the operator to publish is not the key that signed the bundle**, and
`verify-bundle --pubkey <that>` fails. Unhit at T6 because that bundle carries no region — rung 2,
using per-host key derivation, hits it immediately.

**P13 — `install_intake_sink` is documented "test-only hook" (`router.rs:342`) yet is on the
production founder-loop path** (`pairing.rs:112` ← `delegation.rs:103` ← `main.rs:3150`). Rung 2
inherits it and should either promote it or replace it.

**P14 — the SCP's own yaml row contradicts its AC1.** `sprint-change-proposal-2026-07-16.md:187` says
the remote daemon runs *"its OWN local codex"*; `:144` requires a non-Codex adapter. The live
`sprint-status` row has been corrected; the SCP text has not.

---

## 3. What exists vs what is net-new

| Capability | State at HEAD |
|---|---|
| `TcpA2ATransport` + mTLS (no plaintext path) | **Real, complete, CI 50×/push** |
| `handle_intake_verified` identity binding | **Real, one production caller, on the TCP path** |
| Forged-`host_id` rejection, `IntentDeniedAtPeer` on live wire | **Proven, zero-ignore tests** |
| Two real daemon processes over mTLS in CI | **Exists** (cohort crossing frames only) |
| Non-Codex adapter (`ClaudeCli`) | **Real, live-proven today** |
| Adapter selection by manifest, fail-closed | **Real** — heterogeneity needs no code change |
| Credential-free wire | **Structurally true** — no `env`/`secrets`/`token` in any of the 11 payload variants |
| Provider-aware secret detection (write path) | **Real** — 16 prefix rules, runs at every TL insert |
| `sealed-export` / `verify-bundle` / Ed25519 / HKDF weld | **Real, single-log** |
| — | — |
| A host B that consumes `TaskAssign` over TCP and runs a worker | **NET-NEW** |
| Composition-root fork (set-once router claims loopback at `main.rs:3150`, before `MAOS_ONE_SHOT` dispatch) | **NET-NEW** |
| Inbound pump on TCP (`bind*` never installs an intake sink) | **NET-NEW** |
| `TaskComplete` return hop (today hardcoded same-host, `host_id: None`) | **NET-NEW** |
| Wire `task_id` correlation + TL journaling on intake (A2A intake writes **no** TL row) | **NET-NEW** (TL already has `correlation_id` column + index + filter, unused by J1) |
| Duplicate-delivery safety | **NET-NEW** (template exists: `DigestReadPort` `Accepted/Duplicate/Unauthorized`, but in-memory only) |
| Task-outcome vocabulary | **NET-NEW** (P5) |
| Real partition semantics on TCP | **NET-NEW** (P3) |
| Pin-mismatch journaling | **NET-NEW** (P7) |
| Durable TOFU pin store | **NET-NEW** (P8) |
| Two-TL reconciliation + a host discriminator to join on | **100% NET-NEW** — `AuditBundle` has no host field; `verify-bundle` takes one bundle; only join key is `frame_id` |
| Cert pairing tooling | **NET-NEW** — no CLI mints certs or computes fingerprints |
| `ClaudeCli` hardening (oracle, ambient auth, argv posture, committed manifests) | **NET-NEW** |
| De-hardcoding `demo-j1` from codex (`:362`, `:1007-1012`, `:1027`) | **NET-NEW** |

---

## 4. SPLIT — RATIFIED 2026-08-15 (Lunarpulse)

The ratified card sketches 6 ACs. The measured scope is three to four times that, and it spans three
distinct risk classes that this lane has already learned not to bundle (1 → 1a/1b; 13.6 → 13.6a/13.6e;
12.4 → 12.4a/12.4b). **Ratified: three stories, sequenced. This document is their shared preflight;
each gets its own story file at its own create-story pass.**

Sequencing: **2a → 2b → 2c.** `2a` is unblocked by the split (see §5.2) and can start as soon as it
has a story file — it does not wait on `1b`. `2b` waits on `1b` reaching `done` and on `2a`. `2c`
waits on `2b`.

### `j1-crosshost-2a` — a signable heterogeneous worker (one host, zero cross-host)
Closes the ship-blocker found by execution, and is independently valuable.
1. **Effect-based completion oracle.** A refusal that exits 0 must NOT score `completed: true`.
   Per-adapter oracle or an effect check; a planted "agent declines the task" vector must red.
   Applies to `CodexCli` too — the defect is shared.
2. **`ClaudeCli::ambient_auth_path`** so `refuse_ambient_auth` (`main.rs:1049-1063`) stops being a
   no-op for claude; negative test mirroring `CodexCli:321-329`. Without this a signed run stamps
   `redaction_result: "verified"` over an un-attestable subscription credential — the runbook's
   explicit abort condition.
3. **argv posture ratified** (`-p` alone cannot execute a coding task) and **committed manifests +
   topology** — `spirits/worker/manifest-codex.toml` and `j1-founder-loop-codex.toml` referenced by
   the runbook **do not exist in the repo**; T6 ran on operator-local uncommitted files.
4. **FS-jail posture stated**, since `claude` has no `--sandbox workspace-write` counterpart and
   external jails are structurally blocked (`select_worker_cli` matches on basename, so `bwrap`
   fails closed). Either accept a narrower claim or state the gap.
5. De-hardcode `demo-j1`'s codex assumptions.

### `j1-crosshost-2b` — cross-host delegation mechanism (two hosts, no signing)
6. Host B: a daemon that **receives `task.assign` over TCP and spawns its own `[cli_wrapper]`
   worker**; `TaskComplete` returns as a frame.
7. Composition-root fork + `TcpA2AConfig` plumbing for `maos run` + a real inbound pump.
8. **Wire `task_id` correlation** end-to-end, journaled on both hosts using the existing
   `correlation_id` lane. (This is also FLAG-E5's named fix — but the FR21 semantic change belongs
   to Story 6.2's owners; coordinate, do not unilaterally change it.)
9. **Duplicate-delivery safety**, given P4's real retry path.
10. **Task-outcome vocabulary** (P5), including the `task.orphaned` bypass.

### `j1-crosshost-2c` — the signed two-host run (the judge)
11. Fault injection: disconnect before execution / during / after-completion-before-ACK; partition
    semantics made real on TCP (P3).
12. Structural credential isolation **negative** + a **read-path** provider-aware TL scan (the write
    path already redacts; nothing scans stored rows, and `demo-j1`'s scans are inert when the env var
    is unset).
13. Pin-mismatch **journaling** (P7).
14. **Two-TL reconciliation**: a host discriminator in the bundle + a verb that takes two bundles.
    Fix P12 first or per-host key derivation publishes the wrong pubkey.
15. New gate (a source-grep oracle cannot judge a two-host run) + `demo-j1` beat flip
    (`two-host-signed-run`, already declared ABSENT and owned by this story at `demo_j1.rs:797-801`).

**Why this boundary:** 2a is provable on one host and unblocks everything; 2b is mechanism and can be
proven with fixtures; 2c is the judge and the paid run. It is the same mechanism/judge seam that made
1a/1b work — *"1a built the frame, 1b built the exam."*

---

## 5. Blockers and traps

1. **`j1-crosshost-1b` must be `done`** with rung-1 evidence reading `PROVEN_BLOCKING`. It is
   currently `ready-for-dev`.
2. **D18 — RESOLVED 2026-08-15; the paradox does not survive measurement.** It was stated as *"a
   decision whose implementation has no budget"* because the fix was assumed to grow
   `maos-a2a-core` (4654/4654, frozen by D10). Measured, three things are different:
   - The **`-32001` pair is already distinguishable** — `IntentDenied{direction: Send}`
     (`router.rs:1673-1683`) vs `IntentDeniedAtPeer` → `direction: Accept` (`:1684-1690`). The
     residual defect is semantic, not a conflation: `IntentDeniedAtPeer` puts the NACK **message**
     into a field named `intent`.
   - The **real loss is the unclassified pair** (`:1773-1782`), which collapses to stringly
     `CrossHostRouteFailure(String)` and discards the typed `UnclassifiedReason`
     (`Absent`/`NonCanonical`/`Oversized`) plus the direction.
   - **Cost in `maos-a2a-core` ≈ ZERO net lines** — two 5-line `format!` arms become two 5-line
     typed constructions. The new variant lands in `maos-domain::iac_bus_types` (`:14-40`) at ~+6,
     which rides with **D14** (14-7 already owes that crate an explicit AC expansion).
   **So no `maos-a2a-core` grant is required and D10 is not implicated.** Deadline **re-pinned to
   "before `j1-crosshost-2b` writes its first line"** — not a weakening, but the same rule applied to
   the right vehicle: **`2a` cannot surface a cross-host deny at all**, so the split unblocks it.
3. **`maos-a2a-core` is the wall.** Cross-host work naturally lands in `router.rs`/`mtls.rs`/
   `tofu.rs`/`identity.rs`. One production line hard-fails `kloc-check`. Mitigation that already
   worked for 1a: production code into `maos-bin`/`maos-a2a-tcp` (~415 spare), tests into
   `crates/*/tests/` and `xtask/tests/` (both kloc-excluded).
4. **`kloc-check` exits 1 at HEAD** on three keys (D13 `maos-kernel-core` +685, D14 `maos-domain` +50,
   D17 `_aggregate` +492). Rung 2 closes red through no fault of its own. **Do not absorb them; do not
   read it as rung-2 damage.**
5. **`cargo test -p maos-bin` is RED under default parallel flags** (D16, `MAOS_HOME` process-global
   isolation). Rung 2's daemon tests land in that suite. Run scoped; D16 belongs to 14-0.
6. **FLAG-E5 / FR21's 60s wall-clock window**: two `maos run --once` on the same `MAOS_HOME` inside
   60s → run 2 exits 1. Two hosts on one dev box hit this constantly. Use distinct data homes.
7. **`ABSENT`/`INDETERMINATE` cannot close J1 is PROSE, not a control.** `ledger_gates()` =
   `check_loom_substrate_drift::contract_jobs()` — the four Postgres substrate gates — so the J1 lane
   is **structurally outside** the evidence-ledger enforcement. `demo-j1` has **zero CI invocation**.
   `Beat::absent` sets `executed: false` and an unlanded beat never fails a run. If rung 2 wants that
   clause mechanical, it must build the linkage.
8. **`PROVEN_LIVE_SIGNED` has never been reached by any leg** — CI holds no operator key by design.
   It is an operator-lane claim; the signing harness is `sealed_export::sign_bundle` +
   `MAOS_AUDIT_KEY_SEED`.
9. **Vocabulary collisions — do not let one AC word cover three things.** `Indeterminate` means a CI
   gate-evidence state (`gate_common.rs:134`) *and* a transport probe result
   (`cohort/halt_receipt.rs:119`) — zero coupling, and **neither is a task outcome**. `correlation_id`
   is a real TL column *and* the name of a function that generates a `frame_id`
   (`mailbox.rs:68`). `bundle` names three unrelated types; `verify` names three operations, **none of
   which is two-log reconciliation.**
10. **Five story-file discipline gates skip this filename** (digit-prefix scoping) — verified for
    `j1-crosshost-2-…md`. Dev record, model tier and review-findings closure are on the honor system.
    A green CI does not mean the §A6 net ran.

---

## 6. Open questions for ratification

**Q1 — RESOLVED 2026-08-15: the three-way split is RATIFIED.** See §4. Next action is a
create-story pass on **`j1-crosshost-2a`**, which is not blocked.

**Q2 — RESOLVED 2026-08-15: the D18 paradox was an unmeasured premise.** See §5.2 — the fix costs
`maos-a2a-core` ≈ zero net lines, so D10's wall was never in the way, and the deadline is re-pinned
to `2b`.

**Q3 — What is the second host, physically?** Two processes on one box (CI-reachable, precedent
exists) or two real machines (matches "different host" literally, but no CI can hold it, and P9 means
bare `IP:port` with no DNS)? This changes AC shape and what the signed bundle can claim.

**Q4 — Does the FS-jail gap block the claim?** `claude` has no `--sandbox workspace-write`
equivalent and external jails are structurally blocked. Accept a narrower claim for the heterogeneous
half, or scope jail support?

---

## Dev Agent Record

### Agent Model Used
_(record `vendor/model` + harness + date — required by policy even though five gates skip this
filename; see §5.10)_

### Debug Log References
### Completion Notes List
### File List

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-15 | **Created.** The `sprint-status` row noted this story had no file and could not leave `backlog` without one. Authored from a four-agent preflight at `5a921c0c`. **Adapter viability RESOLVED: viable, no enablement story** — proven by executing a live `claude` worker end-to-end with manifest + grants only. That execution found a **ship-blocker**: `final_stdout_message_oracle` scores a permission refusal that exits 0 as `completed: true`, so a false completion exists on one host before any cross-host work. **14 premises disproved (P1-P14)**, including: the two-host substrate already exists in CI (P1); rung 1's boundary tripwire can never fire (P2, repair assigned to 1b); partition NACK is doc-only on the TCP wire (P3); the transport DOES retry up to 4× below the kernel while nothing dedups (P4); there is no task-outcome vocabulary at all and abnormal endings bypass the typed payload (P5); `HaltPresence::Indeterminate` is dead in production (P6); pin-mismatch journaling does not exist (P7); no durable TOFU pin store exists and the listen side accepts any active pin (P8); hostnames do not work (P9); T6's bundle swept 8 boots and renders 64% of entries `unknown` (P11); and a latent pubkey/region mismatch that per-host key derivation hits immediately (P12). **Peer-auth is already closed at the transport layer** — `handle_intake_verified` has one production caller and it is on the TCP path, exercised 50× per push with zero ignores — so rung 2 is composition, not construction. **SPLIT PROPOSED into 2a/2b/2c** (signable worker / cross-host mechanism / signed run). Status `blocked`: 1b is not done, D18's deadline precedes this story's first line and its fix has no budget, and the split needs ratification. |
