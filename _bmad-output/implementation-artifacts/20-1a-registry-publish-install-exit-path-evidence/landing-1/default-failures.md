# Default-profile workspace failures — no retry-to-green

The repaired serial workspace run passed all 5,526 selected tests. The separate
unrestricted default-profile workspace run returned 100: 5,520 passed, five
failed and one timed out; 138 skipped. These are separate execution modes, not a
claim that the default run was green. Each case below passed in the repaired
serial run. No failure was retried to manufacture a default-profile pass.

“Family” labels classify the observed symptom, not an unmeasured shared root
cause. Main directed recording known-family or new, with a same-line rule-10
style disposition rather than expanding this kernel-free registry assignment.

- `maos-bin::one_shot_drain_20_3c::smoke_supervision_5_drains_its_audit_writer` — known audit-drain timing symptom family (20-3c); completed drain took 7.042907398 s against a 5 s assertion under unrestricted concurrency; serial PASS. Rule 10: default-concurrency timing residual remains open; no retry, timeout increase or kernel edit.
- `maos-bin::shell_host_16_2::repl_clarification_resolves_with_no_control_json` — **new** default-concurrency REPL clarification/render failure (no established cause/owner); assertion at `shell_host_16_2.rs:852`; serial PASS. Rule 10: ownerless/open, not absorbed into 20-1a and not retried.
- `maos-journey-test::journey_j0::j0_halt_resolved_in_the_repl` — **new** default-concurrency REPL halt-resolution/render failure (same observed surface as the preceding case, shared cause unproven); assertion at `journey_j0.rs:586`; serial PASS. Rule 10: ownerless/open, not absorbed into 20-1a and not retried.
- `maos-journey-test::journey_skill_authoring_18_7::undeclared_live_researcher_is_refused_without_queue_mutation` — known audit-drain-stall symptom family; `journey_skill_authoring_18_7.rs:82` reports `audit drain stalled`; serial PASS. Rule 10: default-concurrency residual remains open; no retry, suppression or widened drain budget.
- `maos-bin::forget_floors_18_6::fifty_door_cascades_and_one_hundred_all_store_probes` — **new** default-concurrency operator-door route-budget failure; the twenty-second forget returned a still-running operation instead of synchronous success; serial PASS. Rule 10: ownerless/open; no retry, weakened operation assertion or invented connection to the separately recorded HTTP-411 family.
- `maos-bin::observer_activity_corpus_17_4::observer_activity_corpus_production_path` — **new** default-concurrency corpus timeout, 120.760 s; serial PASS. Rule 10: ownerless/open; no retry, timeout increase or corpus narrowing.

Evidence: `workspace-default.log` and `workspace-serial-repaired.log`.
