# T2 — `float_roundtrip` is load-bearing (2026-10-20)

Command: `cargo test --locked -q -p observer --test activity_hooks_17_4`

- Feature removed (`serde_json = "1"` in `spirits/observer/Cargo.toml`): RED —
  `values_round_trip_bit_exact_through_payload_and_rendered_line` panicked at
  `activity_hooks_17_4.rs:207` (`assert_eq!(got.to_bits(), value.to_bits())`):
  left `4632409357216853857`, right `4632409357216853856` (51.248178375505404 decoded 1 ulp off).
  7 passed; 1 failed.
- Feature restored (`features = ["float_roundtrip"]`): GREEN — 8 passed.
