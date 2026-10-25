//! Secret materialization port (Story 16-4, ADR-051 / NFR-Sec-19).
//!
//! The domain owns the closed credential names and object-safe seam. OS
//! keyrings, process-environment fallback, and encrypted files remain adapters
//! outside the kernel dependency closure.

/// Credentials MAOS may read and remove. The set is intentionally closed so
/// purge never enumerates or deletes operator-owned keyring entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretKey {
    AnthropicApiKey,
    OpenAiApiKey,
    /// Story 18-8 AC2 — the bearer MAOS sends to one MCP server.
    McpBearer(McpServer),
}

/// The eight MCP servers with a `MAOS_MCP_<SERVER>_URI` row (closed, like
/// [`SecretKey`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpServer {
    Calendar,
    Slack,
    Linear,
    Figma,
    Web,
    Arxiv,
    Github,
    CitationGraph,
}

impl McpServer {
    /// (`as_str`, `environment_variable`) of this server's bearer key.
    const fn bearer_names(self) -> (&'static str, &'static str) {
        match self {
            Self::Calendar => ("mcp-calendar-bearer", "MAOS_MCP_CALENDAR_BEARER"),
            Self::Slack => ("mcp-slack-bearer", "MAOS_MCP_SLACK_BEARER"),
            Self::Linear => ("mcp-linear-bearer", "MAOS_MCP_LINEAR_BEARER"),
            Self::Figma => ("mcp-figma-bearer", "MAOS_MCP_FIGMA_BEARER"),
            Self::Web => ("mcp-web-bearer", "MAOS_MCP_WEB_BEARER"),
            Self::Arxiv => ("mcp-arxiv-bearer", "MAOS_MCP_ARXIV_BEARER"),
            Self::Github => ("mcp-github-bearer", "MAOS_MCP_GITHUB_BEARER"),
            Self::CitationGraph => (
                "mcp-citation-graph-bearer",
                "MAOS_MCP_CITATION_GRAPH_BEARER",
            ),
        }
    }
}

impl SecretKey {
    pub const ALL: [Self; 10] = [
        Self::AnthropicApiKey,
        Self::OpenAiApiKey,
        Self::McpBearer(McpServer::Calendar),
        Self::McpBearer(McpServer::Slack),
        Self::McpBearer(McpServer::Linear),
        Self::McpBearer(McpServer::Figma),
        Self::McpBearer(McpServer::Web),
        Self::McpBearer(McpServer::Arxiv),
        Self::McpBearer(McpServer::Github),
        Self::McpBearer(McpServer::CitationGraph),
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AnthropicApiKey => "anthropic-api-key",
            Self::OpenAiApiKey => "openai-api-key",
            Self::McpBearer(server) => server.bearer_names().0,
        }
    }

    pub const fn environment_variable(self) -> &'static str {
        match self {
            Self::AnthropicApiKey => "MAOS_ANTHROPIC_API_KEY",
            Self::OpenAiApiKey => "MAOS_OPENAI_API_KEY",
            Self::McpBearer(server) => server.bearer_names().1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretDeleteStatus {
    Removed,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretStoreError {
    #[error("secret store unavailable: {0}")]
    Unavailable(String),
    #[error("secret store access failed: {0}")]
    Access(String),
    #[error("secret value is not valid UTF-8")]
    InvalidEncoding,
}

pub trait SecretStore: Send + Sync {
    /// Class: data-movement
    ///
    /// Materialize one known credential. `None` means the backend is healthy
    /// but carries no value for that name.
    fn get(&self, key: SecretKey) -> Result<Option<String>, SecretStoreError>;
    /// Store or replace one known credential.
    fn put(&self, key: SecretKey, value: &str) -> Result<(), SecretStoreError>;

    /// Class: data-movement
    ///
    /// Delete exactly one known credential. There is deliberately no list or
    /// collection-delete operation.
    fn delete(&self, key: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError>;

    /// Class: supervision
    ///
    /// Whether the backend can currently accept an operation.
    fn is_healthy(&self) -> bool;
}
