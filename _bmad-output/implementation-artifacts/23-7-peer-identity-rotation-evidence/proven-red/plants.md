# Proven-red plants (marker `E237-PLANT`, batched run, then removed)

| Plant | Edit | Owning proof | Observed |
|---|---|---|---|
| AC2c | `reload_own_identity`: `if false && &on_disk != declared` (guard dropped) | `self_leaf_rotation_23_7` | binary FAILED: 2 failed, 0 passed (120.55 s: waits hit the outer deadline) |
| AC2d | `swap_serving_cert`: plane C update removed | `peer_rotation_23_7::a_swap_moves_plane_c...` | a2a-tcp group FAILED (4 failed in the first failing binary; per-test names not captured) |
| AC3 | `reload`: `grace = self.grace` (bare base) | cohort `t_grace_matches...`, window-open test, `g_covers_one_pull_lag` | cohort binary: 3 failed |
| AC4 | `refusal_reason`: `true ||` (variant without the join) | `t_14_2a_post_grace_journal` never-promoted test | a2a-tcp group FAILED |
| AC5 | verifier listen side: SAN ignored (`&None::<Arc<str>>`) | `san_scoping_23_7` | a2a-tcp group FAILED |
| AC7 | `apply_reissue`: floor read as 0 (in-memory `seen_version` only) | `peer_rotation_23_7` replay test, cohort floor test | cohort and a2a-tcp FAILED |
| AC8 | `store_with`: pre-fence disabled (fence only after) | persistence unit tests incl. the simulated crash | persistence lib: 3 failed, 3 passed |

Not run: AC6 plant. Limitation: because `cargo test` stops at the first failing binary and the plants were batched, a2a-tcp per-test attribution is not recorded; rerun each plant singly if a reviewer needs it.
