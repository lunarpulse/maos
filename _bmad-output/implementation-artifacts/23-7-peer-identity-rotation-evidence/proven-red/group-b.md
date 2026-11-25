# Review group B proven-red plants (marker `E237-PLANT`, one at a time, removed after each)

## B1a — floor raised after the commit (old order)
Plant: `maos-cohort/src/state.rs` `apply_reissue`: early `self.raise_manifest_floor(candidate.version)` removed, a raise added after `drop(cached)`.
Red: `cert_rotation_14_2a::the_durable_floor_is_written_before_the_reissue_becomes_visible`: `left: Some((true, true)) right: Some((false, false))` (the first floor-2 write saw the open window and the accepted row).

## B1b — loaded floor not installed into the state before rotation
Plant: `CohortManifestState::install_manifest_floor` made a no-op.
Red: `cert_rotation_14_2a::a_floor_loaded_before_the_rotation_install_refuses_a_superseded_reissue`: `v2 is below the loaded floor of 3...: Applied { version: 2 }`.

## B3a — host SAN not tied to this host
Plant: `transport.rs` `check_own_leaf_san` host comparison guarded by `&& false`.
Red: `san_scoping_23_7::an_own_leaf_naming_another_host_is_refused_by_every_swap_path`: `a leaf naming host_y is not this host's: Swapped {..}`.

## B3b — chain/usage check removed from the committed material
Plant: `validate_own_identity` no longer calls `check_own_chain`.
Red: `san_scoping_23_7::an_incomplete_chain_is_refused_and_the_completed_pem_heals_on_the_next_try` (`Swapped` on the intermediate-less PEM) and `::a_leaf_missing_either_tls_usage_is_refused` (`serverAuth only: ()`).

## B4 — server session resumption left enabled
Plant: `build_server_config_with_san` no longer sets `session_storage = NoServerSessionStorage` / `send_tls13_tickets = 0`.
Red: `san_scoping_23_7::a_resumed_connection_cannot_bind_a_reassigned_fingerprint`: `left: Some(Resumed) right: Some(Resumed)` ("the server must not resume a session").

## B5a — the swap gate ignores unconfirmed peers
Plant: `CohortManifestState::commit_own_leaf` hold condition guarded by `false &&`.
Red: `cert_rotation_14_2a::the_own_leaf_swap_waits_for_every_peer_to_confirm_the_version`: first call `Committed { result: "swapped", unconfirmed: ["host_b", "host_c"] }` instead of `Held`.

## B2 — declaration not re-checked at the commit
Plant: `commit_own_leaf`'s `Superseded` check guarded by `false &&`.
Red: `cert_rotation_14_2a::a_declaration_replaced_between_prepare_and_commit_is_never_committed`: `a third leaf: left: Held { unconfirmed: ["host_b"] } right: Superseded`.

## B5s — the same gate plant, seen by the 3-node scene
Plant: as B5a. Red: `peer_rotation_23_7::g_covers_one_pull_lag_and_the_old_leaf_is_refused_only_after_it` at `X<->P2 between X's swap decision and P2's apply: accepted both ways` (the scene's P2 assertion, as AC3 words it: X committed at its decision point, so the P2 handshake was refused).

## AC6 — accept a leaf on first sight after a restart (the AC6 plant that was never run)
Plant: `verifier.rs` dial side `verify_pinned_sync(..).map_err(..).or(Ok(()))` (a presented leaf that is not the pin is accepted).
Red (single run): `peer_rotation_23_7::a_peer_that_missed_the_rotation_fails_closed_heals_on_a_pull_and_keeps_the_promotion` at `P refuses a leaf it holds no promotion for` (the refusal step).
