# 20-3c creation — cites re-measured at HEAD `3341f9e4` (rule 9)

`3341f9e4` = `714a1428` (20-3b final landing) + one docs commit; `git log -1 -- crates` = `714a1428`. Every line below was
re-read at HEAD with `read`/`grep`; "ok" means the cite still points at the named code. D = `deferred-work.md`,
E20 = `epics/epic-20-ship-it-w5.md`, PF = `epic-20-preflight-2026-10-30.md`, S = `sprint-status.yaml`.

## C — code cites in E20 §20-3c (amended paragraph E20:178-182) and AC7 (E20:192)

| # | Where | OLD | NEW at HEAD | Note |
|---|---|---|---|---|
| C1 | E20:178 | `capability/mod.rs:438-448` | ok (`Err(CapError::AuditSinkUnavailable)` `:446`) | member 1 |
| C2 | E20:178 | `cap_audit/mod.rs:26-38,69-98` | ok (enum `:26-35`, `COUNT` `:37-38`, `record_drop`/snapshot/`record_send_error` `:69-98`) | member 1 |
| C3 | E20:178 | `journal/mod.rs:100-103` | ok (`.append(true)` `:101`) | member 3 |
| C4 | E20:178 | `halt/resolver.rs:139-181` | ok (context write `:173-181` before transition) | member 4 |
| C5 | E20:178 | `scheduler_loop.rs:561-580` | ok (`terminate_spirit` `:562`, `fire_on_unload` `:580`) | member 5 |
| C6 | E20:178 | `control_block.rs:445-458,577-582` | ok (`(Loaded, Unloaded)` `:453-458`; test `loaded_to_unloaded_allowed` `:582`) | member 5 |
| C7 | E20:178 | `maos-cli/src/subcommands.rs:2111-2137` | ok | member 6 |
| C8 | E20:178 | `operator_door.rs:1716-1726` | ok (`rollback_resolution` `:1726`) | member 7 |
| C9 | E20:178 | `maos-shell/src/lib.rs:424,462-469` | ok | member 8 |
| C10 | E20:178 | `transport.rs:687-690,730-738,748-759` | ok | member 14 |
| C11 | E20:180 | member 11 "`crates/maos-bin/src/main.rs:4489-4494,5859-5872,6025-6040`" | **wrong target**: those are the `maos shell` (`:4493-4497`), `maos run` topology (`:5863-5872`) and standalone (`:6029-6040`) drains, not the `MAOS_ONE_SHOT` arms. The four arms' drains are **`main.rs:7486-7490`** (smoke-epic-4), **`:7644-7648`** (smoke-spirit-5), **`:7861-7865`** (smoke-supervision-5), **`:9459-9463`** (hello-spirit); the repaired `--once` precedent is `:6880-6955` | measured at HEAD binary: the four arms time out, `maos shell` does not (`runtime/oneshot-drain-3341f9e4.txt`) |
| C12 | E20:180 | `halt/mod.rs:465-474` | ok (`drain_for_spirit` `:466`) | member 12 |
| C13 | E20:180 | `main.rs:3516-3531,3633-3651` | ok; add the mechanism `crates/maos-domain/src/operator_door.rs:362-372` (`probe_loopback_endpoint` binds `:0` and drops) and the single writer `:410-438` | member 16 |
| C14 | E20:180 | `delegation.rs:455,584,713,782` | ok | member 17 |
| C15 | E20:180 | `maos-iac/src/adapter.rs:324-328` | ok (`spirit_pid` contract `:324-325`, `deliver_typed` `:329`) | member 17 |
| C16 | E20:180 | `adapter/transparency_log.rs:342-355` | ok (`RECIPIENT_PREDICATE` `:351-352`) | member 17 |
| C17 | E20:182 | `subcommands.rs:2767-2769,2940` | ok (refusal `:2768`, `_tag` `:2940`) | FR41 |
| C18 | E20:182 | `check_reza_production_path.rs:1070` | ok (+ args `:1078`; ledger `evidence_ledger.rs:252-253`; duplicate `check_multi_tenant_loom.rs:1583`) | Reza leg |
| C19 | E20:192 (AC7) | `delegation.rs:273,384,484,553`; `SpiritAuto` `:820` | `:455,584,713,782`; `completion_frame` `SpiritAuto` `:1133-1159` | live AC |
| C20 | E20:192 (AC7) | `adapter.rs:324-327`; `transparency_log.rs:350-351` | `:324-328`; `:351-352` | live AC |
| C21 | E20:194 | `xtask/kernel-core-baseline.toml:27-60` | obligation `:27-28` + EOF block; physical pin `:505`; set hash `:915`; tokei pin `xtask/kloc.toml:264` | kernel pins |

## D — ledger cites (the ledger grew 64 lines above the old anchors)

| # | Where cited | OLD | NEW |
|---|---|---|---|
| D1 | E20:182 | D:1130 (FR4 CLI) | **D:1194** |
| D2 | E20:182 | D:3 (EC5) | **D:64** |
| D3 | E20:182 | D:1202 (teardown anomaly) | **D:1268** |
| D4 | E20:182 | D:933 (Reza stage2 leg) | **D:996** |
| D5 | E20:192, S:300 | D:1070 (sender pid 0) | **D:1062** |
| D6 | E20:192 | D:1171 (native bus-send sibling) | **D:1166** |
| D7 | PF §3.3 | D:486-487 / 505-506 / 860-861 / 869-870 / 871 / 872-873 / 874-875 / 876-877 / 887-888 / 891-892 / 893 / 894 / 900-901 / 769-771 / 931 / 959 | :547-548 / 566-567 / 921-922 / 930-931 / **932** / 933-934 / 935-936 / 937-938 / 948-949 / 952-953 / **954** / **955** / 961-962 / 827-831 / 994 / **1022** (bold = still open) |
| D8 | 19-1 `:329`, 19-7 `:297` | D:959 (EndpointInUse) | D:1022 (historical records; not edited) |
| D9 | — | FR53 row | D:10 (new at 20-3b landing 2) |

## T — tracker / preflight statements corrected by measurement

| # | Statement | Fact at HEAD |
|---|---|---|
| T1 | PF:216 "no separate recurrence log/run found" for `wasm_form_admission_17_3b` `EndpointInUse` | The recurrence IS recorded, orchestrator-reported without a run log: `19-1-orchestrator-dispatch-loop-and-epic-input.md:329` and `19-7-repin-consent-operator-surface.md:297` (workspace nextest 5260/5261, 5/5 green alone). Mechanism re-reproduced at HEAD (`runtime/endpoint-reproduction-3341f9e4.json`). |
| T2 | Assignment: "root_shutdown / flake families the preflight assigns here" | PF:307 and D:1216-1218 assign the root-shutdown ac4 v4/v5 family to **20-7**; 20-3c's root-shutdown member (15, pin mismatch) is closed by 19-7 (D:994). No flake family other than member 16's `EndpointInUse` belongs here. |
| T3 | E20:180 / PF:211 "Per-arm current failure unmeasured" | Measured: smoke-epic-4 / smoke-spirit-5 / smoke-supervision-5 / hello-spirit each print `audit writer drain timed out after 5s` and exit 0, 3/3 runs, 5.21–6.27 s; `maos shell` 0/2 (304–335 ms). |
| T4 | Member 12 "no receipt" only | Also: the dry-run precheck already predicts `SafeMigrated` for pending halts and counts **every** Spirit's pending halts (`hot_swap/precheck.rs:50-70`, `_predecessor_pid` unused `:39`) while the real swap drains (`halt/mod.rs:466`) — the operator's precheck asserts a migration the swap does not perform. |
| T5 | Members 8 and 14 "closed" | Code shipped, **no regression witness**: `grep -rn 'clarification refused\|MAX_CLARIFICATION'` finds only `maos-shell/src/lib.rs:424,462-469`; `grep -rn '"awaiting response"'` finds only `transport.rs:689`; `t_2c_fault_windows.rs:109-178` stalls the handshake, not the response phase. |
| T6 | FR62(b) | Reader exists (`maos-audit/src/lib.rs:525-590`, test `ratification_loader_17_9.rs`), **no production writer** of `GovernanceEventKind::AbiExtension` (grep); ratifications live in `xtask/abi-ratifications.toml` (19 entries) read by `check_abi_ratification.rs:1-13`, while ADR-045 §4 (`:38-40`) requires a TL ratification frame as a sequence ancestor. Coverage row `tests/coverage-matrix.yaml:862-867` `gates: []`. |
| T7 | I14 ledger | `docs/invariants/I14.md:4-10` `claim: asserted`, `owner: 20-3c` — the owner goes stale when this key closes. |
| T8 | D07 (watchdog) | `schedule_watchdog.rs:216-221` mints after admission; `:326-342` a refused scope `continue`s and `first_id` keeps the first issued token, so a partially refused fire's TL row (`:225-232`) records a token as if the fire held its declared authority. Observation for D07's owner (20-9), not added scope. |
