# 17-4 T9 — proven red (AC4.7, rule 11(c)), 2026-10-20

Every plant is a throwaway patch on production code (`crates/maos-bin/src/hook_pump.rs`), never a knob. Each was applied to the dev tree, the named suite was run, the patch was reverted byte-for-byte (`cmp` against the pre-plant copy), and the suites were run green again. Logs are trimmed of compiler warnings from unrelated crates.

| # | Plant (patch) | Command | Red assertion (first to fail) | Log |
|---|---|---|---|---|
| 1 | Delivery skips every 25th event, uncounted (`plant-1-skip-every-25th.patch`) | `cargo test --locked -p maos-bin --test observer_activity_corpus_17_4 observer_activity_corpus_production_path -- --nocapture --test-threads=1` | `oracle: silent-loss missed != lagged_skipped` (`observer_activity_corpus_17_4.rs:383`, oa-01) | `plant-1-red.log` |
| 1b | Same skip, booked as lag so the counters agree (`plant-1b-skip-every-25th-booked-as-lag.patch`) | same | `oracle: missed-rate 0.03986013986013986 > 0.02` (`:371`; aggregate 5720 emitted / 5492 received) | `plant-1b-red.log` |
| 2 | Delivery task swaps adjacent events, deterministic (`plant-2-swap-adjacent.patch`) | same | `oracle: causal-correctness 0.5938104448742747 < 0.99` (`:376`; 3377/5687 pairs) | `plant-2-red.log` |
| 3 | Final event delivered twice at the flush barrier (`plant-3-final-event-twice.patch`) | same | `oracle: duplicate seq=179` (`:334`; seq 179 is oa-01's last event) | `plant-3-red.log` |
| 4 | One event dropped, not counted (`plant-4-silent-drop.patch`) — `flush()` still acknowledges because it never reads totals | same | `oracle: silent-loss missed != lagged_skipped` left 1, right 0 (`:383`) — not a harness timeout | `plant-4-red.log` |
| 5 | Subscription grant row not written (`plant-5-no-grant-row.patch`) | `cargo test --locked -p maos-bin --test observer_live_17_4 observer_serves_and_reports_a_door_loaded_butler_live -- --nocapture --test-threads=1` | AC1.4 `no grant row at pid 1` (`observer_live_17_4.rs:320`) | `plant-5-red.log` |

Plant 1 is the story's literal "skips every 25th event": it violates both floors, and the per-scenario silent-loss check fires first. Plant 1b isolates the missed-rate gate by making the skipped events visible as lag, so the 2 % floor is shown red on its own.

Green after revert: `cargo test --locked -p maos-bin --test observer_live_17_4 --test observer_activity_corpus_17_4 -- --nocapture --test-threads=1` → 16 passed + 3 passed, aggregate 5720/5720, missed 0.00000000, causal 5687/5687 (`green-after-plants.log`).
