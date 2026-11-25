# Review group A plants (marker `E237-PLANT`, ONE AT A TIME, each removed after its run)

| Plant | Edit | Owning proof | Observed |
|---|---|---|---|
| A2 | `tofu.rs`: both `self.fence_before_retirement(peer);` calls commented out | `maos-a2a-core --test peer_rotation_23_7::both_promotion_closes_fence_before_the_retirement_is_observable` | FAILED (6 passed, 1 failed): left `["write:sha256:10ae..."]`, right `["fence:sha256:04ab...", "write:sha256:10ae..."]`; green again after removal |
| A4 | `maos-persistence` `lift_fence`: `sync(&self.lock)` replaced by `let _ = sync; Ok(())` | `--lib the_fence_lift_is_synced_and_a_refused_sync_is_an_error` | FAILED (6 passed, 1 failed) at the `synced` assertion; green again after removal |
| A1 | `maos-cohort/src/digest.rs` `journal_rupture_frame`: the transport-refusal arm returns the "missing denied intent attribution" error again (envelope required) | `maos-a2a-tcp --test t_14_2a_post_grace_journal` (real `CohortRuptureLogSink` into an in-memory Transparency Log) | FAILED: 3 passed, 4 failed (dial, listen, restart and forged tests: no durable row) |
| A3a | `verifier.rs` `pin_refusal`: `matches!(error, EPinMismatch::Mismatch {..})` replaced by `true` | same binary, `t_23_7_a_restarted_promoted_peer_with_its_current_leaf_is_not_a_retired_key_signal` | FAILED (6 passed, 1 failed) |
| A3b | `verifier.rs` `pin_refusal`: `ChainToRoots` condition made always true | same binary, `t_23_7_a_forged_self_signed_leaf_naming_a_promoted_peer_is_not_a_retired_key_signal` | FAILED (6 passed, 1 failed) |
| A5 | `maos-a2a-tcp/src/transport.rs` `build_server_config`: `.with_peer_san(san_trust_domain)` deleted | `xtask --test decision_adrs_and_provisioning::real_repository_satisfies_...` | FAILED (2 passed, 1 failed): "ADR-063 anti-rot anchor moved: ... no longer contains `.with_peer_san(san_trust_domain)`" |
