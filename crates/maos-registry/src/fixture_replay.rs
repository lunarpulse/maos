//! Test-only FixtureReplay client for deterministic registry testing.
//!
//! Gated by `#[cfg(any(test, feature = "fixture_replay"))]`.
//! Mirrors the `FixtureReplayMcpServer` pattern from `maos-mcp`.

use std::sync::Mutex;

use maos_domain::ports::registry::{
    PublishReceipt, RegistryError, SearchQuery, SearchResults, SignedArtifact, SignedManifest,
    SignedPackage, SpiritId, SpiritRegistryClient, YankList, YankReason, YankReceipt,
};

/// A deterministic, test-only `SpiritRegistryClient` whose responses
/// are pre-loaded via a queue.
#[cfg(any(test, feature = "fixture_replay"))]
pub struct FixtureReplaySpiritRegistryClient {
    responses: Mutex<std::collections::VecDeque<Result<serde_json::Value, RegistryError>>>,
    calls: Mutex<Vec<RegistryCall>>,
}

#[cfg(any(test, feature = "fixture_replay"))]
#[derive(Debug, Clone)]
pub struct RegistryCall {
    pub method: String,
    pub args_json: String,
}

#[cfg(any(test, feature = "fixture_replay"))]
impl FixtureReplaySpiritRegistryClient {
    /// Create a new fixture-replay client with pre-loaded responses.
    pub fn new(responses: Vec<Result<serde_json::Value, RegistryError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Drain and return all recorded calls.
    pub fn take_calls(&self) -> Vec<RegistryCall> {
        self.calls.lock().unwrap().drain(..).collect()
    }

    fn pop(&self, method: &str, args: serde_json::Value) -> Result<serde_json::Value, RegistryError> {
        self.calls.lock().unwrap().push(RegistryCall {
            method: method.to_string(),
            args_json: args.to_string(),
        });
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Err(RegistryError::Transport(
                "fixture replay: empty response queue".into(),
            )))
    }
}

#[cfg(any(test, feature = "fixture_replay"))]
impl SpiritRegistryClient for FixtureReplaySpiritRegistryClient {
    fn search(&self, q: &SearchQuery) -> Result<SearchResults, RegistryError> {
        let args = serde_json::json!({"text": q.text, "include_yanked": q.include_yanked, "limit": q.limit});
        self.pop("registry.search", args)
            .and_then(|v| serde_json::from_value(v).map_err(|e| RegistryError::Transport(e.to_string())))
    }

    fn manifest(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedManifest, RegistryError> {
        let args = serde_json::json!({"spirit_id": spirit_id.as_str(), "version": version});
        self.pop("registry.manifest", args)
            .and_then(|v| serde_json::from_value(v).map_err(|e| RegistryError::Transport(e.to_string())))
    }

    fn artifact(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedArtifact, RegistryError> {
        let args = serde_json::json!({"spirit_id": spirit_id.as_str(), "version": version});
        self.pop("registry.artifact", args)
            .and_then(|v| serde_json::from_value(v).map_err(|e| RegistryError::Transport(e.to_string())))
    }

    fn publish(&self, pkg: &SignedPackage) -> Result<PublishReceipt, RegistryError> {
        let args = serde_json::json!({"spirit_id": pkg.spirit_id.as_str(), "version": pkg.version});
        self.pop("registry.publish", args)
            .and_then(|v| serde_json::from_value(v).map_err(|e| RegistryError::Transport(e.to_string())))
    }

    fn deprecate(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        reason: &YankReason,
    ) -> Result<YankReceipt, RegistryError> {
        let args = serde_json::json!({
            "spirit_id": spirit_id.as_str(),
            "version": version,
            "reason": reason.summary,
        });
        self.pop("registry.deprecate", args)
            .and_then(|v| serde_json::from_value(v).map_err(|e| RegistryError::Transport(e.to_string())))
    }
}

#[cfg(any(test, feature = "fixture_replay"))]
impl FixtureReplaySpiritRegistryClient {
    /// Internal yanks_since op.
    pub fn yanks_since(&self, since_ns: u64) -> Result<YankList, RegistryError> {
        let args = serde_json::json!({"since_ns": since_ns});
        self.pop("registry.yanks_since", args)
            .and_then(|v| serde_json::from_value(v).map_err(|e| RegistryError::Transport(e.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_search_returns_queued_response() {
        let client = FixtureReplaySpiritRegistryClient::new(vec![
            Ok(serde_json::json!({"items": []})),
        ]);
        let q = SearchQuery::new("test".into(), false, 50);
        let result = client.search(&q).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fixture_records_calls() {
        let client = FixtureReplaySpiritRegistryClient::new(vec![
            Ok(serde_json::json!({"items": []})),
        ]);
        let q = SearchQuery::new("test".into(), false, 50);
        let _ = client.search(&q).unwrap();
        let calls = client.take_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "registry.search");
    }

    #[test]
    fn fixture_empty_queue_returns_transport_error() {
        let client = FixtureReplaySpiritRegistryClient::new(vec![]);
        let q = SearchQuery::new("test".into(), false, 50);
        let err = client.search(&q).unwrap_err();
        assert!(matches!(err, RegistryError::Transport(_)));
    }

    #[test]
    fn fixture_errors_propagate() {
        let client = FixtureReplaySpiritRegistryClient::new(vec![
            Err(RegistryError::UnknownSpirit("notfound".into())),
        ]);
        let q = SearchQuery::new("test".into(), false, 50);
        let err = client.search(&q).unwrap_err();
        assert!(matches!(err, RegistryError::UnknownSpirit(_)));
    }
}
