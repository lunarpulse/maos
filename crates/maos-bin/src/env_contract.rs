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
