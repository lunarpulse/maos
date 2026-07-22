#![forbid(unsafe_code)]

//! Pre-write secret-redaction filter at the Transparency Log boundary.
//!
//! Per architecture §8.1 threat-model + NFR-Sec-4 (v0.5 binding for the
//! full surface; v0.1-β ships the filter trait + corpus-backed default).
//!
//! The filter runs on every payload BEFORE it is written to the
//! Transparency Log SQLite row. Detected secrets are replaced with a
//! typed marker `<REDACTED:type=<class>,len=<bytes>,hash=<sha256-prefix>>`
//! per architecture §4.3.2.
//!
//! # Dep-direction note (Story 1b.1 AC4 Option B)
//!
//! The redaction rule data lives as pure-data constants in
//! `maos-domain::redaction` (lifted from `maos-corpus-gen` in this story).
//! Both `maos-corpus-gen` and `maos-kernel-core` consume from
//! `maos-domain`, keeping the dep direction clean: kernel-core does NOT
//! depend on corpus-gen.

use std::borrow::Cow;

/// Trait abstraction over the redaction rule set. The default impl
/// delegates to pattern-based detection; alternate impls (test mocks,
/// FIPS-aware redaction, region-specific PII rules) can be swapped at
/// composition-root construction.
pub trait RedactionPolicy: std::fmt::Debug + Send + Sync {
    /// Redact secrets in the input bytes. Returns `Cow::Borrowed` if no
    /// secrets are found (zero allocation in the common case); returns
    /// `Cow::Owned` with the redacted bytes if any match.
    fn redact<'a>(&self, bytes: &'a [u8]) -> Cow<'a, [u8]>;
}

/// Default redaction policy backed by pattern-based secret detection.
///
/// Detects and redacts:
/// - API keys (Anthropic `sk-...`, OpenAI `sk-...`, GitHub `ghp_...`, etc.)
/// - Capability tokens (32-byte hex sequences matching the Ed25519 token shape)
/// - mTLS private-key bytes (PEM "BEGIN PRIVATE KEY" headers)
/// - AWS/GCP credentials patterns
///
/// Each match is replaced with:
/// `<REDACTED:type=<class>,len=<bytes>,hash=<sha256-prefix>>`
#[derive(Debug, Default)]
pub struct CorpusBackedRedactionPolicy {
    _private: (),
}

impl CorpusBackedRedactionPolicy {
    /// Create a new default redaction policy.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Redaction pattern: a compiled regex-like matcher and its class label.
struct RedactionRule {
    /// Prefix bytes to search for (simple prefix matching; avoids regex dep).
    prefix: &'static [u8],
    /// Class label for the redaction marker.
    class: &'static str,
}

/// Static rule set — the canonical secret patterns at v0.1-β.
/// Lifted from `maos-corpus-gen::secret_redaction::seeds` to `maos-domain`
/// per AC4 Option B recommendation.
static RULES: &[RedactionRule] = &[
    RedactionRule {
        prefix: b"sk-ant-api03-",
        class: "api_key_anthropic",
    },
    RedactionRule {
        prefix: b"sk-ant-",
        class: "api_key_anthropic",
    },
    RedactionRule {
        prefix: b"sk-proj-",
        class: "api_key_openai",
    },
    RedactionRule {
        prefix: b"sk-",
        class: "api_key_generic",
    },
    RedactionRule {
        prefix: b"ghp_",
        class: "api_key_github",
    },
    RedactionRule {
        prefix: b"gho_",
        class: "api_key_github",
    },
    RedactionRule {
        prefix: b"ghs_",
        class: "api_key_github",
    },
    RedactionRule {
        prefix: b"ghu_",
        class: "api_key_github",
    },
    RedactionRule {
        prefix: b"ghc_",
        class: "api_key_github",
    },
    RedactionRule {
        prefix: b"ghr_",
        class: "api_key_github",
    },
    RedactionRule {
        prefix: b"AKIA",
        class: "aws_access_key",
    },
    RedactionRule {
        prefix: b"ASIA",
        class: "aws_access_key",
    },
    RedactionRule {
        prefix: b"-----BEGIN PRIVATE KEY-----",
        class: "private_key",
    },
    RedactionRule {
        prefix: b"-----BEGIN RSA PRIVATE KEY-----",
        class: "private_key_rsa",
    },
    RedactionRule {
        prefix: b"AIza",
        class: "google_api_key",
    },
    RedactionRule {
        prefix: b"ya29.",
        class: "google_oauth_token",
    },
];

/// Token-shaped pattern: 64 consecutive hex chars (32-byte Ed25519 token).
fn is_hex_byte(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
}

/// Minimum length of a hex-encoded token to be considered a secret.
const TOKEN_HEX_MIN_LEN: usize = 32;

fn redact_unscoped<'a>(bytes: &'a [u8]) -> Cow<'a, [u8]> {
    let has_match = RULES.iter().any(|rule| contains_prefix(bytes, rule.prefix));
    if !has_match && !contains_hex_token(bytes) {
        return Cow::Borrowed(bytes);
    }

    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let mut matched = false;
        for rule in RULES {
            let prefix_len = rule.prefix.len();
            if i + prefix_len <= bytes.len() && &bytes[i..i + prefix_len] == rule.prefix {
                let end = find_secret_end(bytes, i + prefix_len);
                let secret_len = end - i;
                let hash_prefix = simple_hash(&bytes[i..end]);
                let marker = format!(
                    "<REDACTED:type={},len={},hash={:04x}>",
                    rule.class, secret_len, hash_prefix
                );
                result.extend_from_slice(marker.as_bytes());
                i = end;
                matched = true;
                break;
            }
        }
        if !matched {
            if i + TOKEN_HEX_MIN_LEN <= bytes.len() {
                let hex_run = count_hex_run(bytes, i);
                if hex_run >= TOKEN_HEX_MIN_LEN {
                    let secret_len = hex_run;
                    let hash_prefix = simple_hash(&bytes[i..i + secret_len]);
                    let marker = format!(
                        "<REDACTED:type=capability_token,len={},hash={:04x}>",
                        secret_len, hash_prefix
                    );
                    result.extend_from_slice(marker.as_bytes());
                    i += secret_len;
                    matched = true;
                }
            }
            if !matched {
                result.push(bytes[i]);
                i += 1;
            }
        }
    }
    Cow::Owned(result)
}

fn is_compact_frame_ref(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn redact_json_value(value: &mut serde_json::Value, under_clause_sources: bool) -> bool {
    match value {
        serde_json::Value::Object(fields) => {
            let mut saw_clause_sources = false;
            for (key, value) in fields.iter_mut() {
                let protected = under_clause_sources || key == "clause_sources";
                saw_clause_sources |= key == "clause_sources";
                saw_clause_sources |= redact_json_value(value, protected);
            }

            let mut redacted_keys: Vec<_> = fields
                .keys()
                .filter_map(|key| {
                    if key == "clause_sources" {
                        return None;
                    }
                    match redact_unscoped(key.as_bytes()) {
                        Cow::Owned(redacted) => Some((
                            key.clone(),
                            String::from_utf8(redacted)
                                .expect("redaction preserves valid UTF-8 JSON object keys"),
                        )),
                        Cow::Borrowed(_) => None,
                    }
                })
                .collect();
            if !redacted_keys.is_empty() {
                let original_fields = std::mem::take(fields);
                let mut redacted_fields = serde_json::Map::with_capacity(original_fields.len());
                for (key, value) in original_fields {
                    let key = redacted_keys
                        .iter()
                        .position(|(original, _)| original == &key)
                        .map_or(key, |index| redacted_keys.swap_remove(index).1);
                    // Two keys can redact to the same marker, and a redacted
                    // marker can equal a pre-existing literal key — a plain
                    // `insert` would silently delete a member. Disambiguate
                    // deterministically so every member survives.
                    let mut key = key;
                    if redacted_fields.contains_key(&key) {
                        let mut n = 2u32;
                        loop {
                            let candidate = format!("{key}#{n}");
                            if !redacted_fields.contains_key(&candidate) {
                                key = candidate;
                                break;
                            }
                            n += 1;
                        }
                    }
                    redacted_fields.insert(key, value);
                }
                *fields = redacted_fields;
            }

            saw_clause_sources
        }
        serde_json::Value::Array(values) => values.iter_mut().fold(false, |seen, value| {
            redact_json_value(value, under_clause_sources) || seen
        }),
        serde_json::Value::String(value) => {
            if !(under_clause_sources && is_compact_frame_ref(value)) {
                if let Cow::Owned(redacted) = redact_unscoped(value.as_bytes()) {
                    *value = String::from_utf8(redacted)
                        .expect("redaction preserves valid UTF-8 JSON strings");
                }
            }
            false
        }
        _ => false,
    }
}

impl RedactionPolicy for CorpusBackedRedactionPolicy {
    fn redact<'a>(&self, bytes: &'a [u8]) -> Cow<'a, [u8]> {
        // Digest payloads need one narrow exception: exact 16-byte frame refs
        // under `clause_sources` are audit identifiers, not credentials. An
        // exact-32-hex secret there is retained as the accepted residual because
        // it is shape-indistinguishable from a frame ref; shorter/longer hex,
        // prefix-rule secrets, and object keys are still scrubbed. Parse only
        // payloads that name the key; all other payloads retain the
        // allocation-free fast path.
        if contains_prefix(bytes, b"\"clause_sources\"") {
            if let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(bytes) {
                if redact_json_value(&mut value, false) {
                    if let Ok(encoded) = serde_json::to_vec(&value) {
                        return Cow::Owned(encoded);
                    }
                }
            }
        }
        redact_unscoped(bytes)
    }
}

/// Check if `haystack` contains `needle` as a contiguous substring.
fn contains_prefix(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Scan for a CREDENTIAL-shaped value (API key, private key, cloud credential)
/// by the same prefix [`RULES`] the redaction filter uses, returning the class
/// of the first match.
///
/// Unlike [`RedactionPolicy::redact`], this deliberately does NOT flag the
/// 32+-hex token heuristic: Transparency-Log references (frame IDs, digest refs)
/// are legitimately hex and are NOT secrets. This is the pre-write TRIPWIRE for
/// human-authored artifacts — e.g. a J1 Tier-2 signed-run capture — that must
/// carry NO credential yet DO carry hex TL references. `redact()` (which scrubs
/// tokens too) remains the boundary filter for machine-generated frames.
pub fn detect_credential(bytes: &[u8]) -> Option<&'static str> {
    RULES
        .iter()
        .find(|rule| contains_prefix(bytes, rule.prefix))
        .map(|rule| rule.class)
}

/// Check if `haystack` contains a hex-encoded token of sufficient length.
fn contains_hex_token(haystack: &[u8]) -> bool {
    let mut run = 0;
    for &b in haystack {
        if is_hex_byte(b) {
            run += 1;
            if run >= TOKEN_HEX_MIN_LEN {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// Count consecutive hex bytes starting at `start`.
fn count_hex_run(bytes: &[u8], start: usize) -> usize {
    let mut count = 0;
    for &b in &bytes[start..] {
        if is_hex_byte(b) {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Find the end of a secret value (delimited by whitespace/newline/end).
fn find_secret_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' || b == b'"' || b == b'\'' {
            break;
        }
        end += 1;
    }
    end
}

/// Simple 16-bit hash for the redaction marker (first 4 hex chars of a
/// deterministic fingerprint). NOT cryptographic — for log grepping only.
fn simple_hash(bytes: &[u8]) -> u16 {
    // FNV-1a 16-bit
    let mut hash: u16 = 0x811c;
    for &b in bytes {
        hash ^= b as u16;
        hash = hash.wrapping_mul(0x001f);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_filter_zero_alloc_on_clean_payload() {
        let policy = CorpusBackedRedactionPolicy::new();
        let clean = b"hello from spirit-butler; calendar event read OK";
        let result = policy.redact(clean);
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "clean payload triggered allocation"
        );
    }

    #[test]
    fn detect_credential_flags_prefix_secrets_but_not_hex_refs() {
        // Prefix-shaped credentials are flagged with their class...
        assert_eq!(
            detect_credential(b"OPENAI_API_KEY=sk-proj-abcdef0123456789"),
            Some("api_key_openai")
        );
        assert_eq!(
            detect_credential(b"token: ghp_ABCDEFGHIJKLMNOPqrstuvwx"),
            Some("api_key_github")
        );
        // ...but a 32-hex Transparency-Log reference is NOT a secret: `redact`
        // would scrub it as a token, yet `detect_credential` (prefix-only) lets
        // it through, which is exactly what a signed-run capture needs.
        assert_eq!(
            detect_credential(b"audit_ref: aabbccddeeff00112233445566778899"),
            None
        );
        assert_eq!(detect_credential(b"plain non-secret metadata"), None);
    }

    #[test]
    fn clause_source_frame_refs_survive_without_exempting_other_hex_or_secrets() {
        let policy = CorpusBackedRedactionPolicy::new();
        let frame_ref = "aabbccddeeff00112233445566778899";
        let long_hex = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        let key_secret = "ffeeddccbbaa99887766554433221100";
        let mut payload = serde_json::json!({
            "digest_payload": {
                "clause_sources": {
                    "agents_ran": [frame_ref],
                    "long_hex": long_hex,
                    "narrative": ["sk-proj-secret-value"]
                },
                "unrelated_hex": frame_ref
            }
        });
        payload
            .as_object_mut()
            .unwrap()
            .insert(key_secret.to_owned(), serde_json::json!(1));
        let encoded = serde_json::to_vec(&payload).unwrap();
        let redacted = policy.redact(&encoded);
        let text = std::str::from_utf8(&redacted).unwrap();

        // This is the documented accepted tradeoff, not a credential-safety
        // guarantee: exact 32-hex values here are indistinguishable from refs.
        assert!(
            text.contains(frame_ref),
            "exact frame refs under clause_sources must survive"
        );
        assert_eq!(
            text.matches(frame_ref).count(),
            1,
            "the same hex outside clause_sources must still be redacted"
        );
        assert!(
            !text.contains(long_hex),
            "longer hex under clause_sources must still be redacted"
        );
        assert!(
            !text.contains(key_secret),
            "object keys must be redacted even when clause_sources is present"
        );
        assert!(!text.contains("sk-proj-secret-value"));
        assert!(text.contains("<REDACTED:type="));
    }

    #[test]
    fn redaction_detects_anthropic_key() {
        let policy = CorpusBackedRedactionPolicy::new();
        let input = b"api_key=sk-ant-api03-abcdef1234567890 endpoint=prod";
        let result = policy.redact(input);
        let s = std::str::from_utf8(&result).unwrap();
        assert!(
            s.contains("<REDACTED:type=api_key_anthropic"),
            "anthropic key not redacted: {s}"
        );
        assert!(!s.contains("sk-ant-api03-abcdef"), "raw key leaked");
    }

    #[test]
    fn redaction_detects_github_token() {
        let policy = CorpusBackedRedactionPolicy::new();
        let input = b"token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh";
        let result = policy.redact(input);
        let s = std::str::from_utf8(&result).unwrap();
        assert!(
            s.contains("<REDACTED:type=api_key_github"),
            "github token not redacted"
        );
        assert!(!s.contains("ghp_ABCDEF"), "raw token leaked");
    }

    #[test]
    fn redaction_detects_aws_key() {
        let policy = CorpusBackedRedactionPolicy::new();
        let input = b"AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let result = policy.redact(input);
        let s = std::str::from_utf8(&result).unwrap();
        assert!(
            s.contains("<REDACTED:type=aws_access_key"),
            "aws key not redacted"
        );
    }

    #[test]
    fn redaction_detects_private_key() {
        let policy = CorpusBackedRedactionPolicy::new();
        let input = b"-----BEGIN PRIVATE KEY-----\nMIIEvQ...\n-----END PRIVATE KEY-----";
        let result = policy.redact(input);
        let s = std::str::from_utf8(&result).unwrap();
        assert!(
            s.contains("<REDACTED:type=private_key"),
            "private key not redacted"
        );
    }

    #[test]
    fn redaction_detects_hex_token() {
        let policy = CorpusBackedRedactionPolicy::new();
        // 64 hex chars = 32-byte token
        let input = b"token=a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let result = policy.redact(input);
        let s = std::str::from_utf8(&result).unwrap();
        assert!(
            s.contains("<REDACTED:type=capability_token"),
            "hex token not redacted"
        );
    }

    #[test]
    fn redaction_preserves_surrounding_text() {
        let policy = CorpusBackedRedactionPolicy::new();
        let input = b"before sk-ant-test-key123 after";
        let result = policy.redact(input);
        let s = std::str::from_utf8(&result).unwrap();
        assert!(s.starts_with("before "), "prefix text corrupted: {s}");
        assert!(s.contains(" after"), "suffix text corrupted: {s}");
    }
}
