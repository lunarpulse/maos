//! Pure arg builders and response extractors for Researcher-facing MCP tool calls.
//!
//! Every function in this module is synchronous and side-effect free.
//! The caller (typically `LiveResearcherMcpPort`) is responsible for issuing
//! capability tokens and performing the final `serde_json::from_value`
//! into researcher domain types (`ClaimPayload`).

use maos_domain::ports::mcp::{McpError, McpResponse};

// ---------------------------------------------------------------------------
// Arg builders — each returns the JSON args object for a specific MCP tool.
// ---------------------------------------------------------------------------

/// Args for `web.search`.
pub fn web_search_args(query: &str) -> serde_json::Value {
    serde_json::json!({ "query": query })
}

/// Args for `web.fetch`.
///
/// The `url` field IS the `source_key` for the citation join (FORK 2).
pub fn web_fetch_args(url: &str) -> serde_json::Value {
    serde_json::json!({ "url": url })
}

/// Args for `arxiv.search`.
pub fn arxiv_search_args(query: &str) -> serde_json::Value {
    serde_json::json!({ "query": query })
}

/// Args for `arxiv.get_paper`.
///
/// The `arxiv_id` field IS the `source_key` for the citation join (FORK 2).
pub fn arxiv_get_paper_args(arxiv_id: &str) -> serde_json::Value {
    serde_json::json!({ "arxiv_id": arxiv_id })
}

/// Args for `github.search_code`.
pub fn github_search_code_args(query: &str) -> serde_json::Value {
    serde_json::json!({ "query": query })
}

/// Args for `github.get_repo`.
///
/// The `repo` field IS the `source_key` for the citation join (FORK 2).
pub fn github_get_repo_args(repo: &str) -> serde_json::Value {
    serde_json::json!({ "repo": repo })
}

/// Args for `citation-graph.traverse`.
pub fn citation_graph_traverse_args(seed: &str) -> serde_json::Value {
    serde_json::json!({ "paper_id": seed })
}

/// Args for `citation-graph.get_citations`.
///
/// The `paper_id` field IS the `source_key` for the citation join (FORK 2).
pub fn citation_graph_get_citations_args(paper_id: &str) -> serde_json::Value {
    serde_json::json!({ "paper_id": paper_id })
}

// ---------------------------------------------------------------------------
// Response extractors
// ---------------------------------------------------------------------------

/// Extract the `content` field from an MCP response.
///
/// Returns `Err(McpError::Decode)` when the server signalled an error
/// (i.e. `response.is_error` is true).
pub fn extract_content(response: &McpResponse) -> Result<serde_json::Value, McpError> {
    if response.is_error {
        return Err(McpError::Decode(
            response
                .content
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("MCP server error")
                .to_string(),
        ));
    }
    Ok(response.content.clone())
}

/// Parse a Phase-1 (search/traverse) response into a list of source keys.
///
/// Returns the ids/urls that should be fed to Phase-2 fetch calls.
/// Each source key is the canonical form used in the citation join:
/// - `web.search`  → `url` values
/// - `arxiv.search` → `arxiv_id` values
/// - `github.search_code` → `repo` values
/// - `citation-graph.traverse` → `paper_id` values extracted from edges
pub fn parse_search_results(content: &serde_json::Value, server: &str) -> Vec<String> {
    match server {
        "web" => content
            .get("results")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("url").and_then(|v| v.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        "arxiv" => content
            .get("papers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.get("arxiv_id")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        "github" => content
            .get("results")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("repo").and_then(|v| v.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        "citation-graph" => {
            // Extract unique paper_ids from edges + clusters
            let mut ids = std::collections::HashSet::new();
            if let Some(edges) = content.get("edges").and_then(|v| v.as_array()) {
                for edge in edges {
                    if let Some(from) = edge.get("from").and_then(|v| v.as_str()) {
                        ids.insert(from.to_string());
                    }
                    if let Some(to) = edge.get("to").and_then(|v| v.as_str()) {
                        ids.insert(to.to_string());
                    }
                }
            }
            if let Some(clusters) = content.get("clusters").and_then(|v| v.as_array()) {
                for cluster in clusters {
                    if let Some(paper_ids) = cluster.get("paper_ids").and_then(|v| v.as_array()) {
                        for pid in paper_ids {
                            if let Some(s) = pid.as_str() {
                                ids.insert(s.to_string());
                            }
                        }
                    }
                }
            }
            ids.into_iter().collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::mcp::{McpAttribution, McpResponse};

    fn fake_response(content: &str, is_error: bool) -> McpResponse {
        McpResponse::new(
            serde_json::from_str(content).unwrap(),
            is_error,
            McpAttribution::new(
                "test".into(),
                maos_domain::ports::mcp::McpTransportId::Stdio,
                "test".into(),
            ),
        )
    }

    #[test]
    fn arg_builders_carry_source_key_fields() {
        let args = web_fetch_args("https://example.com/paper");
        assert_eq!(args["url"].as_str(), Some("https://example.com/paper"));

        let args = arxiv_get_paper_args("2501.12345");
        assert_eq!(args["arxiv_id"].as_str(), Some("2501.12345"));

        let args = github_get_repo_args("owner/name");
        assert_eq!(args["repo"].as_str(), Some("owner/name"));

        let args = citation_graph_get_citations_args("2501.12345");
        assert_eq!(args["paper_id"].as_str(), Some("2501.12345"));
    }

    #[test]
    fn parse_web_search_results() {
        let content = serde_json::json!({
            "results": [
                { "url": "https://a.com", "title": "A" },
                { "url": "https://b.com", "title": "B" }
            ]
        });
        let keys = parse_search_results(&content, "web");
        assert_eq!(keys, vec!["https://a.com", "https://b.com"]);
    }

    #[test]
    fn parse_arxiv_search_results() {
        let content = serde_json::json!({
            "papers": [
                { "arxiv_id": "2501.00001", "title": "A" },
                { "arxiv_id": "2501.00002", "title": "B" }
            ]
        });
        let keys = parse_search_results(&content, "arxiv");
        assert_eq!(keys, vec!["2501.00001", "2501.00002"]);
    }

    #[test]
    fn parse_citation_graph_traverse_results() {
        let content = serde_json::json!({
            "edges": [
                { "from": "2501.00001", "to": "2501.00002" }
            ],
            "clusters": [
                { "paper_ids": ["2501.00003"] }
            ]
        });
        let keys = parse_search_results(&content, "citation-graph");
        assert!(keys.contains(&"2501.00001".to_string()));
        assert!(keys.contains(&"2501.00002".to_string()));
        assert!(keys.contains(&"2501.00003".to_string()));
    }

    #[test]
    fn extract_content_ok() {
        let resp = fake_response(r#"{"claim": {"statement": "x"}}"#, false);
        let content = extract_content(&resp).unwrap();
        assert_eq!(content["claim"]["statement"].as_str(), Some("x"));
    }

    #[test]
    fn extract_content_error_yields_decode() {
        let resp = fake_response(r#"{"message":"boom"}"#, true);
        let err = extract_content(&resp).unwrap_err();
        match err {
            McpError::Decode(msg) => assert!(msg.contains("boom")),
            other => panic!("expected Decode, got {other:?}"),
        }
    }
}
