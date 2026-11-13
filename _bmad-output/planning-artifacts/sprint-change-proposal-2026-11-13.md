# Sprint Change Proposal — 2026-11-13, rev. 2 (Epic 21 audit; Enterprise-Fit lane, Epics 22–26)

**Status: APPROVED by the operator, 2026-11-13 ("approved the proposal"), and APPLIED by the documents commit that adds this file (§11 is the application record; it also corrects four facts stated below). `bmad-correct-course`, Batch mode, complete.**
**Measured at:** `HEAD` = `origin/main` = `1a57035e`; every `file:line` is from that tree unless marked *dated*. No `v1.0.0*` tag exists. The pre-release fuzz chain `43334961225` is on link 2 of 5 (read 2026-11-13T08:27Z). The tag procedure requires the candidate to be the `origin/main` tip (`docs/release/tag-procedure.md:58-66, 220-221`; ruling 13 condition (a), `decisions-2026-11-12-acceleration.md:43`), so nothing lands on main before the rc.1 tag and this file stays untracked.
**Corrections in this revision (2026-11-13T08:30Z):** the GA arithmetic in §2; "operator signs" in §7 (no operator key exists or is needed); the architecture finding in §1 and §5; the workflow record in §9.
**Relation to `sprint-change-proposal-2026-11-12.md`:** this proposal changes that one's GA set (§2). Its five open decisions were ruled on 2026-11-13 by the operator's directive and are recorded there (§10 ruling block, §12) and summarised here in §10. That proposal is applied first, this one on top.
**Labels:** `[INFERENCE]` = not measured. *search synthesis* = an external fact taken from a search summary, not a full read of the specification.

## 0. Operator words and applied meaning

> "As a founder I want the maos to be easy to deploy in any places and domain specific agent concept always on and work in an enterprise server. did we think about it? prd user journeys are about the domain specific agents, sprits as actors, and their governance by the harness as the theater. I acknolwedge the containerised and small foot print and flexibility with enterprise level governance and interops . if epic 21 has tickets or stories not necessary, drop or simplify them to make the SPC brings forward faster. audit epic 21 and make the plan impelmented faster. 1. after v1.0.0 2, 3, 4. think the future proof decisions take that."

| Words | Applied meaning |
|---|---|
| "easy to deploy … always on … enterprise server" | The lane now opens with Epic 22, The Always-On Host. Rev. 1 led with vendor coding Workers and put packaging last; that order is reversed (§4). |
| "domain specific agents, spirits as actors … harness as the theater" | Positioning and the new journey follow the PRD's own model (§3 item 4). Coding Workers become the last epic. |
| "audit epic 21 … drop or simplify" | §2. Three GA stories shrink, two non-GA stories are dropped, two move into the lane. |
| "1. after v1.0.0" | The lane starts after the GA tag. |
| "2, 3, 4 … future proof decisions take that" | Decided in §3. |

## 1. Did we think about it?

Partly. The PRD did: its journeys are domain Spirits (Butler, Researcher, Mira and Nash) under a kernel that enforces the rules. The always-on mechanism also exists. What was not planned is the server around it.

| Need | State at HEAD |
|---|---|
| Spirits stay on | Present. `maos run <topology>` enters a continuous serving loop with a safe drain on SIGTERM (`crates/maos-bin/src/main.rs:6215-6218`); idle and schedule watchdogs exist (`crates/maos-kernel-core/src/scheduler/idle_watchdog.rs`, `schedule_watchdog.rs`). |
| Runs as a service | Absent. No systemd unit: `rg -il 'systemd\|ExecStart' packaging docs/runbooks docs/maos.dev` = 0. The image has an entrypoint and no documented run (`Dockerfile`; `rg -i 'docker run\|compose' docs/maos.dev/run-maos.md docs/runbooks` = 0). No Helm chart. |
| One place to configure | Absent. Configuration is spread over about 125 environment variables (`crates/maos-bin/src/env_contract.rs`), `~/.config/maos/operator.toml` (`crates/maos-kernel-core/src/security/operator_config.rs:56-61`) and `<home>/config.toml` (`crates/maos-bin/src/crypto_provider.rs:64`). |
| Cast returns after a restart | Only what the topology file names. No restore of Spirits loaded over the door was found (`rg -i 'restore\|rehydrat\|auto.?load' crates/maos-bin/src` = 0) `[INFERENCE: grep-negative]`. |
| Other systems hand a Spirit work | Absent on a running daemon. Door verbs for a Spirit are lifecycle only (`crates/maos-control/src/lib.rs:1488-1584`); the shell reaches installed Spirits only inside its own process (`crates/maos-shell/src/lib.rs:547-549`). |
| Disk stays bounded | No. The Transparency Log is never pruned: "no production `DELETE FROM transparency_log`; retention is `VACUUM INTO` a copy" (`crates/maos-bin/src/escape_detector_consumer.rs:18-19`). Memory tiers have no quota until 21-6. |
| Small footprint | Stated for the binary only: release bound 35,651,584 B (`xtask/src/prepush.rs:509`). Idle memory and image size are not measured. |
| Governance and interop | As rev. 1 found: static MCP bearers, a non-standard A2A wire, OpenTelemetry not linked, one shared operator bearer, reference-grade identity and SIEM. Cites are carried in §6. |
| Architecture names a server topology | No. §11 has two shapes: a single-user Host "on a laptop or workstation", networking "localhost-only by default", Spirits "loaded on demand"; and the bilateral pair (`architecture-maos-minimal-opus/11-deployment-topologies.md:3-19`). The v2.2 section promised §11.4 (team mesh) and §11.5 (Reza single-org Cortex) (`15-full-spectrum-v2-2.md:129`); §11 does not contain them. |

**Correction carried from rev. 1.** An approval-required capability is refused (`crates/maos-kernel-core/src/capability/mod.rs:221-223`). The auto-allow `ApprovalManager::prompt` has no production caller (`main.rs:3443` constructs it, nothing calls it).

## 2. Epic 21 audit

Test applied to each story: does v1.0.0 make a claim that is false without it, or does an always-on Host break without it? If neither, it is dropped or moved.

| Key | GA today | Disposition | Reason | Size |
|---|---|---|---|---|
| `21-1` J4 fixture + live Mira/Nash | no | **Simplify, move → `22-8`.** Keep the inference seams and the in-process router (AC2, AC3). Cut the 50-incident cassette and the closure-rate line (AC1). | A domain pair that runs from the product binary serves the vision; a 50-incident corpus authored by agents measures nothing (`epic-21…w6.md:54`: closure rate "NOT MEASURED"). | L → part of `22-8` |
| `21-2` nightly Postgres | no | **Drop.** | Its own AC1 says the nightly "adds … the end-to-end narrated run, NOT coverage" (`epic-21…w6.md:72`). AC3 adds `xtask` lines against the diet. AC4 is hygiene. | M → 0 |
| `21-3` durable TOFU + rotation | yes | **Split.** GA keeps AC1 (durable pin store and `GET /v1/a2a/pins`), which closes `RELEASE-HOLDS.md:86` row 12. AC2–AC4 → `23-7`. AC6 → `22-1`. | A restart must not lose pins. Own-leaf rotation choreography is worth deciding against enterprise PKI first, not extending now. | L → M |
| `21-4` one instrument + env registry | no | **Drop.** AC2's unregistered-variable finding → `22-1`. | Instrument unification and a new `check-cited-kloc-grants` gate are governance of governance; ruling `kfree` and the pending B7c already loosen what it tunes. | M → 0 |
| `21-5` | done | — | — | — |
| `21-6` memory-tier quota | yes | **Keep; prove without Postgres.** Private and shared tiers at GA; the collective-on-Postgres leg is parked. | An always-on Spirit with no quota fills the disk. | M |
| `21-7` consent granter + gate | yes | **Keep** as the 2026-11-12 proposal narrowed it. Its Postgres leg is parked, not sent to 21-2. | FR29 is false in production today. | M |
| `21-8` three-protocol witness | yes | **Move → Epic 23 exit.** | It adds no product code, and it would witness the bespoke A2A wire that 23-5 replaces. Epic 23's exit is the same witness on the standard protocols. | M → 0 |
| `21-9` WASM host default-on | yes | **Split.** GA keeps AC1–AC9 (default-on core, hostile-guest hardening, authority and scope), AC12, and from AC11 only `deferred-work.md:1685` (a swap that writes no FR11 record is an audit gap). AC10 and the rest of AC11 → `22-9`. | Security and audit-truth rows stay with the default-on switch. Nine Low product findings do not have to. | XL → L |
| `21-10a/b/c` invariant proofs | yes | **Keep whole.** One addition: each predicate is a callable function, so `25-1` reuses it. | It is the product claim: "No predicate, no claim". | 3 × M |
| `21-11` agent authoring kit | no | **Simplify, move → `22-7`.** Kit, a working `maos-spirit validate`, one domain Spirit example, three recorded agent runs. No N=12 trial, no clean-VM attestation; the human-denominator legs of the gate are deleted, not extended. | The kit is how domain agents get written. The trial apparatus is not. | M (smaller) |
| `21-12` attestation defects (proposed) | yes | **Keep.** | Ruled "fix it before GA". | M |
| `later-registry-quarantine-closure` | yes | **Keep whole.** | Narrowing it leaves I5 at `asserted` and the ledger at 13 of 14. | L |
| `21-13` first 30 minutes (proposed) | open | **Move → Epic 22 exit.** | It is the "easy to deploy" bar; it belongs to the epic that builds deployment. | → `22-8` |

**GA path.** By size: before, 21-3 L, 21-6 M, 21-7 M, 21-8 M, 21-9 XL, 21-10 3 × M, quarantine L, 21-12 M = 14 median-story units; after, 11. By count: 10 stories become 9. The 2026-11-12 forecast (§5.B8.3 there) counts stories, not size, so its dates barely move: the high case falls from 15 to 14 work units, about one day, and the low case is bounded by the 10-day median cycle. What the audit buys is a smaller critical-path story (21-9) and an earlier lane start: 21-1, 21-2, 21-4 and 21-11 no longer stand between the GA tag and the Epic 21 retrospective that opens Epic 22. The larger GA levers are outside this audit: the placement of the reasoning-model stories S2–S4 (about +5 units if they join GA) and the five diet items.

**Epic 21 exit re-ruled.** Both current lines belong to 21-2 (`epic-21…w6.md:13-16`). New exit: `cargo test --locked -p xtask --test invariant_claims_20_3b` with 14 of 14 `guaranteed`, and the 21-9 AC1 line in which the default artifact answers from the TypeScript Spirit.

**Claim boundaries this creates at v1.0.0** (each one line in `RELEASE-HOLDS.md` and the GA notes):
1. `prd/success-criteria.md:50`: ACP, MCP and A2A are each proven; the one-session witness is not claimed until Epic 23.
2. `RELEASE-HOLDS.md` 14-2 clause (c.2) stands: own-leaf rotation has no production caller.
3. Participant recall and consent do not survive the SQLite→Postgres migration; the Reza scene on Postgres is not run nightly. Parked key: `v25-reza-postgres-legs`.
4. J4 closure rate is not measured.
5. The 21-9 rows moved to `22-9` are listed as known limitations (they are already Low rows in the rc.1 notes).
6. `prd/success-criteria.md:9` (first 30 minutes) is not measured at v1.0.0; it is measured at the Epic 22 exit. This line was added after approval: it is the consequence of the `21-13` row above, and it is option B of item 1 in the 2026-11-12 proposal's §10.

**Rulings this supersedes, by today's directive:** `ga3` for 21-8 and for the rotation half of 21-3; B3 of 2026-11-12 ("21-3 … include in the GA") for that same half only; D-21-PF-21 and D-21-PF-22 (trial gate and N=12).

## 3. Decisions (ruled)

| # | Decision | Taken | Why it is the future-proof choice |
|---|---|---|---|
| 1 | Where the lane sits | **After `v1.0.0`** (operator's words). One minor release per epic. | — |
| 2 | Interactive approval and v1.0 | **Claim boundary at v1.0; build `24-3` after GA; design it with no Spirit-contract change.** The Spirit keeps receiving `capability-denied` and succeeds on retry once an operator has approved; the hold lives on the operator side. | The Spirit contract's error variants are closed (`wit/spirit.wit:342-347, 381`). A design that needs a new variant would force a contract bump after the v1.0 freeze; this one cannot be blocked by it. The unused auto-allow path is deleted inside 21-10a's I4 work so v1.0 ships no approval code that is not real. |
| 3 | Real KMS backend | **No vendor backend in-tree, ever. One generic adapter (`23-8`)** that forwards `wrap_data_key`, `unwrap_data_key` and `is_healthy` (`crates/maos-domain/src/ports/key_management.rs:21-37`) to a local endpoint the operator runs. | Any KMS or HSM becomes a sidecar. The binary stays small and carries no vendor SDK; ruling `drop` stays true. |
| 4 | Positioning | **The theater.** README sentence: "MAOS is the theater: your domain agents are the actors, and the house enforces the rules." New journey J5 is an operator who deploys a Host and keeps a standing cast on. CLI-wrapped coding agents are one actor form, not the headline. | It matches the PRD journeys and the founder's model, so no journey has to be re-argued later. |

## 4. The lane

| Epic | Wave | Makes true | Stories | Size `[INFERENCE]` |
|---|---|---|---|---|
| 22 The Always-On Host | W7 | One config file, one service, one image or chart; the standing cast returns after a restart; other systems can hand a Spirit work; the disk stays bounded | 9 | ≈ 10.5 |
| 23 Standards Interop | W8 | MCP authorization, A2A v1.0, OpenTelemetry, OIDC discovery, network SIEM, external KMS, the enterprise posture in air-gap builds | 8 | ≈ 11.25 |
| 24 Multi-Operator Control Plane | W9 | A named operator and a policy decision on every door command; one inbox; approvals that wait for a human; budgets; automatic containment | 6 | ≈ 8 |
| 25 Proof You Can Run | W10 | A buyer verifies the guarantees on their own install and reads a control map | 2 | ≈ 2.75 |
| 26 Worker Fleet | W11 | Many CLI-wrapped Workers in parallel, metered, capped, crash-safe | 4 | ≈ 4.75 |

Twenty-nine stories, about 37 median-story units. No calendar forecast is given.

**Beside the lane: Epic 27, the reasoning-model adapter** (stories S3a–S9 of the 2026-11-12 proposal's §11; ruled 2026-11-13, §10 below). It is not one of the five lane epics and has no order against them. It gives the lane one rule and one input: `22-1` adds no model-default key to the Host file, and the spend figure used by `22-5`, `24-6` and `26-3` comes from S2, which lands before GA.

**Lane rules.** (1) Adapters stay outside the kernel behind existing ports; `24-3` is the only kernel-touching story. (2) No new `xtask` gate; an exit is a `cargo test` target or a `maos` verb created by a named AC. (3) One hermetic exit per epic. (4) Each external standard is pinned by version in its story, which re-reads the specification. (5) A control is wired or it is labelled. (6) Each epic is preflighted at the previous retrospective.

**Risks.** `23-5` (a second A2A wire and a non-loopback listener) is the largest unknown. `22-6` must rotate the audit store without weakening log-before-deliver or the detector that assumes row ids only grow. T3 Workers need rootless podman, which a pod does not give by default, so `22-3`'s chart excludes them. Assurance stays AI-agent review only.

## 5. Changes to artifacts (applied after the rc.1 tag, in the post-tag worktree)

**PRD.** `functional-requirements.md` gains FR67–FR79; `non-functional-requirements.md` gains six ids; `user-journeys.md` gains J5 (the id is free); `project-scoping-phased-development.md` gains a section "Phase v1.x (enterprise fit) — Epics 22–26, after `v1.0.0`". Ids are confirmed free at application.

| Id | Requirement | Epic |
|---|---|---|
| FR67 | MCP client authorization per the MCP specification at a pinned revision; tokens bound to one server and never visible to a Spirit | 23 |
| FR68 | Signed A2A v1.0 Agent Card and JSON-RPC binding; every inbound request passes typed-intent consent | 23 |
| FR69 | The agent path exported as OpenTelemetry spans under the GenAI conventions at a pinned version | 23 |
| FR70 | Every mutating door command carries a verified operator principal and an operator-policy decision | 24 |
| FR71 | List agents with parent, task, age and spend; read one agent's session; one list of pending decisions | 22, 24 |
| FR72 | An approval-required capability is held, shown to an authorized operator, and minted only after an approval row | 24 |
| FR73 | Budgets per task, Spirit or principal; a breach stops spend and raises a halt; Worker spend is metered | 24, 26 |
| FR74 | Operator rules turn a drift or anomaly signal into a journaled door command | 24 |
| FR75 | An installed Host verifies its own guarantees and writes a signed report | 25 |
| FR76 | An OCI image and a Helm chart ship; the enterprise posture is available in air-gap builds | 22, 23 |
| FR77 | One Host configuration file declares the standing cast; the Host restores it at every boot and refuses to boot on an invalid file | 22 |
| FR78 | An authorized caller hands a named Spirit a task and reads its result, with a task id in the audit trail | 22 |
| FR79 | The audit store is bounded by policy; rotated segments remain independently verifiable | 22 |

| Id | Floor | Epic |
|---|---|---|
| NFR-Sec-20 | A token issued for MCP server A is never sent to server B | 23 |
| NFR-Obs-6 | 100 % of mediated external calls in a task appear in one trace; the convention version is stated | 23 |
| NFR-Ops-13 | An unauthorized operator command returns a typed 403 and a journal row, with zero state change | 24 |
| NFR-Ops-14 | Image size and idle memory are measured, stated and regression-checked; values set at `22-3` | 22 |
| NFR-Cost-2 | Budget overshoot is bounded by one stated reconcile interval | 24 |
| NFR-Comp-6 | Every control-map row cites an invariant or gate and an evidence command | 25 |

`prd/success-criteria.md:9` (first 30 minutes) and NFR-Perf-8 are existing; `22-8` and `26-1` become their owners.

**Journey J5 (draft).** *Dana, platform engineer at a 400-person company.* She installs the chart on the company cluster with one values file. The file names the standing cast: a diagnostic Spirit on the production edge and an architect Spirit beside the repositories. The node is drained overnight; in the morning both are back, with their memory and their pending halt. The incident tool hands the diagnostic Spirit a task through one authenticated call and reads the answer with its task id. Security receives the same events in the SIEM. The audit store rotates monthly and each archived segment still verifies.

**Epics and tracker.** `epics/index.md`: a lane header and five entries. Five epic files with the content of §6. `sprint-status.yaml`: Epic 21 rows per §2; `epic-22` … `epic-26`, the 29 story keys and five retrospective rows as `backlog`; parked key `v25-reza-postgres-legs`. `epic-21…w6.md`: the exit block and the story table per §2.

**Re-owned rows.** `RELEASE-HOLDS.md:84` row 10 → `26-2`; `:94` row 20 → `26-4`; `:76` v2.2 row 2 → `24-1`; 14-2 clause (c.2) → `23-7`; `deferred-work.md:1518` → `26-3`; `deferred-work.md:1738-1742`, `:1665`, `:1677`, `:1721`, `:1760` → `22-9`; `:1328` → `v25-reza-postgres-legs`.

**Architecture records, written by the owning story.** ADR-062 amendment (`22-4`, `24-1`); ADR-051 amendment (`23-2`, `23-4`, `23-6`, `23-8`); ADR-046 amendment (`24-6`, `26-3`); new ADRs for the Host configuration file (`22-1`), audit-store rotation (`22-6`) and the A2A HTTP listener (`23-5`).

**Architecture document.** `11-deployment-topologies.md` gains §11.6 "Standing Host (server)": one Host under a service manager or an orchestrator, the cast declared in the Host file and restored at boot, task ingress through the door, a bounded audit store. `22-1` writes it and `22-3` completes it. §11.4 and §11.5, owed since `15-full-spectrum-v2-2.md:129`, are outside this lane; the gap goes to the Epic 21 retrospective.

## 6. The epics

AC1 is the command and what the operator observes. `bmad-create-story` re-measures every cite and may tighten, not add uncited ACs.

### Epic 22 — The Always-On Host (W7)

**Exit:** `cargo test -p maos-journey-test --test journey_always_on`, created by `22-8` AC1. HEAD: red, no such target. **Kernel-Δ:** forecast 0, except the `22-9` rows (+2…+16 tokei, from the 21-9 forecast).

| Key | Size | Closes |
|---|---|---|
| `22-1-host-config-and-standing-cast` | L | FR77; 21-3 AC6; 21-4 AC2 |
| `22-2-service-mode-and-probes` | M | FR76 (service) |
| `22-3-image-compose-and-helm` | M | FR76 (packaging), NFR-Ops-14 |
| `22-4-directed-task-ingress` | M | FR78 |
| `22-5-agents-list-and-session-view` | M | FR71 (list, session) |
| `22-6-bounded-audit-store` | M–L | FR79 |
| `22-7-domain-spirit-kit` | M | NFR-Onb-1 for agent authors (ex 21-11) |
| `22-8-always-on-scene` | M | J5; first 30 minutes; J4 pair from the binary (ex 21-1, 21-13) |
| `22-9-wasm-form-residuals` | M | The 21-9 rows moved by §2 |

**22-1.** AC1: `maos serve --config <host.toml>` boots the Host and the Spirits the file names; after a kill and restart the same cast is running, read back by `maosctl agents`. AC2: every setting has one documented key; the environment overrides a key and never replaces the file; a `MAOS_*` read with no key or registry entry fails a test (the 21-4 AC2 finding: `MAOS_OPERATOR_HTTP_ENDPOINT`, `crates/maos-cli/src/door_client.rs:32`). AC3: an invalid file, or a region that disagrees with the cohort manifest (21-3 AC6), is a typed boot refusal. AC4: `maos run <topology>` keeps working.

**22-2.** AC1: the deb and rpm install a systemd unit; `systemctl start maos` serves and `stop` drains. AC2: `maosctl status` exits 0 only when the daemon answers and its audit store is writable, so an exec probe needs no unauthenticated route. Not in scope: the unbounded local Worker join at shutdown (`RELEASE-HOLDS.md:85` row 11), which is kernel lifecycle work.

**22-3.** AC1: the release publishes a multi-arch image to the GitHub registry `[INFERENCE: token scope not verified]`; a compose file runs a Host with a persistent home. AC2: `packaging/helm/maos` deploys a Host with a persistent volume and `host.toml` plus secrets from values; the door stays loopback. AC3: image size and idle memory are measured and written into NFR-Ops-14. AC4: the chart states that T3 Workers are not supported in a pod.

**22-4.** AC1: `maosctl ask <spirit> "<text>"` returns a task id; `maosctl task show <id>` returns the result or the halt that stopped it. AC2: the door route is a new `OperatorCommand`, so it gets an operation id and a completion row. AC3: a Spirit that is not running is a typed refusal. Network callers arrive through `23-5`, not through the door.

**22-5.** AC1: `maosctl agents` prints one row per Spirit: id, pid, state, form, originator, current task, age. Sources: `GET /v1/daemon` (`maos-control/src/lib.rs:1139`; no CLI consumer today, `rg -c 'v1/daemon|daemon_status' crates/maos-cli/src` = 0) and a read route over `task_assignments_in_flight` (`crates/maos-kernel-core/src/scheduler/control_block.rs:281`). AC2: `maosctl agents show <id>` renders that pid's log rows; `--follow` polls. AC3: `crates/maos-bin/src/delegation.rs:719,788` journal at the emitter's pid, not 0.

**22-6.** AC1: a retention policy rotates the Transparency Log into a sealed, signed segment and starts a new one; `maosctl audit verify-bundle` accepts each segment alone. AC2: a query that spans segments reads them in order. AC3: log-before-deliver holds across a rotation, and the escape detector's cursor (`escape_detector_consumer.rs:15-19`) survives it; both have a planted red.

**22-7.** AC1: an agent given only the shipped kit generates, builds, signs and loads a domain Spirit that answers `maosctl ask`. AC2: `maos-spirit validate` works (it exits 1 today, `crates/maos-spirit-cli/src/bin/maos-spirit.rs:107-114`). AC3: three agent runs are recorded with their transcripts and labelled as agent-authored. AC4: the human-denominator legs of `check-third-party-trial` are removed.

**22-8.** AC1: `journey_always_on` boots from `host.toml` with Mira and Nash, hands Mira a task, sees her advisory reach Nash through the in-process router under typed-intent consent, restarts the Host, and finds the cast, the memory and a pending halt intact. AC2: Mira and Nash gain inference seams (21-1 AC2) and the topology loader gains the router (21-1 AC3). AC3, operator lane and never blocking: a blank-context agent deploys the released image and completes one task in 30 minutes or less.

**22-9.** The rows listed in §5, each closed with its witness or kept with a stated reason, as 21-9 AC10 and AC11 specify.

### Epic 23 — Standards Interop (W8)

**Exit:** `cargo test -p maos-journey-test --test interop_session_23`: one session with an editor over ACP, an authorized MCP server and an A2A v1.0 peer. It absorbs 21-8. HEAD: red, no such target. **Kernel-Δ:** forecast 0.

| Key | Size | Closes |
|---|---|---|
| `23-1-mcp-oauth-client` | L | FR67, NFR-Sec-20 |
| `23-2-oidc-discovery-and-key-rotation` | M | NFR-Sec-18 extension |
| `23-3-otel-wiring-and-genai-projection` | M–L | FR69, NFR-Obs-2 live, NFR-Obs-6 |
| `23-4-siem-network-sink-and-durable-watermark` | M | NFR-Aud-11 network half |
| `23-5-a2a-v1-agent-card-and-jsonrpc-binding` | L–XL | FR68; `success-criteria.md:50` |
| `23-6-enterprise-posture-in-air-gap` | M | FR76 (air-gap) |
| `23-7-peer-identity-rotation` | M | 21-3 AC2–AC4; 14-2 clause (c.2) |
| `23-8-external-kms-adapter` | S–M | Decision 3 |

**23-1.** HEAD: one static bearer per server (`crates/maos-bin/src/env_contract.rs:117-155`; `crates/maos-mcp/src/transport/streamable_http.rs:44-55`); `rg -i oauth crates/maos-mcp/src` = 0. AC1: against a fixture authorization server and MCP server, a 401 leads to discovery, a token naming that server as its resource, and a successful call. AC2: server A's token is never sent to server B (plant). AC3: tokens live in the `SecretStore` and in no Spirit memory or log row. AC4: the daemon grant is client-credentials with a pre-registered client; a browser flow is a D-row. Specification: MCP authorization 2026-07-28 (*search synthesis*).

**23-2.** HEAD: static keys from the environment (`crates/maos-bin/src/enterprise_identity.rs:505-530`); one process-wide assertion (`crates/maos-bin/src/worker_spawn.rs:420`). AC1: discovery with refresh on an unknown key id, failing closed past the cache TTL. AC2: the assertion can be supplied per request. Static keys remain for air-gap.

**23-3.** HEAD: `with_trace_sink` (`crates/maos-iac/src/adapter.rs:127`) has no production caller and no crate depends on `maos-telemetry`. AC1: with an OTLP endpoint set the daemon installs the sink; unset, it opens no socket. AC2: agent, tool and model spans use GenAI names at a pinned version (*search synthesis*: the conventions are in Development status), projected from redacted rows outside the kernel. AC3: one trace per task whose token counts equal the cost frames.

**23-4.** HEAD: file sink only (`crates/maos-siem/src/lib.rs:16-18`); process-local watermark (`main.rs:222-224`). AC1: a TLS-only network sink behind a feature. AC2: a persisted watermark; a restart no longer re-sends.

**23-5.** HEAD: length-delimited JSON-RPC over mTLS TCP (`crates/maos-a2a-tcp/src/transport.rs:3-8`); no Agent Card anywhere. AC1: the Host serves a signed card for each exposed Spirit, built from its manifest. AC2: an inbound A2A v1.0 request maps to a typed-intent frame and passes `A2ARouterCore::handle_intake`; consent and log-before-deliver are unchanged; it lands as a `22-4` task. AC3: outbound, a peer's card signature is verified before first contact. AC4: the mTLS wire stays for cohort peers. Specification: A2A v1.0.1 (*search synthesis*). New crate: a kloc row and a workspace-count bump.

**23-6.** HEAD: `maos-sso` and `maos-siem` sit under `network` (`crates/maos-bin/Cargo.toml:16-31`; runbook `ent-1…:182`). AC1: an air-gap build accepts SSO with static keys, the at-rest seal and the SIEM file sink; discovery, the TLS sink and OTLP stay compiled out.

**23-7.** AC1, decided first as a D-row: peer certificates issued and rotated by the enterprise's own PKI, or the existing choreography completed as 21-3 AC2–AC4 specify. Either way the own-leaf swap gets a production caller and the post-grace refusal is typed.

**23-8.** AC1: with an endpoint configured, the at-rest seal wraps and unwraps through it; the endpoint down means sealed writes are refused, as ADR-051 requires. AC2: a reference sidecar for one open-source KMS lives under `examples/`, outside the release binary.

### Epic 24 — Multi-Operator Control Plane (W9)

**Exit:** `cargo test -p maos-bin --test operator_plane_24`. HEAD: red, no such target. **Kernel-Δ:** `24-3` only, forecast 0…+120 tokei `[INFERENCE]`; row at HEAD 20,429 of 23,000 (`xtask/kloc.toml:264`).

| Key | Size | Closes |
|---|---|---|
| `24-1-operator-principal-and-door-authz` | L | FR70, NFR-Ops-13, RELEASE-HOLDS v2.2 row 2 |
| `24-2-needs-you-inbox` | M | FR71 (inbox) |
| `24-3-interactive-approval-hold` | L | FR72 |
| `24-4-policy-driven-containment` | M | FR74 |
| `24-5-per-spirit-pdp-subjects` | M | Runbook `ent-1…:181` |
| `24-6-budgets-as-policy` | M | FR73, NFR-Cost-2 |

**24-1.** HEAD: one shared bearer (`maos-control/src/lib.rs:1064`), loopback only (`:697`). AC1: under SSO each door request carries an assertion and the principal is written into the completion row and the Approval Decision Log. AC2: Cedar evaluates operator × verb × target before submit; a deny is a typed 403 and a journal row. AC3: without SSO the single bearer is one operator, and the output says so. AC4: the legal-hold-release and collective-erase one-shots require the operator axis.

**24-2.** HEAD: three separate lists and no live halt read (only `["v1","halts",id,"resolve"]`, `lib.rs:1247`). AC1: `maosctl inbox` lists pending halts, skills, re-pins, approvals and budget halts, each with its decision verb. AC2: `--watch` polls.

**24-3.** AC1: a `RequireApproval` decision records a pending approval with a TTL; `maosctl approve <id> --intent "<why>"` or `deny` resolves it; the approval row names the operator and intent and is written before any mint. AC2: the Spirit receives the existing `capability-denied` and succeeds on retry inside the approved window; no contract change (decision 2). AC3: expiry is a denial.

**24-4.** HEAD: the Observer only notifies (`spirits/observer/src/lib.rs:370-383`). AC1: a rule file maps a signal and threshold to `pause`, a posture shift, a revocation or `unload`; a reconciler submits the command attributed `policy:<rule>`. AC2: no rule, no action. AC3: a drift fixture with a rule stops the Spirit within one reconcile interval.

**24-5.** AC1: a governed collective operation is evaluated with the originating Spirit as subject and the principal's attributes in context (`crates/maos-domain/src/ports/policy_decision.rs:112`; unused today, `crates/maos-pdp/src/adapter.rs:75-76`).

**24-6.** AC1: a policy file declares budgets; a reconciler sums cost frames and on breach adds a per-Spirit deny for inference through `PolicyTable::update()` (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:42,143`) and raises a halt. AC2: an authorized override raises the budget and writes the approval row. AC3: the overshoot bound is stated and tested.

### Epic 25 — Proof You Can Run (W10)

**Exit:** `maos verify --offline` on the built artifact, exit 0 with every verdict printed. HEAD: red, no such verb. **Depends on:** 21-10.

| Key | Size | Closes |
|---|---|---|
| `25-1-maos-verify` | L | FR75 |
| `25-2-control-map` | S–M | NFR-Comp-6 |

**25-1.** AC1: `maos verify` runs the 21-10 predicates, the sandbox-escape corpus (`crates/maos-eval/fixtures/t3-escape-corpus-v1`) and audit-bundle verification, and writes a signed report with version, posture and each verdict. AC2: a leg that cannot run is printed SKIPPED with its reason and fails the run unless allowed. AC3: it needs no network.

**25-2.** HEAD: `STABILITY.md:78-83` maps SOC 2, ISO 27001 and FedRAMP only. AC1: `docs/compliance/control-map.md` adds EU AI Act record-keeping and human oversight, ISO/IEC 42001, NIST AI RMF and AIUC-1; each row names the mechanism, the invariant or gate, the evidence command and what the operator still owns. Clause numbers are read from the primary texts at creation. AC2: an in-crate test enforces NFR-Comp-6.

### Epic 26 — Worker Fleet (W11)

**Exit:** `cargo test -p maos-journey-test --test journey_fleet`, created by `26-1`. HEAD: red, no such target. **Kernel-Δ:** forecast 0.

| Key | Size | Closes |
|---|---|---|
| `26-1-concurrent-worker-fanout` | L | NFR-Perf-8 |
| `26-2-crash-safe-delegation-reconciliation` | M | RELEASE-HOLDS row 10 |
| `26-3-worker-egress-usage-metering` | M | `deferred-work.md:1518`; FR73 (metering) |
| `26-4-worker-workspace-size-cap` | S–M | RELEASE-HOLDS row 20 |

**26-1.** HEAD: each Worker runs inside `block_in_place` (`main.rs:5661`). AC1: `maos run <topology> --epic <file> --parallel N` runs up to N declared-independent sections at once; N = 1 keeps today's order. AC2: with the fixture CLI (`crates/maos-bin/src/worker_cli.rs:944`) and N = 50, zero tasks are dropped; latency is recorded, not gated. AC3: dependent sections still receive the prior distillate.

**26-2.** AC1: after a kill between `journal(Written)` and the spawn, the next boot re-dispatches the task or delivers `task.orphaned` to the originator.

**26-3.** AC1: the egress proxy (`crates/maos-egress/src/proxy.rs`) emits a cost frame at the Worker's pid from the usage the provider returns; an unreadable usage is a row marked unknown. D-row: whether the proxy sees response bodies `[INFERENCE]`.

**26-4.** AC1 as `RELEASE-HOLDS.md:94` specifies, with its proven-red vector.

**Left out:** vendor KMS backends; more Worker CLI adapters than `codex` and `claude`; an evaluation gate at admission; a graphical console; a non-loopback operator door; messaging gateways (FR54 stays v2.5).

## 7. Sequencing and handoff

| Step | When | Content | Owner |
|---|---|---|---|
| 0 | now | Nothing lands on main until the fuzz chain finishes and `v1.0.0-rc.1` is pushed. The tag is an ordinary `git tag`; CI signs the artifacts with the repository secret `RELEASE_SIGNING_KEY` (`tag-procedure.md:28-36`). The orchestrator pushes it under ruling 13 when conditions a–e are green. No operator key is involved. Steps 1–5 of the correct-course workflow run meanwhile; they write only this file. | orchestrator |
| 1 | after the tag | Apply §2 and §5 in one documents-only commit, in the post-tag worktree. | `scribe` |
| 2 | after step 1 | GA set, 10 stories: the prepared model batch with S2 first (before the real-model run); `21-9` next; `21-3`, `21-6`, `21-7`, `21-10a/b/c`, `21-12` and the quarantine closure in parallel. | `story-creator`, `coder` |
| 3 | GA | rc.2, the real-model run, `v1.0.0`. Each tag needs its own fuzz chain at its exact SHA (`.github/workflows/release.yml:155-159`) and the operator's word to push; ruling 13 covers rc.1 only. | orchestrator; operator authorizes each tag |
| 4 | Epic 21 retrospective | Preflight Epic 22. | orchestrator |
| 5 | after GA | Epic 22, opened by `22-1`; then 23, 24, 25, 26. | `story-creator`, `coder` |

**Success criteria.** Epic 21 closes on its new exit with 14 of 14 invariants `guaranteed`. Each lane epic's exit is green with a red-before witness. The six claim boundaries of §2 are in `RELEASE-HOLDS.md` and the GA notes. The `xtask` crate does not grow because of this lane.

## 8. Facts and method

- **Commands (read-only, repo root, `1a57035e`).** `git rev-parse HEAD origin/main`; `git tag -l 'v1.0.0*'`; `gh run list --commit 1a57035e…`; the `rg` lines quoted in §1 and §6; `rg -n '"(start|pause|resume|unload|uninstall|posture|upgrade)" =>' crates/maos-control/src/lib.rs`; `rg -n 'variant .*error|capability-denied' wit/spirit.wit`; `rg -n 'pub trait KeyManagementPort' -A16 crates/maos-domain/src/ports`.
- **Sizes** are in Epic 20 median stories (S < 1, M ≈ 1, L ≈ 2, XL ≈ 3), the basis of the 2026-11-12 proposal §8.2. The split sizes in §2 are estimates.
- **External sources** are search summaries of `modelcontextprotocol.io/specification/2026-07-28/basic/authorization`, `a2a-protocol.org/v1.0.1/specification/` and `github.com/open-telemetry/semantic-conventions-genai`.
- **Not measured.** Whether Spirits loaded over the door survive a restart (grep-negative only); how `check-third-party-trial` behaves with no results file; idle memory and image size; the kernel delta of `24-3`; whether the egress proxy sees response bodies; the registry token scope; every clause number in `25-2`; the `deferred-work.md` row numbers in §2 and §5, which are those printed in `epic-21…w6.md:151-192`. No cargo command was run.

## 9. Correct-course record

Workflow `bmad-correct-course`, Batch mode. Customization resolved by `_bmad/scripts/resolve_customization.py`: no prepend or append steps, `on_complete` empty; the persistent fact `**/project-context.md` matches no file.

| Workflow section | In this file |
|---|---|
| 1 Issue summary | §0, §1 |
| 2 Impact analysis | §1, §2, §5 |
| 3 Recommended approach | §3, §4, path table below |
| 4 Detailed change proposals | §2 (Epic 21, old → new), §5, §6 |
| 5 Implementation handoff | §7 |

| Item | Status | Finding |
|---|---|---|
| 1.1 Trigger story | N/A | No story. The trigger is the founder's direction of 2026-11-13 (§0). |
| 1.2 Core problem | Done | Strategic pivot and a new stakeholder requirement: deploy anywhere; domain Spirits always on, on an enterprise server. |
| 1.3 Evidence | Done | §1. |
| 2.1 Current epic | Done | Epic 21 cannot close as written: both exit lines belong to a dropped story (§2). |
| 2.2 Epic-level changes | Done | Epic 21 scope and exit modified; Epics 22–26 added. |
| 2.3 Remaining epics | Done | None exists after 21: `rg 'epic-2[2-6]' epics/index.md sprint-status.yaml` = 0. |
| 2.4 Obsolete or new epics | Done | None obsolete; five new. |
| 2.5 Order and priority | Done | Lane after `v1.0.0`; Host first, Worker fleet last. |
| 3.1 PRD | Done | No conflict with PRD goals. Additions in §5; ids measured free (highest existing: FR66, NFR-Sec-19, NFR-Obs-5, NFR-Ops-12, NFR-Cost-1, NFR-Comp-5; journeys B, R, 1, 3, 4). v1.0 scope narrows by six claim boundaries. |
| 3.2 Architecture | Action-needed | §11 gains §11.6; ADR amendments by owning story (§5). §11, §15.7 and ADR-062 were read; the other architecture shards were searched, not read in full. |
| 3.3 UI/UX | N/A | No UX specification exists in `planning-artifacts`. Operator surfaces are specified in `22-5` and `24-2`. |
| 3.4 Other artifacts | Done | `Dockerfile`, `container.yml`, packaging, runbooks, `docs/maos.dev`, README, `RELEASE-HOLDS.md`, GA notes. No new `xtask` gate. |
| 4.1 Direct adjustment | Viable | Effort medium, risk low. Epic 21 is edited in place; new epics are appended. |
| 4.2 Rollback | Not viable | Nothing shipped needs reverting. One unused path (auto-allow) is deleted inside 21-10a. |
| 4.3 MVP review | Viable | Effort low, risk medium. 21-8 and half of 21-3 leave the GA set; v1.0 claims narrow. |
| 4.4 Selected path | Done | Hybrid of 4.1 and 4.3. |
| 5.1–5.5 Proposal components | Done | Section map above. |
| 6.1 Checklist review | Done | Open items: 3.2 (owning stories), 6.4 (after the tag). |
| 6.2 Accuracy review | Done | Three corrections made before approval (header); claim boundary 6 added after it (§2). |
| 6.3 Approval | Done | Operator, 2026-11-13: "approved the proposal". No conditions stated. |
| 6.4 `sprint-status.yaml` | Action-needed | After `v1.0.0-rc.1` is pushed and after the 2026-11-12 proposal is applied: Epic 21 rows per §2; `epic-22` … `epic-26`, 29 story keys and five retrospective rows as `backlog`; parked key `v25-reza-postgres-legs`. |
| 6.5 Handoff confirmed | Done | Handoff log below. |

**Scope: Major.** New PRD requirements and a journey, five epics, an architecture section, and narrower GA claims. Route: the operator rules; `scribe` applies §2 and §5; `story-creator` and `coder` follow §7.

**What approval covers.** (1) The proposal as a whole. (2) Two reversals of 2026-11-12 rulings: 21-8 leaves the GA set (`ga3`), and the rotation half of 21-3 leaves it (B3). (3) Order against `sprint-change-proposal-2026-11-12.md`, which is also unapproved with five open items and edits the same tracker rows and `epic-21…w6.md`: that proposal is applied first, this one on top.

**Handoff log (2026-11-13T08:40Z).**

| To | Receives | Starts when |
|---|---|---|
| operator | Items 2–5 of the 2026-11-12 proposal's §10. **Closed 2026-11-13** by directive (§10 below). | done |
| orchestrator | Push of `v1.0.0-rc.1` under ruling 13 when conditions a–e are green. Chain `43334961225` was on link 2 of 5 at this read. | chain complete |
| `scribe` | §2 and §5 as one documents-only commit on top of the 2026-11-12 application. One file overlaps the post-tag worktree's uncommitted work: `RELEASE-HOLDS.md` (`git -C ../dev_ws-posttag status`). The tracker, `epics/` and the PRD requirement files are not modified there. | tag pushed, 2026-11-12 proposal applied |
| `story-creator`, `coder` | GA set, `21-9` first (§7 step 2). | after `scribe` |

**Effect on the 2026-11-12 proposal.** By the operator's directive of 2026-11-13 that file now carries a ruling block after its §10 item 5 and a §12 that lists every place its body is superseded by this proposal. Its body is otherwise unchanged.

## 10. Rulings on the 2026-11-12 open items

Operator, 2026-11-13, verbatim: "1. we wait for the gag. 2. take what are align with current proposal, amend those conflicts or obsolete in the four open items in yesterday's proposal." The choices are the orchestrator's application of that directive; the full reasons and cites are in that proposal's ruling block.

| Item | Taken | Amended because of this proposal |
|---|---|---|
| Kernel exact pin | Keep the content-hash pin; retire the aggregate hard-fail and the per-landing re-pin (option A). | Lands before the prepared model batch, so that batch needs one kernel re-pin and no per-crate re-pin. |
| Security wording and channel | Wording approved; GitHub private vulnerability reporting is the channel. | One wrong path in the README text is corrected to `architecture-maos-minimal-opus/`. |
| Branch protection | `aggregate` required, `strict=false`, `enforce_admins=true`, landing flow as drafted. | "Operator signing" is read as "the operator authorises the push". |
| Reasoning-model stories | S2 (usage fixes) joins GA; S3a–S9 become Epic 27 after GA. | Not option A or B as printed. The post-GA epic takes number 27 because 22–26 are this lane. S2 is pulled forward because a missing Anthropic usage field is recorded as 0 tokens at HEAD (`crates/maos-providers/src/anthropic.rs:140-148`) under a v1.0 cost requirement. |

**Consequences here.** The GA set is 10 stories, not 9 (§7 step 2). Two more GA-notes lines come from the last row and are worded in that proposal: reasoning-model handling at v1.0.0 is limited to pin serving and the Anthropic stop-gap, and a Spirit may call any model admitted for its provider. `sprint-status.yaml` also gains `epic-27` at application; its story keys are assigned at story creation.

**Open, not blocking GA.** The lane ships one minor release per epic, but "v1.1" and "v1.5" already name PRD phases with other content (`prd/non-functional-requirements.md:179`; 52 `v1_5` keys in `xtask/gate-registry.toml`). The numbering of lane releases is a D-row for the Epic 22 preflight.

## 11. Application record (documents commit, prepared 2026-11-13)

Applied together with `sprint-change-proposal-2026-11-12.md` as one documents-only commit, prepared on branch `docs/scp-2026-11-12-13` from `1a57035e` and landed only after `v1.0.0-rc.1` pointed at `1a57035e` (operator: "we wait for the [t]ag"). The four edit slices were written by `coder` agents under one contract and integrated by the orchestrator; the `scribe` agent was unavailable (provider quota).

**Files.** New: the five lane epic files (`epics/epic-22-…w7.md` to `epic-26-…w11.md`), `tracker-notes-archive-2026-11-13.md`, and the two proposal files. Edited: `epics/index.md`, `epics/epic-21-multi-host-on-live-substrate-w6.md`, `epics/requirements-inventory.md`, `sprint-status.yaml`, `deferred-work.md`, four PRD files (functional, non-functional, journeys, scoping), `decisions-2026-11-12-retro-preflight.md` (now a pointer, K1), `RELEASE-HOLDS.md`. Not touched: `prd/success-criteria.md`, anything under `crates/`, `xtask/`, `.github/` or `docs/`, `README.md`.

**Checked before the commit** (prebuilt `xtask` binary run in the worktree; no cargo): `check-epic-close-coherence` (26 epics), `check-exit-commands` (6 epics, 6 blocks, 2 tokens resolved, 1 owed: `maos verify` → `25-1`), `check-dev-record-completeness`, `check-decision-register`, `check-review-findings-resolved`, `check-coverage-matrix-completeness`, `check-fr47`: all exit 0, as at `1a57035e`. The `xtask` integration tests were not run locally; the hosted `aggregate` run on the pull request is their proof.

**Where the application differs from the text above.**

| # | Text above | Applied | Why |
|---|---|---|---|
| 1 | §10: the tracker gains `epic-27` | No `epic-27` key, no Epic 27 file, no S2 key | `check-exit-commands` needs one exit block per open epic key (`xtask/src/check_exit_commands.rs:895-925`), and the adapter design is not on `main`. The tag-lane session files them at its preflight. |
| 2 | §2: 21-4 dropped | The key `21-4-one-instrument-and-env-registry` stays `backlog`, marked "dropped as a story" | 16 open `deferred-work.md` rows and the decision-register rows at `epics/epic-14-preflight-decisions.md:67-79` name it; removing it reds two gates. The rows are triaged one by one by the 2026-11-12 proposal's B7e. **Epic 21 cannot be marked `done` until that triage removes the key.** |
| 3 | §2: the new Epic 21 exit has two lines | Three lines | `invariant_claims_20_3b` passes at HEAD with zero `guaranteed`. The added line fails unless 14 of 14 invariant files read `claim: guaranteed`. |
| 4 | §6: exits for Epics 23 and 24 named without a creating story | `interop_session_23` is created by `23-5`; `operator_plane_24` by `24-1` | Confidence rule 1: an exit verb exists or is an AC in the epic. |
| 5 | §5: six NFR ids added | Added as one block at the end of `non-functional-requirements.md` | No existing line of that file moves, so every existing cite into it stays valid. |
| 6 | §5: re-owned rows | Two more: `deferred-work.md:1154` → `v25-reza-postgres-legs` (it named removed key 21-2); `RELEASE-HOLDS.md` 14-2 clause (c.8) → `23-7` | Each still named a key that no longer exists. |
| 7 | — | The 2026-11-12 evidence directory is not committed | Ruling B7a of that proposal keeps bulk evidence out of git. |

**Facts above that were wrong, corrected in the applied files.**
1. **§1, "Cast returns after a restart".** The grep does not return 0; it returns 80 lines. A restart already brings back installed Spirits when the daemon is a shell root or a serving root with no run arguments (`crates/maos-bin/src/main.rs:4384-4390`, `4454-4455`; `crates/maos-bin/src/installed_spirits.rs:16`). Under `maos run <topology>` that list is empty, so only the topology's Spirits return. `22-1` therefore gives two existing boot paths one declared source; its size L may be high.
2. **§6, Epic 24 kernel line.** `xtask/kloc.toml:264` holds the current row (20,429). The 23,000 envelope is stated at `xtask/kernel-core-baseline.toml:514`.
3. **§5 and §2, `deferred-work.md` rows.** F1–F5 are at `:1739-1743` and the swap-invariant row at `:1761`, one line below the numbers printed here.
4. **§6, `24-6`.** `PolicyTable::update()` is at `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:251`, not `:42,143`.

**Narrowed at application.** Claim boundary 4 (J4 closure rate): `22-8` keeps the Mira and Nash seams and cuts the closure-rate corpus, so the rate stays unmeasured after Epic 22 as well; `RELEASE-HOLDS.md` says so. Claim boundary 5: the clause "already Low rows in the rc.1 notes" was not verified and is not in `RELEASE-HOLDS.md`.

**Left for others.**
- `deferred-work.md:1021` (two locks in the self-identity read path) is a candidate for `23-7`; not moved.
- `docs/adr/ADR-064-one-inference-mode-selector.md:114` and `docs/runbooks/provisioning-checklist.md:63` still name the removed keys 21-1 and 21-11.
- `RELEASE-HOLDS.md` GA-tag procedure item 4 still says the tag needs a key holder; the tag-lane batch edits the lines beside it.
- `epics/epic-17-workers-and-third-party-form-w2.md:528` cites `user-journeys.md:329`; that table row is at `:349` after Journey 5 was added.
- Line numbers of `sprint-status.yaml` after `:422` and of `RELEASE-HOLDS.md` after `:95` moved in this commit. Cites printed above are as measured at `1a57035e`.

**Operator rulings recorded after approval.** B7c-A (the 2026-11-12 proposal's §10 item 2) is implemented by the tag-lane session ("the other session manages the B7c-A", 2026-11-13), as the first commit of its batch.

*End of proposal.*
