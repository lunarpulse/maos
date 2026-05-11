#![no_std]

/// ABI version constant for the MAOS Spirit ABI.
/// Bumped according to the ABI Stability Triple rules.
pub const ABI_VERSION: u32 = 0;

/// Placeholder ABI version type.
/// Will be expanded in Story 1a.1 with the full ABI surface.
pub struct AbiVersion;

/// Compliance module stub — to be fleshed out in Story 1a.1.
pub mod compliance {
    /// Placeholder for the ABI version triple component.
    pub const ABI_VERSION: u32 = super::ABI_VERSION;
}
