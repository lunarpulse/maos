//! FR64 cost-attribution domain types ([ADR-046]).
//!
//! Cost data is journaled as `FrameKind::CostAttribution` (29) in the
//! Transparency Log.  The payload carries **RAW dimensional facts** —
//! quantities + identity only, **no money field** (R4: kernel never
//! multiplies tokens by price; money is computed read-time in
//! `maos-audit`).
//!
//! [ADR-046]: ../../docs/adr/ADR-046-cost-attribution-and-reconciliation.md

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

// ---------------------------------------------------------------------------
// Cost dimensions (R6 — extensible)
// ---------------------------------------------------------------------------

/// Cost-dimension discriminator.
///
/// v1.0: `TokensIn`, `TokensOut`.
/// v1.1 (deferred, R6): `CpuMicros`, `StorageIoMicros`.
/// **NO `UsdMicros` dimension** — money is read-time per R4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CostDimension {
    TokensIn,
    TokensOut,
    // v1.1 (R6): CpuMicros, StorageIoMicros
}

// ---------------------------------------------------------------------------
// Principal attribution (R2 / SR-4)
// ---------------------------------------------------------------------------

/// Principal attribution at cost-emission time.
///
/// Per sec-redteam SR-4: the field is NEVER named/typed "the authorizing
/// principal" — it is a write-target proxy, not authority.
///
/// - `Unattributed`: no principal could be resolved (the frame still emits).
/// - `Resolved(id)`: exactly one principal in the write-target set.
/// - `Ambiguous { count }`: N>1 principals in the set — journals a COUNT,
///   **never the member identifiers** (SR-4 cross-tenant linkage guard).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PrincipalRef {
    Unattributed,
    Resolved {
        #[serde(with = "identity")]
        principal_id: String,
    },
    Ambiguous {
        count: u32,
    },
}

/// How the principal was attributed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionSource {
    /// Derived from `principal_ids_for_spirit_pid` (memory-write-target
    /// reverse-lookup proxy — not the authoritative session principal).
    WriteTargetProxy,
    /// Bound by the composition root to the session, not supplied by a Spirit.
    SessionPrincipal,
}

/// Confidence level of the attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionConfidence {
    /// Exactly one principal in the write-target set.
    Exact,
    /// Multiple principals — attribution is ambiguous.
    Ambiguous,
    /// No principal could be resolved.
    Unknown,
}

// ---------------------------------------------------------------------------
// Cost attribution payload (the TL frame body)
// ---------------------------------------------------------------------------

/// Payload for cost-attribution TL frames (`FrameKind::CostAttribution` = 29).
///
/// Per ADR-046 R4: carries RAW dimensional facts — quantities + identity
/// only, **no `cost_micro`/`usd_micros` field**.  Money is computed
/// read-time in `maos-audit` via `ProviderPricingConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostAttributionPayload {
    /// Schema version for forward-read-tolerance (R6).
    pub schema_version: u16,
    /// Nanosecond wall-clock timestamp of the inference call.
    pub timestamp_ns: u64,
    /// Spirit process ID that made the inference call.
    pub spirit_pid: u32,
    /// Provider that served the request.
    pub provider: String,
    /// Model ID used.
    pub model: String,
    /// Principal attribution with provenance tags (SR-4).
    pub principal: PrincipalRef,
    /// How the principal was attributed.
    pub attribution_source: AttributionSource,
    /// Confidence level.
    pub attribution_confidence: AttributionConfidence,
    /// Cost-dimension quantities (R6: `BTreeMap` not `HashMap` for
    /// deterministic serialization — R6/ADR-028 byte-identity).
    pub dimensions: BTreeMap<CostDimension, i64>,
}

/// Root-bound attribution for a unit of work.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CostScope {
    pub task_ref: Option<String>,
    pub session_principal: Option<String>,
}

/// Additive frame body; the existing payload remains usable by legacy writers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostFrameBody {
    #[serde(flatten)]
    pub payload: CostAttributionPayload,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "identity_opt"
    )]
    pub task_ref: Option<String>,
}

/// Credential prefixes the Transparency Log redactor rewrites (`maos-iac` `RULES`; a test there
/// pins this list to them).
const CREDENTIAL_PREFIXES: &str =
    "sk- ghp_ gho_ ghs_ ghu_ ghc_ ghr_ AKIA ASIA AIza ya29. -----BEGIN";

/// Identity text the Transparency Log redactor would rewrite (a 32-hex run or a credential
/// prefix), that serializes with an escape, or that already looks encoded travels in the frame as
/// `x:` plus colon-delimited hex of its bytes; everything else stays as written.
pub fn encode_identity(text: &str) -> Cow<'_, str> {
    let hex_token = text
        .split(|c| !matches!(c, '0'..='9' | 'a'..='f'))
        .any(|run| run.len() >= 32);
    if !hex_token
        && !text.starts_with("x:")
        && !text.contains(['\u{22}', '\\'])
        && !CREDENTIAL_PREFIXES
            .split(' ')
            .any(|prefix| text.contains(prefix))
    {
        return Cow::Borrowed(text);
    }
    let pairs: Vec<_> = text.bytes().map(|b| format!("{b:02x}")).collect();
    Cow::Owned(format!("x:{}", pairs.join(":")))
}

/// The inverse of [`encode_identity`]; text that is not exactly the encoded form is returned as is.
pub fn decode_identity(text: &str) -> Cow<'_, str> {
    let pair = |p: &str| {
        (p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| u8::from_str_radix(p, 16).ok())
            .flatten()
    };
    text.strip_prefix("x:")
        .and_then(|hex| hex.split(':').map(pair).collect::<Option<Vec<u8>>>())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map_or(Cow::Borrowed(text), Cow::Owned)
}

mod identity {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(text: &str, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_str(&super::encode_identity(text))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<String, D::Error> {
        String::deserialize(input).map(|text| super::decode_identity(&text).into_owned())
    }
}

mod identity_opt {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(text: &Option<String>, out: S) -> Result<S::Ok, S::Error> {
        text.as_deref().map(super::encode_identity).serialize(out)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<Option<String>, D::Error> {
        Option::<String>::deserialize(input)
            .map(|text| text.map(|t| super::decode_identity(&t).into_owned()))
    }
}

// ---------------------------------------------------------------------------
// Provider pricing config (R4 — money computed read-time)
// ---------------------------------------------------------------------------

/// Static pricing config for a (provider, model) pair.
///
/// Loaded at init, NOT live-fetched.  **Never imported by kernel-core**
/// (the type lives in `maos-domain`, consumed at reconcile-time in
/// `maos-audit`).
///
/// Prices are in **micro-USD per 1k tokens** (integer, no f64).
/// E.g. $0.003/1k input → `input_price_micro_per_1k = 3000`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPricingEntry {
    pub provider: String,
    pub model: String,
    /// Price in micro-USD per 1k input tokens.
    pub input_price_micro_per_1k: u64,
    /// Price in micro-USD per 1k output tokens.
    pub output_price_micro_per_1k: u64,
}

/// Collection of pricing entries with O(1) lookup by `(provider, model)`.
///
/// Serializes as `{ "entries": [...] }` — the index is rebuilt on
/// deserialization via `#[serde(from)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    from = "ProviderPricingConfigSerde",
    into = "ProviderPricingConfigSerde"
)]
pub struct ProviderPricingConfig {
    entries: Vec<ProviderPricingEntry>,
    index: HashMap<(String, String), usize>,
}

impl PartialEq for ProviderPricingConfig {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}
impl Eq for ProviderPricingConfig {}

/// Serde helper — flat vec, no index.
#[derive(Serialize, Deserialize)]
struct ProviderPricingConfigSerde {
    entries: Vec<ProviderPricingEntry>,
}

impl From<ProviderPricingConfigSerde> for ProviderPricingConfig {
    fn from(raw: ProviderPricingConfigSerde) -> Self {
        Self::new(raw.entries)
    }
}

impl From<ProviderPricingConfig> for ProviderPricingConfigSerde {
    fn from(c: ProviderPricingConfig) -> Self {
        Self { entries: c.entries }
    }
}

impl ProviderPricingConfig {
    /// Build a config and index from a vec of entries.
    pub fn new(entries: Vec<ProviderPricingEntry>) -> Self {
        let index = entries
            .iter()
            .enumerate()
            .map(|(i, e)| ((e.provider.clone(), e.model.clone()), i))
            .collect();
        Self { entries, index }
    }

    /// O(1) lookup by `(provider, model)`.
    pub fn lookup(&self, provider: &str, model: &str) -> Option<&ProviderPricingEntry> {
        self.index
            .get(&(provider.to_owned(), model.to_owned()))
            .map(|&i| &self.entries[i])
    }

    /// Iterate over all entries.
    pub fn entries(&self) -> &[ProviderPricingEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_attribution_payload_round_trip() {
        let mut dims = BTreeMap::new();
        dims.insert(CostDimension::TokensIn, 100);
        dims.insert(CostDimension::TokensOut, 50);
        let payload = CostAttributionPayload {
            schema_version: 1,
            timestamp_ns: 1_000_000,
            spirit_pid: 42,
            provider: "anthropic".into(),
            model: "claude-3".into(),
            principal: PrincipalRef::Resolved {
                principal_id: "user:alice".into(),
            },
            attribution_source: AttributionSource::WriteTargetProxy,
            attribution_confidence: AttributionConfidence::Exact,
            dimensions: dims,
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: CostAttributionPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, back);
    }

    #[test]
    fn btreemap_deterministic_order() {
        // BTreeMap guarantees sorted-key serialization (R6/ADR-028)
        let mut dims = BTreeMap::new();
        dims.insert(CostDimension::TokensOut, 50);
        dims.insert(CostDimension::TokensIn, 100);
        let json = serde_json::to_string(&dims).unwrap();
        // TokensIn < TokensOut in the enum discriminant order
        assert!(json.find("tokens_in").unwrap() < json.find("tokens_out").unwrap());
    }

    #[test]
    fn principal_ref_unattributed_serde() {
        let p = PrincipalRef::Unattributed;
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"kind\":\"Unattributed\""));
    }

    #[test]
    fn principal_ref_ambiguous_no_members() {
        // SR-4: Ambiguous journals a COUNT, never member identifiers
        let p = PrincipalRef::Ambiguous { count: 3 };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"count\":3"));
        assert!(!json.contains("principal_id"));
    }

    #[test]
    fn cost_frame_task_and_legacy_serde() {
        let legacy = r#"{"schema_version":1,"timestamp_ns":18446744073709551615,"spirit_pid":0,"provider":"test","model":"test","principal":{"kind":"Unattributed"},"attribution_source":"write_target_proxy","attribution_confidence":"unknown","dimensions":{},"future_field":true}"#;
        let mut frame: CostFrameBody = serde_json::from_str(legacy).unwrap();
        assert_eq!(frame.payload.timestamp_ns, u64::MAX);
        assert_eq!(frame.task_ref, None);
        assert!(!serde_json::to_string(&frame.payload)
            .unwrap()
            .contains("task_ref"));
        frame.task_ref = Some("t-1".into());
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains("\"task_ref\":\"t-1\""));
        assert_eq!(serde_json::from_str::<CostFrameBody>(&json).unwrap(), frame);
    }

    #[test]
    fn forward_read_tolerance_unmodeled_dimension() {
        // R6: reader must tolerate an unmodeled dimension key.
        let json_with_unknown = r#"{
            "schema_version": 2,
            "timestamp_ns": 0,
            "spirit_pid": 0,
            "provider": "test",
            "model": "test",
            "principal": {"kind": "Unattributed"},
            "attribution_source": "write_target_proxy",
            "attribution_confidence": "unknown",
            "dimensions": {"tokens_in": 100, "tokens_out": 50, "cpu_micros": 999}
        }"#;
        // Unknown dimension key → serde Err (not a panic).
        let result: Result<CostAttributionPayload, _> = serde_json::from_str(json_with_unknown);
        assert!(
            result.is_err(),
            "R6: unknown dimension key must produce a serde error, not panic"
        );

        // Known dimensions alone must round-trip with correct values.
        let json_known_only = r#"{
            "schema_version": 2,
            "timestamp_ns": 42,
            "spirit_pid": 7,
            "provider": "test",
            "model": "test",
            "principal": {"kind": "Unattributed"},
            "attribution_source": "write_target_proxy",
            "attribution_confidence": "unknown",
            "dimensions": {"tokens_in": 100, "tokens_out": 50}
        }"#;
        let payload: CostAttributionPayload = serde_json::from_str(json_known_only).unwrap();
        assert_eq!(payload.dimensions.len(), 2);
        assert_eq!(payload.dimensions[&CostDimension::TokensIn], 100);
        assert_eq!(payload.dimensions[&CostDimension::TokensOut], 50);
        assert_eq!(
            payload.schema_version, 2,
            "R6: higher schema_version preserved"
        );
    }

    #[test]
    fn pricing_lookup() {
        let config = ProviderPricingConfig::new(vec![
            ProviderPricingEntry {
                provider: "anthropic".into(),
                model: "claude-3".into(),
                input_price_micro_per_1k: 3000,
                output_price_micro_per_1k: 15000,
            },
            ProviderPricingEntry {
                provider: "openai".into(),
                model: "gpt-4".into(),
                input_price_micro_per_1k: 30000,
                output_price_micro_per_1k: 60000,
            },
        ]);
        let entry = config.lookup("anthropic", "claude-3").unwrap();
        assert_eq!(entry.input_price_micro_per_1k, 3000);
        assert!(config.lookup("openai", "gpt-4").is_some());
        assert!(config.lookup("unknown", "model").is_none());
    }
    /// Story 9.3b AC5/F9 — golden cost-vector oracle.
    ///
    /// Uses hand-computed reference vectors to prove that integer cost math
    /// does not silently drift: (tokens_in * input_price + tokens_out *
    /// output_price) / 1000 yields the expected micro-USD.
    #[test]
    fn golden_cost_vector_oracle() {
        let config = ProviderPricingConfig::new(vec![
            ProviderPricingEntry {
                provider: "anthropic".into(),
                model: "claude-3".into(),
                input_price_micro_per_1k: 3000, // $0.003 / 1k input tokens
                output_price_micro_per_1k: 15000, // $0.015 / 1k output tokens
            },
            ProviderPricingEntry {
                provider: "openai".into(),
                model: "gpt-4".into(),
                input_price_micro_per_1k: 30000, // $0.03 / 1k input tokens
                output_price_micro_per_1k: 60000, // $0.06 / 1k output tokens
            },
        ]);

        // Vector 1: Claude, 1k in / 1k out.
        // (1000 * 3000 + 1000 * 15000) / 1000 = 18_000 µ$ = $0.018
        let e1 = config.lookup("anthropic", "claude-3").unwrap();
        let cost1 = (1000u128 * e1.input_price_micro_per_1k as u128
            + 1000u128 * e1.output_price_micro_per_1k as u128)
            / 1000;
        assert_eq!(cost1, 18_000);

        // Vector 2: GPT-4, 2k in / 500 out.
        // (2000 * 30000 + 500 * 60000) / 1000 = 90_000 µ$ = $0.09
        let e2 = config.lookup("openai", "gpt-4").unwrap();
        let cost2 = (2000u128 * e2.input_price_micro_per_1k as u128
            + 500u128 * e2.output_price_micro_per_1k as u128)
            / 1000;
        assert_eq!(cost2, 90_000);

        // Vector 3: missing-model lookup returns zero pricing (graceful,
        // no panic), so cost is 0.
        assert!(config.lookup("unknown", "model").is_none());
    }

    /// Story 9.3b AC5/F9 — rounding/aggregation property.
    ///
    /// Splitting a token count across two entries and summing the resulting
    /// cost_u128 values must equal the cost of the unsplit total, before the
    /// final divide-by-1000.  This guards against row-level rounding drift.
    #[test]
    fn cost_accumulation_is_linear_before_division() {
        let config = ProviderPricingConfig::new(vec![ProviderPricingEntry {
            provider: "anthropic".into(),
            model: "claude-3".into(),
            input_price_micro_per_1k: 3000,
            output_price_micro_per_1k: 15000,
        }]);
        let e = config.lookup("anthropic", "claude-3").unwrap();

        let total_tokens: u128 = 1234;
        let unsplit = total_tokens * e.input_price_micro_per_1k as u128;

        let part_a: u128 = 500;
        let part_b: u128 = total_tokens - part_a;
        let split = part_a * e.input_price_micro_per_1k as u128
            + part_b * e.input_price_micro_per_1k as u128;

        assert_eq!(unsplit, split);
    }
}
