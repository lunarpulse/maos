# Runbook — J1 Tier-2 Signed Live-Agent Run (Founder's Loop)

> **Purpose.** The operator (human) procedure to close the **Tier-2** gate in
> `release-gate-8-12-tier-2-cli-wrapper.md`: one real codex Developer-Worker,
> delegated through `maos run`, observed and **signed**. This is **T6** of
> `spec-j1-tier-2-live-agent-demonstration.md` — the one task no test can close.
>
> **Audience.** Lunarpulse (runs + signs). Devs also read it: **this runbook IS
> the acceptance target for T1–T5** — every `[T-task adds]` command below must
> become real before the run.
>
> **Command legend:** `[exists]` works today · `[T# adds]` lands with that task.

---

> **Executing T8 (the paid two-host run)? Use
> [`runbook-j1-t8-two-host-paid-run.md`](runbook-j1-t8-two-host-paid-run.md) instead.**
> That document is the **linear, verified execution sequence** — every command in it was executed on
> 2026-08-22 with a fake `claude` fixture, through the crossing, the worker spawn, a shared `frame_id`
> in both Transparency Logs, both signatures verified and `reconcile-hosts` green. It also carries the
> **seven traps** that procedure hit and that were documented nowhere.
>
> THIS document remains normative for **judgment**: what was wrong and why, what the release may not
> claim, and the abort conditions. It is a record, not a checklist.

## Preconditions (before you touch a key)

- [ ] **T1–T5 landed and green** on the pre-Epic-13 bridge branch (`cargo test --workspace --locked`, discipline gates, `check-kernel-baseline` @23147). `[T1–T5]`
- [ ] **§A6 seal done** — Murat's live drill (planted codex crash reds the completion leg), Winston kernel-baseline, Amelia compiler/measurement, Vex redaction+egress. *You sign on top of their seal, not instead of it.*
- [ ] You have decided the **c2 task** (below) and an **exact spend ceiling** (c1).

---

## Phase 0.0 — The `maos run` invocation contract (read once; it costs nothing)

Every `maos run` line in this runbook takes a **manifest path**, and nothing in the CLI helps you
find it. This is not a hypothetical: an operator following the two-host guide substituted the literal
string `./topology/a` for a `<topology>` placeholder and got a failure only **after ~30 lines of
successful initialization** — including `A2A delegation leg installed` — which reads as a late,
deep-in-the-system fault rather than a bad argument.

```
maos run <manifest> [--live] [--once]
```

- **There is no `maos run --help`.** `--help` is parsed as the manifest and you get
  `maos run: unknown argument '--help' — expected: <manifest> [--live] [--once]`.
- **The manifest is read LAST**, after the crypto provider, capability registry, Transparency Log,
  memory tiers, IAC bus, A2A delegation leg and Spirit Scheduler are all wired. A wrong path
  therefore prints a full healthy boot and *then* `failed to read manifest …`. **Init lines are not
  evidence that your argument was accepted.**
- **Run from the repository root.** Paths are relative to the working directory; a `--live` codex run
  is the one exception and needs an absolute path because it `cd`s into `$DEMO` first (Phase 3).

| Role | Exact path |
|---|---|
| Loopback / falsifier / fixture mechanics | `spirits/topologies/j1-founder-loop.toml` |
| Live codex single-host (Phase 3) | `spirits/topologies/j1-founder-loop-codex.toml` |
| **Cross-host, both hosts (Phase 7)** | `spirits/topologies/j1-founder-loop-crosshost.toml` |

⚠ `j1-founder-loop.toml` is **pinned by two Blocking controls and must not be edited.** If you need a
variant, copy it.

### Troubleshooting Phase 0

| Symptom | Cause | Fix |
|---|---|---|
| `failed to read manifest ./topology/a: No such file or directory` | placeholder taken literally; the directory is `spirits/topologies/` and files are `.toml` | use a path from the table above |
| `unknown argument '--help'` | `run` has no help; the first positional IS the manifest | see the signature above |
| `orchestrator dispatch references raw worker output not a distillate` on the **second** run | state accumulated in the same `MAOS_HOME`; a real pre-existing defect, unrelated to whatever you were testing | give each run a fresh state home (`H=$(mktemp -d)`) |
| `journal: WARNING — skipping corrupted line …` | a prior daemon died mid-write in your **real** state home | harmless here, but it is why Phase 0 checks should run in a throwaway home; rotate the journal if you want it clean |
| `maos not found at target/debug/maos` | you passed `--skip-build` without building | drop the flag, or `cargo build --workspace` |

---


## Phase 0 — Operator decisions & keys (do now; no code needed)

**0.1 — Fix the c2 task.** Write the one bounded, non-destructive-to-the-repo, story-sized task. It runs in a disposable demo dir with full CRUD *inside that dir only*.

- Task text: `__________________________________________________`  *(e.g. "scaffold a tiny Rust CLI in ./: add main.rs + a passing test, run it, then delete the scratch file")*
- Spend ceiling (c1): `$______`  · one-shot, metered API key.

**0.2 — Three secrets, three lives (Vex's rule — never mix):**

| Secret | Role | Where it lives |
|---|---|---|
| `CODEX_API_KEY` | codex **Worker** spends it | inherited host-side into sandbox env; denylisted; scrubbed. **NOT `OPENAI_API_KEY`** — `codex exec` ignores that var and reads `CODEX_API_KEY` (codex-rs `login/src/auth/manager.rs:1226` + `exec/src/lib.rs:571`). Set `CODEX_API_KEY="$OPENAI_API_KEY"`. |
| Orchestrator provider key (e.g. `ANTHROPIC_API_KEY`) | the class Spirits' **reasoning** under `--live` | MAOS provider port; host-side |
| Operator **audit key** (Ed25519) | **you sign** with it | host-only; **never enters the sandbox** |

**0.3 — Generate the signer key (once) and publish its fingerprint:**

```
maosctl audit keygen --output ~/.maos/keys/j1-tier2-signer.key   # [exists]
# prints: key written to … (fingerprint: <FPR>)
```

- [ ] Record `<FPR>` somewhere public ("Lunarpulse signed J1 Tier-2 with `<FPR>`"). Verifiers need it. Run `maosctl audit sealed-export --help` and `maosctl audit keygen --help` to confirm exact flag spellings on your build. `[exists]`

---

## Phase 1 — Clean environment

**1.1 — Fresh worktree (preserve the dirty `epic-12` tree):**

```
git worktree add ../maos-j1-live j1-tier2-live-agent-signed-bridge   # [exists]
cd ../maos-j1-live
```

**1.2 — Disposable demo dir, isolated so a delete cannot reach the repo:**

```
DEMO=$(mktemp -d /tmp/maos-j1-demo.XXXXXX)   # outside the repo tree on purpose
```

- The T3 host grant scopes the Worker's `fs.rw` to **exactly `$DEMO`** — CRUD inside is a feature; anything outside fails closed. `[T3]`

**1.3 — Clean-home invariant (the auth.json footgun):**

- [ ] Confirm **no ambient `~/.codex/auth.json`** in the sandbox home — a live ChatGPT session shadows the injected `OPENAI_API_KEY` and leaves an un-scrubbable token. The run must **refuse or wipe** it, never inherit. `[T5 negative test proves it]`
- [ ] `codex --version` present and pinned; record it (it becomes the "live-agent identity" in the capture).

**1.4 — codex sandbox prerequisite (bubblewrap / user namespaces):** `[found 2026-07-15]`

- [ ] codex's OWN sandbox (`--sandbox workspace-write`) uses **bubblewrap**, which needs **unprivileged user namespaces**. On a hardened host / default Docker you'll see `bwrap: No permissions to create a new namespace` and **every codex write fails**. Enable it at the *container/host* (e.g. `sudo sysctl -w kernel.unprivileged_userns_clone=1`, or run the container `--privileged` / with userns allowed), then re-test **1.5**. This is an environment prerequisite, **not** a MAOS setting.
- [ ] **Do NOT** "fix" it with codex `--sandbox danger-full-access` for the *signed* run — that removes codex's FS jail, and MAOS's T3 FS-scope is *declared-not-enforced* at v0.1, so the c2 demo-dir bound would be enforced by **nothing**. The signed run requires codex's workspace-write jail to actually work.

**1.5 — Pin the exact codex invocation that WRITES to `$DEMO`, standalone:** `[found 2026-07-15]`

```
cd "$DEMO" && git init -q .
RUST_LOG=error codex exec --sandbox workspace-write "write hello to ./hello.txt"
#   PASS = hello.txt exists, exit 0, a final message on stdout.
#   If it fails on bwrap → fix 1.4 first. Then transcribe the EXACT flags into
#   spirits/worker/manifest-codex.toml `argv_prefix` (it is TOCTOU-hashed).
```

> **MAOS admission note (fixed 2026-07-15):** real CLIs do NOT implement MAOS's
> `--maos-bridge-probe` output-shape handshake (only the fixture does), so the
> codex worker is admitted by a **liveness probe** (`codex --version` exits 0) +
> the T3 floor; the output shape is verified at *completion* by the codex
> adapter. Your `manifest-codex.toml` needs no probe handler.

> **MAOS stdin note (fixed 2026-07-15, kernel):** `codex exec` reads its prompt
> from argv **and** keeps reading **stdin-until-EOF**. The bridge used to hold
> the worker's stdin open, so codex hung at `Reading additional input from
> stdin…` (`ps`: `Sl+`, 0 % CPU) while the bridge waited for output — a deadlock.
> The kernel now **closes the worker's stdin** for a `Signals`-driven worker
> (the codex/fixture path) so codex gets EOF and runs on its argv prompt. **No
> `< /dev/null` shim needed** on a `maos` built from `epic-13` at `0a03468f` or
> later. If you see the hang, your `maos` binary predates the fix — rebuild
> (`cargo build --workspace`).

---

## Phase 2 — Dry run (fixture mechanics OR a direct codex sanity check)

**Important:** a *subscription* codex run **through the bridge is refused** — `MAOS_LIVE_AGENT=1` + codex triggers the clean-home refusal whenever `~/.codex/auth.json` exists, and it's the API-key path that Tier-2 requires. So the free dry run is one of:

- **Fixture mechanics (no codex, no cost):** run the hermetic worker to watch the topology/bridge/serving-loop. Needs `worker-cli-fixture` on PATH — easiest is to run the freshly-built binary so it's a daemon-sibling:
  ```
  cargo build --workspace
  ./target/debug/maos run spirits/topologies/j1-founder-loop.toml --once
  ```
- **Direct codex sanity check (subscription OK):** just `codex exec …` outside `maos` (Phase 1.5) — confirms codex itself works; nothing signed.

- [ ] Fixture path: Orchestrator loads Architect + Reviewer + the worker; a typed task routes; the worker subprocess is real (`child_pid`); `worker_completion completed=true`; clean drain.

---

## Phase 3 — The signed live run (API key; the real thing)

**Two "live" axes — set both:** `--live` = real provider for the Spirits' *reasoning*; `MAOS_LIVE_AGENT=1` = real *codex subprocess* instead of the fixture.

```
export CODEX_API_KEY="$OPENAI_API_KEY"   # codex worker — metered, capped, revocable (the child INHERITS this; MAOS never reads/holds it).
                                         # MUST be CODEX_API_KEY: `codex exec` IGNORES OPENAI_API_KEY for auth (→ 401 Missing bearer).
export ANTHROPIC_API_KEY=…         # (or your configured provider) — Orchestrator reasoning
export MAOS_LIVE_AGENT=1           # [lands] permit the real agent subprocess (CI never sets this → CI cannot spawn a paid agent)
export MAOS_HOST_GRANTS=~/.maos/host-grants.toml   # [lands] operator grant for codex (see below) — without it, codex fails closed

#   ~/.maos/host-grants.toml must contain:
#     [[grant]]
#     attested_image = "codex"          # the manifest's [cli_wrapper] command
#     signing_key_id = "OpenAI"         # the manifest's [author] name
#     permitted_tier = "T3"
#     permitted_egress_destinations = ["api.openai.com"]
#   AND a codex worker manifest (command="codex", argv_prefix=["exec","--sandbox","workspace-write"])
#   referenced by the topology (swap the fixture worker entry).

#   Clean-home invariant: MAOS REFUSES the live run if ~/.codex/auth.json exists.
#   (CODEX_API_KEY actually takes precedence OVER auth.json in codex, so this is
#   not about shadowing — it is to keep an un-attestable subscription token out of
#   the signed run's sandbox entirely.) Wipe it first.
#   Run maos FROM $DEMO so codex inherits cwd=$DEMO (workspace-write binds writes
#   to cwd) — otherwise codex writes into the launch dir, breaking the c2 bound.

cd "$DEMO" && maos run <abs>/spirits/topologies/j1-founder-loop-codex.toml --live   # codex topology, NOT the fixture one
#   continuous service; use safe shutdown (Ctrl-C, not --once). NOTE: the full
#   halt/resume digest-citation is a DEFERRED seam (FOLLOWUP-J1-RESUME-SEAM) — not
#   a gate for this run; continuous service + safe shutdown ARE verified.
#   PROVEN 2026-07-15: worker_completion completed=true, exit 0, completion_tl_ref set.
```

- [ ] Delegation → codex executes the c2 task in `$DEMO` → completion **parsed by the adapter** (codex: final line on stdout), never inferred from exit code.
- [ ] Digest cites the Worker-produced Transparency Log reference through the distillate chain.
- [ ] (Ideal) a halt/resume: post-resume digest contains the exact pre-halt typed ref; no in-flight delegation preempted.
- [ ] Revoke the OpenAI API key (`CODEX_API_KEY` / `OPENAI_API_KEY`) when done.

---

## Phase 4 — Capture the evidence (non-secret only)

Write the capture doc as a **JSON file** (e.g. `./j1-tier2-capture.json`). Phase 5 journals it as a `run.capture` audit row (`maosctl audit record-capture`), so the sealed-export signature covers it. `record-capture` **validates these fields fail-closed** and **refuses any capture carrying a credential-shaped value** — so use the exact keys below (non-secret only):

> ⛔ **The template printed here until 2026-08-22 was INVALID and would have been refused at Phase 5a — after the agent was billed.** It omitted `fs_jail` and `fs_jail_followup`, both required since `j1-crosshost-2a` (`crates/maos-cli/src/subcommands.rs:2479`, value-constrained at `:2504-2514`). Reproduced by feeding the old template — which is byte-identical to the shipped T6 capture `_bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-capture.json` — to the real CLI:
>
> ```
> $ maosctl audit record-capture --capture ./old-phase4-template.json
> maosctl: audit record-capture — capture field `fs_jail_followup` is required and must be non-empty   # exit 2
> ```
>
> The T6 capture is not retroactively invalid; it was written before those fields existed. **This template was.**

```json
{
  "signer": "<your name> (named human signer)",
  "live_agent_identity": "<adapter> <version> (e.g. \"Anthropic Claude Code v2.1.235 (adapter ClaudeCli)\")",
  "command_metadata": "<argv WITH the key redacted>; <PROVIDER>_API_KEY injected host-side (value redacted)",
  "host_grant_disposition": "exact-match host grant admitted (attested_image=<cli>, signing_key_id=<author>, tier T3); a mismatch would have refused",
  "audit_refs": ["<audit TL ref>", "<digest TL ref the digest cited>"],
  "egress": "declared-not-enforced",
  "egress_followup": "FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT",
  "fs_jail": "adapter-enforced-maos-declared",
  "fs_jail_followup": "FOLLOWUP-EPIC14-MAOS-ENFORCED-WORKER-ISOLATION",
  "redaction_result": "verified",
  "outcome": "<worker completed; no secret persisted; digest cites the worker ref>"
}
```

- [ ] agent identity + version → `live_agent_identity`
- [ ] non-secret command metadata (argv **with the key redacted**) → `command_metadata`
- [ ] host-grant disposition (exact-match grant admitted) → `host_grant_disposition`
- [ ] audit + digest Transparency Log refs → `audit_refs` (≥1 required)
- [ ] `egress` **must be exactly** `declared-not-enforced` + `egress_followup` ID (enforced egress = Epic-14 v2.0 hardening; claiming "enforced" is refused)
- [ ] `fs_jail` **must be exactly** `adapter-enforced-maos-declared` + `fs_jail_followup` ID — the jail is the ADAPTER's, DECLARED by MAOS in a hashed `argv_prefix`; claiming `maos-enforced` is refused
- [ ] `redaction_result` **must be exactly** `verified` (the injected key's value is absent from the TL — MAOS held it, so it can prove this)
- [ ] `outcome` (+ the row's timestamp is stamped automatically). Extra fields you add are preserved verbatim **in this document** — but see the warning below, and note that `two-host-capture.json` (Phase 7.7) is a *different* file with the opposite rule.

> ⚠ **The `sk-` substring landmine — this refusal happens AFTER the agent is billed.**
> The credential tripwire is a raw **substring** search (`contains_prefix`, `crates/maos-iac/src/adapter/redaction.rs:291-297`), not a token match, and the refusal at `crates/maos-cli/src/subcommands.rs:2598-2606` **journals nothing**. Any field containing `sk-` is refused as `class: api_key_generic` — and `task-`, `risk-`, and `disk-` all *contain* `sk-`. Measured 2026-08-22 against the real CLI:
>
> | Field value fragment | Result |
> |---|---|
> | `codex exec --task-file /tmp/goal.txt …` | **REFUSED** (`api_key_generic`) |
> | `residual egress risk-accepted by the signer` | **REFUSED** (`api_key_generic`) |
> | `disk-backed TL flushed before export` | **REFUSED** (`api_key_generic`) |
>
> **T6 escaped only by luck** — its `command_metadata` reads `<c2 task>`, i.e. `task>` and not `task-`.
> **Dry-run your exact intended capture against a scratch Transparency Log before you spend anything.** Any home with a real TL works; `record-capture` opens it `READ_WRITE`, never `CREATE`:
>
> ```bash
> export MAOS_HOME=$(mktemp -d) XDG_DATA_HOME=$MAOS_HOME
> maos run spirits/topologies/j1-founder-loop.toml --once   # produces the scratch TL
> maosctl audit record-capture --capture ./j1-tier2-capture.json
> ```
>
> ⚠ **Set `two_host_shape` in the dry run** (Phase 7.6) or `validate_two_host` is skipped **entirely** (`subcommands.rs:2528-2574`) and the block you most need to exercise never runs. Measured: with `two_host_shape` empty, a capture asserting `two_host_trust_anchor: "protocol-negotiated"` and `two_host_host_b_audit_key: "derived-from-shared-root"` — **both of them the explicitly-refused overclaim direction** — was accepted and journaled, exit 0.

---

## Phase 5 — Journal the capture, then sign (sealed-export = the signature)

**5a — journal the capture as an audit row** so the signature covers it `[record-capture — LANDED dev 2026-07-15; host-level path PROVEN 2026-07-16]`:

```
maosctl audit record-capture --capture ./j1-tier2-capture.json     # [exists — J1 Tier-2]
#   NO --spirit: the v0.1 `resolve_spirit_name` (maos-audit) accepts ONLY
#   `hello-spirit`, so `--spirit orchestrator` (or worker/etc.) is REJECTED at
#   v0.1. Omit --spirit → a host-level attestation (pid/boot = 0), which is the
#   correct posture anyway (an operator/host run attestation). Its audit_refs
#   already cite the worker's completion_tl_ref, so the worker linkage is IN the
#   doc. Cover it with `sealed-export --range <window>` (5b), NOT --spirit.
```

This validates the Phase-4 fields, **refuses** a capture that carries a credential or overclaims a control (egress "enforced", redaction not "verified"), and writes a `run.capture` row (host-level). It prints the row's `frame_id` (e.g. `journaled run.capture d301a233…`).

**5b — sign the covered window (time-range, not --spirit):**

```
maosctl audit sealed-export --range 1d \            # [exists — FR44]
  --audit-key ~/.maos/keys/j1-tier2-signer.key \
  --output ./j1-tier2-bundle.json
#   --range covers the whole run window (worker rows + the fresh capture row).
#   Widen (7d/30d) if the run is older. Prints entry count + the pubkey hex.
#   (sealed-export --spirit would ALSO hit the hello-spirit-only limit — use --range.)
```

**5c — verify the signature (the gate close):**

```
maosctl audit verify-bundle ./j1-tier2-bundle.json --pubkey <FPR>
#   Expect: "audit verify-bundle — OK (<N> entries, seq <n>)".
```

> ⚠ **`<FPR>` is NOT the fingerprint `keygen` printed.** `maosctl audit keygen` prints a
> **truncated** form — first 8 and last 8 hex, joined by `..`
> (`crates/maos-domain/src/audit_key.rs:397-405`) — and `verify.py:135-142` rejects it outright.
> The full **64-hex** pubkey is printed only by `sealed-export`, on **stderr**
> (`crates/maos-cli/src/subcommands.rs:2302-2307`). Measured 2026-08-22 on one key:
>
> ```
> $ maosctl audit keygen --output signer.key
> maosctl: audit keygen — key written to signer.key (fingerprint: 66dce5f2..91a2dfce)   # TRUNCATED — not usable
>
> $ maosctl audit sealed-export --range 1d --audit-key signer.key --output b.json
> maosctl: sealed export written to b.json (19 entries,
>   pubkey 66dce5f2ec2bc9d8a567dd77ef9de2e66ae8cc4fd904e91522a1afba91a2dfce)         # ← THIS is <FPR>
> ```
>
> ⚠ **`keygen` also ignores `MAOS_AUDIT_KEY`** (`audit_key.rs:101-106`): with the variable set to an
> existing key it still mints a *fresh, different* key and prints that one's fingerprint. Measured —
> the same command with `MAOS_AUDIT_KEY=signer.key` printed `ef9dec0f..72b97221`. If you publish a
> fingerprint read from `keygen` while signing with a key selected by `MAOS_AUDIT_KEY`, **every
> verification a stranger attempts will fail**, and it will look like your signature is bad.

> **How the capture is covered (dev 2026-07-15 — journal-capture LANDED):** `sealed-export`
> writes **ONE self-contained signed JSON bundle** (`--output` is a FILE) — a canonical bundle
> of the covered **audit entries** signed with Ed25519 over `sha256(canonical)`, signature
> embedded. It is **NOT** a `SHA256SUMS` + separate `.sig` file set (that shape is the
> *offline-import* verify path). Because it signs audit **rows**, the capture is covered by
> running **5a first**: `record-capture` journals the capture as a `run.capture` row, so a
> `sealed-export --range <window>` covering the run signs it alongside the worker's
> `CliSubprocessOutput` + `host_grant_disposition` + `worker_completion` rows. (Earlier drafts
> said this wiring was OPEN — it landed 2026-07-15. **PROVEN 2026-07-16:** `record-capture`
> → `run.capture d301a233…`; `sealed-export --range 1d` → 247 entries incl. the capture row +
> worker completion `019f67ef…`, pubkey `61f4f495…`.)

- [ ] `record-capture` accepted the capture and printed a `run.capture` frame_id (a refusal here means the capture overclaims or carries a secret — fix it; the gate stays open).
- [ ] Signed bundle produced: one JSON file with an embedded Ed25519 signature over the audit entries.
- [ ] **Verify it yourself** before recording — the verify path must pass against `<FPR>`. If it doesn't verify, the gate stays open.

---

## Phase 6 — Record & close the gate

Edit `_bmad-output/test-artifacts/release-gate-8-12-tier-2-cli-wrapper.md`:

- [ ] **Named owner:** Myoungki Jung (Lunarpulse)
- [ ] **Signed artifact path:** `./j1-tier2-signed-bundle/`
- [ ] **Date:** `<YYYY-MM-DD>`
- [ ] Check the five Tier-2 boxes **only on observed evidence**; commit the bundle + gate on the bridge branch.
- [ ] Flip `sprint-status.yaml` `j1-tier2-live-agent-signed-bridge: backlog → done`; merge the bridge before `13-1`.

---

## Abort conditions — ANY of these → Tier-2 stays OPEN, do not sign

- A secret value persisted anywhere in the Transparency Log or capture.
- The Worker created/deleted **outside `$DEMO`** (capability scope escape).
- The signed run used **subscription / `~/.codex/auth.json`** auth (redaction unattestable).
- An ambient `auth.json` was inherited into the sandbox.
- A raw process exit was treated as task completion.
- The sealed-export signature does not verify against `<FPR>`.

---

## What a skeptic re-runs to trust your signature

1. Fetch the bundle + your published `<FPR>`.
2. `maosctl` verify (signature → SHA256) → must pass.
3. Read the capture doc: real codex identity, host-managed grant, resolving digest→TL citations, `egress: declared-not-enforced`, redaction verified, named human signer.

---
## Phase 7 — The TWO-HOST run (`j1-crosshost-2c`)

Everything above is **one host**, codex only. This phase extends it to two hosts and
a heterogeneous worker. It is a different claim with different failure modes, and it
has **two steps no protocol performs for you**.

> ⚠ **`claude` appears zero times in Phases 0–6.** Host B runs the non-Codex adapter.
> `ClaudeCli` is proven viable end to end (`j1-crosshost-2` preflight: a live `claude`
> worker through `maos run` with a manifest + `MAOS_HOST_GRANTS`, zero repo changes),
> but nothing before this phase ever pointed a runbook at it.

### 7.0 — Manual boot-nonce pairing on a RELEASE build

> ✅ **REPAIRED AND EXECUTABLE as of `j1-crosshost-2e` AC5 (2026-08-22).** Kept in full because the
> diagnosis explains why the new steps are shaped the way they are.
> **What was wrong:** `cohort:daemon-started` is written **only in daemon mode**
> (`crates/maos-bin/src/main.rs:9381`, emitted `:9548-9555`). Host A is `maos run … --once`, which
> takes the cross-host arm *precisely because* `MAOS_ONE_SHOT != "cohort-a2a-daemon"` (`:2455`).
> **Host A never emitted the row this procedure said to read** — it is the *sender*; that row belongs to
> a *receiver*. "Run host A as a daemon instead" is not a fix either: daemon mode sets the cross-host
> router to `None`, so the arm you are testing is gone. Worse, the nonce did not exist before the dial:
> minted at `:1865-1878`, transport binds at `:2454-2471`, delegation emitted at `:3237-3270` — all in
> one process, with no pause. And there is **no retry window to wait in**: a refused or timed-out
> `TcpStream::connect` returns `Io` **immediately**, because `is_retryable` admits only
> `BadCertificate`/`CertExpired` (`crates/maos-a2a-core/src/mtls.rs:73-83` — note the crate; there is
> **no** `maos-a2a-tcp/src/mtls.rs`, a path earlier revisions cited); the `[100, 300, 1000] ms` ±20%
> backoff (`:12-28`, ~1.4 s total) is the **cert-class retry** schedule, not a startup grace period.
> **This is why publishing alone was insufficient and a bounded HOLD was required** — see 7.1.

**Why a release build matters.** `main.rs:1865-1878` gates the `MAOS_TEST_BOOT_NONCE` override behind
a **runtime** `cfg!(debug_assertions)`, so in release the nonce is always random. `2b`'s two-daemon
harness uses the debug shortcut, so the documented operator path was never exercised by any test.

**Prove your binary is genuinely release-built first.** `debug_assertions` is a codegen flag, not the
profile: `RUSTFLAGS="-C debug-assertions=yes" cargo build --release` re-enables the shortcut silently,
and `check-mock-not-in-release` cannot see it (it greps the symbol table for
`MockHaltResolver`/`FailingHaltResolver` only, `xtask/src/check_mock_not_in_release.rs:31`).

```bash
# Build BOTH. release.yml:44 builds `maos-bin` only, so a downloaded release artifact
# ships NO maosctl — and maosctl is the only read-back tool.
cargo build --release -p maos-bin -p maos-cli

# The falsifier: a LOOPBACK double boot. No daemon, no cohort manifest, no cost.
export MAOS_TEST_BOOT_NONCE=424242
for i in 1 2; do
  export MAOS_HOME=$(mktemp -d) XDG_DATA_HOME=$MAOS_HOME
  ./target/release/maos run spirits/topologies/j1-founder-loop.toml --once
  ./target/release/maosctl audit query --range 1d --format ndjson | jq -r '.boot_nonce' | sort -u
done
#   TWO DISTINCT values, neither == 424242  ⇒ genuine release build. Proceed.
#   ONE value == 424242                     ⇒ debug assertions are ON. STOP; do not spend.
```

Executed 2026-08-22 (`j1-crosshost-2d` AC1): release read back `9046754445710571789` then
`1928460524043859277`; the **debug** control read back `424242` twice. The check discriminates.

```bash
# ── HOST A (the SENDER) ────────────────────────────────────────────────────────
# Do NOT run host A as a daemon. `MAOS_ONE_SHOT=cohort-a2a-daemon` makes it a
# RECEIVER and sets the cross-host router to None — the arm you are testing
# disappears, and its `cohort:daemon-started` row is a receiver's row.
#
# Host A publishes its OWN nonce under `cohort:crosshost-started` after the
# transport binds and before the dial, then HOLDS so you can transcribe it.
export MAOS_CROSSHOST_PAIRING_READY_FILE=/tmp/host-b-ready
export MAOS_CROSSHOST_PAIRING_TIMEOUT_SECS=600     # default 300; expiry FAILS CLOSED
rm -f /tmp/host-b-ready                            # must NOT pre-exist

MAOS_HOME=$HOST_A_HOME \
MAOS_HOST_GRANTS=/abs/path/host-a-grants.toml \
MAOS_COHORT_DAEMON_CONFIG=/abs/path/host-a-crosshost.toml \
MAOS_LIVE_AGENT=1 \
MAOS_DELEGATED_GOAL="<the concrete task the remote worker must perform>" \
  ./target/release/maos run spirits/topologies/j1-founder-loop-crosshost.toml --once

# Host A prints, in DECIMAL, and then waits:
#   maos: cross-host sender ready — boot_nonce 6900850299039067033 (decimal). …
#   maos: pairing rendezvous — holding up to 600s for /tmp/host-b-ready …

# Machine path for the same value, from a SECOND shell. The flag is
# `--intent-contains`, NOT `--intent` (crates/maos-cli/src/cli.rs:376-378).
MAOS_HOME=$HOST_A_HOME ./target/release/maosctl audit query \
  --frame-kind TelemetryEvent \
  --intent-contains cohort:crosshost-started --format ndjson
```

> ⚠ **Never read a boot nonce with `--format plain`.** That renderer prints the value as `{:016x}`
> under a column header literally named `boot_nonce` (`crates/maos-audit/src/lib.rs:808`), and the
> TOML field you paste it into is parsed as **decimal**. Most hex contains `a`–`f`, so TOML usually
> rejects it loudly — but an **all-decimal-digit hex string parses silently as a different number**
> and fails only at the first inbound frame, as a nonce mismatch you will misdiagnose as a pinning
> error. It is a human-readable format that is only readable by a machine that knows the base. Use
> `--format ndjson`.

Hand-transcribe the nonce into host B's static peer-pin config. **There is no automated channel**, and
RELEASE-HOLDS row 9 records this as a boundary. The file is the daemon TOML named by
`MAOS_COHORT_DAEMON_CONFIG` — **not `a2a-peers.toml`, which exists nowhere in this repo.** The real
surface is `[[tcp.peer_pins]]` (`crates/maos-a2a-tcp/src/config.rs:25-37`), and `PinnedFingerprint` is
`#[serde(deny_unknown_fields)]` with **all three keys required** — an omitted `boot_nonce` is a parse
error, not a default:

```toml
[[tcp.peer_pins]]
peer_id     = "host-a"                  # the operator-declared peer identity
fingerprint = "<host A's SHA-256 leaf-cert fingerprint>"
boot_nonce  = 9046754445710571789       # DECIMAL — transcribed from ndjson, never from plain
```

Then start host B and **release the hold**. Host A is still waiting, so unlike every previous revision
of this runbook there is now a window in which both of these can happen in order:

```bash
# ── HOST B (the RECEIVER) ─────────────────────────────────────────────────────
MAOS_ONE_SHOT=cohort-a2a-daemon \
MAOS_HOME=$HOST_B_HOME \
MAOS_AUDIT_KEY=$HOST_B_HOME/audit-signing.key \
MAOS_COHORT_DAEMON_CONFIG=/abs/path/host-b-daemon.toml \
  ./target/release/maos run spirits/topologies/j1-founder-loop-crosshost.toml

# Once host B is listening WITH the nonce above pinned, release host A's dial:
touch /tmp/host-b-ready
```

Host A prints `pairing rendezvous — host B signalled ready, dialling` and proceeds. **If you never
create the file, host A exits non-zero and never dials** — deliberately: `--once` has exactly one
non-retryable connect attempt, so refusing to spend it blind is the only safe failure.

**If the transcription is wrong the dial fails closed — but read the two asymmetries below before you
conclude anything from the logs.**

> ⚠ **Asymmetry 1 — a `-32004` nonce refusal writes NO Transparency-Log row on host B.** Only the
> sender learns. `crates/maos-a2a-tcp/src/transport.rs:715/750/1018` journals the **TLS-handshake**
> fingerprint mismatch on both sides, and Phase 7.0 previously generalized that into *"both sides
> journal a `PeerIdentityUnverified` `ConsentRupture`"* — **which is wrong for the nonce case.**
> Querying host B for `ConsentRupture` and finding nothing does **not** mean host B was never dialled.
>
> ⚠ **Asymmetry 2 — the refusal is permanent and cascading.**
> `invalidate_if_boot_nonce_differs` (`crates/maos-a2a-core/src/tofu.rs:351-372`) **invalidates the
> pin**, so the *second* attempt fails with a **different and misleading** error — you will chase a
> pinning bug that your first attempt created. Recovery requires **restarting host B**, not editing
> the config again.

> ⚠ **If the run stalls instead of failing** — no error, no progress — the rendering you will get is
> untyped: `TransportFailed("awaiting response")` with **no frame id**. The read phase is bounded by
> `idle` rather than the operator partition window
> (`crates/maos-a2a-tcp/src/transport.rs:602-603`), so this message **does not distinguish a network
> partition from a slow-but-live agent**. Typing it is deferred work owned by
> `14-4-v2-0-sweep-operational-surfaces` (see `deferred-work.md`); until then, treat it as "unknown",
> not as "partition".

### 7.1 — Give the two hosts INDEPENDENT audit roots

**Do not derive host B's key from host A's.** The region→team derivation template
exists to make keys derivable from ONE base seed — the exact property a two-host
claim must disprove. A welded per-host key would let one seed holder legitimately
sign **both halves**: valid signatures, host field inside them, a perfect "two-host"
bundle produced by one machine.

```bash
maosctl audit keygen --output $HOST_A_HOME/audit-signing.key
maosctl audit keygen --output $HOST_B_HOME/audit-signing.key
test "$(cat $HOST_A_HOME/audit-signing.key)" != "$(cat $HOST_B_HOME/audit-signing.key)" \
  || { echo "ABORT: one root cannot attest two identities"; exit 1; }

# Read each key's FULL 64-hex pubkey — keygen prints only a TRUNCATED fingerprint (see 5c).
# sealed-export prints the real thing on STDERR.
maosctl audit sealed-export --range 1d --audit-key $HOST_A_HOME/audit-signing.key --output /tmp/probe-a.json
maosctl audit sealed-export --range 1d --audit-key $HOST_B_HOME/audit-signing.key --output /tmp/probe-b.json
```

> **Publish `FPR_A` and `FPR_B` BEFORE the run, to a named file in the repository:**
> **`_bmad-output/test-artifacts/j1-two-host-evidence/PUBLISHED-FINGERPRINTS.md`.**
> This instruction previously said "publish FPR_A/FPR_B" and named no file, which in practice meant
> reading the pubkeys out of our own `sealed-export` *after* the bundles existed — that proves the
> bundle is internally consistent with whoever signed it, and nothing about identity. Committing the
> fingerprints first is what converts the stranger's later check into a real one, because the
> commitment predates the artifact in git history. The two roots for this lane are already published
> there (`j1-crosshost-2d` AC4.1, 2026-08-22); **use those keys, or update the file before the run and
> never after it.**

> `2b`'s two-process-one-box shape defaults to **one HOME and therefore one key
> file**. The mechanism proof and the signed proof want *opposite* setups; this step
> is what separates them. ⚠ And `MAOS_HOME` does **not** redirect the audit signing key
> (`crates/maos-domain/src/audit_key.rs:88-118`) — two `MAOS_HOME`s alone still share one key file.

### 7.2 — Run the crossing, then export ONE half per host

> **Phase 7 named none of the cross-host substrate until 2026-08-22.** Everything in this box is
> required and none of it appeared anywhere in Phases 0–7.
>
> | What | Value | Why |
> |---|---|---|
> | Host A topology | `spirits/topologies/j1-founder-loop-crosshost.toml` | ships at HEAD; `j1-founder-loop.toml` is pinned by two Blocking controls and **must not be edited** |
> | Host B worker | `worker_manifest = "spirits/worker/manifest-claude.toml"` in `MAOS_COHORT_DAEMON_CONFIG` | a top-level key of the daemon TOML (`crates/maos-bin/src/main.rs:8874`), **not** a topology key. Absent ⇒ **no intake sink is installed**, and a verified frame is ACKed and silently dropped (`:9327-9329`). |
> | Host grant | `attested_image = "claude"`, `signing_key_id = "Anthropic"`, `permitted_tier = "T3"` | must equal `[author].name` in `manifest-claude.toml:107` or admission refuses |
> | Provider key | `ANTHROPIC_API_KEY` | host B's claude worker spends it. Named nowhere in Phases 0–6, which are codex-only. |
> | Audit key | `MAOS_AUDIT_KEY` **or** `--audit-key` | **zero occurrences across all three J1 docs before today**, yet it is required by `verify_capture_signature` and is the default for `--receipt-key` |
>
> ⚠ **`MAOS_HOME` does not redirect the audit signing key** (`crates/maos-domain/src/audit_key.rs:88-118`) —
> two `MAOS_HOME`s still share **one** key file, which is precisely the `key_a == key_b` refusal 7.1 exists to avoid.
> ⚠ **`MAOS_HOME` silently outranks `MAOS_AUDIT_DB`** (`crates/maos-audit/src/lib.rs:872-889`), so mixing
> the two puts both hosts in **one Transparency Log** — and a single-TL "two-host" run has nothing to reconcile.
> ⚠ **`MAOS_HOST_GRANTS` fails OPEN.** An unreadable grants file **warns on stderr and continues** with the
> built-in grants (`crates/maos-bin/src/worker_spawn.rs:227-241`). Read the stderr; a typo in the path does not stop the run.

```bash
# Host A delegates; host B's claude worker executes. Both TLs record the SAME
# sixteen frame_id bytes (deterministic `seq ‖ run_nonce`) — that is the join key.

MAOS_HOME=$HOST_A_HOME maosctl audit sealed-export --range 1d \
  --audit-key $HOST_A_HOME/audit-signing.key --host host-a --output host-a-bundle.json
MAOS_HOME=$HOST_B_HOME maosctl audit sealed-export --range 1d \
  --audit-key $HOST_B_HOME/audit-signing.key --host host-b --output host-b-bundle.json
```

`--host` is **not a label**. Both halves carry the same `frame_id`s and are otherwise
indistinguishable: `region` cannot separate two hosts in one jurisdiction (same
derived key), `boot_nonce` is per-boot and one `--range 1d` export swept eight, and
`attester_pubkey` is bundle-supplied so R-RG1 forbids trusting it. **Without `--host`
one host can produce both halves.** It is covered by the signature, so altering it
post-signing fails verification.

**Record each half's 64-hex pubkey from `sealed-export`'s stderr as you go** — these are `FPR_A` and
`FPR_B`, they are what 7.3 and 7.4 consume, and they must match what you pre-published (7.1).

### 7.3 — Reconcile, from a THIRD home

```bash
# $OPERATOR_KEY — DEFINE THIS. It is the operator's OWN Ed25519 audit key, a THIRD key
# distinct from both hosts' signing keys; it signs the two-host receipt, not either half.
# It defaults to MAOS_AUDIT_KEY when --receipt-key is omitted. It must NEVER be
# $HOST_A_HOME/audit-signing.key or $HOST_B_HOME/audit-signing.key.
OPERATOR_KEY=~/.config/maos/audit-signing.key

maosctl audit reconcile-hosts \
  --bundle-a host-a-bundle.json --seed-a $HOST_A_HOME/audit-signing.key \
  --bundle-b host-b-bundle.json --seed-b $HOST_B_HOME/audit-signing.key \
  --receipt-out two-host-receipt.json --receipt-key $OPERATOR_KEY
#   → OK (hosts host-a + host-b, N shared frame_ids, …)
#   → claim scope: two keyed identities signed; not two machines, two processes,
#                  or two operators
```

> ⛔ **Use `--seed-a`/`--seed-b`, never `--pubkey-a`/`--pubkey-b`. This is not a style preference —
> it decides whether the one-root check runs at all.**
> `sign_bundle` derives **per-region** (`crates/maos-audit/src/sealed_export.rs:305-313`), so **one**
> base seed under two `MAOS_REGION_HOME` values yields two *distinct* pubkeys and the in-code
> `key_a == key_b` comparison never fires. The cross-derivation guard that catches this
> (`crates/maos-cli/src/subcommands.rs:2900-2937`) re-derives each supplied seed under the *other*
> half's claimed region — but it is written `if let Some(seed) = &base_seed_a`, and with `--pubkey-*`
> both `base_seed_*` are `None` (`:3020-3026`). **Both guards are skipped entirely.** The source says
> so in place at `:2905-2908`.
>
> **Simplest safe posture: leave `MAOS_REGION_HOME` unset on both hosts.** The two-invocation
> `MAOS_REGION_HOME` shape is undetectable at reconcile even with seeds, and stays bounded only by the
> sworn capture field (RELEASE-HOLDS row 9).

Each half is verified against **the key supplied for that half**. Never read `attester_pubkey` out of
the artifact — R-RG1 forbids it, and `xtask/tests/j1_crosshost_2c_proven_red.rs:458-470` machine-enforces
the prohibition. Two halves attested by ONE root are **refused** with `SharedAttesterRoot`.

### 7.4 — The stranger's check (NOT optional)

Our own `verify-bundle` is a self-check. The premise of this artifact is a claim a
**stranger** can check, and no stranger has ever checked one.

```bash
python3 tools/verify-audit-bundle/verify.py host-a-bundle.json $FPR_A
python3 tools/verify-audit-bundle/verify.py host-b-bundle.json $FPR_B
```

The Python twin is field-agnostic: it drops `signature_block` and sorts the rest, so
the `host` field flows through untouched. Its output goes in the capture verbatim.

> ✅ **FIXED as of `j1-crosshost-2e` AC1 (2026-08-22). This step is still an abort condition — it just
> no longer aborts by default.** Retained because it is the defect that would have killed the paid run
> and the reason this phase is not optional.
> `verify.py:93` omitted `ensure_ascii=False`, so Python escaped non-ASCII to `\uXXXX` while Rust's
> `canonicalize_value` (`crates/maos-audit/src/sealed_export.rs:632-639`) emits raw UTF-8. **A bundle
> containing a single non-ASCII byte failed verification even though its signature was valid** — and
> this phase is a mandatory abort, so the run died here *after both agents were billed*. Reproduced
> free against the real T6 artifact (12 non-ASCII bytes, valid signature) on 2026-08-19 and 2026-08-22.
> **T6 — the only signed run this project has ever performed — was unverifiable by its own published
> stranger's path from the day it was signed until the fix landed.** Now:
>
> ```
> $ python3 tools/verify-audit-bundle/verify.py \
>     _bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json \
>     61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766
> OK — signature verified                       # exit 0
> ```
>
> ⚠ **The OpenSSL fallback had THREE defects, not one**, and could never have worked: the same missing
> `ensure_ascii=False`; `-pkeyopt digest:SHA256`, which OpenSSL **refuses** for Ed25519; and `xxd`-ing a
> raw 32-byte key, which OpenSSL **cannot read** (needs SPKI DER/PEM). Rewritten, every line executed.
> CI now **executes** `verify.py` against the committed T6 bundle.
>
> **Still run it here on your own halves.** The fix is verified; your bundles are not.

### 7.5 — Scan what was STORED, not what was sent

```bash
for H in $HOST_A_HOME $HOST_B_HOME; do MAOS_HOME=$H maosctl audit scan-credentials; done
#   → "<N> rows scanned, 0 prefix escapes, 0 hex-run escapes"; non-zero exits 1
```

Every redaction call site is **pre-write**; this walks rows already on disk. Both
classes are reported distinctly, because the write path scrubs long hex runs
*silently* — a miss in that class would never have been logged in the first place.

### 7.6 — Capture the bounded claim

Add these to the Phase-4 capture JSON. The overclaim direction of each is **refused**
by `record-capture`, so a dishonest capture cannot be journaled and then signed:

```json
"two_host_shape": "two-processes-one-box",
"two_host_trust_anchor": "out-of-band-human-operator",
"two_host_host_b_audit_key": "hand-provisioned-separately",
"two_host_stranger_verification": "verify.py: signature OK (both halves)"
```

- `two_host_shape` — say which it was. `2b`'s mechanism proof is two real OS
  processes on one box; a reader hears "two machines". Free prose is refused.
- `two_host_trust_anchor` — `"protocol-negotiated"` is refused. A reader told *"these
  two hosts authenticated"* will not guess *"these two hosts were introduced."*
- `two_host_host_b_audit_key` — `"derived-from-shared-root"` is refused; that is the
  property that collapses "two hosts" into "two identities".

### 7.7 — `two-host-capture.json`: copy the published template, do not invent it

The four `two_host_*` fields above belong to the **Phase-4 `CaptureDoc`** (`record-capture`).
`two-host-capture.json` is a **different artifact** with its own seven required fields, and leg 9
of `check-j1-two-host-signed-run` compares `claim_scope` **byte for byte**.

**Do not author it from scratch — an invented capture is rejected after the agent is billed.**

```bash
# Run this FROM THE REPOSITORY ROOT and stay there. The previous revision `cd`-ed into
# the evidence directory here and then, twelve lines later, copied into a path relative
# to the repo root — from inside that directory the second command cannot resolve.
EVID=_bmad-output/test-artifacts/j1-two-host-evidence
cp $EVID/two-host-capture.example.json $EVID/two-host-capture.json
$EDITOR $EVID/two-host-capture.json
# Fill in host_a / host_b ONLY. `claim_scope` ships correct and must not be paraphrased
# (78 bytes, compared untrimmed). Delete the template's `_comment` key before landing:
# it is an extra TOP-LEVEL STRING and is fed to the overclaim tripwire like any other.
```

The full contract — all four artifacts, the seven fields, the verbatim `claim_scope`, the overclaim
tripwires, the type contract, and the two manual operator steps — is published at
`_bmad-output/test-artifacts/j1-two-host-evidence/README.md`. The template's admissibility is proven
executably by `published_capture_template_is_admissible_by_the_real_gate` in
`xtask/tests/j1_crosshost_2c_proven_red.rs`, so the template cannot drift from the validator.

Place the bundle halves beside it — **from the repository root** — then run the judge:

```bash
cp host-a-bundle.json host-b-bundle.json $EVID/
cargo run -p xtask -- check-j1-two-host-signed-run --json | jq .
```

**Three files, not four.** `two-host-evidence.txt` is deliberately **not** copied: it is read by no
leg of this gate, and per F2 nothing in the workspace can produce a `MAOS-EVIDENCE-V1` transcript
whose nonce would verify — the nonce is recomputed at gate-run time. R1 re-scoped this lane's evidence
to **the two bundle signatures**. An earlier revision of this block listed the transcript among the
copies, which would have had an operator hand-writing a file nothing reads and believing it was
evidence. ✅ As of `j1-crosshost-2e` AC3 the `CAPTURE_TRANSCRIPT` const is **deleted from the gate**,
so the file now has neither a producer nor a consumer.

> ⛔ **Read the JSON fields, never the exit code.** `passed` and `oracle_green` are green whether the
> capture is **absent, valid, or fabricated** — `xtask/tests/j1_crosshost_2c_proven_red.rs:386-414`
> commits the proof that a single-root, unsigned, forged pair passes. The discriminators are
> `paid_run_capture_present` and `two_host_signed_run_claimed`.
> Per R1, `two_host_signed_run_claimed: false` is the **expected and honest** outcome of this run and
> is published as a true fact — `PROVEN_LIVE_SIGNED` is unreachable **for this gate** (this claim is
> narrow and deliberate: 27 legs reach it on the operator lane).
>
> ⚠ `capture_signature_verified` and `capture_signature_reason` **no longer exist** — `2e` AC3 deleted
> them along with the unreachable verifier. Any checklist or script that greps for either field is
> pre-`2e`, and its green means nothing.

The gate **validates the capture when present** and **refuses to let anything claim it
when absent**. Absent is the honest CI state — not a failure.

### Phase-7 abort conditions — ANY of these → the two-host claim stays OPEN

- The two hosts share a key file or key material (→ "two identities", never "two hosts").
- The pairing was done on a **debug** build (the nonce shortcut proves nothing).
- `reconcile-hosts` refuses for any reason: shared root, missing host claim,
  duplicate host claim, or disjoint logs.
- `verify.py` rejects either half, or was not run at all.
- `scan-credentials` reports any escape on either host.
- The capture claims two machines, two operators, an automated pairing, or a
  shared-root key.

---


## 실행 요약 (한국어)

**T6 = 사람(당신)만 닫을 수 있는 게이트.** 테스트 초록불로는 안 닫힘. 순서:

- **0단계 (지금 가능):** c2 작업 문장 + 지출 상한 확정 → 세 개의 비밀 분리(`CODEX_API_KEY`=codex worker / provider key=Orchestrator 추론 / **audit key**=서명, 샌드박스 진입 금지) → `maosctl audit keygen`으로 서명 키 생성 + 지문 공개. (**codex worker는 `CODEX_API_KEY`를 읽음 — `OPENAI_API_KEY`가 아님**; `codex exec`는 후자를 무시 → 401.)
- **1단계:** `main`에서 브릿지 브랜치 worktree(더러운 epic-12 트리 보존) → 일회용 `$DEMO` 디렉터리(repo 밖) → 샌드박스 홈에 `~/.codex/auth.json` 없음 확인 → `codex --version` 기록.
- **2단계 (선택, 무서명):** 구독으로 한 번 구경 — 서명 안 하니 토큰 상관없음.
- **3단계 (서명 실행):** `CODEX_API_KEY="$OPENAI_API_KEY"`+provider key+`MAOS_LIVE_AGENT=1` → `cd "$DEMO" && maos run …j1-founder-loop-codex.toml --live`(codex 토폴로지, 연속, `--once` 아님 → halt/resume 확인; `$DEMO`에서 실행해야 codex cwd=$DEMO). codex가 `$DEMO`에서 작업 → 완료는 **어댑터가 파싱**(종료코드 아님) → 다이제스트가 Worker TL ref 인용. 끝나면 키 폐기. (**2026-07-15 실증: `worker_completion completed=true`, exit 0, `completion_tl_ref` 발급.**)
- **4단계:** 캡처 문서를 **JSON 파일**로 작성(비밀 제외): `signer`, `live_agent_identity`, `command_metadata`(redacted argv), `host_grant_disposition`, `audit_refs`, `egress`=`declared-not-enforced`+`egress_followup`, `redaction_result`=`verified`, `outcome`. (필수 필드 미달·비밀 포함·과잉주장 시 5a에서 거부됨.)
- **5단계 (저널링 → 서명 → 검증):** **5a** `maosctl audit record-capture --capture <file.json>` (**`--spirit` 없이** — v0.1 `resolve_spirit_name`은 `hello-spirit`만 받으므로 `orchestrator`는 거부됨; 생략하면 host-level 증명) → `run.capture` audit row 저널링(서명이 파일이 아니라 audit **행**을 서명하므로 필수) → **5b** `maosctl audit sealed-export --range 1d --audit-key <signer.key> --output <bundle.json>` (**`--spirit` 대신 `--range`**) → **하나의 서명된 JSON 번들**(embedded Ed25519) → **5c** `maosctl audit verify-bundle <bundle.json> --pubkey <FPR>` → `OK (<N> entries)` 통과해야 함. (`SHA256SUMS`+`.sig`가 아님 — 그건 오프라인 임포트 경로. journal-capture 배선 2026-07-15 착륙; host-level+range+verify 경로 2026-07-16 실증: 247 entries.)
- **6단계:** release-gate에 서명자=Myoungki Jung/날짜/번들 경로 기록, 관찰된 증거로만 체크 → 커밋 → 스프린트 라인 done → `13-1` 앞에서 머지.

**중단 조건(하나라도 → 서명 금지, Tier-2 OPEN 유지):** 비밀 잔존 / `$DEMO` 밖 CRUD / 서명 실행에 구독·auth.json 사용 / auth.json 상속 / 종료코드=완료 오인 / 서명 검증 실패.

**오늘 이미 있는 명령:** `maosctl audit keygen`, `maosctl audit sealed-export`, `maos run … --live/--once`. **T1–T5가 추가:** cli_wrapper Worker의 토폴로지 편입 + `WorkerCli` codex 어댑터 + 실제 task 라우팅 + `MAOS_LIVE_AGENT` 게이트 + 캡처 문서 봉인.

---

## 2-호스트 유료 런 실행 요약 (한국어) — `j1-crosshost-2d` AC8 / T8

위 요약은 **T6(단일 호스트, 2026-07-16 완료)** 입니다. 아래는 **아직 실행되지 않은** 2-호스트 유료
런이며, `j1-crosshost-2e`(2026-08-22)가 코드 블로커 6개를 모두 닫은 뒤의 절차입니다.
**남은 것은 코드가 아니라 운영자 기반**입니다: 프로비저닝된 두 호스트, 깨끗한 sandbox home,
과금형 API 키, 그리고 지출 결정.

### 0단계 — 무료. 전부 통과해야 지출 시작 (과금 지점은 2곳이고 중단 조건은 그 뒤에 있음)

1. **호출 계약을 먼저 읽을 것** — Phase 0.0. `maos run <manifest>`이며 `maos run --help`는 없고,
   **매니페스트는 마지막에 읽힙니다.** 잘못된 경로는 정상 부팅 로그 ~30줄(`A2A delegation leg
   installed` 포함) 뒤에야 실패하므로, 초기화 로그는 인자가 수락된 증거가 **아닙니다.**
   Phase 7 양쪽 호스트 = `spirits/topologies/j1-founder-loop-crosshost.toml`.
2. **릴리스 빌드 판별** — `README.md`의 fresh-home 블록. **런마다 새 state home**을 주세요. 같은
   home으로 두 번 돌리면 `orchestrator dispatch references raw worker output not a distillate`가
   나는데, 이는 이 검사와 무관한 기존 결함이고 falsifier 실패로 오독됩니다.
   판정: 서로 다른 nonce 2개 = 진짜 릴리스 / override(`424242`)가 되읽히면 **중단, 지출 금지.**
3. **`verify.py` 동작 확인** — Phase 7.4는 **필수 중단 조건**입니다. `OK — signature verified`가
   나와야 합니다. `FAIL`이면 2e 수정이 없는 트리입니다.
4. ⛔ **앰비언트 자격증명 제거** — `~/.claude/.credentials.json`이 있으면 라이브 런이 거부됩니다
   (redaction 입증 불가 = Tier-2 실패). 구독 토큰이 없는 sandbox home을 쓰세요. ⚠ `MAOS_HOME`은
   audit 서명키를 리다이렉트하지 **않습니다** — `--audit-key`/`MAOS_AUDIT_KEY`만 유효합니다.
5. **과금형 키** `ANTHROPIC_API_KEY` 확보 + 지출 상한 기록.

### 1단계 — 키: 새로 만들지 말 것

`PUBLISHED-FINGERPRINTS.md`는 **런 전에 커밋된 약정**입니다. 다른 키를 쓰면 약정이 무효가 되고,
올바른 대응은 파일 갱신이 아니라 **"이 런은 그 런이 아니다"라고 말하는 것**입니다. `FPR_A`/`FPR_B`,
그리고 **별개의 세 번째** `$OPERATOR_KEY`(receipt 서명자, T6 서명자와 **다른 키**)를 쓰세요.

### 2단계 — 호스트 B cohort manifest 서명 (2e AC2 신규)

`maosctl cohort sign --manifest <in> --authority-key <key> --output <out>`.
2e 이전에는 서명 수단이 워크스페이스에 **없었고**, 그래서 호스트 B가 부팅하지 못했습니다
(`EInvalidSignature("expected 64 bytes (128 hex chars), got 0 bytes")`).
`--authority-key`는 필수이며 **env 폴백이 없습니다** — 있었다면 cohort 신뢰근원과 audit 근원이
용접됩니다.

### 3단계 — 부트 nonce 페어링 (2e AC5 신규, 이전엔 실행 불가)

호스트 A가 `MAOS_CROSSHOST_PAIRING_READY_FILE`로 **발행 후 대기** → nonce를 **10진수**로 호스트 B의
`[[tcp.peer_pins]].boot_nonce`에 전사 → 호스트 B 기동 → `touch`로 해제. 파일이 안 생기면 호스트 A는
**다이얼하지 않고 종료**합니다(`--once`의 connect 시도는 재시도 없이 단 한 번).
⛔ nonce를 `--format plain`으로 읽지 마세요 — 16진수로 렌더되고 TOML은 10진수로 파싱합니다.
⚠ 전사 오류는 실패보다 나쁩니다: 핀이 무효화되어 두 번째 시도가 **다른** 에러로 실패하고, 복구는
호스트 B **재시작**입니다.

### 4~6단계 — 증거, capture 두 문서, 게이트

`sealed-export` ×2(각자 키) → `verify.py` ×2(**커밋된 지문과 대조**, 아티팩트의 `attester_pubkey`가
아님) → `reconcile-hosts`. capture는 **규칙이 서로 반대인 두 문서**입니다: `CaptureDoc`은
`two-machines` 토큰을 **강제**하고, 게이트의 `two-host-capture.json`은 같은 문자열을 **거부**합니다.
`claim_scope` 78바이트를 다른 top-level 문자열에 복사하면 RED입니다.

게이트는 **exit code가 아니라 JSON 필드**로 읽습니다 — 판별자는 `paid_run_capture_present`와
`two_host_signed_run_claimed` 둘뿐이며, `capture_signature_verified`는 2e AC3가 **삭제**했습니다.
`two_host_signed_run_claimed: false`가 **정상이자 정직한 결과**입니다.
