use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

pub struct CassetteReplayPort {
    entries: Vec<CassetteEntry>,
    cursor: Mutex<usize>,
    strict: bool,
}

struct CassetteEntry {
    prompt_sha256: String,
    prompt_len: usize,
    response: InferenceResponse,
}

impl CassetteReplayPort {
    pub fn from_file(path: &std::path::Path, strict: bool) -> Result<Self, String> {
        let data =
            std::fs::read_to_string(path).map_err(|e| format!("cassette read: {path:?}: {e}"))?;
        let root: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("cassette parse: {e}"))?;

        let schema = root
            .get("schema_version")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if schema != "maos.journey.cassette/v1" {
            return Err(format!(
                "cassette: unsupported schema_version '{schema}' (expected maos.journey.cassette/v1)"
            ));
        }

        let entries_arr = root
            .get("entries")
            .and_then(|v| v.as_array())
            .ok_or("cassette: missing 'entries' array")?;

        let mut entries = Vec::with_capacity(entries_arr.len());
        for (i, e) in entries_arr.iter().enumerate() {
            let prompt_sha256 = e
                .get("prompt_sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let prompt_len = e.get("prompt_len").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let resp = e
                .get("response")
                .ok_or_else(|| format!("cassette entry {i}: missing 'response'"))?;
            let text = resp
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let stop_reason = match resp
                .get("stop_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("stop_sequence")
            {
                "max_tokens" => StopReason::MaxTokens,
                "stop_sequence" => StopReason::StopSequence,
                other => StopReason::ProviderStop(other.to_string()),
            };
            let usage_obj = resp.get("usage");
            let usage = TokenUsage {
                input_tokens: usage_obj
                    .and_then(|u| u.get("input_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                output_tokens: usage_obj
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            };
            let attr_obj = resp.get("provider_attribution");
            let provider_attribution = ProviderAttribution {
                provider_id: attr_obj
                    .and_then(|a| a.get("provider_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("replay")
                    .to_string(),
                endpoint_url: attr_obj
                    .and_then(|a| a.get("endpoint_url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("cassette://replay")
                    .to_string(),
                model_id: attr_obj
                    .and_then(|a| a.get("model_id"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
            };
            entries.push(CassetteEntry {
                prompt_sha256,
                prompt_len,
                response: InferenceResponse {
                    text,
                    stop_reason,
                    usage,
                    provider_attribution,
                },
            });
        }

        Ok(Self {
            entries,
            cursor: Mutex::new(0),
            strict,
        })
    }
}

impl InferencePort for CassetteReplayPort {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let mut cursor = self.cursor.lock().unwrap_or_else(|e| e.into_inner());
        let idx = *cursor;
        if idx >= self.entries.len() {
            let msg = format!(
                "cassette replay: exhausted ({} entries consumed, no more available)",
                self.entries.len()
            );
            eprintln!("maos: CASSETTE EXHAUSTED — {msg}");
            return Err(InferenceError::ProviderTransport(msg));
        }
        let entry = &self.entries[idx];

        let mut hasher = Sha256::new();
        hasher.update(req.prompt.as_bytes());
        let actual_hash = hex::encode(hasher.finalize());
        let actual_len = req.prompt.len();

        if entry.prompt_sha256 != "0000000000000000000000000000000000000000000000000000000000000000"
            && (entry.prompt_sha256 != actual_hash || entry.prompt_len != actual_len)
        {
            let msg = format!(
                "cassette drift at seq {idx}: expected sha256={} len={}, got sha256={actual_hash} len={actual_len}",
                entry.prompt_sha256, entry.prompt_len
            );
            if self.strict {
                eprintln!("maos: CASSETTE STRICT ERROR — {msg}");
                return Err(InferenceError::ProviderTransport(msg));
            }
            eprintln!("maos: CASSETTE DRIFT WARNING — {msg}");
        }

        *cursor = idx + 1;
        Ok(entry.response.clone())
    }
}

pub struct CassetteRecordPort {
    inner: Box<dyn InferencePort + Send + Sync>,
    path: std::path::PathBuf,
    entries: Mutex<Vec<serde_json::Value>>,
    spirit_id: String,
    session: String,
}

impl CassetteRecordPort {
    pub fn new(
        inner: Box<dyn InferencePort + Send + Sync>,
        path: std::path::PathBuf,
        spirit_id: String,
        session: String,
    ) -> Self {
        Self {
            inner,
            path,
            entries: Mutex::new(Vec::new()),
            spirit_id,
            session,
        }
    }
}

impl Drop for CassetteRecordPort {
    fn drop(&mut self) {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if entries.is_empty() {
            return;
        }
        let cassette = serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "recorded_at": chrono_stub(),
            "model_id": "live-recorded",
            "spirit_id": self.spirit_id,
            "session": self.session,
            "entries": *entries,
        });
        let serialized = match serde_json::to_string_pretty(&cassette) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("maos: cassette serialization failed: {e}");
                return;
            }
        };
        if let Err(e) = std::fs::write(&self.path, serialized) {
            eprintln!("maos: cassette record write failed: {e}");
        }
    }
}

impl InferencePort for CassetteRecordPort {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let resp = self.inner.complete(req.clone())?;
        let mut hasher = Sha256::new();
        hasher.update(req.prompt.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let entry = serde_json::json!({
            "sequence": self.entries.lock().unwrap_or_else(|e| e.into_inner()).len(),
            "prompt_sha256": hash,
            "prompt_len": req.prompt.len(),
            "response": {
                "stop_reason": match resp.stop_reason {
                    StopReason::MaxTokens => "max_tokens".to_string(),
                    StopReason::StopSequence => "stop_sequence".to_string(),
                    StopReason::ProviderStop(ref s) => s.clone(),
                },
                "usage": {
                    "input_tokens": resp.usage.input_tokens,
                    "output_tokens": resp.usage.output_tokens,
                },
                "provider_attribution": {
                    "provider_id": resp.provider_attribution.provider_id,
                    "endpoint_url": resp.provider_attribution.endpoint_url,
                    "model_id": resp.provider_attribution.model_id,
                },
            },
        });

        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(entry);
        Ok(resp)
    }
}

fn chrono_stub() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert unix timestamp to YYYY-MM-DD using civil calendar arithmetic.
    // Algorithm adapted from Howard Hinnant's public-domain `civil_from_days`.
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}T00:00:00Z")
}

/// Convert day-number since Unix epoch (1970-01-01 = day 0) to (year, month, day).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let noe = z - era * 146097;
    let yoe = (noe - noe / 1460 + noe / 36524 - noe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = noe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i1::CapabilityToken;
    use maos_domain::ports::inference::InferenceOptions;

    fn test_token() -> CapabilityToken {
        use maos_domain::invariants::i1::TokenId;
        CapabilityToken::new(TokenId([0u8; 16]), 0, u64::MAX, [0u8; 64])
    }

    fn test_cassette() -> std::path::PathBuf {
        static NEXT_CASSETTE_TEST_ID: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_CASSETTE_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "maos-cassette-test-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.json");
        let data = serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "recorded_at": "2026-06-11T00:00:00Z",
            "model_id": "test",
            "spirit_id": "test",
            "session": "test",
            "entries": [
                {
                    "sequence": 0,
                    "prompt_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                    "prompt_len": 0,
                    "response": {
                        "text": "test response",
                        "stop_reason": "stop_sequence",
                        "usage": {"input_tokens": 10, "output_tokens": 5},
                        "provider_attribution": {
                            "provider_id": "replay",
                            "endpoint_url": "cassette://test",
                            "model_id": "test"
                        }
                    }
                }
            ]
        });
        std::fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
        path
    }

    fn make_req(prompt: &str) -> InferenceRequest {
        InferenceRequest::new(
            0,
            test_token(),
            prompt.to_string(),
            InferenceOptions::default(),
            None,
            vec![],
        )
    }

    #[test]
    fn replay_serves_sequenced_entries() {
        let path = test_cassette();
        let port = CassetteReplayPort::from_file(&path, false).unwrap();
        let resp = port.complete(make_req("anything")).unwrap();
        assert_eq!(resp.text, "test response");
    }

    #[test]
    fn replay_exhausted_returns_error() {
        let path = test_cassette();
        let port = CassetteReplayPort::from_file(&path, false).unwrap();
        port.complete(make_req("a")).unwrap();
        let err = port.complete(make_req("b")).unwrap_err();
        assert!(matches!(err, InferenceError::ProviderTransport(_)));
    }

    #[test]
    fn strict_mode_rejects_hash_mismatch() {
        let dir = std::env::temp_dir().join("maos-cassette-strict");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("strict.json");
        let data = serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "recorded_at": "2026-06-11T00:00:00Z",
            "model_id": "test",
            "spirit_id": "test",
            "session": "test",
            "entries": [{
                "sequence": 0,
                "prompt_sha256": "aaaa000000000000000000000000000000000000000000000000000000000000",
                "prompt_len": 999,
                "response": {
                    "text": "resp",
                    "stop_reason": "stop_sequence",
                    "usage": {"input_tokens": 1, "output_tokens": 1},
                    "provider_attribution": {
                        "provider_id": "replay",
                        "endpoint_url": "cassette://test"
                    }
                }
            }]
        });
        std::fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
        let port = CassetteReplayPort::from_file(&path, true).unwrap();
        let err = port.complete(make_req("wrong prompt")).unwrap_err();
        assert!(matches!(err, InferenceError::ProviderTransport(_)));
    }
}
