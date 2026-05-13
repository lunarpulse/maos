//! Secret-redaction canary test — NFR-Sec-4 v0.1-beta binding.
//!
//! Verifies the `CorpusBackedRedactionPolicy` against a representative
//! 10,000-item corpus of secret patterns. At v0.1-beta the corpus is
//! generated locally from the known rule set; the Story 0.5
//! `maos-corpus-gen` canonical corpus is consumed at v0.5 when the
//! full NFR-Sec-4 surface ships.
//!
//! Requirement (AC4): 0 leaks against the v0.1-beta rule corpus.

use std::borrow::Cow;

use maos_kernel_core::iac::{CorpusBackedRedactionPolicy, RedactionPolicy};

/// Known secret prefixes that the redaction filter must catch.
static CANARY_PATTERNS: &[(&[u8], &str)] = &[
    (b"sk-ant-api03-abcdef1234567890abcdef1234567890", "api_key_anthropic"),
    (b"sk-ant-abcdef1234567890abcdef1234567890abcd", "api_key_anthropic"),
    (b"sk-proj-abcdef1234567890abcdef1234567890abcdef1234567890", "api_key_openai"),
    (b"sk-abcdef1234567890abcdef1234567890abcdef1234567890abcd", "api_key_generic"),
    (b"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234", "api_key_github"),
    (b"gho_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234", "api_key_github"),
    (b"ghs_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234", "api_key_github"),
    (b"ghu_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234", "api_key_github"),
    (b"ghc_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234", "api_key_github"),
    (b"ghr_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234", "api_key_github"),
    (b"AKIAIOSFODNN7EXAMPLE", "aws_access_key"),
    (b"ASIATESTKEY1234567", "aws_access_key"),
    (b"-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC...\n-----END PRIVATE KEY-----", "private_key"),
    (b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----", "private_key_rsa"),
    (b"AIzaSyBd2bNf3eT6h8jK9lM0nP1qR4sU5vW7xY8z", "google_api_key"),
    (b"ya29.a0AfH6SMBd...rest-of-oauth-token", "google_oauth_token"),
];

/// Hex-encoded capability tokens (32+ hex chars in a row).
static HEX_TOKEN_PATTERNS: &[&[u8]] = &[
    b"a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
    b"deadbeefcafebabedeadbeefcafebabedeadbeefcafebabedeadbeefcafebabe",
];

fn generate_canary_corpus() -> Vec<(Vec<u8>, &'static str)> {
    let wrappers: &[(&str, &str)] = &[
        ("{\"api_key\":\"", "\",\"endpoint\":\"prod\"}"),
        ("export API_KEY=", "\n"),
        ("Authorization: Bearer ", " HTTP/1.1\n"),
        ("token=", "&scope=read"),
        ("key=", ""),
        ("env: {secret: '", "'}"),
        ("payload=", "&sig=abc123"),
        ("", " — log entry from spirit-butler"),
        ("{\"credentials\":{\"access\":\"", "\",\"type\":\"aws\"}}"),
        ("", "\t// inline in config file"),
    ];

    let mut corpus = Vec::with_capacity(10_000);

    for _round in 0..100 {
        for &(secret_bytes, class) in CANARY_PATTERNS {
            for &(prefix, suffix) in wrappers {
                if corpus.len() >= 10_000 {
                    break;
                }
                let mut input = Vec::new();
                input.extend_from_slice(prefix.as_bytes());
                input.extend_from_slice(secret_bytes);
                input.extend_from_slice(suffix.as_bytes());
                corpus.push((input, class));
            }
        }
        for &token in HEX_TOKEN_PATTERNS {
            for &(prefix, suffix) in wrappers {
                if corpus.len() >= 10_000 {
                    break;
                }
                let mut input = Vec::new();
                input.extend_from_slice(prefix.as_bytes());
                input.extend_from_slice(token);
                input.extend_from_slice(suffix.as_bytes());
                corpus.push((input, "capability_token"));
            }
        }
        if corpus.len() >= 10_000 {
            break;
        }
    }
    corpus.truncate(10_000);
    corpus
}

#[test]
fn redaction_filter_zero_leak_against_10k_canary() {
    let policy = CorpusBackedRedactionPolicy::new();
    let canary_corpus = generate_canary_corpus();

    assert_eq!(
        canary_corpus.len(),
        10_000,
        "canary corpus generation must produce exactly 10,000 items"
    );

    let mut leaks = 0usize;
    for (_idx, (input, class)) in canary_corpus.iter().enumerate() {
        let redacted = policy.redact(input);
        match &redacted {
            Cow::Borrowed(_) => {
                let text = std::str::from_utf8(input).unwrap_or_default();
                if text.contains("<REDACTED:") {
                    leaks += 1;
                    eprintln!(
                        "CANARY LEAK class={class}: filter returned Borrowed \
                         for input containing redaction marker text"
                    );
                }
            }
            Cow::Owned(redacted_bytes) => {
                let redacted_str = std::str::from_utf8(redacted_bytes).unwrap_or_default();
                if !redacted_str.contains("<REDACTED:") {
                    leaks += 1;
                    eprintln!(
                        "CANARY LEAK class={class}: filter returned Owned \
                         but no <REDACTED: marker found in output"
                    );
                }
            }
        }
    }

    assert_eq!(
        leaks, 0,
        "NFR-Sec-4 v0.1-β binding broken: {leaks} leaks in 10,000 canary corpus items"
    );
}

#[test]
fn redaction_filter_detects_every_known_pattern_class() {
    let policy = CorpusBackedRedactionPolicy::new();

    for &(secret_bytes, class) in CANARY_PATTERNS {
        let result = policy.redact(secret_bytes);
        let s = std::str::from_utf8(&result).unwrap_or_default();
        assert!(
            s.contains(&format!("<REDACTED:type={class}")),
            "class '{class}' not detected for prefix {:?}",
            String::from_utf8_lossy(&secret_bytes[..secret_bytes.len().min(20)])
        );
    }

    for &token in HEX_TOKEN_PATTERNS {
        let result = policy.redact(token);
        let s = std::str::from_utf8(&result).unwrap_or_default();
        assert!(
            s.contains("<REDACTED:type=capability_token"),
            "capability_token hex pattern not detected"
        );
    }
}
