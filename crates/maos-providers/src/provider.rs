//! Internal provider-driver abstraction.
//!
//! `Provider` is the uniform surface that `maos-providers` drivers implement.
//! It is deliberately narrower than `maos-domain::ports::InferencePort` —
//! the domain port carries capability/telemetry/audit concerns; the driver
//! trait is pure request/response translation over an injected transport.

use maos_domain::ports::inference::{InferenceRequest, InferenceResponse};

/// Provider driver trait — implemented by each LLM backend (Anthropic, OpenAI, …).
///
/// Story 8.11 — `Send + Sync` so an `Arc<dyn Provider>` (hence `MultiProviderRouter`
/// and `InferencePortAdapter`) is `Send + Sync` and can be injected into a Spirit
/// dispatched across the multi-threaded runtime (the Researcher live-inference
/// seam). Every concrete driver is already thread-safe (stateless HTTP clients /
/// `Mutex`-wrapped state), so this only makes the existing guarantee explicit.
pub trait Provider: Send + Sync {
    /// Perform a single completion call.
    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError>;

    /// Story 6.4 / NFR-Scale-4 — return a 64-bit fingerprint of this driver's
    /// credential bytes for cross-credential isolation in the rate-limit
    /// bucket map. The default returns `u64::MAX` as a sentinel; each concrete
    /// provider MUST override with `first-8-bytes-of-sha256(api_key)`. The
    /// sentinel ensures that a driver forgetting to override shares a bucket
    /// only with other unconfigured drivers (same type), NOT with real keys.
    /// Ollama (which has no api_key) overrides with a hash of `base_url`.
    fn credential_fingerprint(&self) -> u64 {
        u64::MAX
    }
}

/// Provider-level error — concrete variants, no blanket `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// HTTP transport failure.
    #[error("transport: {}", BoundedError(.0))]
    Transport(String),
    /// Provider returned a non-success status code.
    #[error("provider rejected: status={status}, body={}", BoundedError(.body))]
    ProviderRejected { status: u16, body: String },
    /// Request or response body could not be serialized / deserialized.
    #[error("serde: {}", BoundedError(.0))]
    Serde(String),
    /// Provider is not configured (missing API key, etc.).
    #[error("unconfigured")]
    Unconfigured,
}

/// Refuse header-unsafe credentials without formatting their bytes.
pub(crate) fn validate_api_key(
    key: maos_domain::ports::SecretKey,
    value: &str,
) -> Result<(), ProviderError> {
    if !value.is_empty() && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        Ok(())
    } else {
        Err(ProviderError::Transport(format!(
            "{}: invalid credential bytes (value withheld)",
            key.as_str()
        )))
    }
}

struct BoundedError<'a>(&'a str);
impl std::fmt::Display for BoundedError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        let mut bytes = 0;
        for ch in self.0.chars() {
            bytes += ch.len_utf8();
            if bytes > 512 {
                break;
            }
            if !ch.is_control() {
                f.write_char(ch)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn error_text(text: &str) -> String {
    BoundedError(text).to_string()
}

pub(crate) fn transport_error(error: maos_domain::ports::io_subsystem::IoError) -> ProviderError {
    let text = match error {
        maos_domain::ports::io_subsystem::IoError::Transport(text) => text,
        other => other.to_string(),
    };
    ProviderError::Transport(error_text(&text))
}

/// A credential whose alphanumeric skeleton is shorter than this cannot be
/// told apart from vendor prose, so the excerpt is withheld. Real keys are far
/// longer (Anthropic `sk-ant-api03-…` ~108, OpenAI `sk-…` 48+, Gemini `AIza…`
/// 39, Kimi `sk-…` ~51).
const MIN_CREDENTIAL_SKELETON: usize = 16;
/// `maos-kernel-core` cuts a non-2xx body at this many bytes; a body this long
/// may be truncated.
const TRANSPORT_BODY_CAP: usize = 10 * 1024 * 1024;

/// POST and return the body of a 2xx; any other status is a typed
/// [`ProviderError::ProviderRejected`] carrying a bounded (512-byte) excerpt of
/// the vendor's own error body (see [`rejection_excerpt`]), so its reason
/// survives and 4xx is never mistaken for a (retryable) transport fault.
pub(crate) fn post_json(
    transport: &dyn maos_domain::ports::IoSubsystemPort,
    url: &str,
    body: &serde_json::Value,
    headers: &[(&str, &str)],
) -> Result<Vec<u8>, ProviderError> {
    let bytes = serde_json::to_vec(body).map_err(|e| ProviderError::Serde(e.to_string()))?;
    let reply = transport
        .http_post_exchange(url, &bytes, headers, "retry-after")
        .map_err(transport_error)?;
    if (200..=299).contains(&reply.status) {
        return Ok(reply.body);
    }
    Err(ProviderError::ProviderRejected {
        status: reply.status,
        body: rejection_excerpt(&reply.body, headers),
    })
}

/// The bounded excerpt of a rejection `body`: detect a credential echo and
/// withhold, otherwise render and truncate. Nothing is redacted in place, so
/// no spelling of a key can survive by being split, escaped or re-joined.
///
/// A credential's *skeleton* is its ASCII-alphanumeric characters in order
/// (every non-benign header value, and the `Bearer` token). Separators,
/// controls, markers and escape punctuation are dropped from both sides, so a
/// key interleaved with any of them still matches. The excerpt is withheld
/// (status kept) when a skeleton occurs in the skeleton of the raw body, of
/// what would be shown, or of the decoded JSON scalars walked in **document
/// order** (not `Value` order, which sorts members): keys, strings and numbers;
/// strings and numbers; strings alone (decoded: `\/`, `\uXXXX` and controls are
/// already resolved). A masked vendor echo (`sk-abc***xyz`) does not match and
/// stays readable; a false positive costs only the excerpt (that includes a
/// duplicate member the parser discards: it is walked, though not rendered).
///
/// It is also withheld when a skeleton is under [`MIN_CREDENTIAL_SKELETON`], the
/// body reaches the transport cap, the body looks like JSON (first
/// non-whitespace byte `{` or `[`) but does not parse (truncated, or nested
/// past serde_json's recursion limit, so it cannot be walked), or the body is
/// not JSON yet has a `\` (an escape such as `\u0073` or `\n` that this code
/// cannot decode).
///
/// Shown text: JSON is re-rendered compactly from the parsed value (discarded
/// duplicate members are gone), any other body is lossy UTF-8; control
/// characters become spaces; then a UTF-8-safe cut at 512 bytes.
fn rejection_excerpt(body: &[u8], headers: &[(&str, &str)]) -> String {
    let benign = ["content-type", "accept", "anthropic-version"];
    let secrets: Vec<String> = headers
        .iter()
        .filter(|(name, _)| !benign.iter().any(|b| name.eq_ignore_ascii_case(b)))
        .flat_map(|&(_, value)| [value, value.rsplit(' ').next().unwrap_or(value)])
        .filter(|secret| !secret.is_empty())
        .map(skeleton)
        .collect();
    let withhold = |why: &str| format!("[body withheld: {why}]");
    if secrets.iter().any(|s| s.len() < MIN_CREDENTIAL_SKELETON) {
        return withhold("a request credential is too weak to check for echoes");
    }
    if body.len() >= TRANSPORT_BODY_CAP {
        return withhold("body reached the transport size cap");
    }
    let lossy = String::from_utf8_lossy(body);
    let (shown, decoded) = match serde_json::from_slice::<serde_json::Value>(body) {
        Ok(mut json) => {
            let Some(seen) = walk(body) else {
                return withhold("unparsable body");
            };
            scrub(&mut json);
            (json.to_string(), seen)
        }
        // Looks like JSON yet does not parse (truncated, or nested past the
        // recursion limit): its fields cannot be walked, so fail closed.
        Err(_) if looks_like_json(body) => return withhold("unparsable JSON body"),
        Err(_) if body.contains(&b'\\') => return withhold("unparsable body with escapes"),
        Err(_) => (blank(&lossy), Seen::default()),
    };
    let haystacks = [
        skeleton(&lossy),
        skeleton(&shown),
        decoded.keys_and_scalars,
        decoded.scalars,
        decoded.strings,
    ];
    if secrets
        .iter()
        .any(|s| haystacks.iter().any(|h| h.contains(s.as_str())))
    {
        return withhold("credential echo detected");
    }
    let mut end = shown.len().min(512);
    while !shown.is_char_boundary(end) {
        end -= 1;
    }
    shown[..end].to_owned()
}

/// The first non-whitespace byte of `body` opens a JSON object or array.
fn looks_like_json(body: &[u8]) -> bool {
    matches!(
        body.iter().find(|b| !b.is_ascii_whitespace()),
        Some(b'{' | b'[')
    )
}

/// The ASCII-alphanumeric characters of `text`, in order.
fn skeleton(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphanumeric).collect()
}

/// `text` with every control character replaced by a space.
fn blank(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// [`blank`] every decoded string value and object key of `value` (display only).
fn scrub(value: &mut serde_json::Value) {
    use serde_json::Value;
    match value {
        Value::String(text) => *text = blank(text),
        Value::Array(items) => items.iter_mut().for_each(scrub),
        Value::Object(map) => {
            *map = std::mem::take(map)
                .into_iter()
                .map(|(key, mut v)| {
                    scrub(&mut v);
                    (blank(&key), v)
                })
                .collect();
        }
        _ => {}
    }
}

/// Skeletons of the decoded scalars of a JSON document, in **document
/// order** (`serde_json::Value` sorts object members, which would put a
/// later field's value before an earlier one's).
#[derive(Default)]
struct Seen {
    /// Keys, strings and numbers.
    keys_and_scalars: String,
    /// Strings and numbers (a split over fields has other fields' keys
    /// between the pieces in `keys_and_scalars` only).
    scalars: String,
    /// Strings only (unrelated numbers may sit between the pieces).
    strings: String,
}

impl Seen {
    fn key(&mut self, key: &str) {
        self.keys_and_scalars.push_str(&skeleton(key));
    }

    fn string(&mut self, text: &str) {
        let skeleton = skeleton(text);
        self.keys_and_scalars.push_str(&skeleton);
        self.scalars.push_str(&skeleton);
        self.strings.push_str(&skeleton);
    }

    fn number(&mut self, text: &str) {
        let skeleton = skeleton(text);
        self.keys_and_scalars.push_str(&skeleton);
        self.scalars.push_str(&skeleton);
    }
}

/// Walks one JSON document in order, recording into a [`Seen`]; booleans and
/// `null` are skipped. Depth is bounded by serde_json's recursion limit.
struct Walk<'a>(&'a mut Seen);

impl<'de> serde::de::DeserializeSeed<'de> for Walk<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for Walk<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, n: u64) -> Result<(), E> {
        self.0.number(&n.to_string());
        Ok(())
    }

    fn visit_i64<E>(self, n: i64) -> Result<(), E> {
        self.0.number(&n.to_string());
        Ok(())
    }

    fn visit_f64<E>(self, n: f64) -> Result<(), E> {
        if let Some(number) = serde_json::Number::from_f64(n) {
            self.0.number(&number.to_string());
        }
        Ok(())
    }

    fn visit_str<E>(self, text: &str) -> Result<(), E> {
        self.0.string(text);
        Ok(())
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq.next_element_seed(Walk(&mut *self.0))?.is_some() {}
        Ok(())
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            self.0.key(&key);
            map.next_value_seed(Walk(&mut *self.0))?;
        }
        Ok(())
    }
}

/// The document-order [`Seen`] of `body`, or `None` when it does not parse.
fn walk(body: &[u8]) -> Option<Seen> {
    use serde::de::DeserializeSeed;
    let mut seen = Seen::default();
    let mut de = serde_json::Deserializer::from_slice(body);
    Walk(&mut seen).deserialize(&mut de).ok()?;
    de.end().ok()?;
    Some(seen)
}

pub(crate) fn usage_count(
    usage: Option<&serde_json::Value>,
    key: &str,
) -> Result<u32, ProviderError> {
    let count = usage
        .and_then(|u| u.get(key))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ProviderError::Serde(format!("missing or invalid usage.{key}")))?;
    u32::try_from(count).map_err(|_| ProviderError::Serde(format!("usage.{key} exceeds u32")))
}

pub(crate) fn vendor_stop(reason: &str) -> maos_domain::ports::inference::StopReason {
    maos_domain::ports::inference::StopReason::ProviderStop(
        if reason.len() > 64 {
            "vendor stop reason exceeded 64 bytes"
        } else {
            reason
        }
        .into(),
    )
}
