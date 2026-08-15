pub struct EnvVar {
    pub name: &'static str,
    pub purpose: &'static str,
    pub stability: EnvStability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvStability {
    HarnessOnly,
    UserFacing,
}

pub const MAOS_ENV_REGISTRY: &[EnvVar] = &[
    EnvVar {
        name: "MAOS_ONE_SHOT",
        purpose: "Legacy one-shot smoke-arm dispatch key",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_NOTIFY_DISABLE",
        purpose: "Disable mobile-push notification dispatch",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_REGISTRY_URI",
        purpose: "Spirit registry server URI (stub/http(s)://…)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_OLLAMA_URL",
        purpose: "Ollama inference provider base URL",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_CALENDAR_URI",
        purpose: "Butler MCP calendar server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_SLACK_URI",
        purpose: "Butler MCP Slack server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_LINEAR_URI",
        purpose: "Butler MCP Linear server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_FIGMA_URI",
        purpose: "Butler MCP Figma server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_WEB_URI",
        purpose: "Researcher MCP web-search server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_ARXIV_URI",
        purpose: "Researcher MCP arXiv server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_GITHUB_URI",
        purpose: "Researcher MCP GitHub server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MCP_CITATION_GRAPH_URI",
        purpose: "Researcher MCP citation-graph server URI",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_TEST_ONLY_STRIP_SCALAR_PORT",
        purpose: "Test-only: strip EpistemicScalarPort to prove boot-loud (never in production)",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_TEST_BOOT_NONCE",
        purpose: "Debug-build-only deterministic process boot nonce for real multi-daemon integration tests",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_SPIRIT_ID",
        purpose: "Spirit identifier for one-shot lifecycle verbs",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_POSTURE",
        purpose: "Target posture for posture-shift verb",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_FORGET_PRINCIPAL",
        purpose: "Principal identifier for forget verb",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_FORGET_REASON",
        purpose: "Optional human-readable reason for forget verb",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HALT_LIMIT",
        purpose: "Max number of halt events to display",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HALT_SPIRIT",
        purpose: "Spirit filter for halt-list/halt-resolve",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HALT_ID",
        purpose: "Halt event ID for halt-resolve verb",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HALT_KIND",
        purpose: "Resolution kind for halt-resolve (continue/abort/delegate)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HALT_TEXT",
        purpose: "Human text for halt-resolve resolution",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HALT_OPERATOR_POLICY",
        purpose: "Operator halt-policy TOML path",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HOTSWAP_TO_MANIFEST",
        purpose: "Target manifest path for hot-swap upgrade",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HOTSWAP_FROM_VERSION",
        purpose: "Source version for hot-swap upgrade",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_UPGRADE_TO_MANIFEST",
        purpose: "Target manifest path for cold upgrade",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_UPGRADE_POLICY",
        purpose: "Upgrade policy (hot-swap | cold)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_UPGRADE_PLAN",
        purpose: "When set, --plan resolves+hashes+persists the migration chain instead of executing",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_UPGRADE_FROM_VERSION",
        purpose: "Source version for a --plan migration chain resolution",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_UPGRADE_CANDIDATES",
        purpose: "JSON array of candidate manifest paths for a --plan migration chain",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_REVOKE_TOKEN_ID",
        purpose: "Token ID for revocation verb",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_REVOKE_REASON",
        purpose: "Human reason for token revocation",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_ORCHESTRATOR_SPIRIT",
        purpose: "Spirit name receiving orchestrator instruction",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_ORCHESTRATOR_INSTRUCTION",
        purpose: "Instruction text for orchestrator verb",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_BENCH_INVOCATIONS",
        purpose: "Number of invocations for benchmark runner",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_SMOKE_SLOW",
        purpose: "Enable slow-path smoke-arm variants",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_CRL_PATH",
        purpose: "Certificate Revocation List file path",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CRL_TRUST_ANCHOR_PUB_HEX",
        purpose: "CRL trust-anchor public key (hex-encoded)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CRL_FORCE_REAPPLY",
        purpose: "Force CRL reapplication on startup",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_REPLAY_CASSETTE",
        purpose: "Path to cassette file for replay InferencePort (journey-test harness)",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_JOURNEY_MODE",
        purpose: "Journey test mode: 'record' appends live responses to cassette",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_REPLAY_STRICT",
        purpose: "When set to '1', cassette prompt-hash drift is a hard error",
        stability: EnvStability::HarnessOnly,
    },
    EnvVar {
        name: "MAOS_HOME",
        purpose: "MAOS home directory (init'd home, overrides XDG)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_REGISTRY_YANK_POLL_INTERVAL_S",
        purpose: "Yank-poller interval in seconds (0 disables)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_DEPLOYMENT_OPERATOR_ID",
        purpose: "Deployment operator identity for region-pinned tenancy reservation (Story 9.4)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_REQUIRE_MODEL_PROVENANCE",
        purpose: "When set, refuse inference without a valid model provenance attestation (Story 9.4b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_MODEL_PROVENANCE_MAX_AGE_SECS",
        purpose: "Max age in seconds for a model provenance attestation before it is considered stale (Story 9.4b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_GOVERNANCE_SCHEMA_ID",
        purpose: "Governance artifact schema identifier for ratification envelope (Story 9.3/9.3b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_GOVERNANCE_VERSION",
        purpose: "Governance artifact monotonically-increasing version number (Story 9.3/9.3b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_GOVERNANCE_CONTENT_HASH",
        purpose: "SHA-256 content hash of the governance artifact being ratified (Story 9.3/9.3b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_GOVERNANCE_RATIFIED_BY",
        purpose: "Identity that ratified the governance artifact (Story 9.3/9.3b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_GOVERNANCE_EFFECTIVE_AT_NS",
        purpose: "Effective-at timestamp in nanoseconds for the governance artifact (Story 9.3/9.3b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_GOVERNANCE_SUPERSEDES",
        purpose: "Content hash of the governance artifact this one supersedes (Story 9.3/9.3b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_LOOM_POSTGRES",
        purpose: "Collective-tier (Loom-lite) Postgres connection string; absent disables the collective tier (Story 10.4a)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_LOOM_HOME_TEAM",
        purpose: "Canonical tenant team id for the Loom-lite store; requires a refreshable verified schema-v2-or-newer cohort source, and in cohort-a2a-daemon mode MUST equal the signed CohortMember.team for this host or the boot fails (Stories 13.1/13.3/13.6b AC4)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_VETTER_KEYRING",
        purpose: "Path to the CBOR vetter-key lifecycle keyring required to verify a public-vetted target against the operator audit root (Story 13.4)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_LEGAL_HOLD_PRINCIPAL",
        purpose: "Principal identifier whose legal hold the operator one-shot control lists or releases; empty values are refused (Story 13.5b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_COLLECTIVE_ERASE_PID",
        purpose: "Spirit PID whose collective-tier row is erased by the operator one-shot control; must be a u32 (Story 13.5b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_COLLECTIVE_ERASE_NAMESPACE",
        purpose: "Collective namespace for the operator one-shot erase: default, coordination, or forgotten (Story 13.5b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_COLLECTIVE_ERASE_KEY",
        purpose: "Collective row key required by the operator one-shot erase control (Story 13.5b)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CROSS_TEAM_BASE_SEED",
        purpose: "Hex-encoded 32-byte root for cross-team key derivation. Story 13.3 read it only to derive PUBLIC row-verification keys; Story 13.6b widened it to the SIGN side for the crossing emitter, so a host holding it can produce a validly-signed bundle under ANY team's key — the applier's envelope/payload weld, not this seed, is what binds a crossing to its team (Stories 13.3/13.6b D-7)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CROSS_TEAM_SHARE_PEER",
        purpose: "Destination cohort host_id for a boot-time cross-team share emitted by the cohort-a2a-daemon; presence is the emit trigger, absence leaves the daemon byte-for-byte unchanged (Story 13.6b AC1)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CROSS_TEAM_SHARE_TO_TEAM",
        purpose: "Canonical destination team id requested for a boot-time cross-team share; the applier rejects any request that does not name its own home team (Story 13.6b AC1)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CROSS_TEAM_SHARE_PID",
        purpose: "Spirit pid attributed to a boot-time cross-team share row and its Transparency Log event (Story 13.6b AC1)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CROSS_TEAM_SHARE_NAMESPACE",
        purpose: "Collective namespace for a boot-time cross-team share (default|coordination|forgotten); principal is refused at both ends (Story 13.6b AC1)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CROSS_TEAM_SHARE_KEY",
        purpose: "Collective key for a boot-time cross-team share (Story 13.6b AC1)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_CROSS_TEAM_SHARE_VALUE",
        purpose: "Text value for a boot-time cross-team share (Story 13.6b AC1)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_SIEM_FILE",
        purpose: "Path to the local SIEM file sink; when set and non-empty, enables enterprise SIEM export (Story 11.4c)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_KMS_MASTER_KEY",
        purpose: "Hex-encoded 32-byte org master key for the LocalMasterKeyKms at-rest AEAD envelope; absent keeps byte-identical Option-A plaintext (Story 11.4c)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_SSO_JWKS",
        purpose: "Static JWKS document for the OIDC assertion verifier (Story 11.4c)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_SSO_ISSUERS",
        purpose: "Comma-separated allowlist of accepted OIDC issuers (Story 11.4c)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_SSO_AUDIENCE",
        purpose: "Required OIDC audience claim for SSO assertion verification (Story 11.4c)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_SSO_ALGS",
        purpose: "Optional comma-separated allowed JWS algorithms (default RS256,ES256) (Story 11.4c)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_SSO_ASSERTION",
        purpose: "OIDC assertion (JWT) presented at enterprise-governed capability issuance when SSO is configured (Story 11.4c)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_PDP_POLICY_FILE",
        purpose: "Explicit path to a Cedar (.cedar) PDP policy file (Story 11.4a)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_PDP_POLICY_INLINE",
        purpose: "Explicit inline Cedar PDP policy text (Story 11.4a)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_PDP_POLICY",
        purpose: "Legacy PDP policy source: inline Cedar text or file path via file:/inline: prefix (Story 11.4a)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_PDP_REFRESH_INTERVAL_MS",
        purpose: "PDP policy refresh interval (ms) for the enterprise PDP runtime reconciler (Story 11.4a; read via duration_ms_from_env at enterprise_pdp_runtime.rs:86)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_PDP_STALENESS_TTL_MS",
        purpose: "PDP staleness TTL (ms); after expiry PDP-granted caps revert to deny (Story 11.4a; enterprise_pdp_runtime.rs:90)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_COHORT_DAEMON_CONFIG",
        purpose: "Path to the cohort A2A daemon TOML config (manifest path + digest summary) (Story 12.1)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_LIVE_AGENT",
        purpose: "Opt in to spawning a REAL agent-CLI Worker subprocess (codex/claude) instead of the hermetic fixture; local-only, never CI. Distinct from --live (which selects the real reasoning provider for class Spirits) (j1-tier2-live-agent-signed-bridge)",
        stability: EnvStability::UserFacing,
    },
    EnvVar {
        name: "MAOS_HOST_GRANTS",
        purpose: "Path to an operator TOML file of host-managed CliWrapper grants ([[grant]] attested_image/signing_key_id/permitted_tier/permitted_egress_destinations); replaces the v0.9 self-grant. Absent → built-in fixture grant only, real agent CLIs fail closed (j1-tier2-live-agent-signed-bridge)",
        stability: EnvStability::UserFacing,
    },
];

pub fn lookup(name: &str) -> Option<&'static EnvVar> {
    MAOS_ENV_REGISTRY.iter().find(|e| e.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for e in MAOS_ENV_REGISTRY {
            assert!(
                seen.insert(e.name),
                "duplicate env-contract entry: {}",
                e.name
            );
        }
    }

    #[test]
    fn all_entries_start_with_maos_prefix() {
        for e in MAOS_ENV_REGISTRY {
            assert!(
                e.name.starts_with("MAOS_"),
                "env-contract entry '{}' must start with MAOS_",
                e.name
            );
        }
    }

    #[test]
    fn lookup_finds_known_var() {
        assert!(lookup("MAOS_HOME").is_some());
        assert!(lookup("MAOS_NONEXISTENT").is_none());
    }
}
