#![forbid(unsafe_code)]

#[cfg(feature = "network")]
pub mod cross_team_consent;
#[cfg(feature = "network")]
pub mod enterprise_identity;
#[cfg(feature = "network")]
pub mod tenant_map;
