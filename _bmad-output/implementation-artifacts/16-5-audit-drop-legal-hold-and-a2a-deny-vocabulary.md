---
baseline_commit: "**`df45c609`** (Story 16-4 landed). Authored 2026-09-17 from six scouts — a STATIC scout over the drop class / `CapError` / call sites, a STATIC scout over the eight audit-truth residuals routed here by 16-2/16-3/16-4, a STATIC+RUNTIME scout over the legal-hold check-then-act and the I9 surface, a STATIC scout over `map_a2a_error_to_iac_bus` / `IacBusError` / `CrossWallRecallRefusal`, a RUNTIME scout that measured journal interleaving under a faithful `open()` replication and counted `write(2)` syscalls with an `LD_PRELOAD` interposer, and a DOCUMENTARY scout that read the decision register in full and RAN `check-decision-register`, `check-kernel-baseline` and `kloc-check`. **Cites are SYMBOL-first, line second** wherever a decision rests on them (16-4 §15 R2/F6 precedent). ⚠ **T0 re-measures. A grant is a global, not a reservation.** ⚠ **HEAD IS GREEN** — `check-kernel-baseline` PASSED (24477, 98 files), `kloc-check` PASSED, `check-decision-register` PASSED (23 rows / 12 open / 0 findings). This story's FIRST commit is what breaks two of those; see §6."
depends_on: "**`16-0-kernel-pin-content-hash` (`done`)** — the pin this story moves is now a file set + per-file SHA-256 + `--emit-pin` (`xtask/src/check_kernel_baseline.rs`, `fn file_digest:438`, `fn collect_files:363`); the epic ordered 16-0 first *because* it blocks 16-5. **`16-1-…` (`done`)** — `OperatorDoor::submit` `tokio::spawn`s each command (`crates/maos-bin/src/operator_door.rs:1414`) and `OperatorCommand::spirit_key` returns `None` for `ForgetMemory`/`ReleaseLegalHold` (`crates/maos-control/src/lib.rs:357-361`), which is what makes AC3's race REACHABLE rather than theoretical; `main.rs:8917-8920` pre-filed AC4 by name. **`16-2-…` (`done`)** — `crates/maos-audit/src/fr4_classifier.rs` (`WRITER_SHAPES`, 28 entries `:184-453`; `NON_CALL_KINDS` `:59-71`) and its doorbell `crates/maos-audit/tests/fr4_classifier_16_2.rs`; `fr4_classifier.rs:368-374` names **`16-5`** in production source. **`16-3-…` (`done`)** — `TaskAssignmentRecord.capability_token: Option<TokenId>` and the FIRST post-16-0 re-pin (24474 → 24477), whose cleanup shape §6 copies. **`16-4-…` (`done`)** — `deferred-work.md:957` (T7 wrote the cascade row); `SCANNED_SOURCE_FILES` is **23**."
blocks: "**`epic-16`'s close** — this is the epic's last story (Dependencies line: *16-5 last*), and the only one holding a kernel grant. **`epic-16-retrospective`** (four residual rows, §11). **D3, D7 and D18** in `epics/epic-14-preflight-decisions.md` — **all three are discharged by this one vehicle under the WHOLE ruling**, and whose deadlines all read `` before `16-5-…` leaves `backlog` `` at `df45c609` and would have fired `expired-and-open` on this key's own `backlog` → `ready-for-dev` transition — **re-anchored to `` reaches `done` `` in the creation commit (D-16-5-H, §12 rows 8-10), after which the Blocking gate returns `passed: true` with 0 findings.** See §6a."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT (rule 9, Round 9 — the 16-5 section, the stories-table row, the Kloc-asks paragraph, the Kernel-Δ paragraph, the Dependencies line, plus `epic-14-preflight-decisions.md` and `sprint-status.yaml`).** Measurement disproved premises in **all five** of the section's ACs. **AC1's own instrument is broken** — `grep -rn '\\.record_invocation(' crates/*/src` finds **1 of 4** production call sites (three are UFCS through the port trait), and the ONE swallowing site it names runs *after* the response is already on the operator's terminal, so *\"the invocation is refused\"* cannot be true there. **AC1 also contradicts the binding register**: D3 was RULED `ZERO kernel-Δ, repaired out-of-kernel, class-wide` — and the class is **nine**, not one. **AC2's cost model is inverted** (`MemoryManagerAdapter` is ALREADY `#[i9_exempt]`, so the `Mutex` is free) and **its `completed:false` does not exist on this path**. **AC3's collapse is 12 variants across 11 arms, not 7 across 6**, and the typed fix cannot cost `+6` because `maos-domain` may not name `maos-a2a-core`'s enums. **AC4's every cite is one story stale** and it names 1 of the 5 artifacts a re-pin actually touches. **AC5 is +2, not line-neutral**, and a naive two-process falsifier PASSES at HEAD. ⚠ **Twelve `deferred-work.md` rows name this key as Owner; the epic's section carries seven.** The rulings are the Decisions table; the applied edits are §12."
split_from: "Not a split. **Sizing: WHOLE — OPERATOR-RULED 2026-09-17 (§14 Q1, §15 R6), against the preflight's own SPLIT recommendation.** All five decisions the epic files here — D3, D5.1, D7, D12, D18 — and **all twelve** `deferred-work.md` rows that name this key are discharged in one vehicle. The rejected seam was `16-5b-cross-host-deny-vocabulary` (D7 + D18 + `deferred-work.md:707`/`:834` at ZERO kernel-Δ); it is refused because the two halves are **one capability, not two** — *an operator being told the true cause of an outcome* — and the deny-vocabulary half is the **only place that invariant is already proven against a shipped ADR** (`ADR-058:52`, *\"distinguishable without string matching\"*), which the audit-truth half has no equivalent of. Splitting would also have shipped D18's typed variants **after** AC1 had already reproduced D18's own collapse at `main.rs:1275` (D-16-5-K) — the fix and the defect it names in separate commits. ⚠ **The key name does not change**: 12 `deferred-work.md` Owner strings and 3 decision-register Target cells cite it verbatim. **Measured duration: 20–26 d** (14–18 d audit-truth + kernel + 6–8 d deny vocabulary)."
kernel_grant: "**REQUIRED — ◆ FLAG-Winston. This is the epic's only kernel grant and its second post-16-0 re-pin.** Files edited under `crates/maos-kernel-core/src`: `capability/mod.rs` (AC1 propagate, +1), `memory/mod.rs` (AC3 serializer, +4…+6), `journal/mod.rs` (AC4 append, **+2 measured under `rustfmt`, NOT line-neutral**), `halt/resolver.rs` (AC2 non-JSON payload; AC5 ordering), and — only if the re-kind fork is ruled IN — `scheduler/scheduler_loop.rs` + `supervision/crash_detector.rs` (line-neutral). Every one is in the pinned 98-file set, so **each will red `check-kernel-baseline` BY NAME** and each moves a per-file SHA-256. ⚠ **`maos-kernel-core` is the SOLE member of `ZERO_HEADROOM_CRATES` (`xtask/tests/recovery_lane_ceiling_rule.rs:46`) and is explicitly carved OUT of the RECOVERY-LANE CEILING RULE** — the easing does not authorize this; a standalone operator-authorized grant does, measured after `cargo fmt --all`, with a HISTORY row. ⚠ **A re-pin touches FIVE artifacts, and the epic's AC4 names one** (§6b)."
kloc_grant: "MEASURED at `df45c609` by `cargo run -p xtask -- kloc-check --json` (PASSED, `over_budget: []`): `maos-kernel-core` **18938/18938 = ZERO** (`kloc.toml:247`), `maos-capability` 1043/2000 (**+957**, `:248`), `maos-domain` 9140/9192 (**+52 only** — `:394` books `16-4`/`16-5` **jointly** and **16-4 already consumed +99 against a joint ledger of +36–66**), `maos-audit` 7164/7341 (**+177** — the epic says ZERO; 16-4 retired two dead resolvers), `maos-iac` 7025/7060 (**+35** — ⚠ **absent from the epic's Kloc asks entirely**, yet four routed obligations land there), `maos-shell` 571/573 (**+2**), `maos-bin` 22938/22938 (**ZERO**), `maos-a2a-core` 4856/4986 (**+130**, in scope under WHOLE). ⚠ **`maos-domain` is the binding crate and WILL cross**: AC1's `CapError` (+3…+6) plus AC8's mirror enums (**+50…+59**, forced by ADR-010 — `maos-domain` may not name `maos-a2a-core`'s `UnclassifiedReason`/`IntentDirection`/`CohortConsentDenial`) exceeds +52, so it takes a measured raise citing the bare token `16-5`. **Aggregate 166542** against an alarm of 158608 (`:657`) — **the alarm is FIRING, and the epic's own figure of 164216 is stale by 2326**; hardfail 170884 (`:658`) leaves **4342**. Non-kernel crates may raise under the RECOVERY-LANE CEILING RULE (`kloc.toml:49-95`; lane OPEN, this key not `done`) provided the code exists and is `cargo fmt --all`-measured BEFORE the ask and the figure + driver land in the same commit. ⚠ Inline `#[cfg(test)]` is charged; integration tests under `tests/` are not."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` — it is a NEGATIVE marker: `check_dev_model_used_populated.rs:302` treats any line containing it as boilerplate and SKIPS it."
review: "§A6 full-layer net **BINDING and NON-DEGRADABLE** — the epic marks this story ◆ (`epic-16-…:57`). Blind + Edge Case + Acceptance + Test-Infra + non-author runtime re-execution. It is the epic's only kernel-byte story, it moves a pin that audits its own edit, it changes what the audit record *says happened*, and its whole subject is **controls that report success while the thing they name did not occur** — the failure mode this net exists to catch. E15-A6 binding by name — *does this test read the tree, or only what it set itself?* Obligations (a)–(ab) under **Review obligations**."
---

# 16-5 — When MAOS records that something happened, it happened

Status: review

> **The capability:** *An audit record that MAOS wrote can be trusted to mean what it says. When the audit sink
> cannot take an event, the mediated call is refused instead of silently proceeding — and every place in the
> tree that drops an audit event is counted by one observable instrument, not seven-plus-two. A row that says
> "capability invocation" was a capability invocation. A token column that holds a token holds a whole one. A
> proof that says "provably nothing to erase" has looked everywhere the data could be. A redactor that scrubs
> secrets does not scrub the task ids the record needs to be readable. Two operators erasing and holding the
> same principal at the same moment cannot both win. Two writers to the Lifecycle Journal cannot tear each
> other's lines. And a halt whose resolution was never durably recorded does not report itself resolved.*

**Closes:** **D3** (`epic-14-preflight-decisions.md:66`, RULED at `:234-264`) · **D5.1 only** — the legal-hold
half, merged into D3's vehicle 2026-08-30 (`:278-284`) · **all twelve** `deferred-work.md` rows that name
this key (`:527`, `:545`, **`:707`**, **`:834`**, `:924`, `:932`, `:934`, `:935`, `:936`, `:946`, `:949`, `:957`) ·
**D7** (`:73`) and **D18** (`:84`) — both discharged here under the WHOLE ruling · the epic's **only** kernel grant and the pin re-pin the epic's Dependencies line orders
last.

**Does NOT close, and says so:**
- **D5** (`:71`). **It cannot be closed here, and the epic's `Closes: … D5 …` is an over-claim.** D5's deadline
  is anchored to `` before `v25-erasure-crash-reconciliation` leaves `backlog` ``, not to this key, and the
  register states the reason verbatim at `:461-467`: *"D5 therefore cannot close until **all three** vehicles
  land — and one of them, `v25-erasure-crash-reconciliation`, is an explicit v2.5 deferral."* This story
  discharges **one of three** (D5.1). D5.2/D5.3 belong to `20-3a-gate-honesty` (`deferred-work.md:546,547`).
  Stated, not silently dropped (binding rule 4: *"silence is not an outcome"*).
- **D12** (`:78`). **Already `CLOSED`, repaired by `14-0` — this story owes nothing and the epic's claim is
  stale.** Verified three ways at HEAD: the bench is correct (`crates/maos-bench/benches/audit_query_latency.rs:249`
  uses `"capability.invocation"`; the `"capability.invoke"` at `:24` is the free-text `intent` column and is
  deliberately left alone, recorded at `:244-246`); it has an execution path (`discipline.yml:1549` job
  `bench-audit-query-latency`, not `continue-on-error`); and it fails loudly on a vacuous measurement (`:255-265`
  asserts the kind filter matched a non-empty set before timing). §11 row 6 routes the one real residual.
- **FR4** (`prd/functional-requirements.md:27`). AC2 makes four FR4-shaped rows honest but **does not advance the
  100%-mediation floor**, and the re-kind fork (D-16-5-C) is declined with its cost recorded, per 16-2 §15 R3's
  binding *"decide with a recorded reason, never silently skip."*
- **NFR-Ops-11's operator axis** (`RELEASE-HOLDS.md:52`, boundary row 2). AC3 touches legal hold; its proof must
  not imply that boundary moved. `RELEASE-HOLDS.md:53` row 3 further records that *"an unauthorized legal-hold
  bypass is **not constructible** on collective rows"* — AC3 may not claim a negative control that does not exist.
- **`CollectivePortError`'s eight-into-one collapse** (`RELEASE-HOLDS.md:51` row 1, `deferred-work.md:664` (section heading `:662`)). Its
  successor is named *"Future FLAG-Winston kernel-widening decision"* and this is the only open FLAG-Winston
  vehicle — but it is the deny-vocabulary charter, not audit truth. §11 row 3.

---

## What this story actually is

The epic files this as a *"Debt slot."* It is not. Measured against the tree, the five sketched ACs are one
coherent capability with one adversary — **a control that reports success for something that did not happen** —
and six shapes of it:

1. **The audit sink can refuse and nobody notices** (§1, §2) — `record_invocation` calls `record_drop()` and
   returns `Ok(())`. Nine places in the tree drop an audit event; one of them does not even count it; the counter
   none of them feed has **zero production readers**.
2. **The audit record claims things that are not true** (§2) — kernel rows typed `capability.invocation` with no
   token, a 16-byte token zero-padded into a 32-byte column, `task.complete` rows at pid 0, a redactor that
   scrubs every `task-…` id, and a cascade that proves emptiness from one tier of two.
3. **Two operators can race and both be told they won** (§3) — the legal-hold check-then-act, newly reachable
   through the door 16-1 built.
4. **Two writers can tear the Lifecycle Journal** (§4) — and the one-line fix everyone assumed is two lines and
   does not work by itself.
5. **A halt can report itself resolved after the resolution was lost** (§5, U1–U3) — three review rows routed
   here by name and carried in no epic AC.
6. **Story creation itself reds a Blocking gate** (§6) — and the re-pin the epic describes touches five artifacts,
   not one.

---

## 1. 🔴 AC1's INSTRUMENT FINDS A QUARTER OF ITS SUBJECT, AND ITS HEADLINE CANNOT BE TRUE WHERE IT POINTS

AC1 says: *"`grep -rn '\.record_invocation(' crates/*/src` finds no other production caller."* Measured — it
returns **exactly one line**, `crates/maos-bin/src/shell_host.rs:107`. It is blind to three more, which call
through the port trait by fully-qualified syntax:

```
crates/maos-bin/src/main.rs:1269:  maos_domain::ports::capability::CapabilityRegistryPort::record_invocation(
crates/maos-bin/src/main.rs:1298:  ... (collective_read)
crates/maos-bin/src/main.rs:1326:  ... (collective_scan)
```

The sound instrument is `grep -rn 'record_invocation' crates/*/src`. **An AC written to a grep that measures a
quarter of its subject is not a control.**

**The four production sites, classified, with the ordering that decides the AC:**

| # | site | disposition | audit runs… |
|---|---|---|---|
| 1 | `fn collective_write`, `crates/maos-bin/src/main.rs:1269` | `.map_err(…)?` → `ResearcherCollectiveError::Denied` | **BEFORE** `self.memory.collective_write` |
| 2 | `fn collective_read`, `main.rs:1298` | `.map_err(…)?` | **BEFORE** the read |
| 3 | `fn collective_scan`, `main.rs:1326` | `.map_err(…)?` | **BEFORE** the scan |
| 4 | `impl ShellHost::record_turn`, `crates/maos-bin/src/shell_host.rs:103-108` | tail expression — **pass-through**, neither `?` nor `let _ =` | n/a |

And one level removed, the **only** swallowing site in the tree:

```rust
// crates/maos-shell/src/lib.rs:684-699, fn record_turn_row
695:    // The ONE `record_invocation` swallow left in the shell (Story 16-5
696:    // AC1's cite): `record_turn` passes the error through unchanged and
697:    // this call site decides to drop it.
698:    let _ = host.record_turn(token, payload.as_bytes());
```

⚠ **The epic's cites are stale three times over** — `maos-shell/src/lib.rs:265,278` → re-pinned `:304,317` → at
HEAD it is `:698`, inside a helper that did not exist when either cite was written, reached from **two** call
sites (`:511` and `:663`), not one. D3's own *"two via `let _ =`"* is also false at HEAD: Story 16-2 deleted the
`shell.halt` call (recorded in-source at `maos-shell/src/lib.rs:513-518`); **there is one.**

🔴 **And at that one site, "the invocation is refused" cannot be true.** `crates/maos-shell/src/lib.rs:508-512`
runs `render_turn(output, msg, &resp)?` **before** `record_turn_row(...)`. The LLM call is spent and the response
is already printed on the operator's terminal. Propagating an `Err` there can log or exit — it cannot un-invoke
anything. **AC1's headline verb is only achievable at sites 1–3, which already propagate and which AC1 does not
name.** That is where the falsifier belongs (AC1 below), and the shell site becomes a *disclosure* obligation,
not a refusal one.

## 2. 🔴 THE DROP CLASS IS NINE — AND D3's STATED BLOCKER IS FALSE

D3 was **RULED**, and the ruling is binding: *"repair out-of-kernel, class-wide."* Verified at HEAD:
`fn record_drop` lives in `crates/maos-capability/src/cap_audit/mod.rs:24-26` over
`static AUDIT_DROP_COUNTER: AtomicU64` (`:21`) — **outside** `check-kernel-baseline`'s scope
(`const KERNEL_SRC = "crates/maos-kernel-core/src"`, `xtask/src/check_kernel_baseline.rs:74`), outside
`xtask/kernel-crates.toml:3`, and **not** in `ZERO_HEADROOM_CRATES`. Headroom **+957**.

**The class, enumerated at HEAD. D3 says seven, plus an eighth. It is nine.**

| # | site | calls `record_drop()` |
|---|---|---|
| 1 | `fn issue`, `crates/maos-capability/src/cap_tokens/mod.rs:210` | ✅ |
| 2 | `fn revoke_all`, `cap_tokens/mod.rs:315` | ✅ |
| 3 | `fn record_verification`, `crates/maos-kernel-core/src/capability/mod.rs:243` | ✅ |
| 4 | `fn record_invocation`, `capability/mod.rs:349` | ✅ |
| 5 | `fn emit_sandbox_block`, `crates/maos-kernel-core/src/security/mod.rs:509` | ✅ |
| 6 | `fn emit_t3_escape_block`, `security/sandbox/t3/cap_audit_bridge.rs:26` | ✅ |
| 7 | deferred quarantine, `security/sandbox/t3/quarantine.rs:53` | ✅ |
| **8** | **`fn revoke` (operator revoke path), `cap_tokens/mod.rs:294-296`** — D3 cites `:280-282`, **stale +14** | ❌ **uncounted** |
| **9** | **`crates/maos-acp/src/notification_channel.rs:59-64`** — carries its own `// TODO: integrate cap_audit::record_drop when cross-crate audit bridge is available` | ❌ **uncounted, in no document** |

Site 8 is D3's own finding and it survives: an Epic-1b reviewer patched `issue()` and `revoke_all()` and missed
`revoke()`. Site 9 is new: a production drop site whose own comment names the function it does not call.

**D3's carried constraint is FALSE for the repair actually on the table.** The register says
`cap_audit_backpressure.rs:121-124` asserts `drops > 0`, so *"a 'never drop' repair reds a shipped test"*, and
*"the vehicle must rule on that test in the same breath."* Measured: the file is
`crates/maos-kernel-core/tests/cap_audit_backpressure.rs` (**not** `crates/maos-capability/tests/`), and its body
(`:92-108`) exercises `adapter.issue(…)` ×100 000, `adapter.verify(…)` ×100 000 and `adapter.revoke(…)` ×100.
**It never calls `record_invocation`.** Propagating `Err` there leaves it untouched; so does an out-of-kernel
observability hook. **The ruling is: no test is red by either half of AC1.**

⚠ **The tripwire D3 missed, which IS real.** `cap_audit_backpressure.rs:106` is
`adapter.revoke(token.token_id).unwrap()`. If the propagate discipline is extended to site 8, that `.unwrap()`
panics under saturation and this test reds — for a reason the register never states. **Site 8 is repaired by
making the drop observable, not by propagating** (D-16-5-B).

**What the counter is worth today.** `audit_drop_count()` has **four references repo-wide** — its definition
(`cap_audit/mod.rs:29`), its own unit test twice (`:127`, `:129`), one integration test
(`cap_audit_backpressure.rs:120`). **Zero production readers.** The in-repo precedent for fixing that is exact:
`OtelTraceSink::drop_count` (`crates/maos-telemetry/src/otel_sink.rs:415`; the bounded-probe `fn drop_count` is `:107`) **is** read by a named gate
(`crates/maos-telemetry/tests/otel_gates.rs:818`, `gate:otel-degradation`), with a proven-red vector built from
`with_bounded_channel(exporter, cfg, 1)` + `pause_consumer()`. Copy that shape; do not invent one.

**The mechanism neither document states, and it is the strongest argument for AC1.** `CapAuditWriter::spawn`
(`crates/maos-kernel-core/src/capability/cap_audit/writer_task.rs:28-32`) moves the `Receiver` into the task.
The writer's I2 panic (`crates/maos-iac/src/adapter/transparency_log.rs:870-880` — **not** the `:1364-1406` the
register cites, which is a different writer) drops that receiver. Every subsequent `try_send` then returns
`Closed`, not `Full`. **After writer death the process is in silent no-audit mode for its lifetime**: all seven
counting sites fire forever, sites 8 and 9 discard silently forever, `record_invocation` returns `Ok(())`
forever, and nothing reads the counter. The drain only `eprintln!`s — at **eleven** sites
(`main.rs:4111, 5046, 5196, 6154, 6671, 6828, 7045, 7335, 8269, 8566, 8807`, shape verified at `:8807-8810`),
never `?`, never an exit code.

### 2b. The audit record's own claims — eight residuals, seven routed here by prior stories

| # | defect | measured at HEAD | crate |
|---|---|---|---|
| a | **Eight `lifecycle.*` rows typed `CapabilityInvocation` (7), tokenless** — rows that claim a mediated call | `scheduler_loop.rs:326,357,385,412,439,492,615`; `crash_detector.rs:207` — ⚠ the 8th is `FrameOrigin::Kernel`, **not** `SpiritAuto` as the epic says | kernel-core |
| b | ⚠ **A NINTH lifecycle writer, in no document at all** — `fn resolve_verb`, `crates/maos-kernel-core/src/scheduler/verb_resolver.rs:124-137`, writes `lifecycle.{verb:?}` as **kind 2 `DecisionDispatch`** at a **real Spirit pid**, `FrameOrigin::HumanAuthored`, tokenless. Production-wired (`main.rs:3036`). It is an `Fr4SchemaViolation` at HEAD **and invisible to 16-2's doorbell** — `scan_tree`'s kind filter (`fr4_classifier_16_2.rs:659-673`) accepts only a literal `CapabilityInvocation`, a literal `TaskComplete`, or a variable; a literal `DecisionDispatch` hits `continue` at `:672` | kernel-core / maos-audit |
| c | **kind-1 `task.orphaned` zero-pads a 16-byte TASK token into the 32-byte column** — `crash_detector.rs:161-165`. ⚠ 16-3's `Option<TokenId>` amendment fixed only the **absence** case; the `Some` case still pads. A reader gets 64 hex chars whose trailing 32 are zeros and cannot round-trip | kernel-core |
| d | **Three tokenless kind-1 dispositions at pid 0** — `fn emit_task_complete_nack` `crates/maos-iac/src/adapter.rs:239`, `fn emit_task_complete_escalated` `:258`, `fn reassign_task_to` `:280`. Cites exact; already `NonCall`, so not red — the defect is semantic | maos-iac |
| e | **The redactor's `sk-` rule is a SUBSTRING scan** — `RedactionRule{prefix: b"sk-"}` at `crates/maos-iac/src/adapter/redaction.rs:80-83`, applied by `fn redact_unscoped` `:142-190` which tests **every** index `i` (`:154`). **Measured against the real crate:** `{"task_id":"task-worker-1"}` → `{"task_id":"ta<REDACTED:type=api_key_generic,len=11,hash=a9b1>"}`. ⚠ The fixtures 16-2 and 16-3 ship are exactly this shape (`fr4_classifier_16_2.rs:200,273,281,289`; `fr4_worker_rows_16_3.rs:26,30,165,169`), and `crates/maos-bin/src/supervision.rs:225-230` **designed the four-group delegation-id format around this bug** (*"No id contains `sk-`…"*) | maos-iac |
| f | **`fn insert_kernel_event_returning_id` writes raw kind-1 rows** — `crates/maos-iac/src/adapter/transparency_log.rs:1373-1414` (epic's `:1364-1405` off by 9), hardcoding `TaskComplete` (`:1400`) and the `None` token (`:1399`). ⚠ **Nine call sites, seven production — not the three the epic names.** Real-pid: `main.rs:6350`, `:11144`, `:11246`. ⚠ **Four production pid-0 callers the epic never mentions, two of them inside kernel-core**: `main.rs:9076`, `memory/mod.rs:462`, `:492`, `:593`. Confirmed `Fr4SchemaViolation` (falls to `Call` at `fr4_classifier.rs:486-487`) and confirmed invisible to the inventory (`fn scan_source` `fr4_classifier_16_2.rs:492`, search loop `:533`, matches only `insert_frame_event*`) | maos-iac / maos-audit |
| g | **A non-JSON `task.orphaned` at pid 0** — `fn emit_task_orphaned`, `crates/maos-kernel-core/src/halt/resolver.rs:218-230`; the payload is a bare `format!` string (`:221`), so `fn payload_matches` (`fr4_classifier.rs:528`) cannot parse it. The table dispositions it `Call` **deliberately**, and `fr4_classifier.rs:368-374` names **16-5** in production source | kernel-core |
| h | **The cascade proves emptiness from ONE tier of two** — `fn run_uninstall_cascade_inner`, `crates/maos-bin/src/main.rs:9087`; gates at `:9118-9119` and `:9227-9231` rest on `shared_tier_principal_row_count` (`crates/maos-audit/src/lib.rs:1868-1896`) **alone**. Grepping `maos-audit` + `maos-kernel-core` for a private-tier counterpart returns exactly one hit — this one. **There is none.** The tree's own comment at `:9123` calls it *"provably nothing to erase"* and `:9126-9127` warns it is *"the 13.5b false-success shape"* — then walks into it on the private-tier axis. ⚠ **Precision correction to the routed claim:** `NotFound` returns **before** `build_erasure_proof` (`:9354`), so no signed proof is written; what is durably committed is a TL audit terminal (`main.rs:9076`, intent `principal.uninstall.not-found`, exit 4) — which is itself defect (f) | maos-audit / maos-bin |

🔴 **The epic's compensating claim on the re-kind is FALSE.** It says *"16-2's writer-shape table stays either
way (TLs written before a re-kind keep their rows forever)."* Two independent mechanisms break it. (i) The
doorbell key is `(basename, fn, ordinal)` (`fr4_classifier.rs:141`) and the inventory asserts **both** directions
including *"no stale table entries for sites that no longer exist"* (`fr4_classifier_16_2.rs:797-807`) — a
re-kind makes `scan_tree` skip those eight sites and all eight entries red. (ii) Flipping each entry's `kind: 7`
to `N` instead breaks the classifier's kind-7 exemption filter (`fr4_classifier.rs:492`, `.filter(|w| w.kind == 7)`),
so every **historical** `lifecycle.*` row becomes a tokenless `Call` — the legacy logs the note promises to
protect are exactly what breaks. Preserving both needs a **third option the epic never prices**: a second
disposition per call site, changing `WriterShapeEntry`, `scan_tree`'s kind filter, both assertion loops and the
**20**-row fixture table (`fr4_classifier_16_2.rs:136-301`, held 1:1 with the table's 20 `shape.is_some()` entries by the invariant at `:302-308`). **That unpriced rework is why D-16-5-C declines the
re-kind** — with the cost recorded, never silently.

## 3. 🔴 THE LEGAL-HOLD RACE IS REAL AND NEWLY REACHABLE — AND THREE OF AC2's PREMISES ARE FALSE

**The window, measured.** `fn forget_with_reason` (`crates/maos-kernel-core/src/memory/mod.rs:424`, `pub fn` —
**sync, zero `.await`**): check at `:481-485` (`is_under_legal_hold`), act at `:530-531`
(`scrub_distillate_body`). Between them: **8 statements, no lock held**, two SQLite round-trips on **two
different `Connection`s** (`principal_index.lookup` `:509` over `PrincipalNamespaceIndex.conn`
`memory/principal.rs:34`; `distillate_frames_for_pids` `:521-524` over the TL), plus an O(frames × body_len)
byte scan at `:527`. `TransparencyLogAdapter.inner` is a `Mutex` (`transparency_log.rs:367`) taken and dropped
**per call** — the check's guard is dead before the act's guard is taken.

**The harmful interleave, named:**

| Thread A — `ForgetMemory{principal, reason: None}` | Thread B — `ForgetMemory{principal, reason: Some("legal-hold:case-N")}` |
|---|---|
| `:481-485` check → no hold → falls through | |
| | `:447` `place_legal_hold` **commits** |
| | `:464` journals `principal.forget.held`; `:475` returns `Suspended{hold}` → door answers `"outcome":"held"`, terminal code 3 |
| `:530-531` scrub, `:580` `private.forget_principal`, `:581` `principal_index.forget` → `Erased{receipt}` | |

**The operator is told the hold was placed and erasure suspended, while the data was destroyed — and a
`principal.forget.held` frame is signed into the log saying so.**

**It is reachable — three proofs.** (i) `forget_with_reason(&self, …)`, held as
`memory: Arc<MemoryManagerAdapter>` (`DoorInner.memory`, `crates/maos-bin/src/operator_door.rs:186`; used at `:946` and `:1163`).
(ii) `OperatorDoor::submit` spawns each command independently (`operator_door.rs:1414`) on a
`#[tokio::main(flavor = "multi_thread")]` runtime (`main.rs:1827`) — two POSTs run on two OS threads in genuine
parallel. (iii) 🔴 **There is no per-command lock on this class**: `run_command` takes a permit only from
`command.spirit_key()` (`operator_door.rs:1322-1335`), and `crates/maos-control/src/lib.rs:357-361` returns
`None` for `ForgetMemory` and `ReleaseLegalHold` because they are principal-scoped, not Spirit-scoped. **16-1
built the surface that makes this reachable; this story closes it.** The `flock` store-lock set is
*process*-level, so offline CLI invocations are already mutually exclusive — **the race is confined to two
concurrent door POSTs against one daemon**, which is precisely what Epic 16 exists to build.

**`place_legal_hold` has exactly one production caller in the tree** — `memory/mod.rs:447`, inside
`forget_with_reason` itself. That is why one guard at that function's entry is sufficient for the harmful
direction.

🔴 **FALSE PREMISE 1 — "since `Mutex` is I9-denylisted" implies a cost that does not exist.**
`MemoryManagerAdapter` is **already** `#[maos_attrs::i9_exempt(…)]` (`memory/mod.rs:76-78`), and the gate
**skips field checks entirely on an exempt struct** — `xtask/src/check_empty_kernel.rs:229-242`. Adding a
`Mutex` field costs **zero** I9 work: no new attribute, no new `docs/invariants/i9-exemptions.md` entry. The
denylist (`xtask/i9-denylist.toml`) is broad by design — 25 types including `Arc`, `Vec`, `RwLock`, `HashMap`
(`:5`: *"Adding entries is mechanical… does NOT require review"*) — not a special prohibition on `Mutex`.
⚠ **What IS owed:** the existing exemption reason says *"Does NOT retain mutable state across calls"*
(`memory/mod.rs:77`, `i9-exemptions.md:260`). A serializer makes that sentence false. The gate is a bare
substring match on the struct name (`check_empty_kernel.rs:166-169`) and will still pass — so this is a
correctness obligation, not a gate one, and it is exactly the kind a review net has to catch.

🔴 **FALSE PREMISE 2 — the TL-transaction alternative cannot deliver AC2 at all.** The acts span **three stores
on two SQLite connections plus the filesystem**: `principal_index` has its own connection (`principal.rs:34,41`),
`private.forget_principal` is filesystem + an in-memory `RwLock<HashMap>` (`memory/private.rs:60-63`), and only
`scrub_distillate_body` is on the TL's connection. **No single SQLite transaction spans that set.** It would
cover the check and one of four acts. Shape B is disproved, not merely more expensive.

🔴 **FALSE PREMISE 3 — "the erasure proof carries `completed:false`". There is no `completed` field on this
path.** `ErasureProof` (`crates/maos-audit/src/erasure/proof.rs:70-87`) has none; `ForgetReceipt`
(`crates/maos-domain/src/memory.rs:364-379`) has none, and under hold **no receipt is built at all** (both
`Suspended` arms return at `:475`/`:505`, before `ForgetReceipt::new` at `memory/mod.rs:599`). What actually
happens: `main.rs:9338-9348` pushes `ErasureCategory{name: "legal_hold", status: CategoryStatus::CoverageGap{reason}}`,
and `main.rs:9397-9400` **suppresses the regional teardown receipt entirely** when any principal is held
(its own comment: *"a held run is NOT a teardown"*). The `completed: true` the register mentions is
`regional_teardown.rs:151-160`, reachable only from the `held_principal_ids.is_empty()` branch — **off this
path**. ⚠ **A dev following AC2 as written will look for a flag that does not exist and may invent one.** The
correct assertion target is the `CoverageGap` category plus the **absence** of a `regional-teardown-*.json`.

⚠ **Two more corrections.** The corpus contract is `crates/maos-audit/src/bin/gen_gdpr_cascade.rs:123-128`
with `"legal_hold" => "held"` at **`:124`** (epic says `:122-126`). And `main.rs:5359` is
`calendar: Option<McpClientImpl>` inside `struct ButlerLiveMcpClient` — **`fn run_uninstall_cascade` is at
`main.rs:9047`**, its forget call at `:9177`, its `Suspended` arm at `:9210-9216`, its `Held` terminal at `:9430-9439` (exit 3 mapped at `:8879`). The doctrine *"unchanged"* holds; the cite does not.

⚠ **There is no race coverage in the tree.** `loom` appears nowhere under `crates/` or `xtask/`. Every threaded
test nearby is off-target. **Nothing would red if a dev shipped a broken lock** — so AC3 must ship the first one,
and the tree's own precedent for the shape is `std::sync::Barrier::new(2)` (`crates/maos-bin/tests/shell_host_16_2.rs:1043`).
✅ Construction risk is **zero**: there is no struct-literal construction of `MemoryManagerAdapter` anywhere —
the only `Self { … }` is inside `new()` (`memory/mod.rs:112-131`) and the fields are private.

## 4. 🔴 THE JOURNAL FIX IS +2, NOT LINE-NEUTRAL — AND A NAIVE FALSIFIER PASSES AT HEAD

`fn open` (`crates/maos-kernel-core/src/journal/mod.rs:97-103`) is `.create(true).read(true).write(true)` with
**no `.append(true)`** ✅. `fn append_transition` (`:177-181`) writes at the handle's own cursor with
`write!(file, "{line}\n")` at `:180` ✅.

**Measured, with an `LD_PRELOAD` interposer on `write`/`writev`** (strace is absent on this host):

```
write(fd=3, len=27) [{"a":1,"b":"hello-journal"}]     <- write!(file, "{line}\n")
write(fd=3, len=1)  [\n]                              <- the newline, a SEPARATE syscall
write(fd=5, len=28) [{"a":1,"b":"hello-journal"}\n]   <- write_all of a prebuilt String
```

**`write!` on a raw `File` = 2 `write(2)` calls; `write_all` of a pre-built line = 1.** Confirmed behaviourally:
`.append(true)` + `write!` (O_APPEND, two syscalls) produced 3200 lines of which only **1039 were well-formed —
2161 torn**. **Both halves of the fix are load-bearing**, exactly as 16-1's V-13 claimed.

🔴 **REFUTED: the fix is not line-neutral.** Measured on a scratch copy under `rustfmt --edition 2021`:
489 → **491**. `.write(true)` → `.append(true)` is net 0 (Rust's `append` implies write); `let mut line` pushes
the `serde_json::to_string` call past 100 cols so rustfmt wraps it (+1), and `line.push('\n')` is the other (+1).
Keeping *both* `.write(true)` and `.append(true)` costs +3 — **take the +2 form.** Zero import delta
(`std::io::{BufReader, Read, Write}` already at `:54`).

🔴 **A naive two-process falsifier PASSES at HEAD and proves nothing.** `open()` ends at EOF-at-open-time because
`:106-111` `read_to_string`s through the same handle — so two processes collide only when their write lifetimes
**overlap**. Measured, 4 procs × 800 records:

| shape | lines / 3200 | well-formed | lost or torn |
|---|---|---|---|
| HEAD, simultaneous open | 904 | 891 | **2309** |
| HEAD, opens staggered 150 ms, no overlap | 3200 | 3200 | **0** |
| fixed (`.append` + one `write_all`), realistic 440-record shape | 440 | 440 | **0** (HEAD: 52 lost/torn) |

✅ **Use the deterministic single-process falsifier instead.** `fn recover_in_flight_with_tasks` (`:219-229`)
does `file.seek(SeekFrom::Start(0))` on the **write** handle and swallows the read error (`:227`
`let _ = reader.read_to_string(…)`). A read that fails partway leaves the cursor mid-file and the next
`append_transition` overwrites from there. Measured: HEAD produces a torn first line
`{"seq":0,{"seq":99,"pad":"ZZZ…}` and **5 lines instead of 6**; with `.append(true)` the first line is intact
and there are 6. **O_APPEND is fully compatible with the recovery seek** (it forces the write offset to EOF;
reads are unaffected — measured both arms).

⚠ **The concurrency that is unambiguously production is INTRA-process, not two-process.** `maosctl` never opens
the journal; `maos shell` shares the daemon's `Arc<JournalAdapter>` (`shell_host.rs:46,126`). But a single
`maos` process opens `shared_journal` unconditionally at `main.rs:2946` **and again** at `:6485`
(`smoke-epic-4`), `:7018`/`:7031` (cold restart) and `:8447` (hello-spirit admission) — **two `JournalAdapter`s
on one path in one process**, two `File`s, two cursors, **no shared mutex** (the `Arc<Mutex<File>>` serializes
only within one adapter), both opened at the same EOF. Deterministic, no timing. The two-process leg is real but
`smoke-*`/demo-grade; **write the falsifier against the intra-process pair.**

⚠ **The epic's comment cite is stale**: `main.rs:2863-2864` is cross-host boot-nonce text. The comment is at
**`main.rs:3047`** (*"file-append is atomic for small writes on POSIX"* — false on **both** clauses). And the
authoritative provenance is better: `fn append_lifecycle_journal`, `main.rs:8917-8920`, already says the hazard
is *"routed to 16-5."* Both move in this story's commit.

⚠ Watch `crates/maos-kernel-core/tests/journal_fsync_assertion.rs` — it enforces **P99 < 1.5 ms over 10 000
appends**. Halving the syscall count should improve it; it is the one measurable regression surface.
`fn append_in_flight` (`:213`) delegates to `append_transition` and is fixed for free.

## 5. 🔴 THREE HALT-DURABILITY ROWS ARE ROUTED HERE BY NAME AND CARRIED IN NO EPIC AC

`deferred-work.md` names this key as **Owner** on **twelve** rows (`:527, 545, 707, 834, 924, 932, 934, 935,
936, 946, 949, 957`). The epic's §16-5 carries **seven**. Three of the five unlisted are one coherent failure —
*a halt reports itself resolved after the resolution was lost* — and each names this key because it is the only
Epic-16 story holding a kernel grant:

| dw | defect | measured |
|---|---|---|
| **`:932`** | **The registry flips to `Resumed` BEFORE the resolver's context write.** A failed `memory.write` strands the halt terminal-`Resumed` forever: a retried `maosctl halt resolve` answers `halt_already_resolved` and **the operator's clarification text is unrecoverable** | `crates/maos-kernel-core/src/halt/resolver.rs:143-175`; REPL side `crates/maos-shell/src/lib.rs:603-613`. Kernel-core byte — 16-2 was pinned out of kernel-core (its Trap 12) |
| **`:935`** | **A journal-write failure after the resolver commits yields a resolution with no `approval_decision_log` row.** The next tick sees `Resumed`, `take_context` succeeds, and **the Spirit proceeds on operator context the approval log never recorded** | `crates/maos-bin/src/operator_door.rs:954-981` over `crates/maos-director-surface/src/halt_ui.rs:80-84`. The row's own words: *"D3 audit durability is exactly 'an audit event dropped while the operation returns'"* |
| **`:934`** | **`classify_halt_record` labels the kernel's serde-failure termination receipt `raised`** — a termination row renders as a raised halt in `maosctl halt list`. Unreachable today; the clean fix is kernel-side | `crates/maos-cli/src/subcommands.rs:2003-2004` against `crates/maos-kernel-core/src/halt/termination.rs:105-110` |

And one more, same surface, different axis:

| **`:936`** | **The REPL clarification has no size cap while the HTTP surface caps at 64 KiB.** One pasted line grows memory and the journal without bound | `crates/maos-shell/src/lib.rs:431` vs `MAX_BODY_BYTES`, `crates/maos-control/src/lib.rs:215`. ⚠ `maos-shell` has **+2** headroom — a measured raise |

**These are not scope creep, and each is routed by charter.** Rows `:932` and `:935` are the same failure at
two layers of the halt resolution path this story's kernel grant already opens — **AC5** takes both. Row `:934`
is the surface that *renders* that path (`maosctl halt list`): a list that mislabels a termination as a raised
halt makes AC5's repairs unobservable to the operator they are for — **AC5** takes it as its one `maos-cli` row.
Row `:936` is not a halt row at all: the oversized clarification lands in the **Lifecycle Journal**, which is
AC4's object, so **AC4** takes it. Rule 10(b) asks *"which story already touches that file"* first, not last;
for all four the answer is this one.

## 6. 🔴 THIS STORY'S CREATION COMMIT REDS A BLOCKING GATE — AND THE RE-PIN TOUCHES FIVE ARTIFACTS

### 6a. The decision-register tripwire

`cargo run -p xtask -- check-decision-register --json` at HEAD → `{"rows":23,"open_rows":12,"findings":[],"passed":true}`.
It is Blocking (`xtask/src/check_decision_register.rs:52`: *"no advisory tail and no phase coupling — findings
mean exit 1"*), defined at `discipline.yml:379` and enrolled in **`v1-0-ship-gate.needs`** (`:3723`, job head
`:3677`). ⚠ **It is NOT in `aggregate.needs`** (job `:3812`, needs `:3814-3939`, 125 entries) — the same
gate-binding defect §11 row 6 files against `bench-audit-query-latency`, which is why that row now carries both.

```rust
// xtask/src/check_decision_register.rs:418-424
ClauseKind::LeavesBacklog => status != "backlog",
// :620-633 — if status == RowStatus::Open && has_passed(...) => finding `expired-and-open`
```

**D3 (`:66`), D7 (`:73`) and D18 (`:84`) all carried the deadline `` before `16-5-…` leaves `backlog` ``, and
`sprint-status.yaml:251` was `backlog`.** The moment this story file landed and that row flipped to
`ready-for-dev`, all three would have become `expired-and-open` and red the gate **in the same commit**.
✅ **APPLIED at story creation (§12 rows 8-10); `check-decision-register` re-run after the edits returns
`{"rows":23,"open_rows":12,"findings":[],"passed":true}`.** ⚠ The gate caught one attempt on the way: the first
re-anchor put a `;` inside D7's parenthetical, which `fn deadline_clauses` split into a second, unresolvable
clause — the exact trap this section documents, firing on its own repair. (D5 does not:
its anchor is `v25-erasure-crash-reconciliation`. D12 does not: it is `CLOSED`, and `:620` gates on
`status == Open`.)

**Two shapes exist and only one is lawful.** The register's `Status` cell (`:39`) defines `OPEN` as *"the
obligation stands — **including rows whose decision is ruled but whose implementation is not yet landed**, so a
recorded ruling can never make an outstanding obligation invisible,"* and binding rule 1 (`:43`) forbids closing
by implication. **So the rows cannot honestly flip to `CLOSED` at story creation.** The lawful shape is to
record the rulings here and **re-anchor the deadlines to `reaches done`** — the exact move 14-1 made for D5
(`:452-480`). ⚠ **Mechanically it must REPLACE the clause, not append one**: `fn deadline_clauses`
(`check_decision_register.rs:396-409`) splits on `;` and evaluates **every** clause independently, pinned by
`xtask/tests/decision_register_gate.rs:241-260`, and the register records the lesson at `:476-480` — *"The `14-1`
clause was **replaced**, not supplemented."* D-16-5-H rules it; §12 applies it.

⚠ **A live mis-pointer inside the register's own prose.** `epic-14-preflight-decisions.md:91` says
*"`14-d3`→`16-6` … `14-4`→`16-6` (D18) … the deny-vocabulary fix is named in the `16-6` row."* Every **table
cell** says `16-5`. The prose is round-1 numbering (`sprint-change-proposal-2026-09-04.md:68`), superseded by
round 2 (`…-round2.md:86`: *"D3/D5/D7/D12/D18 → 16-5"*) — and since 2026-09-13 **`16-6` is a real and entirely
different story**. The gate cannot see it: it parses table cells only. This is the register's own signature
failure mode firing on itself. §12 repairs it.

### 6b. The re-pin: the epic names one artifact of five, and its numbers are one story stale

🔴 **The epic's premise that this is the first post-16-0 re-pin is DISPROVED. Story 16-3 already moved it**
(`822070d7`: `src_lines` 24474→24477, `set_hash` `5575b00a…`→`a031a2e7…`, two per-file digests). **16-5 is the
second.** ⚠ **But the precedent is dirty**: 16-3's commit left the pin suite red and *two* follow-up commits
cleaned up after it — `df45c609` (16-4) fixed four stale `24477` literals, and `1db6f262` fixed the ratchet.

| AC4 says | measured at HEAD |
|---|---|
| `kernel-core-baseline.toml:481`, `src_lines = 24474` | **`:488`, `src_lines = 24477`** — 16-3 moved both the value *and* the line (its 7-line HISTORY block pushed the assignment down) |
| `kloc.toml:247` = `18935` | **`18938`** |
| *(no mention)* | **98-entry `[kernel_src.files]` at `:525-623`** with a per-file SHA-256 — editing `capability/mod.rs` moves `:532` and the `set_hash`; a commit that re-pins only `src_lines` reds on `changed: [...]`. **AC4 is written against the instrument 16-0 replaced** — the exact drift 16-0 was promoted into this epic to stop |

✅ **`--emit-pin` exists**, so no hand-editing of 98 hashes: `cargo run -p xtask -- check-kernel-baseline --emit-pin`
prints a paste-ready 103-line block and **round-trips byte-identical** at HEAD. It does **not** emit `src_lines`.
The five artifacts are listed in T6 and obligation (v).

🔴 **The third doorbell the epic never names.** `xtask/tests/recovery_lane_ceiling_rule.rs:268`
`const RATIFIED_AT_EASING: i64 = 18_938;` with `:275` `assert!(actual <= RATIFIED_AT_EASING, …)`. Any
`maos-kernel-core` ceiling raise reds it unless `:268` moves in the same commit. And `:46`
`ZERO_HEADROOM_CRATES = ["maos-kernel-core"]` + `:76-78` `Refusal::ZeroHeadroomCrate` mean **the recovery-lane
easing does not authorize this raise at all** — only a standalone FLAG-Winston grant does. The epic states that
correctly at `:51` and then contradicts it with AC4's numbers.

---

## 10. SIZING AND BUDGET (rule 6, ×1.3; interval arithmetic)

| crate | headroom @ HEAD | change | raw | ×1.3 upper |
|---|---|---|---|---|
| `maos-kernel-core` | **0 — FLAG-Winston only** | `capability/mod.rs` propagate +1 · `memory/mod.rs` serializer field + init + guard +4…+6 · `journal/mod.rs` `.append(true)` + `write_all` **+2 measured** · `halt/resolver.rs` JSON payload +2…+4 and AC5's commit-ordering repair +8…+15 · `#[i9_exempt]` reason amendment +0…+2 | **+17…+30** | **+39** |
| `maos-capability` | +957 | class-wide observability over `record_drop` (`OtelTraceSink::drop_count` shape), site-8 repair at `fn revoke` `cap_tokens/mod.rs:294-296`, a typed drop-class enum so the gate can name the site, **plus D-16-5-L's latched degraded flag and its accessor — the audit of last resort** | +75…+145 | **+189** |
| `maos-domain` | **+52 — WILL CROSS (`kloc.toml:394` books `16-4`/`16-5` jointly and 16-4 already spent +99 of a +36–66 ledger)** | `CapError` variant +3…+6 · **AC8's typed `IacBusError` variants + the MIRROR enums ADR-010 forces (`CrossHostUnclassifiedReason`, the `CohortConsentDenial` mirror, the direction discriminator) + `#[non_exhaustive]`** +50…+59 | **+53…+65** | **+85 (measured raise)** |
| `maos-audit` | +177 | private-tier principal count beside `fn shared_tier_principal_row_count` (`lib.rs:1868-1896`) · `NON_CALL_KINDS`/`WRITER_SHAPES` entries for defects (b) and (f) · doorbell kind-filter extension | +70…+140 | **+182 (will cross — measured raise)** |
| `maos-iac` | **+35 — ⚠ absent from the epic's asks entirely** | redactor: `sk-` becomes a token-boundary rule, not a substring scan (`redaction.rs:80-83,142-190`) · `fn insert_kernel_event_returning_id` gains a caller-supplied kind + token (`transparency_log.rs:1373-1414`) · the three pid-0 dispositions (`adapter.rs:239,258,280`) | +80…+160 | **+208 (will cross — measured raise)** |
| `maos-bin` | **0** | cascade emptiness proof reads both tiers (`main.rs:9118,9227`) · AC5's `operator_door.rs:954-981` ordering · the eleven drain sites gain one honest exit path · **D-16-5-K's distinct refusal cause at all three `main.rs:1269/1298/1326` sites** +6…+12 · **D-16-5-L's degraded latch read + its `GET /v1/daemon` field and `maos audit query` disclosure** +25…+50 | +71…+152 | **+198 (will cross — measured raise)** |
| `maos-shell` | **+2 — effectively full** | the REPL clarification cap (`lib.rs:431`) beside `MAX_BODY_BYTES` · the `let _ =` at `:698` becomes a disclosed, counted drop | +12…+30 | **+39 (will cross — measured raise)** |
| `maos-cli` | +788 | `fn classify_halt_record` ordering (`subcommands.rs:2003-2004`) | +5…+15 | **+20** |
| `maos-a2a-core` | +130 | AC8: rewrite 11 collapse arms to typed constructions + the `CohortConsentDenial`/`UnclassifiedReason` translation arms, delete the five "no new variant" comment blocks (−15), fix `IntentDeniedAtPeer`'s lying `intent` field | +17…+45 | **+59** |
| `maos-a2a-tcp` | measure at T0 | `deferred-work.md:834`: type `TransportFailed("awaiting response")` with a frame id and the operator partition window (`transport.rs:602-603`) | +20…+40 | **+52** |
| `maos-iac` (AC7 half) | shares the +35 above | D7's TL journal surface: `fn journal_cross_wall_recall` emits the typed refusal, not `ToString` free text (`adapter/log_recall.rs:96-119`) | +10…+20 | folded above |
| `maos-control` · `maos-journey-test` · `xtask` | — | untouched; `xtask` changes are the pin/ceiling artifacts only, which are `tests/` and TOML | 0 | 0 |

**Seven** crates cross — `maos-kernel-core` (FLAG-Winston only), `maos-bin`, `maos-audit`, `maos-iac`, `maos-shell`, `maos-domain` and `maos-a2a-tcp` (pending its T0 measurement). **Code first, `cargo fmt --all`, `kloc-check --json`, then the `kloc.toml` row carries the
bare token `16-5` followed by a non-digit non-dash character, the measured figure and the driver, same commit.**
⚠ The RECOVERY-LANE CEILING RULE itself is `kloc.toml:49-95`, but **the tokenization rule is not in that file** —
it is enforced by `fn story_keys` in `xtask/tests/d11_xtask_ceiling_ratchet.rs:164-179` (`.split(|c: char|
!c.is_ascii_digit() && c != '-')` at `:166`, two-all-digit-parts at `:168-175`). A dev who reads `kloc.toml:49-95`
looking for it will not find it and may book all five raises wrong. ⚠ **`maos-kernel-core` is NOT covered by that rule** — it is the sole `ZERO_HEADROOM_CRATES`
member and needs its own operator-authorized grant plus `RATIFIED_AT_EASING` moving in the same commit.
⚠ **The aggregate alarm is already FIRING** (166542 vs 158608) and hardfail is 170884 — **+4342 of room, and
this story's ×1.3 upper bound totals ≈ +769.** State the new aggregate; do not silence the alarm.

**Dependencies:** none. No new crate, no lockfile entry, no new file under `crates/maos-bin/src` ⇒
**`SCANNED_SOURCE_FILES` stays 23** (`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:882`, directory-listing
assert `:971-985`). **No new verb** ⇒ `verb_table_15_3.rs` untouched, `check-exit-commands` unaffected.

---

## Decisions ruled in this story (rule 7 — one decision per fork)

Rulings marked **(§14)** are recommendations pending operator ratification; every other row is ruled by
measurement recorded in §1–§6.

| # | Fork | Ruling | Rejected, and why |
|---|---|---|---|
| **D-16-5-A** | Sizing | **WHOLE — OPERATOR-RULED 2026-09-17 (§14 Q1, §15 R6), OVERRULING this preflight's SPLIT recommendation.** One vehicle discharges D3, D5.1, D7, D12 and D18 and all twelve `deferred-work.md` rows. The operator's criterion is *spec fidelity + long-term correctness*, and on that criterion the seam fails: the two halves are **one capability** — *an operator being told the true cause of an outcome* — and AC1's own defect (D-16-5-K: an audit failure rendered as `Denied`) is **an instance of D18**, so splitting would have shipped the defect and its vocabulary fix in separate commits. The deny-vocabulary half also carries the only **shipped-ADR** statement of the shared invariant (`ADR-058:52`, *"distinguishable without string matching"*), which the audit-truth half has no equivalent of — so the half that proves the rule would have left with the half that needed it. Budget consequence accepted: `maos-domain` takes a measured raise. **The superseded recommendation, kept so the trade is auditable:** *SPLIT 2 WAYS — This key keeps the kernel grant + audit-truth charter; **`16-5b-cross-host-deny-vocabulary`** takes D7 + D18 + `deferred-work.md:834` at ZERO kernel-Δ. `16-5b-cross-host-deny-vocabulary` takes D7 + D18 + `deferred-work.md:707`/`:834` at ZERO kernel-Δ; 8 crates and 20 obligations is a lot for one story, and `maos-domain`'s +52 cannot hold AC8's mirrors and AC1's `CapError` without a raise.* **Overruled.** | *SPLIT* — see above; the seam cuts one capability in two and separates D-16-5-K's defect from its own vocabulary fix. The `maos-domain` raise it avoided is authorized by the RECOVERY-LANE CEILING RULE anyway (lane OPEN, key not `done`, measure-first), so the budget argument bought nothing a rule already grants. *Split by AC* — would put the journal fix and the legal-hold lock in different commits, needing two kernel grants. *Split off the audit-truth residuals* — they are the same objects AC1 touches and share the classifier table. |
| **D-16-5-B** | D3: kernel propagate vs out-of-kernel class-wide | **BOTH, and the register's ZERO-Δ rationale does not survive this story's existence (§14 Q2).** D3 chose out-of-kernel *because* it cost "zero kernel-core lines and zero re-pin" — but AC3 and AC4 already buy the grant and the re-pin, so the marginal cost of the propagate is **+1 kernel-core line, +3 `maos-domain`, zero broken call sites**, and only the propagate makes AC1's stated command true. The class-wide half is **not** optional either: the propagate repairs **1 of 9** sites. So: observability in `maos-capability` for all nine (sites 8 and 9 included), **plus** the propagate at `record_invocation`. ⚠ **Site 8 is repaired by counting, not propagating** — `cap_audit_backpressure.rs:106` is `adapter.revoke(…).unwrap()` and would panic under saturation. ⚠ **`record_verification` stays `()`** — re-typing the verify signature is outside D3 and outside the grant, as the epic says. | *D3's ruling alone* — AC1's headline command cannot pass; nothing in the tree changes behaviour. *The epic's propagate alone* — leaves 8 of 9 sites unrepaired and buys "refusal" at the one site where the response is already printed. *Propagate at all nine* — reds `cap_audit_backpressure` and turns a backpressure drop into a hard failure on the operator revoke path. |
| **D-16-5-C** | Re-kinding the eight/nine `lifecycle.*` rows | **DECLINED, with the cost priced (16-2 §15 R3 requires a recorded reason, never silence).** The epic's compensating claim — *"16-2's writer-shape table stays either way"* — is **FALSE** (§2b): a re-kind either orphans all eight doorbell entries or turns every historical row into a tokenless `Call`. Preserving both needs a second disposition per call site, changing `WriterShapeEntry`, `scan_tree`'s kind filter, both assertion loops and the **20**-row fixture table (`fr4_classifier_16_2.rs:136-301`, held 1:1 with the table's 20 `shape.is_some()` entries by the invariant at `:302-308`) — **unpriced rework on top of an already-crossing budget**. **The cost of declining, recorded:** every reader outside `maos-audit` — a SIEM, `verify.py`, a sealed-export consumer — sees a capability invocation without a token on every boot, and a false alarm trains people to ignore the real one. **AC2 instead makes the rows *classifiable and named*** — defect (b)'s ninth writer gets a disposition and the doorbell learns literal `DecisionDispatch`, so nothing is invisible even though nothing is re-kinded. Routed to `epic-16-retrospective` (§11 row 1) with this measurement attached. | *Re-kind now* — the rework is unbudgeted and the story is already crossing five ceilings. *Silently skip* — forbidden by 16-2 §15 R3. *Re-kind only the new writer (b)* — inconsistent: two lifecycle writers with two kinds is worse than eight with one. |
| **D-16-5-D** | AC1's falsifier site | **The three `?` sites in `main.rs` (`fn collective_write/read/scan`, `:1269/:1298/:1326`), never the shell.** Only there does `record_invocation` run **before** the work, so a fault-injected sink can actually refuse the invocation. **The shell site gets a disclosure obligation instead**: `let _ = host.record_turn(…)` (`crates/maos-shell/src/lib.rs:698`) becomes a counted, named drop that the class-wide instrument sees — the honest repair for a site where refusal is impossible. | *The shell site as the falsifier* — `render_turn` already printed (`lib.rs:508-512`); "the invocation is refused" would be theatre. *Reorder the shell to record first* — changes the operator's latency profile for a row the REPL does not block on, for no audit gain. |
| **D-16-5-E** | Fault injection | **Test-only, zero new production code. Drop the receiver.** `let (tx, rx) = tokio::sync::mpsc::channel::<CapAuditEvent>(1); drop(rx);` then pass `tx` to `CapabilityRegistryAdapter::new` (`capability/mod.rs:144-168`): every `try_send` returns `Err(Closed)` on the **first** call, deterministically, with no load. Both `cap_audit::Sender` (`crates/maos-capability/src/cap_audit/mod.rs:108`) and the constructor are `pub`, and `tokio` `features = ["sync"]` is a direct dep of both crates. The saturation vector (depth-1 + unpolled receiver) is the secondary. | *A new injectable-sink trait* — a seam invented for a test when `Closed` is already reachable; ADR-free production surface for nothing. *Rely on the 100K saturation loop* — non-deterministic, slow, and it cannot produce `Closed`, which is the writer-death mode §2 shows is the real hazard. |
| **D-16-5-F** | AC3's fix shape | **Shape A1 — a `std::sync::Mutex<()>` serializer field on `MemoryManagerAdapter`, guard taken at the TOP of `fn forget_with_reason` (before `:431`)**, so it spans the write branch (`:447`), the check (`:481`) and all four acts (`:530-531` / `:572-574` / `:580` / `:581`). I9 cost **zero** (already exempt, §3). Lock ordering is safe — the new lock is outermost, the TL is a leaf and never calls back. ⚠ **The `#[i9_exempt]` reason at `memory/mod.rs:77` and `docs/invariants/i9-exemptions.md:260` says *"Does NOT retain mutable state across calls"* — it becomes false and is amended in the same commit.** ⚠ **Do not "fix" the blocking guard with `tokio::sync::Mutex`**: `forget_with_reason` is sync with no `.await`, so the guard can never be held across a yield; a tokio mutex would force an `async fn` signature change and a far larger kernel-Δ. | *Shape B, a TL transaction* — **disproved** (§3): the acts span three stores on two connections plus the filesystem; it covers the check and one of four acts. *Shape A2, the field in `maos-iac`* — cheaper in kernel bytes (~+1 vs +5) and it would close the `release_legal_hold` direction for free, but it puts the kernel's atomicity invariant in an adapter the kernel pin does not cover, which is the wrong home for a correctness guarantee. Recorded as the fallback if the grant is contested (§14 Q3). *Serialize at the door* — `spirit_key()` is `None` by design for principal-scoped commands; keying them would serialize unrelated principals. |
| **D-16-5-G** | AC4's falsifier | **The deterministic single-process `recover_in_flight_with_tasks` seek (`journal/mod.rs:219-229`), plus the intra-process double-`JournalAdapter` pair (`main.rs:2946` + `:6485`).** Both red at HEAD with no timing. **The fix is `.append(true)` AND one `write_all` of a pre-built line** — measured, `O_APPEND` alone still tore 2161 of 3200 lines. | *A two-process race harness* — **it passes at HEAD** when opens are staggered (3200/3200 clean); a falsifier that can pass against the defect is the null control E15-A6 forbids. *Only `.append(true)`* — measured insufficient. *Only `write_all`* — does not fix cross-process cursor overwrite. |
| **D-16-5-H** | The register tripwire (§6a) | **Re-anchor D3, D7 and D18 from `` leaves `backlog` `` to `` reaches `done` `` by REPLACING the clause, in this story's creation commit.** The `Status` cell (`:39`) and binding rule 1 (`:43`) both forbid flipping them to `CLOSED` while implementation is unlanded, and the 14-1 precedent (`:452-480`) is exactly this move. ⚠ **Replace, never append** — `fn deadline_clauses` splits on `;` and evaluates each clause independently (`check_decision_register.rs:396-409`, pinned `decision_register_gate.rs:241-260`). D3's ruling text is recorded against its ID here per rule 1. D7/D18 keep `16-5` as Target until §14 Q1 is answered; if the split is ratified a follow-up edit re-points them to `16-5b`, which is a re-point to a **real** vehicle and so is not the rule-5 move. | *Close all three at preflight* — overrides `:39`'s explicit "including rows whose decision is ruled but whose implementation is not yet landed" and rule 1's "shipping adjacent work does not close a row." *Accept the red and repair in the first dev commit* — a Blocking gate red between story creation and dev start, with no attribution, is the shape 16-4 §0 had to write a whole section about. *Declare the anchor `UNQUERYABLE`* — it is perfectly queryable; the anchor is right and only its threshold is wrong. |
| **D-16-5-I** | The cascade emptiness proof (defect h) | **Add a private-tier count and make BOTH gates require it; never widen the claim.** `fn shared_tier_principal_row_count` (`crates/maos-audit/src/lib.rs:1868-1896`) gains a sibling over `maos_audit::default_memory_root()`, and `main.rs:9118-9119` / `:9227-9231` require both to be zero before `NotFound`. ⚠ **Precision the routed row gets wrong:** `NotFound` returns **before** `build_erasure_proof` (`:9354`), so no signed proof is involved — the artifact repaired is the **TL audit terminal** at `main.rs:9076` (`principal.uninstall.not-found`, exit 4). State which. | *Repair the signed proof* — not on this path; would be a fix to something that does not happen. *Report "indeterminate" when the private root is unreadable* — correct in spirit, but `UninstallCascadeTerminal` has no such variant and adding one is 13.5b contract surface this story has no charter for. §11 row 2. |
| **D-16-5-J** | The redactor rule (defect e) | **`sk-` becomes a token-boundary rule, not a substring scan** — it matches only at a position preceded by start-of-input or a non-`[A-Za-z0-9_-]` byte. Narrowest change that stops scrubbing `task-…` while still catching `sk-` at a real boundary (`"key":"sk-abc"`, ` sk-abc`). ⚠ **No test asserts that a `task-…` id IS scrubbed, so there is no red to fight** — but `crates/maos-bin/src/supervision.rs:225-230` and `crates/maos-bin/tests/worker_supervision_16_3.rs:186-232` built the four-group delegation-id format **around** this bug. They stay green (absence stays absent); **the story records whether that design constraint is retired or kept, and keeps it** — a format change is a separate, unbudgeted churn. | *Prefix-only at the start of a value* — misses `Bearer sk-…`. *Delete the rule* — it is a real secret-detection rule. *Also change the delegation-id format* — unrelated churn in a story already crossing five ceilings. |
| **D-16-5-K** | AC1's refusal cause | **A DISTINCT variant — never `ResearcherCollectiveError::Denied`. Round-table 2026-09-17, operator-ratified.** `crates/maos-bin/src/main.rs:1275` maps the audit failure into `Denied(String)`, the identical variant `:1286` uses for a real policy denial, so the two causes collapse into one stringly outcome. Routing AC1's new `CapError` through it would **reproduce D18's `-32001`-vs-`-32009` defect inside the repair filed beside it** — the story would ship a control whose failure mode is reporting the wrong cause, which is its own stated thesis inverted. All three sites (`:1269`, `:1298`, `:1326`) carry the distinct cause. | *Reuse `Denied`* — zero new lines, and it makes "policy refused you" and "the audit sink is down" indistinguishable to the operator and to every machine reader. *A free-text prefix on `Denied`* — preserving cause only in a string is the precise insufficiency D7's row already names. |
| **D-16-5-L** | Can the refusal itself be audited? | **NO — and the class-wide instrument is therefore the AUDIT OF LAST RESORT, stated as such, plus a latched degraded state. Round-table 2026-09-17, operator-ratified.** A refusal caused by an unavailable sink cannot be written to the sink; the counter is out-of-band by construction (an `AtomicU64` in `maos-capability`, never the channel) and is the only durable trace. And because writer-task death makes every subsequent `try_send` return `Closed` **for the process's lifetime** (§2), one refusal is never the real case — so the process latches a named degraded state that `GET /v1/daemon` reports and `maos audit query` discloses. Without it, pressure on the audit channel is a denial of service that reads as policy working normally. | *Count and move on* — `audit_drop_count()` already has zero production readers, and *"at least it is counted"* is the claim-standing-in-for-a-control shape this story exists to stop. *Write the refusal to the TL* — that is the component that failed. *Exit non-zero instead* — useless to a running daemon, which is the case that matters; it only helps at death. *Route the latch to the retrospective* — this story holds the only open kernel grant and the only vehicle whose charter is audit truth. |
| **D-16-5-M** | AC3's atomicity granularity | **PER-PRINCIPAL, asserted — not inferred. Round-table 2026-09-17, operator-ratified.** The cascade loops one principal at a time (`main.rs:9176-9177`), so the guard spans one principal's check-and-acts. A hold arriving mid-cascade protects every principal not yet reached and cannot recall those already erased. **This is the tree's shipped contract, not a new decision** — `mixed_held_uninstall_writes_partial_proof` (`erasure_uninstall_13_5b.rs:544`) already pins exit 3 and a `legal_hold`/`CoverageGap` category naming the held set (`:583-596`) — so the work is one vector and a sentence, not a design change. | *Leave it unstated* — a reviewer runs the two-thread test, sees green, and the interleaving the GDPR path actually executes stays untested; an undecided semantic is what this story exists to stop. *Whole-cascade atomicity* — needs a lock spanning N principals × four stores and breaks the 13.5b mixed-terminal contract (`uninstall_exit_codes_13_5b.rs:17` pins `("held", 3)`). *A separate story* — the answer is already in the tree; a second vehicle for a sentence is the bucket rule 10 forbids. |

---

## Acceptance Criteria (8)

1. **AC1 (command — a refused audit sink refuses the call, and every drop is counted once).** With a
   fault-injected sink (**D-16-5-E**: a `tokio::sync::mpsc::channel::<CapAuditEvent>(1)` whose receiver is
   dropped, passed to `CapabilityRegistryAdapter::new`), a collective write through
   `fn collective_write` (`crates/maos-bin/src/main.rs:1269`) **is refused with a cause the operator can act on
   and `self.memory.collective_write` is never reached** — asserted by observing the store, not the return value.
   ⚠ **NOT `Denied` (D-16-5-K, round-table 2026-09-17).** `main.rs:1275` today maps an audit failure to
   `ResearcherCollectiveError::Denied(error.to_string())` — **the same variant `:1286` uses for a real policy
   denial.** Wiring the new refusal into it would make *"policy refused you"* and *"the audit sink is down"*
   indistinguishable at the operator surface: **the exact `-32001`-vs-`-32009` collapse AC3/D18 exists to remove,
   reproduced inside AC1's own repair.** A distinct variant carries it, and **`Denied` is never the audit-failure
   cause at any of the three sites.**
   `fn record_invocation` (`crates/maos-kernel-core/src/capability/mod.rs:332-352`) returns a new
   `CapError` variant (`crates/maos-domain/src/ports/capability.rs` — enum head `:113`, append after the last variant at `:141`, never *at* `:140` which is `PolicyDenied`'s ident line) instead of `Ok(())` at
   `:351`; the same vector at `fn collective_read` (`:1298`) and `fn collective_scan` (`:1326`) refuses
   identically. **`fn record_verification` (`capability/mod.rs:233-245`) is left returning `()`** — re-typing the
   verify signature is outside D3 and outside the grant. Separately, **all nine drop sites become observable
   through one instrument in `maos-capability`** (§2 table), built on the `OtelTraceSink::drop_count` (`otel_sink.rs:415`) +
   `gate:otel-degradation` precedent (`crates/maos-telemetry/tests/otel_gates.rs:818`) and **read by a named
   test**, because `audit_drop_count()` has zero production readers today and *"at least it is counted"* is a
   claim standing in for a control. Site 8 (`fn revoke`, `crates/maos-capability/src/cap_tokens/mod.rs:294-296`)
   gains the missing `record_drop()`; site 9 (`crates/maos-acp/src/notification_channel.rs:59-64`) closes its own
   `// TODO`. The swallow at `crates/maos-shell/src/lib.rs:698` becomes a **counted, named** drop (D-16-5-D) —
   it is not converted to a refusal, and the AC says why. ⚠ **The instrument IS the audit of last resort, and the AC says so (D-16-5-L, round-table 2026-09-17).**
   A refusal on an unavailable sink **cannot be written to the audit log — the log is what failed** — so the
   counter in `maos-capability` is not merely a tally, it is the only durable trace that a refusal happened.
   It is out-of-band by construction (an `AtomicU64`, never the sink), which is precisely why it can carry this.
   Two consequences the AC asserts: **(i)** every audit-caused refusal increments it, and **(ii)** once the sink
   is `Closed` the process **latches a named degraded state** — because writer-task death is not one refusal, it
   is *every* call for the process's lifetime (§2) — which `GET /v1/daemon` reports and `maos audit query`
   discloses, so an operator reading the log learns it is incomplete **from the log itself**. Without (ii) an
   attacker who can pressure the audit channel gets a denial of service that reads as policy working normally.
   **Proven red five ways:** (1) revert the propagate ⇒
   the collective-write vector passes with the data written — recorded, reverted; (2) point the instrument at a
   single site ⇒ the nine-site assertion reds on the count, not on a name grep — *"the instrument exists" is NOT
   the assertion*; (3) run the whole vector with a **healthy** sink ⇒ zero drops and the write succeeds, so the
   test cannot pass by refusing everything; (4) map the audit failure to `Denied` ⇒ the vector can no longer tell
   it from a policy denial and reds — *the assertion is the cause an operator reads, not that a refusal
   occurred*; (5) kill the writer task, then query ⇒ at HEAD the log is silently incomplete and exit code is
   unchanged; after the repair the degraded state is on the door and in the query output.

2. **AC2 (the audit record stops claiming what did not happen).** Six measured repairs, each with its own
   vector, and one recorded decline:
   **(a)** `fn insert_kernel_event_returning_id` (`crates/maos-iac/src/adapter/transparency_log.rs:1373-1414`)
   stops hardcoding `TaskComplete` (`:1400`) and the `None` token (`:1399`): the kind and token come from the caller,
   and **all nine call sites** are audited — the three real-pid ones (`main.rs:6350`, `:11144`, `:11246`) and the
   **four pid-0 production callers the epic never names** (`main.rs:9076`; `crates/maos-kernel-core/src/memory/mod.rs:462`,
   `:492`, `:593`). **(b)** `fn emit_task_orphaned` (`crates/maos-kernel-core/src/halt/resolver.rs:218-230`)
   writes a JSON payload instead of the bare `format!` at `:221`, so `fn payload_matches`
   (`crates/maos-audit/src/fr4_classifier.rs:528`) can parse it and its `WriterShapeEntry::call(…)` at `:374`
   — the entry that names **16-5** in production source — becomes a `NonCall` with an exact shape. **(c)** The
   ninth lifecycle writer (`fn resolve_verb`, `crates/maos-kernel-core/src/scheduler/verb_resolver.rs:124-137`)
   gains a disposition **and the doorbell learns literal `DecisionDispatch`** (`fr4_classifier_16_2.rs:659-673`,
   whose `continue` at `:672` is what hides it) — so it is no longer both a violation and invisible.
   **(d)** `fn handle_crash` (`crates/maos-kernel-core/src/supervision/crash_detector.rs:161-165`) stops
   zero-padding a 16-byte `TokenId` into the 32-byte column; the row carries a token a reader can round-trip, or
   NULL. In the same pass the three tokenless kind-1 dispositions written at pid 0 (`fn emit_task_complete_nack`
   `crates/maos-iac/src/adapter.rs:239`, `fn emit_task_complete_escalated` `:258`, `fn reassign_task_to` `:280`)
   are **dispositioned with a recorded reason**: T0 determines whether the originating Spirit's pid is
   recoverable at each site — if it is, the row carries it; if it is not, the AC records **why a pid-0
   `task.complete` row is the honest shape** rather than leaving it undecided. They are already `NonCall`, so
   nothing is red; the defect is semantic, and an undecided semantic defect is the shape this story exists to
   stop.
   **(f)** ⚠ **The redactor stops scrubbing the ids the record needs to be readable** (D-16-5-J). The `sk-`
   `RedactionRule` (`crates/maos-iac/src/adapter/redaction.rs:80-83`), applied by `fn redact_unscoped`
   (`:142-190`) which tests **every** index `i` (`:154`), becomes a **token-boundary** rule — matching only at
   start-of-input or after a non-`[A-Za-z0-9_-]` byte. Asserted both ways on the real redactor:
   `{"task_id":"task-worker-1"}` survives intact where it is `ta<REDACTED:type=api_key_generic,len=11,hash=a9b1>`
   at HEAD, **and** `{"key":"sk-abcdef"}` is still redacted. The four-group delegation-id format
   (`crates/maos-bin/src/supervision.rs:231-246`) is **kept** and the story records that its motivating
   constraint is retired — a format change is separate, unbudgeted churn.
   **(g)** ⚠ **The cascade's emptiness proof looks in both tiers** (D-16-5-I). `fn shared_tier_principal_row_count`
   (`crates/maos-audit/src/lib.rs:1868-1896`) gains a private-tier sibling over `maos_audit::default_memory_root()`
   (`:1515`), and **both** gates — `crates/maos-bin/src/main.rs:9118-9119` and `:9227-9231` — require both counts
   to be zero before `NotFound`. **Proven red:** empty the shared tier, populate the private tier ⇒ at HEAD the
   cascade returns `NotFound` and durably commits a terminal that says *"provably nothing to erase"* over data
   that exists; after the repair it does not. ⚠ The repaired artifact is the **TL audit terminal**
   (`main.rs:9076`, whose intent literal `"principal.uninstall.not-found"` is at `:8889` and whose exit code 4
   is at `:8880`) — **not** a signed proof: `NotFound` returns before `build_erasure_proof` (`:9354`).
   **(e) The re-kind is DECLINED and the cost is recorded in the story (D-16-5-C)** — never silently
   skipped. **Proven red (c):** revert (c) ⇒ `verb_resolver`'s row is an `Fr4SchemaViolation` that the inventory
   test still reports as clean — **the doorbell passing is NOT evidence, and the AC asserts the row, not the
   doorbell.** **Proven red (a):** revert the caller-supplied kind ⇒ each of the nine sites writes kind 1 again
   and the four pid-0 production callers reappear as tokenless `Call` rows.

3. **AC3 (two operators cannot both win).** Two threads on one `Arc<MemoryManagerAdapter>` against the same
   principal — one `forget_with_reason(p, None)`, one `forget_with_reason(p, Some("legal-hold:case-16-5"))`,
   released together from a `std::sync::Barrier::new(2)` (the tree's precedent:
   `crates/maos-bin/tests/shell_host_16_2.rs:1043`) — **never** produce `(Erased, Suspended)` together over
   ≥200 iterations. The serializer is **D-16-5-F Shape A1**; under hold the outcome stays
   `ForgetOutcome::Suspended{hold}` (`crates/maos-kernel-core/src/memory/mod.rs:475`, `:505`;
   `ForgetOutcome::Suspended{hold}` is `crates/maos-domain/src/memory.rs:429-430`, enum head `:423`) and **no `MemoryError` variant is added**. The erasure artifact
   assertion is `ErasureCategory{name: "legal_hold", status: CategoryStatus::CoverageGap}`
   (`crates/maos-bin/src/main.rs:9338-9348`) **and the absence of a `regional-teardown-*.json`** (`:9397-9400`)
   — ⚠ ***"the proof carries `completed:false`" is NOT the assertion: there is no `completed` field on this
   path*** (§3). The corpus contract is unchanged: `crates/maos-audit/src/bin/gen_gdpr_cascade.rs:124`
   (`"legal_hold" => "held"`), its 9 `legal_hold` scenarios, and `crates/maos-audit/tests/gdpr_cascade_corpus_test.rs:325-331`
   (`hold.reason.starts_with("legal-hold")`, `hold.status.contains("SUSPENDED")`). The `#[i9_exempt]` reason
   (`memory/mod.rs:77`, `docs/invariants/i9-exemptions.md:260`) is amended in the same commit because
   *"Does NOT retain mutable state across calls"* becomes false.
   ⚠ **The guarantee is PER-PRINCIPAL, and the AC states it rather than leaving the granularity to be inferred
   (D-16-5-M, round-table 2026-09-17).** `fn run_uninstall_cascade_inner` calls `forget_with_reason` in a loop,
   one principal at a time (`crates/maos-bin/src/main.rs:9176-9177`), so the guard spans **one principal's
   check-and-acts**, not the whole cascade: a hold arriving mid-cascade suspends every principal not yet reached
   and cannot recall those already erased. **This is the tree's existing contract, not a new choice** —
   `mixed_held_uninstall_writes_partial_proof` (`crates/maos-bin/tests/erasure_uninstall_13_5b.rs:544`) already
   pins exit 3 plus a `legal_hold`/`CoverageGap` category naming the held principals (`:583-596`). **Second
   vector:** place a hold between two principals of a running cascade ⇒ the earlier principals are `Erased`, the
   later ones `Suspended`, the terminal is the mixed one, and the proof names exactly the held set — never a
   whole-cascade rollback and never a silent full erase. ⚠ *"The two-thread test is green"* is NOT sufficient:
   without this vector the interleaving that the GDPR path actually runs is untested. **Proven red:** remove the guard ⇒ the race test
   reds within the iteration budget, transcript recorded — and ⚠ **run it at HEAD first**: there is no race
   coverage in the tree (`loom` appears nowhere), so this test is the first, and a test that has never been seen
   red is not a control. **Declines, stated:** `RELEASE-HOLDS.md:52` (NFR-Ops-11's operator axis) and `:53`
   (*"an unauthorized legal-hold bypass is not constructible on collective rows"*) do not move; AC3 claims
   neither.

4. **AC4 (the Lifecycle Journal cannot tear or overwrite).** `JournalAdapter::open`
   (`crates/maos-kernel-core/src/journal/mod.rs:97-103`) gains `.append(true)` **replacing** `.write(true)`, and
   `fn append_transition` (`:177-181`) writes **one `write_all` of a pre-built line including its newline**
   instead of `write!(file, "{line}\n")` at `:180`. **Both halves, because measured:** `write!` on a raw `File`
   is **2 `write(2)` syscalls** and `O_APPEND` + `write!` still tore **2161 of 3200** lines. The falsifier is
   **D-16-5-G**: the deterministic single-process `fn recover_in_flight_with_tasks` seek (`:219-229`, which
   swallows its read error at `:227`) — HEAD yields a torn first line and **5 records where 6 were written** —
   plus the intra-process double-`JournalAdapter` pair (`crates/maos-bin/src/main.rs:2946` and `:6485`).
   ⚠ ***A two-process harness with staggered opens PASSES at HEAD (3200/3200 clean) and is NOT the assertion.***
   The false comment at `main.rs:3047` (*"file-append is atomic for small writes on POSIX"*) is corrected, and so
   is the one at `main.rs:8917-8920` that already routes this hazard here by name.
   `crates/maos-kernel-core/tests/journal_fsync_assertion.rs`'s **P99 < 1.5 ms over 10 000 appends** is re-run and
   reported — ⚠ **it `panic!`s on the budget only when `CI`/`GITHUB_ACTIONS` is set or `MAOS_ENFORCE_NFR=1`
   (`:69-84`), and otherwise only `eprintln!`s**, so a local re-run is green either way: set `MAOS_ENFORCE_NFR=1`
   or the measurement is not a control.
   **Proven red three ways:** (1) run the `recover_in_flight_with_tasks` vector at HEAD ⇒ 5 records where 6 were
   written, first line torn — recorded before the repair; (2) apply `.append(true)` **alone** ⇒ lines still tear
   (measured 2161 of 3200); (3) apply `write_all` **alone** ⇒ the cross-process cursor overwrite persists. Each
   recorded and reverted. Also here: the REPL clarification gains a size cap (`crates/maos-shell/src/lib.rs:431`) consistent
   with `MAX_BODY_BYTES` (`crates/maos-control/src/lib.rs:215`), because the oversized string lands in this
   journal (`deferred-work.md:936`).

5. **AC5 (a halt reports resolved only if the resolution is durable).** Three routed rows, one failure:
   **(a)** `crates/maos-kernel-core/src/halt/resolver.rs:143-175` stops flipping the registry to `Resumed`
   **before** the context write — either the write precedes the flip, or a failed write rolls the state back, so
   a retried `maosctl halt resolve` is accepted instead of answering `halt_already_resolved` over an
   unrecoverable clarification (`deferred-work.md:932`).
   **(b)** ⚠ **The routed row's shape is INVERTED and the repair site is a different file — measured.**
   `fn submit_resolution` (`crates/maos-director-surface/src/halt_ui.rs:74-85`) calls
   `self.resolver.resolve(&halt_id, resolution.clone())?` at **`:81`** and journals only afterwards at
   **`:82-83`**. The door is **already honest**: `HaltUiError::Audit` falls through to
   `OperatorOutcome::Failed{code: "halt_resolution_failed"}` (`crates/maos-bin/src/operator_door.rs:978-981`),
   so *"the door returns success with no approval row"* is **false at HEAD, and AC5(b) written that way would
   pass with zero code change.** The real defect is (a)'s shape one layer up: a journal failure leaves the halt
   **`Resumed` in the registry with no `approval_decision_log` row** while the operator is correctly told it
   failed — so the retry answers `halt_already_resolved` and the clarification is unrecoverable. **The repair is
   `crates/maos-director-surface/src/halt_ui.rs:80-84`** — journal before `resolver.resolve`, or roll the
   resolve back when the journal write fails.
   **(c)** `fn classify_halt_record` (`crates/maos-cli/src/subcommands.rs:2003-2004`) classifies the kernel's
   serde-failure **termination** receipt (`crates/maos-kernel-core/src/halt/termination.rs:105-110`) as a
   termination, not as `raised` (`:934`) — the one `maos-cli` row in this AC, here because it renders the very
   halt list (a) and (b) make trustworthy.
   **Proven red:** inject a `memory.write` failure at (a) and a journal-write failure at (b) ⇒ at HEAD **the
   halt is left resolved and un-retryable in both cases** — (a) reporting success, (b) reporting `Failed` — and
   after the repair both leave it retryable with registry and approval log agreeing. ⚠ ***"The door reports
   success" is NOT the assertion for (b) and would pass at HEAD***: the assertion is registry state versus
   `approval_decision_log` content. Both transcripts recorded.

6. **AC6 (the grant, the pin, and all five artifacts).** `cargo run -p xtask -- check-kernel-baseline` PASSES
   after the re-pin, and **every one of the five artifacts a re-pin touches moves in this one commit** —
   because 16-3's re-pin moved three and needed **two follow-up commits** to clean up (§6b):
   (i) `xtask/kernel-core-baseline.toml` `[kernel_src]`/`[kernel_src.files]` replaced from
   `check-kernel-baseline --emit-pin`, `src_lines` hand-set to the measured value at **`:488`**, and a
   `# HISTORY:` block naming the value, the driver, the changed files and the FLAG-Winston authorization (copy
   `822070d7`'s shape); (ii) `xtask/kloc.toml:247` `maos-kernel-core = <measured>` with the driver named inline;
   (iii) `xtask/tests/recovery_lane_ceiling_rule.rs:268` `RATIFIED_AT_EASING` to the same value — ⚠ **the
   recovery-lane easing does NOT authorize this crate (`:46` `ZERO_HEADROOM_CRATES`, `:76-78`
   `Refusal::ZeroHeadroomCrate`); only the operator grant does**; (iv) `xtask/tests/kernel_pin_content_hash_16_0.rs`
   — **four** `24477` literals at `:802`, `:820`, `:829`, `:840` **and the line-position assert at `:846-851`**
   (`assert_eq!(numbered + 1, 488, …)`), which the new HISTORY block **will** move, exactly as 16-3's block moved
   it 481 → 488; (v) the other four crates' measured `kloc.toml` raises, each citing the bare token `16-5`.
   `PINNED_ENTRIES = 98` (`:33`) does **not** move — no kernel file is added or deleted.
   **Proven red:** re-pin only `src_lines` and leave the digests ⇒ the gate reds naming each changed file — the
   evidence that AC4-as-written (which names only `src_lines`) was measuring the pre-16-0 instrument.

7. **AC7 (an operator learns WHY a cross-wall recall was refused — D7).** `CrossWallRecallRefusal`'s six variants
   (`crates/maos-domain/src/log_recall.rs:291-304`: `NoConsentProvider`, `NoGrant`, `WrongDirection`,
   `ConsentStateStale`, `ConsentStateUnavailable`, `ReadPortUnavailable`) **stop collapsing into the token
   `refused` at both machine-readable surfaces.** ⚠ **D7's headline is imprecise and this AC states the true
   shape:** the type is *already* typed in Rust and carried typed through
   `LogRecallError::ECrossWallRecallDenied{team, reason}` (`:309-313`) — it collapses **only at the surfaces**,
   and that is the defect. (i) The TL journal writes `"outcome": "refused"` with the cause as free text
   (`"refusal": refusal.map(ToString::to_string)`, `crates/maos-iac/src/adapter/log_recall.rs:96-119`, written
   at `:406` and `:414`); (ii) the operator verb — **`maos traceback --team <T> --spirit-pid <pid>`**, parser
   `crates/maos-bin/src/main.rs:385`, runner `:429`, render `:2611-2646` — emits
   `"outcome": "refused"` from `.then_some("refused")` at **`:2633`**, with the cause surviving only inside the
   `error` string. ⚠ **This violates a binding ADR verbatim**:
   `docs/adr/ADR-058-cross-wall-provenance-and-consented-recall.md:52` requires all six *"remain distinguishable
   **without string matching**."* True in Rust, **false at every surface a machine reads** — which is a stronger
   lever than D7's own wording and is what AC7 asserts. ⚠ **There is no `maosctl` verb** (`grep traceback
   crates/maos-cli/src` = zero hits); do not invent one. Neither surface touches `maos-domain`, so AC7 does not
   compete for the crate AC8 exhausts. **Proven red:** drive each of the six refusals and assert six distinct
   machine-readable outcomes ⇒ at HEAD all six render the identical `outcome` token and are separable only by
   substring-matching `error`; after the repair `WrongDirection` and `NoGrant` — which tell an operator
   completely different things — are distinguishable without reading prose.

8. **AC8 (a cross-host deny says which deny it was — D18).** `fn map_a2a_error_to_iac_bus`
   (`crates/maos-a2a-core/src/router.rs:1933-2063`, 18 arms, no wildcard) stops flattening the deny vocabulary.
   ⚠ **The collapse is TWELVE variants across ELEVEN arms, not the seven-across-six the epic names** — the six
   the epic omits are `PeerInternalFailure` (`:1986`), `PeerIntakeTimeout` (`:1991`), `ConfigInvalid` (`:2005`,
   which carries **no peer and no prefix**), `SpiritRestartDetected` (`:2008`), and the two security-path
   refusals `PeerIdentityMismatch` (`:2016`) and `ConsentGranterMismatch` (`:2021`). Typed variants land on
   `IacBusError` (`crates/maos-domain/src/iac_bus_types.rs:14-109`) — which **already marks
   `CrossHostRouteFailure(String)` DEPRECATED at `:69-70`, *"use the typed sub-variants above instead"***, so
   this completes a stated intent rather than inventing one. ⚠ **D18's measured `+6 maos-domain` is unreachable
   and the AC says why:** `crates/maos-domain/Cargo.toml` has **no `maos-a2a*` dependency** and the edge is
   one-way by ADR-010 (restated inline at `iac_bus_types.rs:66-68`), so `maos-domain` **cannot name**
   `UnclassifiedReason` (`maos-a2a-core/src/error.rs:30`), `IntentDirection` (`:18`) or `CohortConsentDenial`
   (`cohort.rs:79`) — a *typed* reason needs **mirror enums in `maos-domain` plus translation arms in
   `maos-a2a-core`**, measured **+50…+59**, not +6. A `reason: String` would fit +52 and is **refused**: it
   reinstates the defect. Also here: `IntentDeniedAtPeer` (`:1946-1952`) stops stuffing the NACK **message**
   into a field named `intent` while its sibling (`:1935-1945`) puts a real `consent_match_key` there — *the
   field lies about itself*; and the `deferred-work.md:834` row is closed by typing
   `TransportFailed("awaiting response")` (`crates/maos-a2a-tcp/src/transport.rs:602-603`) with a frame id, so a
   partition is separable from a slow-but-live peer. ⚠ **`IacBusError` is NOT `#[non_exhaustive]` and there is
   no exhaustive `match` on it anywhere in the tree** — every one of the 77 references has a catch-all — so
   adding variants breaks **zero** call sites; add `#[non_exhaustive]` in the same commit so the next widening
   stays free. **Proven red twice:** (1) `crates/maos-a2a-core/tests/fail_closed_8_8.rs:305-348` currently
   **pins the collapse as correct** — it asserts both unclassified variants produce `CrossHostRouteFailure(msg)`
   and checks only `msg.contains("absent")` — so it must be rewritten to assert the typed pair; capture its
   pre-rewrite green as the evidence the collapse was load-bearing. (2) Drive a `-32001` and a `-32009` refusal
   to the operator surface ⇒ at HEAD they are separable only by substring-matching `"route failed"` vs
   `"intent denied"`, while `fail_closed_8_8.rs:216-240` proves the router seam already distinguishes them —
   the invariant exists one layer down and is destroyed one layer up. ⚠ **Do not touch
   `crates/maos-bin/src/main.rs:10567-10585`** (the 14-2a `TcpA2ATransport` pin note) — the epic's `:10037` cite
   for it is stale by ~530 lines and points at enterprise governed-read wiring. ⚠ **`router.rs:1938`'s
   `IntentDirection::Accept` arm is DEAD in production** (the sole constructor is `:925-932` with
   `direction: Send`; every other hit is a match pattern) and nothing pins that — so `direction` is the wrong
   axis for the local/remote distinction and the `…AtPeer` **variant** carries it; either pin the Send-only
   invariant with a test or retire the field, and say which. ⚠ **FR63 check at T0:**
   `xtask/src/check_error_catalog.rs:8-14` enforces catalog bijection only for `E`-prefixed variant names, so
   `CrossHost*` needs no row today — **but if any new variant is named `E…`, `requirements-inventory.md:90`
   (FR63) and NFR-Doc-2 fire and the catalog + troubleshooting page land in the same commit.** Measure it before
   naming anything.

---

## Review obligations (§A6)

**(a)** Run `grep -rn '\.record_invocation(' crates/*/src` and `grep -rn 'record_invocation' crates/*/src` and
confirm 1 vs 4; the story's instrument must be the second. **(b)** Point AC1's falsifier at
`crates/maos-shell/src/lib.rs:698` instead ⇒ confirm the response is already rendered and "refused" is
unachievable; recorded. **(c)** Count the drop class independently; it must be **nine**, and sites 8
(`cap_tokens/mod.rs:294-296`) and 9 (`maos-acp/src/notification_channel.rs:59-64`) must both be in it.
**(d)** Run `cargo test -p maos-kernel-core --test cap_audit_backpressure` before and after ⇒ green both times;
then extend the propagate to site 8 ⇒ confirm `:106`'s `.unwrap()` panics, and that this is why D-16-5-B does
not. **(e)** Kill the audit writer task mid-run ⇒ confirm every `try_send` returns `Closed` and the
class-wide instrument reports it; confirm the process's exit code is unchanged at all eleven drain sites.
**(f)** Delete the new `CapError` variant ⇒ AC1 reds at the three `?` sites, not at the shell. **(g)** Run AC1's
vector with a **healthy** sink ⇒ the write succeeds; a test that passes by refusing everything is vacuous.
**(h)** Re-derive the eight `lifecycle.*` writers; confirm the 8th (`crash_detector.rs:207`) is
`FrameOrigin::Kernel`, not `SpiritAuto`, and that the epic's claim was wrong. **(i)** Confirm the re-kind decline
is written with its cost, per 16-2 §15 R3; confirm the `verb_resolver` writer is dispositioned **and** the
doorbell's `continue` at `fr4_classifier_16_2.rs:672` no longer hides a literal `DecisionDispatch`.
**(j)** Write `{"task_id":"task-worker-1"}` through the real redactor before and after ⇒ before it becomes
`ta<REDACTED…>`, after it survives; then write `{"key":"sk-abcdef"}` ⇒ still redacted both times.
**(k)** Confirm `crates/maos-bin/tests/worker_supervision_16_3.rs:186-232` stays green and that the four-group
delegation-id format is **kept**, with the story recording that its motivating constraint is retired.
**(l)** Enumerate all nine `insert_kernel_event_returning_id` call sites; confirm the two `#[cfg(test)]` ones
(`crates/maos-bin/src/cross_team_crossing.rs:935`, `:962`) are excluded and the four pid-0 production ones are covered.
**(m)** Empty the shared tier but populate the private tier ⇒ the cascade must NOT return `NotFound`; confirm
the repaired artifact is the TL audit terminal at `main.rs:9076`, not a signed proof. **(n)** Run AC3's race
test **at HEAD first** ⇒ it must red; a race test never seen red is not a control. **(o)** Replace AC3's
assertion with `"the proof carries completed:false"` ⇒ confirm there is no such field and the assertion cannot
be written. **(p)** Delete the `#[i9_exempt]` reason amendment ⇒ confirm `check-empty-kernel` passes anyway
(substring match, `check_empty_kernel.rs:166-169`); recorded — this is a correctness obligation the gate cannot
enforce. **(q)** Attempt Shape B (a TL transaction) far enough to confirm it cannot span
`principal_index.forget` and `private.forget_principal`; recorded. **(r)** Run AC4's falsifier as a
**staggered-open two-process** harness ⇒ it passes at HEAD (3200/3200); recorded as the null control that was
rejected. **(s)** Apply `.append(true)` alone ⇒ confirm lines still tear (measured 2161/3200); apply `write_all`
alone ⇒ confirm cross-process overwrite persists. **(t)** Re-run
`crates/maos-kernel-core/tests/journal_fsync_assertion.rs` and report the P99 both ways.
**(u)** For AC5(a) and (b), inject the write failure at HEAD ⇒ both must report success; recorded before the
repair. **(v)** Confirm **all five** re-pin artifacts moved in one commit; then revert artifact (iv) alone ⇒
`cargo test -p xtask --test kernel_pin_content_hash_16_0` reds on the line-position assert. **(w)** Revert
`RATIFIED_AT_EASING` ⇒ `cargo test -p xtask --test recovery_lane_ceiling_rule` reds; confirm the message names
the operator-decision requirement. **(x)** Run `cargo run -p xtask -- check-decision-register` after the §12
register edits ⇒ PASSED; then restore one `leaves backlog` clause ⇒ `expired-and-open`. Confirm the re-anchor
**replaced** rather than appended (`decision_register_gate.rs:241-260`). **(y)** Confirm D5 and D12 are declined
**in writing** with their reasons, and that no row was closed by implication (binding rule 1).
**(z)** Confirm `SCANNED_SOURCE_FILES` is still **23** and no new file was added under `crates/maos-bin/src`.
**(aa)** Every AC assertion reads state the production code wrote, not state the test set (E15-A6).
**(ab)** State the new aggregate against the alarm (158608) and hardfail (170884); confirm the alarm was **not**
silenced and the hardfail was not crossed. **(ac)** Map AC1's audit failure to `ResearcherCollectiveError::Denied`
⇒ the vector must red because the cause is no longer distinguishable from `main.rs:1286`'s policy denial; confirm
`Denied` appears at none of the three audit-failure paths (D-16-5-K). **(ad)** Kill the writer task, then run a
mediated call, then `maos audit query` and `GET /v1/daemon` ⇒ both must name the degraded state; at HEAD both are
silent and the exit code is unchanged. Then revert the latch ⇒ only the first refusal is visible and the
process's remaining lifetime is silent again (D-16-5-L). **(ae)** Confirm the instrument is read by a named test
and that the degraded flag is set from the **`Closed`** path, not only from `Full` — writer death and
backpressure are different failures and only one of them is permanent. **(af)** Start a cascade over ≥3
principals, place a legal hold between principals 1 and 2 ⇒ 1 is `Erased`, 2 and 3 `Suspended`, terminal is the
mixed one and the proof names exactly {2,3}; confirm no whole-cascade rollback and no silent full erase, and that
`uninstall_exit_codes_13_5b.rs:17`'s `("held", 3)` still holds (D-16-5-M). **(ag)** Confirm **no** `16-5b` row was created and that D7's and D18's Target cells still read `16-5` — Q1 was
ruled WHOLE, so the three-step transaction must NOT have been performed. **(ah)** Drive all six
`CrossWallRecallRefusal` variants through `maos traceback` and through the TL journal ⇒ six distinct
machine-readable outcomes, none requiring a substring match; at HEAD all six render the same `refused` token
(`main.rs:2633`). Confirm ADR-058:52 is cited in the test. **(ai)** Capture `fail_closed_8_8.rs:305-348` **green
before** rewriting it — it currently asserts the collapse is correct, and that transcript is the evidence the
defect was load-bearing; then confirm the rewrite asserts the typed pair and that `fail_closed_8_8.rs:216-240`
still passes unchanged. **(aj)** Grep for an exhaustive `match` on `IacBusError` after adding variants ⇒ zero;
confirm `#[non_exhaustive]` landed. **(ak)** Confirm `main.rs:10567-10585` (the 14-2a `TcpA2ATransport` note) was
NOT touched, and that the dead `IntentDirection::Accept` arm (`router.rs:1938`) was either pinned by a test or
retired — with the choice stated. **(al)** Run `cargo run -p xtask -- check-error-catalog` after AC8's variants
land ⇒ green, and confirm no variant was named `E…` without its catalog row and troubleshooting page.

---

## 11. Declared cut lines (rule 10 — each names an owner, never a bucket)

| # | Item | Measured evidence | Destination |
|---|---|---|---|
| 1 | **Re-kinding the eight/nine `lifecycle.*` rows off kind 7** | §2b: the epic's "table stays either way" is false; preserving both directions needs a second disposition per call site + `WriterShapeEntry` + `scan_tree`'s kind filter + both assertion loops + the **20**-row fixture table (`fr4_classifier_16_2.rs:136-301`, pinned 1:1 at `:302-308`) | **`epic-16-retrospective`** — D-16-5-C declines it with the cost priced; AC2(c) removes the invisibility half so nothing is hidden meanwhile. Retro row (§12) carries this measurement |
| 2 | **An `Indeterminate` uninstall terminal** when the private root is unreadable | `UninstallCascadeTerminal` has no such variant; adding one is 13.5b contract surface (`crates/maos-cli/tests/uninstall_exit_codes_13_5b.rs:17` pins `("held", 3)`) | **`epic-16-retrospective`** — AC2/D-16-5-I makes the count honest; a new terminal is a contract change with no charter here |
| 3 | **`CollectivePortError` collapses eight causes into one** | `RELEASE-HOLDS.md:51` row 1; `deferred-work.md:664` (section heading `:662`). Its successor names *"Future FLAG-Winston kernel-widening decision"* and this is the only open FLAG-Winston vehicle — but it is deny-vocabulary, not audit truth **`epic-16-retrospective`** — its own stated fallback, now the operative one since the WHOLE ruling retired `16-5b`. ⚠ **§14 Q4 remains open**: this is the only vehicle holding a FLAG-Winston grant, so *pull it in* is still available to the operator; rule 10 says route by charter, not by grant, which is why the preflight does not take it unasked |
| 4 | **D7 + D18, and both their `deferred-work.md` rows `:707` and `:834`** | — | ⚠ **NOT CUT — IN SCOPE, operator-ruled WHOLE 2026-09-17 (§15 R6).** They are **AC7 and AC8**. `16-5b-cross-host-deny-vocabulary` is retired and never created; D7's and D18's register Target cells stay `16-5`, so no re-point and no three-step transaction is needed |
| 5 | **FR63's typed-error-catalog obligation** — `xtask/src/check_error_catalog.rs:8-14` enforces bijection only for `E`-prefixed variant names, so `CrossHost*` needs no catalog row **today** | `error-catalog.toml:26-32` scans both crates; `ECrossWallRecallDenied` is already catalogued at `:53-57`, so touching `CrossWallRecallRefusal`'s variant NAMES would break it | ⚠ **NOT CUT — IN SCOPE under WHOLE.** Measured at **T0** inside AC8: if any new variant is named `E…`, FR63 (`requirements-inventory.md:90`) and NFR-Doc-2 fire and the catalog row + troubleshooting page land in the same commit. Name nothing before measuring |
| 6 | **`bench-audit-query-latency` is not in `aggregate.needs`** (125 entries; 165 jobs), and `discipline.yml:4-7` triggers only on `main` push/PR — so on `recovery-lane` it runs only in a PR into main | D12 is genuinely repaired (the bench is correct, enrolled, and fails loudly on a vacuous measurement); this is a **gate-binding** defect, not a bench defect. ⚠ **`check-decision-register` has the SAME defect and this row carries it too** — it is in `v1-0-ship-gate.needs` (`discipline.yml:3723`) but not in `aggregate.needs`, so the register gate this story depends on does not block a merge through `aggregate` either (§6a). ⚠ Its comment at `discipline.yml:1539` also mis-cites **NFR-Aud-13** (the 30-day RTBF SLA) for what is **FR41**/NFR-Aud-1 (`requirements-inventory.md:82`, `:147`) | **`20-3b-gate-retirement-and-coverage-generator`** — charter: gate dispositions. `deferred-work.md` row at T7. The one-word comment correction rides there too |
| 7 | **The register's prose mis-pointer** (`epic-14-preflight-decisions.md:91` says `16-6` where every table cell says `16-5`, and `16-6` is now a different story) | §6a; the gate parses table cells only | **NOT CUT — repaired here** (§12), because this story is the row's own vehicle and a reader following the prose is sent to the wrong story |

**Not cut (EFFORT, not SCOPE):** the nine-site drop class, the class-wide instrument with a named reader, all
four AC2 repairs, the private-tier count, the redactor boundary rule, the serializer, both halves of the journal
fix, all three AC5 rows, and all five re-pin artifacts. Each is required for one of this story's own ACs to be
true.

---

## Dev notes

**Reuse, do not reinvent.**
- **The drop-counter + gate pattern:** `OtelTraceSink::drop_count` (`crates/maos-telemetry/src/otel_sink.rs:415`; the probe's own `fn drop_count` is `:107`) read by `gate:otel-degradation` (`crates/maos-telemetry/tests/otel_gates.rs:802,810,818,844`), with
  the proven-red vector built from `with_bounded_channel(exporter, cfg, 1)` + `pause_consumer()`. This is the
  in-repo precedent the register itself cites. Copy it.
- **Fault injection:** `cap_audit::channel()` (`crates/maos-capability/src/cap_audit/mod.rs:111-113`, `pub type Sender` `:108`,
  `AUDIT_CHANNEL_DEPTH = 8192` at `:18`) and `CapabilityRegistryAdapter::new`
  (`crates/maos-kernel-core/src/capability/mod.rs:144-168`) are both `pub`. Drop the receiver; do not build a seam.
- **The race-test shape:** `std::sync::Barrier::new(2)` — `crates/maos-bin/tests/shell_host_16_2.rs:1043`.
- **The FR4 classifier:** `crates/maos-audit/src/fr4_classifier.rs` — `NON_CALL_KINDS` `:59-71`, `WRITER_SHAPES`
  `:184-453` (**28** entries; the *test's* stale "~45" comment is `fr4_classifier_16_2.rs:756`, not a module comment — `fr4_classifier.rs` is 583 lines), key `(basename, fn, ordinal)`
  `:141`, `fn payload_matches` `:528`, kind-7 exemption filter `:492`, fall-through to `Call` `:486-487`.
  Its doorbell is `crates/maos-audit/tests/fr4_classifier_16_2.rs` (`fn scan_tree` `:700` over `fn scan_source` `:492` (search loop `:533`), kind filter
  `:659-673`, both-direction assertions `:745-808`, fixture table `:136-301`, held 1:1 with the table's 20 `shape.is_some()` entries by the invariant at `:302-308`).
- **The re-pin:** `cargo run -p xtask -- check-kernel-baseline --emit-pin` (`xtask/src/main.rs:396-405`;
  instructions in `xtask/kernel-core-baseline.toml:509-513`). The HISTORY shape is `822070d7`'s.
- **Store resolvers:** `maos_audit::default_memory_root()` (`crates/maos-audit/src/lib.rs:1515`) for the
  private-tier count — never re-derive a path.

**Do not.**
- Do not re-type `record_verification` (`capability/mod.rs:233-245`) — outside D3, outside the grant.
- Do not propagate at site 8 (`cap_tokens/mod.rs:294-296`) — `cap_audit_backpressure.rs:106`'s `.unwrap()` panics.
- Do not use `tokio::sync::Mutex` for AC3 — `forget_with_reason` is sync with no `.await`; a tokio mutex forces
  an `async fn` signature change and a much larger kernel-Δ.
- Do not write AC4's falsifier as a staggered two-process race — **it passes at HEAD**.
- Do not change the four-group delegation-id format (`crates/maos-bin/src/supervision.rs:231-246`).
- Do not re-kind anything (D-16-5-C), and do not silently skip the decision.
- Do not silence the aggregate alarm.

### Project Structure Notes

**No new crate, no new workspace member, no lockfile entry, no new file under `crates/maos-bin/src`** ⇒
`SCANNED_SOURCE_FILES` stays **23** (`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:882`, equality assert
`:971-985`) and `check-workspace-count` is unaffected. **No new verb** ⇒ `verb_table_15_3.rs` and
`check-exit-commands` untouched. New test files belong under each crate's `tests/` (**not** charged by
`kloc-check`); inline `#[cfg(test)]` **is** charged. The new `maos-capability` instrument is a module under
`crates/maos-capability/src/cap_audit/` — that crate is outside `check-kernel-baseline`'s scope
(`const KERNEL_SRC`, `xtask/src/check_kernel_baseline.rs:74`), outside `xtask/kernel-crates.toml:3` and outside
`ZERO_HEADROOM_CRATES`, which is the whole point of D3's ruling. ⚠ **Test isolation:** AC3's race test and AC1's
fault-injection vector must not set process-global env (`MAOS_HOME`); the suite's known isolation defect is
D16's, and every new test builds its own `TempDir`.

### References

⚠ **Every cite below is pinned at `df45c609`.** This story's own rule-9 amendment to `epic-16-…md` and
`epic-14-preflight-decisions.md` is APPLIED in the creation commit, so line numbers in those two files shift in
the working tree (epic-16 by +2, or +4 below `:180`). Re-derive against the commit, not the tree.

- [Source: `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md`] — §16-5 `:174-182`,
  stories-table row `:71`, Kernel-Δ `:51`, Kloc asks `:53`, Dependencies `:194`, exit block `:26-32`.
- [Source: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`] — machine contract `:34-39`,
  binding rules `:41-58`, D3 `:66` + ruling `:234-264`, D5 `:71` + `:266-290` + `:452-480`, D7 `:73`, D12 `:78`,
  D18 `:84` + `:390-406`, re-homing prose `:91`, clause-replacement lesson `:476-480`.
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md`] — rows `:527`, `:545`, `:707`, `:834`,
  `:924`, `:932`, `:934`, `:935`, `:936`, `:946`, `:949`, `:957`.
- [Source: `RELEASE-HOLDS.md`] — boundary rows `:51`, `:52`, `:53`, `:60-61`.
- [Source: `_bmad-output/implementation-artifacts/16-1-…md`] §11 row 1, §17 V-13 · `16-2-…md` §11 rows 1-2, §15 R3 ·
  `16-3-…md` §11 rows 7-8, 12 · `16-4-…md` §11 row 3, §10 (the budget-raise procedure).
- [Source: `xtask/kloc.toml`] — RECOVERY-LANE CEILING RULE `:49-95`, `maos-kernel-core` `:247`, ledger `:394`,
  alarm `:657`, hardfail `:658`.

---

## Tasks / Subtasks

- [x] **T0 — re-measure; trust nothing in this file** (all ACs)
  - [x] `git status --short` clean; record HEAD. Run `kloc-check --json`, `check-kernel-baseline`,
        `check-decision-register --json`, `check-empty-kernel`, `cargo test -p xtask --test
        recovery_lane_ceiling_rule --test kernel_pin_content_hash_16_0 --test decision_register_gate`. **All are
        green at HEAD** — any red after your first commit is yours.
  - [x] Re-derive every cite in §1–§6 and the Decisions table. **Re-measure this counts box first — every number
        in it is load-bearing and each is asserted somewhere below:** `record_invocation` production call sites
        **4** (a loose grep returns ~10 hits: 4 calls, 3 comments, 2 defs, 1 `#[cfg(test)]` double at
        `capability/working_memory/policy_runtime.rs:203`) · `CapAuditEvent` drop sites **9** · audit-writer
        drain sites **11** · kernel `lifecycle.*` writers **8 + 1** (7× `SpiritAuto`, 1× `Kernel` at
        `crash_detector.rs:207`; `scheduler_loop.rs:615` writes at **pid 0**, unlike the other six) ·
        `insert_kernel_event_returning_id` sites **9** (7 production) · `WRITER_SHAPES` **28** · fixture rows
        **20** · pin entries **98** · `SCANNED_SOURCE_FILES` **23**. ⚠ Define the drop class as *discards of a
        `CapAuditEvent` send*: `Backpressure::DropWithAudit` (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:417-421`)
        names itself "WithAudit", never calls `record_drop()`, and counts into `PumpOutcome::dropped` instead —
        a reviewer counting *silent drops* gets **ten**. Dispose of it explicitly in AC1 rather than letting
        obligation (c) red on a boundary question.
  - [x] Re-run the three runtime probes on this host: the `LD_PRELOAD` syscall count, the staggered-vs-overlapped
        journal harness, and the real-redactor `task-worker-1` vector. Record each HEAD red **before** any repair.
  - [x] Confirm `maos-domain`'s remaining headroom against the joint `16-4`/`16-5` ledger (`kloc.toml:394`) —
        **a grant is a global, not a reservation.**
  - [x] Confirm at HEAD that `main.rs:1275` and `:1286` both construct `ResearcherCollectiveError::Denied`, and
        record that transcript — it is D-16-5-K's proven-red baseline and the reason AC1 does not reuse it.
  - [x] Confirm `check-kernel-baseline --emit-pin` still round-trips byte-identical against
        `xtask/kernel-core-baseline.toml:521-623` **before** writing any kernel line — AC6 depends on it entirely,
        and a drifted emitter would be discovered at the end of the story instead of the start.
- [x] **T1 — the register edits, as their own commit, BEFORE any code** (§6a, D-16-5-H) — re-anchor D3, D7 and
      D18 by **replacing** the `leaves backlog` clause with `reaches done`; record D3's ruling text against its
      ID; repair the `:91` prose mis-pointer; flip `sprint-status.yaml:251` to `ready-for-dev`. Then run
      `check-decision-register` and paste the PASSED transcript. **This commit is what makes story creation
      lawful; it must land with the story file, not after it.**
- [x] **T2 — the drop class, its cause, and the last-resort path** (AC1, D-16-5-B/D/E/K/L) — the **distinct
      refusal cause** at all three `main.rs` sites (never `Denied`); the latched degraded state, its
      `GET /v1/daemon` field and its `maos audit query` disclosure, set from the `Closed` path; then the `maos-capability` instrument with a named reader on the
      `otel_gates` pattern; site 8's missing `record_drop()`; site 9's `TODO` closed; the shell swallow becomes a
      counted named drop; then the `+1` propagate at `capability/mod.rs:349-351` and the `CapError` variant at
      `ports/capability.rs:141` (append point; `:140` is the last variant's ident). `record_verification` untouched.
- [x] **T3 — the audit record's claims** (AC2, D-16-5-C/I/J) — `insert_kernel_event_returning_id`'s caller-supplied
      kind + token across all nine sites; `emit_task_orphaned`'s JSON payload and its `WriterShapeEntry`
      re-disposition; the `verb_resolver` writer's disposition **and** the doorbell's `DecisionDispatch` filter;
      the token-padding fix; the private-tier count in both cascade gates; the redactor's token-boundary rule.
      **Write the re-kind decline and its priced cost into the story, not a commit message.**
- [x] **T4 — legal-hold serialization** (AC3, D-16-5-F/M) — the serializer field, `new()` init, the guard at the
      top of `forget_with_reason`; the `#[i9_exempt]` reason amendment in both places; **the race test, run red at
      HEAD first**; **and the cascade-interleave vector** (hold placed between principals) asserting per-principal
      atomicity and the mixed terminal — the granularity is stated in AC3, never left to inference.
- [x] **T5 — the journal and the halt** (AC4, AC5, D-16-5-G) — `.append(true)` + one `write_all`; the
      `recover_in_flight_with_tasks` falsifier and the intra-process pair; the two false comments; the P99 re-run;
      the REPL size cap. Then AC5's three rows: the resolver ordering, the door's approval-row honesty, and
      `classify_halt_record`.
- [x] **T5b — the deny vocabulary** (AC7, AC8) — **FR63 measured FIRST: name nothing `E…` before checking
      `check_error_catalog.rs:8-14`.** D7: typed refusal at both surfaces — `fn journal_cross_wall_recall`
      (`maos-iac/src/adapter/log_recall.rs:96-119`) and the `maos traceback` render (`main.rs:2611-2646`, the
      `.then_some("refused")` at `:2633`); assert ADR-058:52 conformance six ways. D18: the mirror enums in
      `maos-domain` (ADR-010 forces them — measure the raise), the 11 typed arms in `router.rs:1933-2063`,
      `#[non_exhaustive]` on `IacBusError`, `IntentDeniedAtPeer`'s `intent` field, the dead `Accept` arm ruled,
      and `transport.rs:602-603`. **Rewrite `fail_closed_8_8.rs:305-348` — capture its pre-rewrite GREEN first**,
      it is the assertion that pins the collapse as correct; then `consent_refusal_1b.rs:17-56`'s rationale and
      its stale `:1671-1783` cite.
- [x] **T6 — the grant and the pin, LAST** (AC6) — `cargo fmt --all`; measure; then in **one** commit:
      `--emit-pin` block + `src_lines` + HISTORY; `kloc.toml:247`; `RATIFIED_AT_EASING:268`;
      `kernel_pin_content_hash_16_0.rs`'s four literals **and** the `:846-851` line-position assert; the four
      non-kernel measured raises each citing the bare token `16-5`.
- [x] **T7 — sweep** — `deferred-work.md` dispositions for all ten rows closed here and rows for §11 items 1, 2,
      6; epic + `sprint-status.yaml` edits verified present (§12); `cargo fmt --all -- --check`; every gate;
      `cargo test --workspace --no-fail-fast`; the new aggregate stated against alarm and hardfail; every
      proven-red run recorded and reverted; `graphify update .`.

---

## 12. Epic OLD→NEW edits (rule 9 — APPLIED at story creation, 2026-09-17)

Every OLD anchor was verified to exist exactly once in the file named before replacement; no row replaces a pin
value or a line number that is data.

| # | File / location | OLD (anchor) | NEW (summary) |
|---|---|---|---|
| 1 | epic-16 header, before `**Wave / duration:**` | — | NEW **Round 9** paragraph: 16-5 re-derived at `df45c609`; premises in all five ACs disproved; 12 `deferred-work.md` rows vs the section's 7; pointer to this file |
| 2 | epic-16 stories table 16-5 row | `` kernel-Δ +5–25 (measured +3–25) · maos-a2a-core +20–40, maos-domain +3–25 `` | measured re-book: kernel-core ≤ +39 (ZERO headroom, FLAG-Winston), maos-capability ≤ +156, maos-audit ≤ +182, maos-iac ≤ +208 (**absent from the epic before**), maos-bin ≤ +117, maos-shell ≤ +39, maos-domain ≤ +8; a2a-core → `16-5b` |
| 3 | epic-16 §16-5, the five AC bullets | the five bullets | ⚠ R9 SUPERSEDED paragraph summarising the six ACs; the original AC1–AC5 kept **verbatim** below it so their cites stay auditable |
| 4 | epic-16 §16-5 AC4 | `` `xtask/kernel-core-baseline.toml:481` `src_lines = 24474` … `xtask/kloc.toml:247` (`maos-kernel-core = 18935`) `` | ⚠ R9: `:488` / `24477` / `18938`, and **five artifacts, not one** — 16-3 already re-pinned and needed two follow-up commits |
| 5 | epic-16 Kernel-Δ paragraph | `` **16-5 is FLAG-Winston** (+5–25 kernel lines, measured +3–25; re-pin `xtask/kernel-core-baseline.toml:481` `src_lines = 24474` → N …) `` | ⚠ R9: measured +17…+30 raw / ≤ +39 ×1.3; the pin is `:488` = 24477; 16-5 is the **second** post-16-0 re-pin, not the first |
| 6 | epic-16 Kloc asks | `` aggregate 164216 against an alarm of 158608 `` | ⚠ R9: **166542** at `df45c609`; maos-audit is **+177**, not ZERO; maos-iac **+35** added to the list |
| 7 | epic-16 Dependencies line | `` 16-5 last `` | `` 16-5 last; `16-5b` (if ratified) before it, ZERO-Δ `` ⚠ R9 |
| 8 | `epic-14-preflight-decisions.md` D3 `:66` Deadline cell | `` Before `16-5-…` leaves `backlog` `` | `` Before `16-5-…` reaches `done` `` — **clause REPLACED, not appended** (`fn deadline_clauses` splits on `;`); D3's ruling recorded against its ID per binding rule 1 |
| 9 | `epic-14-preflight-decisions.md` D7 `:73` and D18 `:84` Deadline cells | `` Before `16-5-…` leaves `backlog` `` | `` Before `16-5-…` reaches `done` `` — same replacement. Target stays `16-5` until §14 Q1; a ratified split re-points them to `16-5b`, a **real** vehicle (not the rule-5 move) |
| 10 | `epic-14-preflight-decisions.md:91` re-homing prose | `` `14-d3`→`16-6` … `14-4`→`16-6` (D18) … named in the `16-6` row `` | `16-5` throughout, with a ⚠ note that `16-6` is since 2026-09-13 a different story (`16-6-maosctl-load-over-the-door`) and that the gate parses table cells only |
| 11 | `sprint-status.yaml:251` 16-5 row | `` 16-5-…: backlog  # ◆ FLAG-Winston +5–25 kernel lines … `` | `ready-for-dev` + a summary naming the nine-site drop class, the four AC2 repairs, the serializer, both journal halves, the three AC5 rows and the five re-pin artifacts; prior comment kept after `PRIOR:` |
| 12 | `sprint-status.yaml` epic-16-retrospective row | the existing OWES list | appends ⚠ ALSO OWES (Story 16-5): the priced re-kind decline (§11 row 1) and the `Indeterminate` uninstall terminal (row 2) |
| 13 | ~~`sprint-status.yaml` NEW row `16-5b-cross-host-deny-vocabulary`~~ | — | ⚠ **WITHDRAWN — §14 Q1 was ruled WHOLE (§15 R6).** No `16-5b` row is created, D7's and D18's register Target cells stay `16-5`, and the three-step transaction this row described is moot. Kept struck rather than deleted so the `unresolved-target` hazard John found stays on the record for the next story that considers a split |

---

## 14. Open questions for the operator

**Q2, Q3 and the two round-table items were RULED 2026-09-17** — D3's re-ruling goes in the register under
Winston's name (below), AC3 keeps Shape A1, AC1 gets a distinct refusal cause (D-16-5-K), the instrument becomes
the audit of last resort with a latched degraded state (D-16-5-L), and AC3 states per-principal granularity with
its own vector (D-16-5-M). **Q1 and Q4 remain open.**

**Q1 — RULED WHOLE (2026-09-17).** See §15 R6. `16-5b` is retired and never created; D7 and D18 are AC7 and
AC8; §11 rows 4 and 5 are in scope; `maos-domain` takes a measured raise. The superseded recommendation is kept
in D-16-5-A so the trade stays auditable.

~~**Q1 — Sizing. SPLIT 2 ways, or WHOLE?**~~ Recommended **SPLIT** (D-16-5-A): this key keeps the kernel grant and
the audit-truth charter; `16-5b-cross-host-deny-vocabulary` takes D7 + D18 + `deferred-work.md:834` at ZERO
kernel-Δ. WHOLE means 8 crates and 20 obligations in one story, and `maos-domain`'s **+52** cannot hold AC3's
typed mirror enums (≈+50–59) **and** AC1's `CapError` (+3). The epic's own name for the story — *"Debt slot"* —
is a bucket, which confidence rule 10 forbids as a routing basis. **If WHOLE is chosen**, §12 row 13 is dropped,
§11 rows 3–5 come back in, and the story gains two ACs and a `maos-domain` measured raise.

**Q2 — D3's fork.** The binding register RULED D3 as *"ZERO kernel-Δ, repaired out-of-kernel, class-wide"*; the
epic's AC1 demands the in-kernel propagate. Recommended **BOTH** (D-16-5-B): the register chose out-of-kernel
*because* it avoided a grant and a re-pin, and this story already pays both for AC3/AC4/AC6 — so the propagate
costs +1 kernel line, and without it AC1's stated command cannot pass. Confirming this also confirms the kernel
grant is spent on four files, not three.

**Q3 — AC3's fix home.** Shape A1 puts the serializer in `maos-kernel-core` (+4…+6 against ZERO headroom).
Shape A2 puts it on `TransparencyLogAdapter` in `maos-iac` behind an accessor, dropping the kernel cost to ~+1
and closing the `release_legal_hold` direction for free — but it moves a kernel atomicity invariant into an
adapter the pin does not cover. Recommended **A1**; A2 is the recorded fallback if the grant is contested.

**Q4 — STILL OPEN. §11 row 3.** `RELEASE-HOLDS.md:51`'s `CollectivePortError` eight-into-one collapse names *"Future
FLAG-Winston kernel-widening decision"* as its successor, and this is the only open FLAG-Winston vehicle. Rule 10
says route by charter, not by grant — so it is cut to `16-5b` or the retrospective. Confirm, or pull it in.

---

## 15. Round-table 2026-09-17 — five rulings, all applied

Convened as a preflight on the authored story under the standing criterion: **spec fidelity + long-term
correctness.** Present: Winston, Murat, Amelia, John, Sally, Mary, Paige, with Grumbal, Vex, Boundary and Dana
summoned. Two rulings were taken by the operator; three were settled on measurement at the table.

| # | Fork | Ruling | Applied at |
|---|---|---|---|
| **R1** | AC1's refusal reuses `ResearcherCollectiveError::Denied` — the same variant a real policy denial uses | **A DISTINCT cause. Sally's question — *"denied by what? Nobody denied them anything, the logging broke"* — exposed that AC1's repair reproduced D18's `-32001`-vs-`-32009` collapse inside the story filed to fix it.** Mary logged it as a new sub-shape: *the-fix-reproducing-the-defect-it-was-filed-beside* (41). | **D-16-5-K**; AC1; §10 `maos-bin`; obligation (ac); T0 baseline + T2 |
| **R2** | A refusal caused by an unavailable sink cannot be audited — raised by Vex, loop confirmed by Boundary, and the room moved on to budget arithmetic before it was closed | **OPERATOR-RULED: close it here — instrument as the audit of last resort, plus a latched degraded state on the `Closed` path.** The counter is out-of-band by construction, and writer-task death silences audit for the process's whole lifetime, so one refusal is never the real case. Dana's *"ship AC1, file the variant"* was overruled on the story's own thesis. | **D-16-5-L**; AC1; §10 `maos-capability` + `maos-bin`; obligations (ad), (ae); T2 |
| **R3** | The register RULED D3 ZERO-Δ; this story overturns it | **The re-ruling goes in the REGISTER under Winston's name, not in a story's decision table** (Murat: *"a story doesn't get to re-rule a ruled decision because its own circumstances changed — that's how rulings rot"*). Winston, the owner of record, re-ruled to BOTH and withdrew this register's `cap_audit_backpressure` constraint as measured-false. | `epic-14-preflight-decisions.md` §"D3 — RE-RULED 2026-09-17"; **D-16-5-B** now cites it |
| **R4** | AC3's atomicity granularity is unstated; the cascade loops per principal | **OPERATOR-RULED: state it in AC3 and assert it with one vector — not a second story.** Murat was right it could not stay undecided; Dana was right about the size, because `mixed_held_uninstall_writes_partial_proof` shows the tree already shipped per-principal semantics. | **D-16-5-M**; AC3; obligation (af); T4 |
| **R5** | The split re-points D7/D18 Targets at a `sprint-status.yaml` row that may not exist yet | **A three-step transaction, in order: create the row → re-point → re-run the gate.** John: *"same tripwire as §6a, and the story that documented it walked into the sequel."* | §12 row 13; obligation (ag) |

| **R6** | §14 Q1 — sizing. The preflight recommended SPLIT 2 ways | **WHOLE — OPERATOR-RULED, overruling the recommendation.** On the standing criterion (*spec fidelity + long-term correctness*) the seam fails: the two halves are one capability — *an operator being told the true cause of an outcome* — and AC1's own defect (D-16-5-K) is an **instance of D18**, so a split ships the defect and its vocabulary fix in separate commits. The half that carries the only shipped-ADR statement of the invariant (`ADR-058:52`) would have left with the half that needed it. `maos-domain`'s raise is authorized by the RECOVERY-LANE CEILING RULE anyway, so the budget argument bought nothing a rule already grants. Dana's SPLIT dissent recorded. | **D-16-5-A**; frontmatter `split_from`; **AC7 + AC8**; §10 (`maos-domain` +85, `maos-a2a-core` +59, `maos-a2a-tcp` +52); §11 rows 3-5; §12 row 13 **withdrawn**; T5b |

**Still open after the round-table:** §14 **Q4** only (`CollectivePortError`'s eight-into-one collapse). Q1 was ruled WHOLE at R6.

---

## Dev Agent Record

### Agent Model Used

openai-codex/gpt-5.6-sol

### Debug Log References

- Baseline and runtime probes: `kloc-check --json`, kernel pin round-trip, decision-register/empty-kernel gates,
  syscall-count and concurrent-journal falsifiers, real redactor vector, and pre-rewrite A2A collapse assertion.
- Red→green coverage: closed audit sink, all nine audit-drop sites, private-tier cascade counts, legal-hold
  interleaving, journal append/recovery concurrency, halt durability/classification, and typed local/peer A2A
  refusals.
- Final gates: `check-kernel-baseline` passed at 24,591 lines/98 files; `kloc-check` passed at aggregate 167,112
  with the alarm acknowledged and no over-budget crate; decision-register and pin/ratchet suites passed 64 tests.
- Final regression: `cargo test --workspace --no-fail-fast` passed 4,467 tests across 542 suites
  (121 ignored, 67 filtered); `cargo fmt --all -- --check` passed.

### Completion Notes List

- Audit delivery now exposes nine named drop sites, a latched degraded state, daemon/CLI disclosure, and a
  distinct fail-closed `AuditSinkUnavailable` cause instead of reporting a capability action that was not
  durably auditable.
- Audit rows preserve caller-supplied kind/token truth; task-orphan, lifecycle, redaction, and private-tier
  classification/counting defects are covered by behavioral regressions.
- Per-principal forget serialization closes the legal-hold check/erase race while preserving mixed-cascade
  outcomes and durable hold evidence.
- Lifecycle journal appends are single-record append writes; halt resolution commits durable state before
  side effects and reports failed completion honestly.
- Cross-wall and cross-host denials retain machine-readable typed causes through domain, A2A, IAC, transport,
  journal, traceback, and operator surfaces.
- Kernel pin and KLOC ceilings were re-ratified in implementation commit `f461f23a`; charter/register edits are
  isolated in `925d2ffe`.

### File List

- `Cargo.lock`
- `_bmad-output/implementation-artifacts/16-5-audit-drop-legal-hold-and-a2a-deny-vocabulary.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/party-mode/memories/installed/.memlog.md`
- `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`
- `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md`
- `crates/maos-a2a-core/src/router.rs`
- `crates/maos-a2a-core/tests/fail_closed_8_8.rs`
- `crates/maos-a2a-tcp/src/transport.rs`
- `crates/maos-acp/Cargo.toml`
- `crates/maos-acp/src/notification_channel.rs`
- `crates/maos-audit/src/fr4_classifier.rs`
- `crates/maos-audit/src/lib.rs`
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-audit/tests/fr4_worker_rows_16_3.rs`
- `crates/maos-audit/tests/gdpr_cascade_test.rs`
- `crates/maos-audit/tests/private_tier_count_16_5.rs`
- `crates/maos-bin/src/cross_team_crossing.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/operator_door.rs`
- `crates/maos-bin/tests/cross_team_consent_13_3.rs`
- `crates/maos-bin/tests/shell_host_16_2.rs`
- `crates/maos-capability/src/cap_audit/mod.rs`
- `crates/maos-capability/src/cap_tokens/mod.rs`
- `crates/maos-cli/src/subcommands.rs`
- `crates/maos-cli/tests/halt_record_16_5.rs`
- `crates/maos-control/src/lib.rs`
- `crates/maos-control/tests/post_surface_16_1.rs`
- `crates/maos-control/tests/submit_and_wait_16_2.rs`
- `crates/maos-domain/src/iac_bus_types.rs`
- `crates/maos-domain/src/log_recall.rs`
- `crates/maos-domain/src/ports/capability.rs`
- `crates/maos-iac/src/adapter/log_recall.rs`
- `crates/maos-iac/src/adapter/redaction.rs`
- `crates/maos-iac/src/adapter/transparency_log.rs`
- `crates/maos-iac/tests/audit_truth_16_5.rs`
- `crates/maos-kernel-core/src/capability/mod.rs`
- `crates/maos-kernel-core/src/halt/mod.rs`
- `crates/maos-kernel-core/src/halt/resolver.rs`
- `crates/maos-kernel-core/src/inference/mod.rs`
- `crates/maos-kernel-core/src/journal/mod.rs`
- `crates/maos-kernel-core/src/memory/mod.rs`
- `crates/maos-kernel-core/src/security/mod.rs`
- `crates/maos-kernel-core/src/security/sandbox/t3/cap_audit_bridge.rs`
- `crates/maos-kernel-core/src/security/sandbox/t3/quarantine.rs`
- `crates/maos-kernel-core/src/supervision/crash_detector.rs`
- `crates/maos-kernel-core/tests/audit_sink_truth_16_5.rs`
- `crates/maos-kernel-core/tests/cap_audit_backpressure.rs`
- `crates/maos-kernel-core/tests/cap_registry_integration.rs`
- `crates/maos-kernel-core/tests/cap_token_verify_assertion.rs`
- `crates/maos-kernel-core/tests/halt_invoke_test.rs`
- `crates/maos-kernel-core/tests/journal_append_16_5.rs`
- `crates/maos-kernel-core/tests/nfr_rel_9_revoke_latency.rs`
- `crates/maos-kernel-core/tests/on_revocation_three_actions.rs`
- `crates/maos-kernel-core/tests/revocation_applier_pipeline.rs`
- `crates/maos-shell/src/lib.rs`
- `crates/maos-wasm-host/tests/equiv_harness.rs`
- `docs/ci-baselines/kernel-surface-v0.1-beta.json`
- `docs/invariants/i9-exemptions.md`
- `spirits/researcher/src/lib.rs`
- `xtask/kernel-core-baseline.toml`
- `xtask/kloc.toml`
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/tests/recovery_lane_ceiling_rule.rs`
- `xtask/tests/story_10_4a_ac1_proven_red.rs`

### Change Log

- 2026-09-17: Story created from six scouts at `df45c609`; premises disproved in all five epic ACs; §12 edits
  applied to `epic-16-…md`, `epic-14-preflight-decisions.md` and `sprint-status.yaml` in the same commit;
  `check-decision-register` re-run green after the re-anchor. Moved to `ready-for-dev`.
- 2026-09-17: Implemented AC1–AC8, closed T0–T7, re-pinned the 24,591-line/98-file kernel
  surface, ratified the 19,040 KLOC ceiling, passed the complete workspace regression suite, and moved to review.

### Review Findings
