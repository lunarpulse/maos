# Scoped live-root / purge-layout scan

- `root_crash.rs`: the deliberately live foreign root was the only success-expected purge twin sharing target ancestors; it is moved to a separate TempDir. First/second target roots intentionally reuse the target home sequentially and are killed/reaped before purge. Foreign CID and label-less impostor survival assertions remain unchanged.
- `purge_residue_20_10.rs`: the only real root spawn is `ac2_live_root_refuses_before_runtime_enumeration` at lines321-370. It intentionally targets the same store and requires refusal69, no runtime call and no receipt. Other-home rows are stub inspect metadata/custody directories, not another live root; no fixture relocation is needed.
- `root_shutdown_unload_16_3.rs`: case-insensitive search for purge/foreign/second-home patterns found no purge invocation or live-foreign purge twin. No change needed.
- Workflow lines172/174/175 still require the specific intended diagnostics; lines183-189 still copy/compare/touch and demand restored green. No expected-red allowlist or production lock policy changed.
