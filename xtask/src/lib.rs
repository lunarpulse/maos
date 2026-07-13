pub mod abi_diff;
pub mod check_fkcs;
pub mod check_host_surface;
pub mod check_kernel_baseline;
pub mod check_mock_not_in_release;
pub mod check_trial_attestation;
// corpus_types + gate_common exposed so the lib-compiled gate modules
// (check_fkcs, check_trial_attestation) can resolve `crate::gate_common`
// (Option C leg-binding, Epic 12 retro B1). corpus_types is self-contained;
// gate_common depends only on it + chrono.
pub mod corpus_types;
pub mod gate_common;
