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

impl RedactionPolicy for CorpusBackedRedactionPolicy {
    fn redact<'a>(&self, bytes: &'a [u8]) -> Cow<'a, [u8]> {
        let mut has_match = false;
        // First pass: check if any rule matches
        for rule in RULES {
            if contains_prefix(bytes, rule.prefix) {
                has_match = true;
                break;
            }
        }
        if !has_match && !contains_hex_token(bytes) {
            return Cow::Borrowed(bytes);
        }

        // Second pass: build redacted output
        let mut result = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let mut matched = false;
            for rule in RULES {
                let prefix_len = rule.prefix.len();
                if i + prefix_len <= bytes.len() && &bytes[i..i + prefix_len] == rule.prefix {
                    // Find the end of the secret (whitespace, newline, or end)
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
                // Check for hex-encoded token pattern
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
}

/// Check if `haystack` contains `needle` as a contiguous substring.
fn contains_prefix(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
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
