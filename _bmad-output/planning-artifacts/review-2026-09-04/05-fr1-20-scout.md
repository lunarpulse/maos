# FR1-20, 47-51 scout (HEAD 4657cace)
CROSS-CUTTING: every maosctl verb except `spirit inspect --sandbox` spawns a fresh `maos` process via MAOS_ONE_SHOT=<verb> (maos-cli/src/subcommands.rs:1461-1842), acts on that process's EMPTY in-memory scheduler/registry, journals, exits (main.rs:2747-2749). Control plane does NOT reach the daemon.
- FR1 PARTIAL: release.yml signs w/ Ed25519 but never ran (no v* tag); packaging/* SCAFFOLD + PLACEHOLDER_SHA256 + dev seed pubkey; not on crates.io.
- FR2 PARTIAL: spirit uninstall forgets principals/revokes tokens/erasure proof; no sandbox mounts/ACP sockets/caches removal; no kernel-binary uninstall.
- FR3 PARTIAL: drivers anthropic/openai/ollama (+fixture_replay) only; gemini/kimi/bedrock ABSENT (rejected as unsupported manifest.rs:3890); endpoints hard-coded; provider_endpoint_pin never enforced at inference.
- FR4 PARTIAL: provider calls token-checked+logged; Worker subprocesses spawned bare w/ creds (runtime.rs:461-473) => file/net/exec unmediated; no 1000-call verifier.
- FR5 PARTIAL: strictest_of live at admission; spawn_sandboxed ZERO prod callers; tier computed+journaled never applied.
- FR6 TEST-ONLY: cgroup writer resource_ceiling.rs:76-138, apply_resource_ceiling no caller outside tests; fd_max never written.
- FR7 ABSENT: no telemetry emitter exists; OTel sink never constructed.
- FR8 PARTIAL: missing skills, explanation_shape, intent_promotion_set, swap_invariants; local admit unsigned.
- FR9 CANNED: start/stop/unload = one journal row; pause/resume only hello-spirit; no load verb.
- FR10 PARTIAL: ADR-020 machinery exists; spirit-upgrade one-shot => NotLoaded unless predecessor in fresh process (only smoke-spirit). No subprocess-form hot-swap.
- FR11 PARTIAL: EMigratorMissing only on upgrade path; plain load never checks archive.
- FR12 PARTIAL: CrashDetector only trigger is in-process hook panic; subprocess exit classified but never routed to handle_crash (scheduler_loop.rs:583 sole caller). task.stalled 30s live.
- FR13 PARTIAL: CRL verify real; import in one-shot process w/ no Spirits => matched 0, forgotten.
- FR14 PARTIAL: frame+cross-host delivery live; every product-side emitter is smoke_* fn; shell doesn't emit task.assign; maos-notify-push never constructed.
- FR15 PARTIAL: halt resolve one-shot writes ADL against empty registry.
- FR16 PARTIAL: shift applied to throwaway PolicyTable; daemon never reads; no TTL.
- FR17 PARTIAL: Butler MorningDigest sole caller is bench; live digest = J3 cohort scene.
- FR18 LIVE (decision_logger.rs:53; main.rs:2398).
- FR19 PARTIAL: parsed, no CLI/schema; carrying frame smoke-only.
- FR20 PARTIAL: orchestrator queue enqueues in fresh process => lost; hard-coded hello-spirit.
- FR47 PARTIAL: Rust Spirits comply; Worker = external claude/codex CLI calling vendors directly w/ host creds => inference outside port+TL.
- FR48 PARTIAL: trait real; one impl RingCryptoProvider hard-wired.
- FR49 PARTIAL: same NotLoaded trap. FR50 PARTIAL: ReassignToReplica always degrades (NullReplicaResolver). FR51 PARTIAL: revoke-token mutates one-shot in-memory ring.
Overshoots: quarantine tier + T3 container spawn + CRL replay protection; 3 crash detectors; compliance fingerprinting of crypto/provider/endpoint signed into claims though pin not enforced.
