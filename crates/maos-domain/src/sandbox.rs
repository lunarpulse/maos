#![forbid(unsafe_code)]

//! Sandbox domain types — T3 container isolation attestation artifacts,
//! image-pin chain, and operator-facing error catalog.
//!
//! Per architecture §4.0.9 dependency-triangle rule, this module lives in
//! `maos-domain::sandbox` so operator HTTP API (Story 5.5a/9.4), CLI
//! (Story 5.5a `maosctl spirit inspect --sandbox`), and future MCP
//! registry (Story 5.5d) can consume the surface without depending on
//! `maos-kernel-core`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// T3ImageAttestation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct T3ImageAttestation {
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub id: ImageAttestationId,
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub schema_version: u32,
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub signed_at_ns: u64,
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub entries: Vec<T3ImageEntry>,
    #[serde(with = "serde_sig64")]
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub signature: [u8; 64],
    #[serde(with = "serde_pubkey32")]
    #[doc = "Construct via [`T3ImageAttestation::new`] to enforce id/signature/sha validation; struct literals bypass schema checks."]
    pub signer_pub_key: [u8; 32],
}

mod serde_sig64 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(sig: &[u8; 64], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(sig)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 64], D::Error> {
        struct SigVisitor;
        impl<'de> serde::de::Visitor<'de> for SigVisitor {
            type Value = [u8; 64];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 64-byte array")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 64], A::Error> {
                let mut arr = [0u8; 64];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq.next_element()?.ok_or_else(|| {
                        serde::de::Error::custom(format!(
                            "expected 64-byte signature, got {i} bytes"
                        ))
                    })?;
                }
                Ok(arr)
            }
        }
        de.deserialize_tuple(64, SigVisitor)
    }
}

mod serde_pubkey32 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(key: &[u8; 32], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(key)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 32], D::Error> {
        struct PubkeyVisitor;
        impl<'de> serde::de::Visitor<'de> for PubkeyVisitor {
            type Value = [u8; 32];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 32-byte array")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 32], A::Error> {
                let mut arr = [0u8; 32];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq.next_element()?.ok_or_else(|| {
                        serde::de::Error::custom(format!("expected 32-byte pubkey, got {i} bytes"))
                    })?;
                }
                Ok(arr)
            }
        }
        de.deserialize_tuple(32, PubkeyVisitor)
    }
}

mod serde_sha256 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(sha: &[u8; 32], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(sha)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 32], D::Error> {
        struct ShaVisitor;
        impl<'de> serde::de::Visitor<'de> for ShaVisitor {
            type Value = [u8; 32];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a 32-byte SHA-256 digest")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 32], A::Error> {
                let mut arr = [0u8; 32];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq.next_element()?.ok_or_else(|| {
                        serde::de::Error::custom(format!("expected 32-byte SHA-256, got {i} bytes"))
                    })?;
                }
                Ok(arr)
            }
        }
        de.deserialize_tuple(32, ShaVisitor)
    }
}

impl T3ImageAttestation {
    /// Construct a `T3ImageAttestation` with validation.
    ///
    /// Enforces:
    /// - `entries` is non-empty.
    /// - Exactly zero or one entry has `default_for_v05 = true`.
    /// - `signature` and `signer_pub_key` are non-zero.
    /// - `schema_version` is 1.
    ///
    /// The `id` is a SHA-256 digest computed by the caller from
    /// `serde_json::to_vec(&entries)` — matching the canonicalization
    /// pattern from Story 5.4's `CrlId`.
    pub fn new(
        id: ImageAttestationId,
        schema_version: u32,
        signed_at_ns: u64,
        entries: Vec<T3ImageEntry>,
        signature: [u8; 64],
        signer_pub_key: [u8; 32],
    ) -> Result<Self, T3Error> {
        if entries.is_empty() {
            return Err(T3Error::SignatureInvalid);
        }

        let default_count = entries.iter().filter(|e| e.default_for_v05).count();
        if default_count > 1 {
            return Err(T3Error::SignatureInvalid);
        }

        if signature == [0u8; 64] {
            return Err(T3Error::SignatureInvalid);
        }
        if signer_pub_key == [0u8; 32] {
            return Err(T3Error::SignatureInvalid);
        }
        if schema_version != 1 {
            return Err(T3Error::UnsupportedSchemaVersion {
                version: schema_version,
            });
        }

        Ok(Self {
            id,
            schema_version,
            signed_at_ns,
            entries,
            signature,
            signer_pub_key,
        })
    }
}

// ---------------------------------------------------------------------------
// T3ImageEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct T3ImageEntry {
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub image_uri: String,
    #[serde(with = "serde_sha256")]
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub image_sha256: [u8; 32],
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub description: String,
    #[doc = "Construct via [`T3ImageEntry::new`] to enforce non-empty image_uri and well-formed image_sha256."]
    pub default_for_v05: bool,
}

impl T3ImageEntry {
    /// Construct a `T3ImageEntry` with validation.
    ///
    /// Enforces:
    /// - `image_uri` is non-empty.
    /// - `image_sha256` is non-zero.
    pub fn new(
        image_uri: impl Into<String>,
        image_sha256: [u8; 32],
        description: impl Into<String>,
        default_for_v05: bool,
    ) -> Result<Self, T3Error> {
        let image_uri = image_uri.into();
        if image_uri.is_empty() {
            return Err(T3Error::SignatureInvalid);
        }
        if image_sha256 == [0u8; 32] {
            return Err(T3Error::SignatureInvalid);
        }
        Ok(Self {
            image_uri,
            image_sha256,
            description: description.into(),
            default_for_v05,
        })
    }
}

// ---------------------------------------------------------------------------
// ImageAttestationId
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ImageAttestationId(#[serde(with = "serde_sha256")] pub [u8; 32]);

impl std::fmt::Display for ImageAttestationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// ---------------------------------------------------------------------------
// ContainerRuntimeKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContainerRuntimeKind {
    Podman,
    Docker,
}

impl ContainerRuntimeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Podman => "podman",
            Self::Docker => "docker",
        }
    }
}

// ---------------------------------------------------------------------------
// T3Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum T3Error {
    #[error(
        "no container runtime found (tried podman, docker); install one or set MAOS_T3_RUNTIME=none to disable T3"
    )]
    RuntimeUnavailable,
    #[error("no container runtime found: podman: {podman}; docker: {docker}")]
    RuntimeUnavailableDiagnostics { podman: String, docker: String },
    #[error("image SHA mismatch: expected {expected}, observed {observed}")]
    ImageMismatch { expected: String, observed: String },
    #[error("image attestation signature invalid")]
    SignatureInvalid,
    #[error("image attestation trust anchor mismatch")]
    TrustAnchorMismatch,
    #[error("T3 image pin '{name}' is not present in the verified image lock")]
    ImagePinMissing { name: String },
    #[error("T3 image trust anchor unavailable: {0}")]
    TrustAnchorMissing(String),
    #[error("image attestation unsupported schema version {version}")]
    UnsupportedSchemaVersion { version: u32 },
    #[error("T3 image lock is a shipped placeholder and cannot be used for admission")]
    PlaceholderImageLock,
    #[error("no default T3 image in t3-image.lock")]
    NoDefaultImage,
    #[error("container spawn failed: {0}")]
    Spawn(String),
    #[error("container runtime inspect failed: {0}")]
    Inspect(String),
    #[error("io error: {0}")]
    Io(String),
    /// Story 17-7 AC2.1 — the per-spawn state directory is not fresh, private,
    /// euid-owned and empty; refused before any runtime call.
    #[error("T3 state directory refused: {0}")]
    StateDir(&'static str),
    /// Story 17-7 AC2.1/AC4.1 — startup refused after the CID was owned; only
    /// that CID is cleaned up. Phase and reason are typed, never runtime text.
    #[error("T3 launch refused at {phase} for container {container_id}: {reason}")]
    LaunchRefused {
        phase: &'static str,
        container_id: String,
        reason: &'static str,
    },
    /// Story 17-7 AC4.1 — no valid private CID receipt: a container may exist
    /// but nothing authorizes a destructive call. The state dir is retained.
    #[error("T3 creation ownership unknown at {phase}; state retained at {locator}")]
    CreationOwnershipUnknown {
        phase: &'static str,
        locator: String,
    },
}

// ---------------------------------------------------------------------------
// Story 17-7 — container terminal facts
// ---------------------------------------------------------------------------

/// A PID observed once. Never a liveness claim and never a kill target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ObservedPid {
    /// The runtime-written pidfile read after `init`, before the payload ran.
    Historical(u32),
}

/// The payload's own exit, as far as the evidence proves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadExit {
    Unknown,
    Exited(u8),
    Signaled(u8),
}

impl PayloadExit {
    /// Decode a raw wait status; `None` unless it is terminal
    /// (`WIFEXITED`/`WIFSIGNALED`).
    pub fn from_wait_status(raw: i32) -> Option<Self> {
        match raw & 0x7f {
            0 => Some(Self::Exited(((raw >> 8) & 0xff) as u8)),
            0x7f => None,
            signal => Some(Self::Signaled(signal as u8)),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Unknown => "unknown".into(),
            Self::Exited(code) => format!("exited(code={code})"),
            Self::Signaled(signal) => format!("signaled(sig={signal})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadExitSource {
    Pidfd,
    Runtime,
    None,
}

/// Why no terminal runtime observation exists. Bounded and non-secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalUnavailable {
    InspectFailed,
    InspectTimedOut,
    MalformedState,
    NonTerminal,
    DeadlineExpired,
}

impl TerminalUnavailable {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InspectFailed => "inspect failed",
            Self::InspectTimedOut => "inspect timed out",
            Self::MalformedState => "malformed state",
            Self::NonTerminal => "container not terminal",
            Self::DeadlineExpired => "snapshot deadline expired",
        }
    }
}

/// A typed terminal `.State` (`exited`/`stopped`). `State.Error` is reduced
/// to whether the runtime reported one; its text is discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerState {
    pub status: String,
    /// Raw runtime code; `None` when it is not representable as `i32`.
    pub exit_code: Option<i32>,
    pub oom_marker: bool,
    pub error_reported: bool,
    pub started_at: String,
    pub finished_at: String,
}

impl ContainerState {
    /// Parse `{{json .State}}`: every required field must have its type; an
    /// integer code outside `i32` is kept as `None`, a non-integer refuses.
    pub fn from_inspect_json(json: &[u8]) -> Option<Self> {
        let state: serde_json::Value = serde_json::from_slice(json).ok()?;
        let text = |key: &str| state.get(key)?.as_str().map(str::to_owned);
        let code = state.get("ExitCode")?;
        let exit_code = match (code.as_i64(), code.as_u64()) {
            (Some(code), _) => i32::try_from(code).ok(),
            (None, Some(_)) => None,
            _ => return None,
        };
        Some(Self {
            status: text("Status")?,
            exit_code,
            oom_marker: state.get("OOMKilled")?.as_bool()?,
            error_reported: !text("Error")?.is_empty(),
            started_at: text("StartedAt")?,
            finished_at: text("FinishedAt")?,
        })
    }

    /// Only `exited`/`stopped` may be decoded as an exit.
    pub fn is_terminal(&self) -> bool {
        matches!(self.status.as_str(), "exited" | "stopped")
    }
}

/// A runtime pidfile: ASCII digits plus at most one final newline, `1..=i32::MAX`.
pub fn parse_pidfile(bytes: &[u8]) -> Option<u32> {
    let digits = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if digits.is_empty() || digits.len() > 10 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let pid: u64 = std::str::from_utf8(digits).ok()?.parse().ok()?;
    u32::try_from(pid)
        .ok()
        .filter(|pid| (1..=i32::MAX as u32).contains(pid))
}

/// A CID receipt: exactly 64 lowercase hex plus at most one final newline.
pub fn parse_container_id(bytes: &[u8]) -> Option<String> {
    let hex = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let valid = hex.len() == 64 && hex.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    valid.then(|| String::from_utf8_lossy(hex).into_owned())
}

/// `/proc/<pid>/cgroup` names `cid` in an exact path component (systemd
/// `libpod-<cid>.scope`, cgroupfs `libpod-<cid>`, split `libpod-payload-<cid>`).
pub fn cgroup_names_container(cgroup: &str, cid: &str) -> bool {
    let names = [
        format!("libpod-{cid}.scope"),
        format!("libpod-{cid}"),
        format!("libpod-payload-{cid}"),
    ];
    cgroup
        .lines()
        .filter_map(|line| line.splitn(3, ':').nth(2))
        .flat_map(|path| path.split('/'))
        .any(|part| names.iter().any(|name| name == part))
}

/// The one immutable terminal snapshot both classifiers read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerTerminalFacts {
    pub state: Result<ContainerState, TerminalUnavailable>,
    pub payload: PayloadExit,
    pub source: PayloadExitSource,
}

impl ContainerTerminalFacts {
    /// Apply the decoding rule: a verified pidfd status wins; otherwise only an
    /// unambiguous terminal runtime code (`0..=128`, `193..=255`) decodes.
    pub fn new(
        state: Result<ContainerState, TerminalUnavailable>,
        pidfd: Option<PayloadExit>,
    ) -> Self {
        let runtime = state.as_ref().ok().and_then(|s| s.exit_code);
        let (payload, source) = match (pidfd, runtime) {
            (Some(exit), _) => (exit, PayloadExitSource::Pidfd),
            (None, Some(code @ (0..=128 | 193..=255))) => {
                (PayloadExit::Exited(code as u8), PayloadExitSource::Runtime)
            }
            _ => (PayloadExit::Unknown, PayloadExitSource::None),
        };
        Self {
            state,
            payload,
            source,
        }
    }

    /// §Classification (T3), excluding the `PlannedStop` latch the caller owns.
    pub fn classify(&self, stderr_tail: Option<String>) -> crate::supervision::CrashCause {
        use crate::supervision::{CrashCause, FaultCause};
        let truncated = |why: &str| {
            CrashCause::Fault(FaultCause::Truncated {
                reason: format!("container terminal facts: {why}"),
            })
        };
        let state = match &self.state {
            Ok(state) => state,
            Err(why) => return truncated(why.as_str()),
        };
        let fault = CrashCause::Fault;
        match (self.payload, state.exit_code) {
            (PayloadExit::Exited(0), _) => CrashCause::Voluntary,
            (PayloadExit::Exited(code), _) => fault(FaultCause::NonZeroExit {
                code: code.into(),
                stderr_tail,
            }),
            (PayloadExit::Signaled(9), _) if state.oom_marker => fault(FaultCause::OomKilled {
                signal: Some(9),
                stderr_tail,
            }),
            (PayloadExit::Signaled(signal), _) => fault(FaultCause::SignaledByKernel {
                signal: signal.into(),
                stderr_tail,
            }),
            (PayloadExit::Unknown, Some(137)) if state.oom_marker => fault(FaultCause::OomKilled {
                signal: None,
                stderr_tail,
            }),
            (PayloadExit::Unknown, Some(code @ 129..=192)) => {
                fault(FaultCause::NonZeroExit { code, stderr_tail })
            }
            (PayloadExit::Unknown, _) => truncated("no valid exit evidence"),
        }
    }

    /// The verified payload exit code the completion oracle may use.
    pub fn completion_exit_code(&self) -> Option<i32> {
        match (&self.state, self.payload) {
            (Ok(_), PayloadExit::Exited(code)) => Some(code.into()),
            _ => None,
        }
    }
}

/// The first unresolved teardown failure, in stage order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerResidue {
    KillFailed,
    TerminationUnconfirmed,
    RemoveFailed,
    RemoveTimeout,
    ExistsFailed,
    StillExists,
    VolumeCleanupUnconfirmed,
    RuntimeClientUnconfirmed,
}

/// The exact `container` object of a T3 `cli.subprocess.exit` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerExitRecord {
    pub runtime_exit_code: Option<i32>,
    pub oom_marker: Option<bool>,
    pub payload_exit: String,
    pub payload_exit_source: PayloadExitSource,
    pub removed: bool,
    pub residue: Option<ContainerResidue>,
}

impl ContainerExitRecord {
    pub fn new(facts: &ContainerTerminalFacts, verdict: Result<(), ContainerResidue>) -> Self {
        let state = facts.state.as_ref().ok();
        Self {
            runtime_exit_code: state.and_then(|s| s.exit_code),
            oom_marker: state.map(|s| s.oom_marker),
            payload_exit: facts.payload.describe(),
            payload_exit_source: facts.source,
            removed: verdict.is_ok(),
            residue: verdict.err(),
        }
    }

    /// The §Exit-row schema cross-field invariants.
    pub fn is_consistent(&self) -> bool {
        let payload = &self.payload_exit;
        let decoded = |prefix: &str, max: u16, min: u16| {
            payload
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix(')'))
                .filter(|n| n.bytes().all(|b| b.is_ascii_digit()))
                .filter(|n| !n.starts_with('0') || n.len() == 1)
                .and_then(|n| n.parse::<u16>().ok())
                .filter(|n| (min..=max).contains(n))
        };
        let exited = decoded("exited(code=", 255, 0);
        let signaled = decoded("signaled(sig=", 64, 1);
        let source_ok = match self.payload_exit_source {
            PayloadExitSource::None => payload == "unknown",
            PayloadExitSource::Pidfd => exited.is_some() || signaled.is_some(),
            // Runtime evidence decodes only where unambiguous (never 128+N).
            PayloadExitSource::Runtime => exited.is_some_and(|code| {
                !(129..=192).contains(&code) && self.runtime_exit_code == Some(i32::from(code))
            }),
        };
        source_ok && self.removed == self.residue.is_none()
    }
}

// ---------------------------------------------------------------------------
// SandboxInspectReport
// ---------------------------------------------------------------------------

/// Startup report served by operator HTTP. Nothing here claims liveness: the
/// kill authorities are the owned CID and the verified payload pidfd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxInspectReport {
    pub spirit_id: String,
    pub payload_pid: ObservedPid,
    /// 64 lowercase hex.
    pub container_id: String,
    pub runtime: String,
    pub image_sha: String,
    pub applied_t2_protections: T2ProtectionSummary,
    pub strictest_of_reasoning: StrictestOfReasoning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct T2ProtectionSummary {
    pub landlock_rules: usize,
    pub seccomp_allow_count: usize,
    pub seccomp_kill_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictestOfReasoning {
    pub manifest_tier: String,
    pub trust_tier_floor: String,
    pub operator_policy_floor: String,
    pub effective_tier: String,
    pub dominant_axis: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t3_image_attestation_new_rejects_empty_entries() {
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_zero_signature() {
        let entry =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test", false).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![entry],
            [0u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_zero_pubkey() {
        let entry =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test", false).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![entry],
            [1u8; 64],
            [0u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_multiple_defaults() {
        let e1 = T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "one", true).unwrap();
        let e2 =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime-2", [2u8; 32], "two", true).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            1,
            0,
            vec![e1, e2],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_attestation_new_rejects_wrong_schema_version() {
        let entry =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test", false).unwrap();
        let err = T3ImageAttestation::new(
            ImageAttestationId([1u8; 32]),
            2,
            0,
            vec![entry],
            [1u8; 64],
            [1u8; 32],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            T3Error::UnsupportedSchemaVersion { version: 2 }
        ));
    }

    #[test]
    fn t3_image_attestation_new_happy_path() {
        let entry = T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [0xAB; 32], "test desc", true)
            .unwrap();
        let attestation = T3ImageAttestation::new(
            ImageAttestationId([0xCD; 32]),
            1,
            42,
            vec![entry],
            [1u8; 64],
            [2u8; 32],
        )
        .unwrap();
        assert_eq!(attestation.id.0, [0xCD; 32]);
        assert_eq!(attestation.schema_version, 1);
        assert_eq!(attestation.signed_at_ns, 42);
        assert_eq!(attestation.signature, [1u8; 64]);
        assert_eq!(attestation.signer_pub_key, [2u8; 32]);
    }

    #[test]
    fn t3_image_entry_new_rejects_empty_uri() {
        let err = T3ImageEntry::new("", [1u8; 32], "test", false).unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_entry_new_rejects_zero_sha() {
        let err =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [0u8; 32], "test", false).unwrap_err();
        assert!(matches!(err, T3Error::SignatureInvalid));
    }

    #[test]
    fn t3_image_entry_new_happy_path() {
        let e =
            T3ImageEntry::new("ghcr.io/maos/spirit-runtime", [1u8; 32], "test desc", true).unwrap();
        assert_eq!(e.image_uri, "ghcr.io/maos/spirit-runtime");
        assert_eq!(e.image_sha256, [1u8; 32]);
        assert_eq!(e.description, "test desc");
        assert!(e.default_for_v05);
    }

    #[test]
    fn image_attestation_id_display_hex() {
        let id = ImageAttestationId([0xAB; 32]);
        let s = id.to_string();
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn container_runtime_kind_as_str() {
        assert_eq!(ContainerRuntimeKind::Podman.as_str(), "podman");
        assert_eq!(ContainerRuntimeKind::Docker.as_str(), "docker");
    }

    #[test]
    fn container_runtime_kind_serde_roundtrip() {
        for kind in [ContainerRuntimeKind::Podman, ContainerRuntimeKind::Docker] {
            let json = serde_json::to_string(&kind).unwrap();
            let de: ContainerRuntimeKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, de);
        }
    }

    #[test]
    fn t3_error_display() {
        let e = T3Error::ImagePinMissing {
            name: "foobar".into(),
        };
        assert!(e.to_string().contains("foobar"));
    }

    #[test]
    fn sandbox_inspect_report_serde_roundtrip() {
        let report = SandboxInspectReport {
            spirit_id: "test-spirit".into(),
            payload_pid: ObservedPid::Historical(42),
            container_id: "a".repeat(64),
            runtime: "podman".into(),
            image_sha: "abc123".into(),
            applied_t2_protections: T2ProtectionSummary {
                landlock_rules: 7,
                seccomp_allow_count: 58,
                seccomp_kill_count: 14,
            },
            strictest_of_reasoning: StrictestOfReasoning {
                manifest_tier: "T0".into(),
                trust_tier_floor: "T2".into(),
                operator_policy_floor: "T3".into(),
                effective_tier: "T3".into(),
                dominant_axis: "operator".into(),
            },
        };
        let json = serde_json::to_string(&report).unwrap();
        let wire: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(wire["payload_pid"], serde_json::json!({"historical": 42}));
        assert!(
            wire.get("pid").is_none(),
            "the liveness-implying `pid` key is gone"
        );
        let de: SandboxInspectReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, de);
    }
}
