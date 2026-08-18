pub mod abi_diff;
pub mod check_fkcs;
pub mod check_host_surface;
pub mod check_kernel_baseline;
pub mod check_mock_not_in_release;
pub mod check_trial_attestation;
// corpus_types + gate_common exposed so the lib-compiled gate modules
// (check_fkcs, check_trial_attestation) can resolve `crate::gate_common`
// (Option C leg-binding, Epic 12 retro B1). corpus_types is self-contained;
// gate_common depends only on it + chrono + sprint_status.
pub mod corpus_types;
pub mod gate_common;
// D19 (14-0, option (a)) — `gate_common::governed_story_keys` derives the governed
// story set from `development_status`, so the single-sourced sprint-status parser
// must resolve in the lib compilation too, not just in `main.rs`. Re-parsing it
// inside `gate_common` would create the second source of truth that D19 exists to
// remove.
pub mod sprint_status;
