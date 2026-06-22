#![no_main]

//! Fuzz target — `maos-manifest` TOML section parsers (NFR-Sec-5).
//!
//! Feeds the SAME fuzzed UTF-8 string to all 23 manifest `*::from_toml_str`
//! entry points. Every parser is invoked independently; any `Err(ManifestError)`
//! is swallowed (a parse/validation failure is the EXPECTED contract for a
//! malformed/adversarial fragment, never a crash). The target fails only if a
//! parser panics or aborts on attacker-controlled TOML — exactly the class of
//! defect this harness exists to catch.
//!
//! Non-UTF-8 input is returned early (non-crash) — TOML is UTF-8 by definition,
//! so we never feed `from_toml_str` non-`&str` data.

use libfuzzer_sys::fuzz_target;
use maos_manifest::{
    Author, Budget, CapabilitiesRequired, ClassSection, CliWrapperConfig, EpistemicPolicySection,
    GatewaysSection, HaltProtocolCompatibilitySection, HotSwapManifestSection, LifecycleSection,
    McpSection, MigratesFromSection, ModelProvenanceSection, OnCrashSection, OnRevocationSection,
    OutputShape, PostureSection, ProvidersSection, ResourceCaps, SandboxConfig, SchedulesSection,
    SchedulingSection, SupervisionSection,
};

fuzz_target!(|data: &[u8]| {
    // TOML is UTF-8; reject non-UTF-8 bytes before any parser sees them.
    let s = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // All 23 manifest section entry points. Each `let _ =` swallows any
    // `Result<_, ManifestError>` Err — a parse failure is non-crashing by
    // design. A panic here is the only failure mode this harness reports.
    let _ = SandboxConfig::from_toml_str(s);
    let _ = ResourceCaps::from_toml_str(s);
    let _ = ClassSection::from_toml_str(s);
    let _ = CapabilitiesRequired::from_toml_str(s);
    let _ = PostureSection::from_toml_str(s);
    let _ = OutputShape::from_toml_str(s);
    let _ = Budget::from_toml_str(s);
    let _ = Author::from_toml_str(s);
    let _ = EpistemicPolicySection::from_toml_str(s);
    let _ = SchedulingSection::from_toml_str(s);
    let _ = LifecycleSection::from_toml_str(s);
    let _ = OnCrashSection::from_toml_str(s);
    let _ = OnRevocationSection::from_toml_str(s);
    let _ = SchedulesSection::from_toml_str(s);
    let _ = SupervisionSection::from_toml_str(s);
    let _ = ModelProvenanceSection::from_toml_str(s);
    let _ = ProvidersSection::from_toml_str(s);
    let _ = McpSection::from_toml_str(s);
    let _ = HotSwapManifestSection::from_toml_str(s);
    let _ = MigratesFromSection::from_toml_str(s);
    let _ = HaltProtocolCompatibilitySection::from_toml_str(s);
    let _ = CliWrapperConfig::from_toml_str(s);
    let _ = GatewaysSection::from_toml_str(s);
});
