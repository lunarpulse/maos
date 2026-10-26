//! Typed error taxonomy for the skill ecosystem (Story 7.4).
//!
//! E-prefix `thiserror` variants matching the `CliWrapperAdmissionError`
//! convention. The `maos.skill.v1` schema-validation precision requirement
//! makes silent defaults a CORRECTNESS bug: an unknown frontmatter field is
//! `ESkillSchema::UnknownField`, NEVER a coerced default.

/// Errors raised while parsing / validating a `maos.skill.v1` document.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ESkillSchema {
    /// The document is missing the `---` … `---` TOML frontmatter fence
    /// (the Anthropic-close convention per ADR-027).
    #[error("missing frontmatter fence: a maos.skill.v1 document must open with `---` and close the frontmatter with `---`")]
    MissingFence,

    /// An unknown field appeared in the TOML frontmatter. `#[serde(deny_unknown_fields)]`
    /// rejects it — the kernel does NOT coerce a silent default.
    #[error("unknown frontmatter field (deny_unknown_fields): {0}")]
    UnknownField(String),

    /// The TOML frontmatter failed to parse (malformed syntax, missing required
    /// field, or wrong value type).
    #[error("malformed TOML frontmatter: {0}")]
    TomlParse(String),

    /// `id` was empty.
    #[error("skill `id` must be non-empty")]
    EmptyId,

    /// `id` contained characters outside the kebab/dotted charset
    /// (`a`–`z`, `0`–`9`, `-`, `.`).
    #[error("skill `id` `{0}` contains invalid characters (allowed: a-z, 0-9, '-', '.')")]
    InvalidIdCharset(String),

    /// `name` was empty.
    #[error("skill `name` must be non-empty")]
    EmptyName,

    /// `version` was not a valid semver string.
    #[error("skill `version` `{0}` is not valid semver: {1}")]
    InvalidSemver(String, String),

    /// The markdown body was empty. A skill with no instructions carries no
    /// meaning; the kernel does NOT interpret the body (§4.0.7) but requires
    /// it be present.
    #[error("skill body must be present (non-empty markdown after the frontmatter fence)")]
    EmptyBody,
}

/// Errors raised while building an FR57 skill-revision proposal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ESkillProposal {
    /// `target_skill_id` was empty.
    #[error("revision proposal `target_skill_id` must be non-empty")]
    EmptyTargetId,

    /// `target_skill_id` contains invalid characters (allowed: a-z, 0-9, '-', '.').
    #[error("revision proposal `target_skill_id` `{0}` contains invalid characters (allowed: a-z, 0-9, '-', '.')")]
    InvalidTargetIdCharset(String),

    /// `target_version` was not a valid semver string.
    #[error("revision proposal `target_version` `{0}` is not valid semver: {1}")]
    InvalidTargetVersion(String, String),

    /// `proposed_diff` was empty. FR57 mandates the proposed diff as a
    /// load-bearing payload field; an empty diff carries no proposal.
    #[error("revision proposal `proposed_diff` must be non-empty")]
    EmptyDiff,
}

/// Errors raised by the operator-admission queue (FR39).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ESkillQueue {
    /// A Pending entry with this SkillId already exists in the queue.
    #[error("duplicate skill id `{0}`: a Pending entry with this id already exists in the admission queue")]
    DuplicateSkillId(String),
    #[error("authored skill id is outside the root-resolved class namespace")]
    ForeignNamespace,
    #[error("revision target is not an admitted skill authored by this class")]
    TargetNotOwned,
    #[error("target version is stale or authored version does not advance")]
    StaleTargetVersion,
    #[error("authored skill id must not exceed 128 bytes")]
    AuthoredIdTooLong,
    #[error("skill requires an action absent from the author's declared scopes")]
    CapabilityEscalation,
    #[error("payload contains an indexed principal id")]
    PrincipalDataRefused,
    #[error("telemetry evidence pid differs from the root-fixed author pid")]
    EvidenceMismatch,
    #[error("telemetry evidence contains a non-finite value or threshold")]
    NonFiniteEvidence,
    #[error("unknown stored entry path `{0}`")]
    UnknownEntryPath(String),
    #[error("invalid stored entry `{0}`")]
    InvalidStoredEntry(String),
    #[error("invalid skill document: {0}")]
    Schema(#[from] ESkillSchema),
    #[error("invalid revision proposal: {0}")]
    Proposal(#[from] ESkillProposal),
    #[error("skill queue entry was not found")]
    EntryNotFound,
    #[error("skill id identifies multiple entries; use an entry id")]
    AmbiguousEntry,
    #[error("skill queue entry already has a terminal decision")]
    AlreadyDecided,
}

impl ESkillQueue {
    /// Stable, non-personal refusal code suitable for an audit receipt.
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateSkillId(_) => "duplicate-skill-id",
            Self::ForeignNamespace => "foreign-namespace",
            Self::TargetNotOwned => "target-not-owned",
            Self::StaleTargetVersion => "stale-target-version",
            Self::AuthoredIdTooLong => "authored-id-too-long",
            Self::CapabilityEscalation => "capability-escalation",
            Self::PrincipalDataRefused => "principal-data-refused",
            Self::EvidenceMismatch => "evidence-mismatch",
            Self::NonFiniteEvidence => "non-finite-evidence",
            Self::UnknownEntryPath(_) => "unknown-entry-path",
            Self::InvalidStoredEntry(_) => "invalid-stored-entry",
            Self::Schema(_) => "skill-schema",
            Self::Proposal(_) => "skill-proposal",
            Self::EntryNotFound => "entry-not-found",
            Self::AmbiguousEntry => "ambiguous-entry",
            Self::AlreadyDecided => "already-decided",
        }
    }
}
