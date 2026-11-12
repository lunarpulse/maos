# Hosted failing-before excerpts: root_shutdown_unload_16_3 ac4_v3b
Source: `gh api repos/lunarpulse/maos/actions/jobs/<id>/logs` (read-only). Root stderr carries no timestamps of its own; the only timing evidence is the test-runner wall time.

## run43215484029-job131173533987-nfr-rel-11-halt-receipt-999pct (23b2200b)

### runner lines before the panic
```
2026-11-12T13:20:36.0212378Z test ac3_operator_door_unload_stops_a_live_worker_once ... ok
2026-11-12T13:20:36.4970303Z test ac4_v1_serving_butler_receipts_the_raised_halt_on_sigterm ... ok
2026-11-12T13:20:36.5427557Z test ac4_v2_butler_once_receipts_the_raised_halt_and_drains ... ok
2026-11-12T13:20:36.5883141Z test ac4_v3_topology_once_unloads_every_loaded_spirit ... ok
2026-11-12T13:20:52.6240980Z test ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker ... FAILED
2026-11-12T13:20:53.8221229Z test ac4_v4_sigterm_during_standalone_worker_leaves_no_orphan ... ok
2026-11-12T13:20:54.9208360Z test ac4_v4b_grandchild_is_stopped_with_its_worker_container ... ok
2026-11-12T13:20:54.9682521Z test ac4_v8_one_shot_acp_server_has_no_listener_and_dies_by_signal_15 ... ok
2026-11-12T13:20:55.0269139Z test every_audit_sender_release_joins_the_yank_owner ... ok
2026-11-12T13:20:55.0447652Z test harness_owns_grandchildren_before_discovery_even_when_startup_panics ... ok
```
### panic + root stderr tail (selected lines)
```
2026-11-12T13:20:55.0448572Z thread 'ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker' (24762) panicked at crates/maos-bin/tests/root_shutdown_unload_16_3.rs:1436:5:
2026-11-12T13:20:55.0449329Z the interrupted line must replace `complete — exiting cleanly`: maos: crypto provider = ring 0.17.14
2026-11-12T13:20:55.0453366Z maos: Transparency Log opened on-disk at /tmp/maos-root-shutdown-16-3-v3b-24483-1794489636588512406/home/audit/transparency.sqlite
2026-11-12T13:20:55.0458169Z maos: operator HTTP listening on 127.0.0.1:44409
2026-11-12T13:20:55.0459533Z maos: root supervision armed
2026-11-12T13:20:55.0460246Z maos run: cli_wrapper cap-token mediation not granted (kernel policy denied capability issuance); proceeding under AC5 host-grant authority (operator policy `proc.exec` grant is Epic 9 surface). The CapabilityInvocation exit row is still journaled.
2026-11-12T13:20:55.0460964Z maos: T3 container bf822c841d7dfb5756ed8df63ca724397a7e773c39d96d79d7e36a61626fece2 CID kill failed; later cleanup outcome is reported separately
2026-11-12T13:20:55.0461520Z maos: worker worker stopped but its output is still held open by a descendant; exiting without the door shutdown and the audit-channel drain (process-tree teardown: 17-1)
2026-11-12T13:20:55.0461843Z 
2026-11-12T13:20:55.0462117Z 
2026-11-12T13:20:55.0462120Z 
2026-11-12T13:20:55.0462485Z 
2026-11-12T13:20:55.0462821Z 
```

## run43211291217-job131159286775-check-multi-tenant-loom (fe44017a)

### runner lines before the panic
```
2026-11-12T12:38:21.7981400Z test ac4_v3_topology_once_unloads_every_loaded_spirit ... ok
2026-11-12T12:38:21.7981878Z test ac4_v1_serving_butler_receipts_the_raised_halt_on_sigterm ... ok
2026-11-12T12:38:21.7982375Z test ac3_operator_door_unload_stops_a_live_worker_once ... ok
2026-11-12T12:38:21.7982878Z test ac4_v8_one_shot_acp_server_has_no_listener_and_dies_by_signal_15 ... ok
2026-11-12T12:38:21.7983408Z test ac4_v4_sigterm_during_standalone_worker_leaves_no_orphan ... ok
2026-11-12T12:38:21.7983995Z test cohort::ac4_v5b_sigterm_during_daemon_startup_exits_through_its_teardown ... ok
2026-11-12T12:38:21.7984568Z test ac4_v4b_grandchild_is_stopped_with_its_worker_container ... ok
2026-11-12T12:38:21.7985143Z test cohort::cohort_sigterm_without_worker_joins_local_yank_timer_and_drains ... ok
2026-11-12T12:38:21.7985778Z test cohort::declared_capable_cohort_worker_prerequisite_never_green_skips ... ok
2026-11-12T12:38:21.7986316Z test every_audit_sender_release_joins_the_yank_owner ... ok
2026-11-12T12:38:21.7986852Z test harness_owns_grandchildren_before_discovery_even_when_startup_panics ... ok
2026-11-12T12:38:21.7987458Z test cohort::ac4_v5_cohort_sigterm_with_live_worker_unloads_and_drains ... ok
2026-11-12T12:38:21.7988283Z test cohort::ac4_v5c_cohort_shutdown_stops_worker_container_descendants ... ok
2026-11-12T12:38:21.7988900Z test ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker ... FAILED
```
### panic + root stderr tail (selected lines)
```
2026-11-12T12:38:21.7990695Z thread 'ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker' (38457) panicked at crates/maos-bin/tests/root_shutdown_unload_16_3.rs:1436:5:
2026-11-12T12:38:21.7991913Z the interrupted line must replace `complete — exiting cleanly`: maos: crypto provider = ring 0.17.14
2026-11-12T12:38:21.8001140Z maos: Transparency Log opened on-disk at /tmp/maos-root-shutdown-16-3-v3b-38372-1794487083439194083/home/audit/transparency.sqlite
2026-11-12T12:38:21.8011539Z maos: operator HTTP listening on 127.0.0.1:38859
2026-11-12T12:38:21.8014062Z maos: root supervision armed
2026-11-12T12:38:21.8015611Z maos run: cli_wrapper cap-token mediation not granted (kernel policy denied capability issuance); proceeding under AC5 host-grant authority (operator policy `proc.exec` grant is Epic 9 surface). The CapabilityInvocation exit row is still journaled.
2026-11-12T12:38:21.8017276Z maos: T3 container 18b207d5d06e23b27d2960a5634c24f01640d3b33572c28bd7bedc47c56850ab CID kill failed; later cleanup outcome is reported separately
2026-11-12T12:38:21.8018747Z maos: worker worker stopped but its output is still held open by a descendant; exiting without the door shutdown and the audit-channel drain (process-tree teardown: 17-1)
2026-11-12T12:38:21.8019432Z 
2026-11-12T12:38:21.8019436Z 
2026-11-12T12:38:21.8019440Z 
2026-11-12T12:38:21.8020169Z 
2026-11-12T12:38:21.8020860Z 
2026-11-12T12:38:21.8020863Z 
```

## run43155033280-job130972887829-workspace-test-suite (5fe97eda)

### runner lines before the panic
```
2026-11-12T05:50:48.3733085Z         PASS [   1.586s] (1292/6073) maos-bin::root_shutdown_unload_16_3 ac3_operator_door_unload_stops_a_live_worker_once
2026-11-12T05:50:48.4582404Z         PASS [   0.083s] (1293/6073) maos-bin::root_shutdown_unload_16_3 ac4_v8_one_shot_acp_server_has_no_listener_and_dies_by_signal_15
2026-11-12T05:50:48.5167484Z         PASS [   1.568s] (1294/6073) maos-bin::root_shutdown_unload_16_3 ac4_v4_sigterm_during_standalone_worker_leaves_no_orphan
2026-11-12T05:50:48.5658854Z         PASS [   0.049s] (1295/6073) maos-bin::root_shutdown_unload_16_3 cohort::ac4_v5b_sigterm_during_daemon_startup_exits_through_its_teardown
2026-11-12T05:50:48.9265370Z         PASS [   1.595s] (1296/6073) maos-bin::root_shutdown_unload_16_3 ac4_v4b_grandchild_is_stopped_with_its_worker_container
2026-11-12T05:50:49.0597580Z         PASS [   0.133s] (1297/6073) maos-bin::root_shutdown_unload_16_3 cohort::cohort_sigterm_without_worker_joins_local_yank_timer_and_drains
2026-11-12T05:50:49.0703650Z         PASS [   0.011s] (1298/6073) maos-bin::root_shutdown_unload_16_3 cohort::declared_capable_cohort_worker_prerequisite_never_green_skips
2026-11-12T05:50:49.4345889Z         PASS [   0.364s] (1299/6073) maos-bin::root_shutdown_unload_16_3 every_audit_sender_release_joins_the_yank_owner
2026-11-12T05:50:49.4717857Z         PASS [   0.036s] (1300/6073) maos-bin::root_shutdown_unload_16_3 harness_owns_grandchildren_before_discovery_even_when_startup_panics
2026-11-12T05:50:50.0905596Z         PASS [   1.634s] (1310/6073) maos-bin::root_shutdown_unload_16_3 cohort::ac4_v5_cohort_sigterm_with_live_worker_unloads_and_drains
2026-11-12T05:50:50.2187159Z         PASS [   1.653s] (1315/6073) maos-bin::root_shutdown_unload_16_3 cohort::ac4_v5c_cohort_shutdown_stops_worker_container_descendants
2026-11-12T05:51:03.3785420Z         FAIL [  16.433s] (1458/6073) maos-bin::root_shutdown_unload_16_3 ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker
2026-11-12T05:51:03.3786932Z     test ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker ... FAILED
2026-11-12T05:51:03.3788461Z     test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 14 filtered out; finished in 16.43s
```
### panic + root stderr tail (selected lines)
```
2026-11-12T05:51:03.3789571Z     thread 'ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker' (66789) panicked at crates/maos-bin/tests/root_shutdown_unload_16_3.rs:1436:5:
2026-11-12T05:51:03.3790395Z     the interrupted line must replace `complete — exiting cleanly`: maos: crypto provider = ring 0.17.14
2026-11-12T05:51:03.3833325Z     maos: Transparency Log opened on-disk at /tmp/maos-root-shutdown-16-3-v3b-66788-1794462646948525965/home/audit/transparency.sqlite
2026-11-12T05:51:03.3840050Z     maos: operator HTTP listening on 127.0.0.1:33977
2026-11-12T05:51:03.3841880Z     maos: root supervision armed
2026-11-12T05:51:03.3842942Z     maos run: cli_wrapper cap-token mediation not granted (kernel policy denied capability issuance); proceeding under AC5 host-grant authority (operator policy `proc.exec` grant is Epic 9 surface). The CapabilityInvocation exit row is still journaled.
2026-11-12T05:51:03.3845800Z     maos: T3 container 41ff6cad9b807c5fdde821d63e15ac200968dfd0971125c0b510477d0b9c968b CID kill failed; later cleanup outcome is reported separately
2026-11-12T05:51:03.3846695Z     maos: worker worker stopped but its output is still held open by a descendant; exiting without the door shutdown and the audit-channel drain (process-tree teardown: 17-1)
2026-11-12T05:51:03.3847388Z 
2026-11-12T05:51:03.3847833Z 
```
