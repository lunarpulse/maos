# Hosted follow-up: run 42224167600

Implementation baseline: `fd4f60f15631ba09464b51024bce2c60236b8245` (Main's cargo handoff). No kernel landing, tracker transition, commit or push is part of this correction.

## Observed hosted inputs

- Current native counts: run [42224167600](https://github.com/lunarpulse/maos/actions/runs/42224167600), commit `ebf2ab03ac64de5b828d995f72ae8b24199fa9a0`, artifact `13504529758` (`tier-c-counts`).
- Reference: run [41910498869](https://github.com/lunarpulse/maos/actions/runs/41910498869), commit `cb0abc06e6b085cb63db1be60d4addbeb2963811`, artifact `13361299405` (`tier-c-counts`).
- The compression-aware parser's sum of exclusive Ir equals each complete callgrind summary exactly. Both use 2000 iterations and an empty-loop subtraction of 36 instructions.
- Reference: 997139.499 instructions/iteration. Current: 1038630.1585. Difference: +41490.6595. Against the unchanged pinned997449.662 baseline, current is +4.1285789%, exceeding the unchanged2% ceiling1017398.65524 by21231.50326.
- Inclusive per-iteration deltas: SQLite prepare +30924.7995; step +4826.067; finalize +1186.442; unchanged-source redaction +4479.294. Constraint-check code generation alone grows +27749.5455. The FairMutex is not the regression: one inner acquisition per append, new lock/drop187 versus old lock/expect/drop228 instructions. No local callgrind run was performed; `callgrind_annotate` is unavailable (`command -v`, exit1).
- Workspace JUnit artifact `13507273876`: 1M repeat assertion failed at old `shell_host_16_2.rs:1704` after the first operation genuinely finished. First1M wall15.54s/max append30.2ms/ratio0.0019 is hosted evidence of the first completion/wall leg, not a green repeat test. The100k test costs5.646s and1M56.675s; combined62.321s, ×1.25=77.90125, rounded finite78s per the test-budget calibration rule. The other12 shell tests sum3.919s.
- Hosted Apple check fails on `rustix::fs::mkfifoat` at old `erasure_inventory_20_11.rs:653`; locked rustix source excludes both mkfifoat and mknodat on Apple. The target `aarch64-apple-darwin` is installed locally. The safe POSIX mkfifo subprocess follows the existing resolver fixture pattern and keeps the test's forbid(unsafe_code).

## Independently observed local before witnesses

- `replay-million-before`: exit100,82.923s test, existing unchanged first pending-completion observation at old line1675 fails before the repeat assertion is reached. It is not claimed as a local reproduction of the hosted repeat assertion; no retry-to-green or deadline change.
- `replay-small-before`: exit100; `ac5_completed_forget_is_durable_marker_scoped_replay` old line1847 observes two real principal.forget receipts instead of one after rebuilding the door. This directly demonstrates a repeated cascade rather than a durable replay.
- `cached-insert-before`: exit100; `review_cached_frame_insert_reuses_the_indexed_program` line677 observes SQLITE_STMTSTATUS_RUN=0 instead of2 on the exact production SQL, demonstrating re-preparation rather than cached reuse.
- Pre-transfer nextest list: exit0, captured before relocating the original test functions. The two heavy test bodies and names move unchanged; only common fixture visibility changes for shared use.

## Measured source envelope

Handoff pins: bin34914, IAC8622, audit8174, registry4412, xtask44676, aggregate194743/hardfail194744. Formatted measurement after the correction: bin35082(+168), IAC8627(+5), audit8174, registry4412, xtask44676, aggregate194916(+173), hardfail194917. Ceilings remain bin35704/IAC8630/audit8261/registry4430/aggregate195404. Kernel tokei21042 and domain10107 are unchanged in the measured gate output; the physical kernel baseline is checked separately.

IAC src8627/tests3601 versus preceding8622/3577: the extra24 test lines are the real cache-reuse witness. The previously approved504-code-line unit-body accounting transfer is unchanged, not a new ceiling relief. No-transfer IAC source would be9131(+580 from the original8551 baseline); counterfactual aggregate195420 is16 above the current aggregate ceiling, disclosed rather than presented as measured production. Actual counted aggregate194916 remains within the unchanged ceiling. No further unit-test relocation or grant is introduced.

The two paragraphs above are the preserved first formatted checkpoint, not the final pins. After the genuine composite-DESC repair, streamed durable receipt serialization, borrowed intent validation and paired allocation correction, the final actual tokei measurement is bin35102/IAC8625/audit8174/registry4412/xtask44676, aggregate194934/hardfail194935. `source-accounting-final.json` and `source-accounting-command.json` preserve that observed command and the current src/tests accounts: bin src35102/tests64146, IAC src8625/tests3640. The original504-code unit transfer is unchanged; no-transfer IAC9129 and aggregate195438 remain counterfactual, not the measured pins or a new ceiling grant. Current handoff deltas are bin+188/IAC+3/aggregate+191; all actual ceilings are unchanged.
