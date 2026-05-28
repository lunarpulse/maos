#![forbid(unsafe_code)]

//! MAOS spirit manifest parsing and validation.
//!
//! Extracted from `maos-kernel-core` in Story 6.5 (Epic 6, AC2).
//! All manifest section parsers — `[sandbox]`, `[resources]`, `[class]`,
//! `[capabilities.required]`, `[posture]`, `[output_shape]`, `[budget]`,
//! `[author]`, `[epistemic_policy]`, `[scheduling]`, `[lifecycle]`,
//! `[on_crash]`, `[on_revocation]`, `[schedules]`, `[supervision]`,
//! `[providers]`, `[mcp]` — live here.

pub mod manifest;

// Re-export the most commonly used types at crate root for ergonomic access.
pub use manifest::{
    Author, Budget, CapabilitiesRequired, ClassSection, CliWrapperConfig, CliWrapperControlChannel,
    CliWrapperPosture, CliWrapperRecoveryPolicy, CliWrapperStdioShape, EpistemicAction,
    EpistemicPolicyRule, EpistemicPolicySection, GatewayEntry, GatewayType, GatewaysSection,
    HaltProtocolCompatibilitySection, HotSwapManifestSection, LifecycleSection, ManifestError,
    McpCapabilities, McpCapabilityServerEntry, McpSection, McpServerEntry, MigratesFromSection,
    OnCrashSection, OnInboundHook, OnRevocationSection, OutputShape, OutputShapePredicate,
    OutputShapeViolation, Posture, PostureSection, ProviderCapabilities, ProviderConfig, ProvidersSection,
    ResolvedCaps, ResourceCaps, SandboxConfig, ScalarPredicate, ScheduleEntry, SchedulingSection,
    SchedulesSection, SupervisionSection, capabilities_required_to_scopes, resolve_caps,
};
