#![forbid(unsafe_code)]

//! # maos-skill — the kernel-MEDIATED skill ecosystem (Story 7.4)
//!
//! This crate hosts the skill *mechanics* the substrate has only referenced
//! until now (the `skill_bundle: Vec<String>` persona-reference placeholder in
//! `CliWrapperConfig`): the `maos.skill.v1` format, filesystem discovery, the
//! operator-admission queue (FR39), and FR57 revision proposals.
//!
//! ## Kernel non-interpretability (§4.0.7)
//!
//! `maos-skill` is NOT a skill-content interpreter, ranker, curator, or
//! executor. It validates the `maos.skill.v1` SCHEMA (frontmatter well-formed
//! + body present), discovers skills, and manages the admission queue + audit.
//! The markdown `body` and the FR57 `proposed_diff` are OPAQUE — the kernel
//! does NOT parse them for meaning. Skill EXECUTION (a Spirit consuming a skill
//! at `on_load`) is Spirit-side and out of scope.
//!
//! ## The three FR39 admission entry paths
//!
//! Every skill — however it arrives — lands `Pending` in the same
//! kernel-mediated [`admission::SkillAdmissionQueue`] and requires an explicit
//! operator `approve`. No skill is ever auto-admitted:
//!
//! 1. **package-shipped** — bundled in a Spirit package;
//! 2. **`skill.author.self`** — written dynamically at runtime under the
//!    `Scope::SkillAuthorSelf` capability (which authorizes the write-to-queue
//!    ONLY, never the activation);
//! 3. **FR57 revision proposal** — built from a Spirit's OWN self-telemetry
//!    (the EXISTING Story 4.3 `SelfTelemetryReport`).

pub mod admission;
pub mod anthropic_adapter;
pub mod approval_target;
pub mod discovery;
pub mod errors;
pub mod proposal;
pub mod schema;
pub mod store;

pub use admission::{PendingEntry, SkillAdmissionQueue, SkillAdmissionState, SkillEntryPath};
pub use anthropic_adapter::parse_anthropic_skill;
pub use discovery::{
    default_search_path, discover_skills, discover_skills_detailed, DiscoveredSkill,
    DiscoveryOutcome,
};
pub use errors::{ESkillProposal, ESkillQueue, ESkillSchema};
pub use proposal::{build_proposal, SkillRevisionProposal};
pub use schema::{parse_skill, validate_manifest, Skill, SkillId, SkillManifest, SkillVersion};
pub use store::{ESkillStore, LocalFsSkillQueueStore, QueueEntry, SkillQueueStore};
