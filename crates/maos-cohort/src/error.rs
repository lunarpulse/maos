#![forbid(unsafe_code)]
//! Typed errors for cohort-manifest parsing, validation, and signature
//! verification (Story 12.1 / Task 1).
//!
//! Three named refinements from the party-mode preflight carry their OWN
//! discriminant so a gate can assert the *specific* cause (R1/R2/RR6):
//!
//! - [`CohortError::ECohortAuthorityUnpinned`] (R1 + RR5) — the manifest's
//!   declared authority is not a subset of the member's operator-pinned key
//!   set. Closes the genesis circularity (a forged v1 cannot self-declare +
//!   self-sign its own authority).
//! - [`CohortError::EUnsupportedAuthorityScheme`] (R2) — `threshold > 1` is
//!   rejected at v1. The `threshold` field is kept for forward-compat but real
//!   m-of-n verification is a follow-up with its own proven-red — never
//!   accept-and-single-verify.
//! - [`CohortError::ECohortStaleBoundViolation`] (RR6) — `t_stale_secs` is
//!   outside the code-constant `[T_STALE_MIN, T_STALE_MAX]` bounds. The signed
//!   field tunes *within* code-owned bounds; it never *defines* them (else the
//!   authority signs away its own fail-closed staleness — e.g. 1yr).
//!
//! [`CohortError::ECohortManifestFork`] (R3, re-issue discipline) lands with
//! Task 3; it is intentionally absent from this Task 1 surface.

#![forbid(unsafe_code)]

/// Errors raised by the cohort-manifest parse/validate/sign/verify path.
///
/// Each structural invariant maps to its OWN variant so a failing test (and a
/// future gate leg) asserts the exact rejection cause — never a catch-all
/// "some validation error" (the 11.2a vacuous-green / 11.4b tautological-
/// falsifier family).
/// The exact local cause of a verified cohort-manifest reissue refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CohortManifestForkReason {
    NonAuthoritySigner,
    VersionRegression,
    ConcurrentFork,
    SchemaDowngrade,
}

impl std::fmt::Display for CohortManifestForkReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::NonAuthoritySigner => "non_authority_signer",
            Self::VersionRegression => "version_regression",
            Self::ConcurrentFork => "concurrent_fork",
            Self::SchemaDowngrade => "schema_downgrade",
        };
        formatter.write_str(value)
    }
}

/// The precise structural reason an operator-declared migration candidate set
/// cannot be walked as a linear chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationChainNotLinearReason {
    ForkAtSource,
    Cycle,
    SelfLoop,
}

impl std::fmt::Display for MigrationChainNotLinearReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::ForkAtSource => "fork_at_source",
            Self::Cycle => "cycle",
            Self::SelfLoop => "self_loop",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CohortError {
    /// TOML failed to deserialize, or an unknown field appeared
    /// (`deny_unknown_fields` — operator typos are rejected, not silently
    /// accepted, per the `A2APeerConfig` precedent).
    #[error("cohort manifest TOML parse error: {0}")]
    ParseError(String),

    /// `schema_version` is outside the explicitly supported reader set.
    #[error("unsupported cohort manifest schema_version: got {got}, supported {supported:?}")]
    EUnsupportedSchemaVersion { got: u64, supported: Vec<u64> },

    /// `version` is not a strictly-positive integer (≤ 0). Strict monotonicity
    /// across re-issues is enforced by the re-issue discipline (Task 3); at
    /// parse every manifest must carry `version ≥ 1`.
    #[error("cohort manifest version must be strictly positive (>= 1), got {version}")]
    EVersionNotPositive { version: u64 },

    /// `authority.keys` is empty — a cohort must declare at least one genesis
    /// authority key.
    #[error("cohort manifest authority key set is empty")]
    EEmptyAuthority,

    /// (R2) `authority.threshold > 1` — v1 is single-authority; m-of-n is a
    /// forward-compat field, never accept-and-single-verify.
    #[error("unsupported authority scheme: threshold={threshold} (v1 requires threshold <= 1; m-of-n is a follow-up)")]
    EUnsupportedAuthorityScheme { threshold: u64 },

    /// (R1 + RR5) the manifest's declared authority key set is not a subset of
    /// the member's operator-pinned genesis key set. `unpinned` lists the
    /// declared keys that have no pinned counterpart (so the gate names the
    /// exact offender, not "some authority mismatch").
    #[error("declared cohort authority is not operator-pinned: {unpinned_count} declared key(s) have no pinned counterpart")]
    ECohortAuthorityUnpinned {
        /// Hex of the declared keys that are absent from the pinned set.
        unpinned: Vec<String>,
        /// Convenience count (== `unpinned.len()`).
        unpinned_count: usize,
    },

    /// (RR6) `t_stale_secs` is outside the code-owned `[T_STALE_MIN,
    /// T_STALE_MAX]` bounds. The signed field tunes within bounds; it never
    /// defines them.
    #[error("t_stale_secs={value} is outside the code-owned bounds [{min}, {max}]")]
    ECohortStaleBoundViolation { value: u64, min: u64, max: u64 },

    /// A declared authority key is not a valid 32-byte Ed25519 verifying key
    /// (bad hex length or rejected by `VerifyingKey::from_bytes`).
    #[error("invalid authority key (expected 32-byte Ed25519 verifying key): {0}")]
    EInvalidAuthorityKey(String),

    /// The cohort declares no members.
    #[error("cohort manifest declares no members")]
    EEmptyMembers,

    /// A `host_id` appears more than once across `members`.
    #[error("duplicate cohort member host_id: {host_id}")]
    EDuplicateHostId { host_id: String },

    #[error(
        "cohort schema/team-map mismatch: schema_version={schema_version}, teams={teams_state}"
    )]
    ECohortSchemaTeamsMismatch {
        schema_version: u64,
        teams_state: &'static str,
    },

    #[error(
        "cross-team consent is only valid in schema v3 or newer, got schema v{schema_version}"
    )]
    ECohortSchemaCrossTeamConsentMismatch { schema_version: u64 },

    /// Story 13.6a — a pre-v4 manifest carried a per-member `team` declaration.
    /// Refused rather than ignored: an ignored declaration is an UNSIGNED
    /// declaration on the shared canonical pre-image.
    #[error(
        "per-member team declaration is only valid in schema v4, got schema \
         v{schema_version} for member {host_id}"
    )]
    ECohortSchemaMemberTeamMismatch {
        schema_version: u64,
        host_id: String,
    },

    /// Story 13.6a — a member declares a team the same signed body never
    /// declares as a [`crate::manifest::TeamEntry`].
    #[error("member {host_id} declares undeclared team {team_id}")]
    ECohortMemberTeamUnknown { host_id: String, team_id: String },

    #[error("cross-team consent references undeclared source team {team_id}")]
    ECrossTeamConsentFromTeamUnknown { team_id: String },

    #[error("cross-team consent references undeclared destination team {team_id}")]
    ECrossTeamConsentToTeamUnknown { team_id: String },

    #[error("cross-team consent cannot grant a self-crossing for team {team_id}")]
    ECrossTeamConsentSelfGrant { team_id: String },

    #[error("cross-team consent intent is not canonical: {intent}")]
    ECrossTeamConsentIntentNotCanonical { intent: String },

    #[error("duplicate cross-team consent grant: {from_team}->{to_team}, intent={intent}")]
    EDuplicateCrossTeamConsent {
        from_team: String,
        to_team: String,
        intent: String,
    },

    #[error("duplicate team id in cohort manifest: {team_id}")]
    EDuplicateTeamId { team_id: String },

    #[error("duplicate team datname in cohort manifest: {datname}")]
    EDuplicateTeamDatname { datname: String },

    #[error("team {team_id} has no Spirit members")]
    EEmptyTeamMembers { team_id: String },

    #[error("team {team_id} region is not canonical: {region}")]
    ETeamRegionNotCanonical { team_id: String, region: String },

    #[error("team {team_id} Postgres datname is invalid: {datname}")]
    ETeamDatnameInvalid { team_id: String, datname: String },

    #[error("Spirit {spirit_id} belongs to multiple teams: {first_team} and {second_team}")]
    ESpiritInMultipleTeams {
        spirit_id: String,
        first_team: String,
        second_team: String,
    },

    /// (Task 2) [`CohortManifest::peer_configs_for`] was asked for the peer
    /// edges of a `self_host` that is not a declared cohort member. The mesh is
    /// full-pairwise over DECLARED members only — an unknown host has no
    /// position in it, so the projection refuses rather than emitting a partial
    /// or empty edge set that would silently masquerade as a connected host.
    #[error("self host {host_id} is not a declared cohort member")]
    EHostNotMember {
        /// The unknown host id passed to `peer_configs_for`.
        host_id: String,
    },

    /// A member `fingerprint` is not the §7.2 TOFU pin wire form
    /// (`sha256:<hex64>`), as parsed by `PeerCertFingerprint::parse`.
    #[error(
        "member {host_id} fingerprint is not the §7.2 pin form (sha256:<hex64>): {fingerprint}"
    )]
    EInvalidFingerprint {
        host_id: String,
        fingerprint: String,
    },

    /// A consent tuple names a peer that is not a declared member (RR-min
    /// referential integrity). `direction` is `"send"` or `"accept"`.
    #[error("consent.{direction} tuple references non-member peer {peer}")]
    EConsentPeerNotMember { direction: String, peer: String },

    /// A consent tuple names a role no member declares (RR-min referential
    /// integrity).
    #[error("consent.{direction} tuple references undeclared role {role}")]
    EConsentRoleUndeclared { direction: String, role: String },

    #[error("cohort frame is missing its acting role")]
    EConsentActingRoleAbsent,

    #[error("cohort frame is missing its sender manifest version")]
    EConsentManifestVersionAbsent,

    #[error("peer {peer} is not entitled to acting role {role}")]
    EConsentRoleNotEntitled { peer: String, role: String },

    #[error(
        "consent.{direction} has no exact grant for peer={peer}, role={role:?}, intent={intent}"
    )]
    EConsentTupleDenied {
        direction: String,
        peer: String,
        role: Option<String>,
        intent: String,
    },

    #[error("multiple acting roles are granted for peer={peer}, intent={intent}")]
    EConsentActingRoleAmbiguous { peer: String, intent: String },

    #[error(
        "cohort manifest skew: sender_version={sender_version}, receiver_version={receiver_version}, delta={delta}"
    )]
    ECohortManifestSkew {
        sender_version: u64,
        receiver_version: u64,
        delta: u64,
    },

    /// An intent (in a consent tuple or the reserved set) is not in canonical
    /// fine-grained form — it could never match a frame's consent key
    /// (`A2AIntent::is_canonical`), so it is rejected fail-closed at parse.
    #[error("intent is not canonical (A2AIntent::is_canonical): {intent}")]
    EIntentNotCanonical { intent: String },

    /// A schema-mandatory reserved intent is missing. The v1 cohort manifest
    /// MUST reserve both `cohort:manifest-reissue` and `cohort:halt-receipt`
    /// (always-allowlisted transport-admission intents).
    #[error("missing reserved intent (v1 requires cohort:manifest-reissue + cohort:halt-receipt): {intent}")]
    EMissingReservedIntent { intent: String },

    /// The `signature.sig` field is not a 64-byte Ed25519 signature
    /// (bad hex length).
    #[error("manifest signature is not a 64-byte Ed25519 signature: {0}")]
    EInvalidSignature(String),

    /// The signature failed cryptographic verification under every pinned
    /// genesis authority key. Per AC3 the signature is verified against the
    /// operator-pinned key set — NEVER a key carried in the manifest body, and
    /// NEVER re-derived from the manifest's own declaration.
    #[error("manifest signature did not verify under any pinned genesis authority key")]
    ESignatureVerificationFailed,

    /// (R3) A reissue does not continue the locally verified manifest lineage.
    /// The discriminant and both versions let callers distinguish an unpinned
    /// signer, a lower valid version, and a concurrent same-version fork.
    #[error(
        "cohort manifest fork ({reason}): seen_version={seen_version}, rejected_version={rejected_version}"
    )]
    ECohortManifestFork {
        reason: CohortManifestForkReason,
        seen_version: u64,
        rejected_version: u64,
    },

    /// A candidate reissue is signed by a permitted authority but names a
    /// different cohort. Authority-key reuse must not permit cross-cohort
    /// manifest replay.
    #[error(
        "cohort reissue changed cohort_id: expected {expected_cohort_id}, got {rejected_cohort_id} (seen_version={seen_version}, rejected_version={rejected_version})"
    )]
    ECohortIdMismatch {
        expected_cohort_id: String,
        rejected_cohort_id: String,
        seen_version: u64,
        rejected_version: u64,
    },

    /// A mandatory cohort audit append failed. State changes and rejection
    /// responses fail closed rather than becoming unauditable.
    #[error("cohort audit append failed: {0}")]
    EAuditAppendFailed(String),

    /// An internal manifest state lock was poisoned; currentness fails closed.
    #[error("cohort manifest state lock poisoned")]
    EStatePoisoned,

    #[error("invalid cohort digest-read request: {0}")]
    EInvalidDigestRequest(String),

    #[error("cohort digest-read capacity exceeded: {0}")]
    EDigestCapacityExceeded(String),

    #[error("cohort manifest control envelope has an invalid intent, frame kind, or event type")]
    EControlEnvelopeInvalid,

    #[error("cohort manifest control envelope could not decode: {0}")]
    EControlEnvelopeDecode(String),

    /// The operator-supplied candidate set has more than one outgoing
    /// migration, a cycle, or a self-loop for the named concrete source.
    #[error(
        "migration candidate chain is not linear ({reason}) at source version {source_version}"
    )]
    ECohortMigrationChainNotLinear {
        reason: MigrationChainNotLinearReason,
        source_version: String,
    },

    /// The candidate set is structurally valid but contains no route from the
    /// requested predecessor version to the requested successor version.
    #[error("no migration path from version {from} to version {to}")]
    ECohortNoMigrationPath { from: String, to: String },

    /// A persisted approved plan does not match the chain re-derived from the
    /// current candidate manifests. This guards trusted operator input against
    /// accidental or third-party drift; it is not an attestation mechanism.
    #[error("migration plan drift: approved hash {expected_plan_hash}, live chain hash {actual_chain_hash}")]
    EMigrationPlanDrift {
        expected_plan_hash: String,
        actual_chain_hash: String,
    },
    #[error("cohort manifest distribution failed: {0}")]
    EDistributionFailed(String),
}
