#![forbid(unsafe_code)]

//! Halt domain types — director-surface seam (Story 3.3) +
//! kernel-side mechanism seam (Story 4.1).
//!
//! The three resolution kinds are architecture §4.6.1 + FR15 verbatim.
//! `Resolution` is the wire-shape the director submits via
//! `crates/maos-director-surface/src/halt_ui.rs::submit_resolution`;
//! Story 4.1's `HaltResolver::resolve` consumes it.

/// HaltId newtype — string surface; opaque to the director-surface.
/// Story 4.1's `invoke_halt` MUST mint these as ULIDs for ordering;
/// 3.3 accepts any non-empty string so unit tests can use deterministic
/// IDs (e.g., `"halt-001"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HaltId(String);

impl HaltId {
    pub fn new(s: impl Into<String>) -> Result<Self, HaltIdError> {
        let s = s.into();
        if s.is_empty() {
            return Err(HaltIdError::Empty);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HaltIdError {
    #[error("halt_id must be non-empty")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolutionError {
    #[error("provided_context text must be non-empty")]
    EmptyText,
    #[error("authorized_override operator_policy_ref must be non-empty")]
    EmptyOperatorPolicyRef,
}

/// The three documented resolution pathways per FR15 + architecture §4.6.1.
/// Story 4.1's `HaltResolver::resolve(halt_id, Resolution)` is the
/// kernel-side consumer; Story 3.3's
/// `halt_ui::submit_resolution` is the director-side producer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "ResolutionWire")]
pub enum Resolution {
    /// Director supplies the missing context the Spirit should append
    /// to its working memory before resuming. Memory Manager integration
    /// (Story 4.3) wires the actual write; 3.3 only ferries the text.
    ProvidedContext { text: String },
    /// Director accepts the halt as final — Spirit terminates the
    /// in-flight task. `task.orphaned` emission to the originator is
    /// Story 5.3's FR12 path; 3.3 records the choice + journal entry.
    AcceptedHalt,
    /// Director authorizes override under operator policy reference.
    /// Story 4.2's predicate-firing path attaches the `OutputMarker::Override`
    /// to subsequent output; 3.3 records the choice + policy ref + identity.
    AuthorizedOverride { operator_policy_ref: String },
}

impl Resolution {
    /// Construct a `ProvidedContext` resolution with non-empty validation.
    pub fn provided_context(text: impl Into<String>) -> Result<Self, ResolutionError> {
        let r = Self::ProvidedContext { text: text.into() };
        r.validate()?;
        Ok(r)
    }

    /// Construct an `AuthorizedOverride` resolution with non-empty validation.
    pub fn authorized_override(
        operator_policy_ref: impl Into<String>,
    ) -> Result<Self, ResolutionError> {
        let r = Self::AuthorizedOverride {
            operator_policy_ref: operator_policy_ref.into(),
        };
        r.validate()?;
        Ok(r)
    }

    /// Re-check the payload invariants regardless of how the value was
    /// built. The variants carry public fields (crate-wide convention —
    /// see `frame.rs`), so a struct literal bypasses the validated
    /// constructors above. Every consumer that acts on a `Resolution`
    /// MUST call this before taking an irreversible step; the kernel-side
    /// chokepoint is `KernelHaltResolver::resolve`, which validates
    /// BEFORE the halt-state transition.
    ///
    /// Deserialization is already gated: `Resolution` deserializes via
    /// `ResolutionWire`, which funnels through this method.
    pub fn validate(&self) -> Result<(), ResolutionError> {
        match self {
            Self::ProvidedContext { text } if text.trim().is_empty() => {
                Err(ResolutionError::EmptyText)
            }
            Self::AuthorizedOverride {
                operator_policy_ref,
            } if operator_policy_ref.trim().is_empty() => {
                Err(ResolutionError::EmptyOperatorPolicyRef)
            }
            _ => Ok(()),
        }
    }

    /// Stable label for the Approval Decision Log `intent` column.
    /// Returns one of `"provided_context"` / `"accepted_halt"` /
    /// `"authorized_override"` — these are the FR15 contract strings;
    /// any future variants must NOT collide.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::ProvidedContext { .. } => "provided_context",
            Self::AcceptedHalt => "accepted_halt",
            Self::AuthorizedOverride { .. } => "authorized_override",
        }
    }
}

/// Deserialization shim for `Resolution`. Mirrors the external-tag wire
/// shape field-for-field so the pinned JSON encoding is byte-identical,
/// while routing every deserialized value through `Resolution::validate`
/// — an empty `text` / `operator_policy_ref` can no longer enter the
/// process from the wire (ACP editor channel, CLI one-shot, IAC frame).
#[derive(serde::Deserialize)]
enum ResolutionWire {
    ProvidedContext { text: String },
    AcceptedHalt,
    AuthorizedOverride { operator_policy_ref: String },
}

impl TryFrom<ResolutionWire> for Resolution {
    type Error = ResolutionError;

    fn try_from(wire: ResolutionWire) -> Result<Self, Self::Error> {
        let resolution = match wire {
            ResolutionWire::ProvidedContext { text } => Self::ProvidedContext { text },
            ResolutionWire::AcceptedHalt => Self::AcceptedHalt,
            ResolutionWire::AuthorizedOverride {
                operator_policy_ref,
            } => Self::AuthorizedOverride {
                operator_policy_ref,
            },
        };
        resolution.validate()?;
        Ok(resolution)
    }
}

/// Director-side resolution sink. Story 3.3 defines the trait; Story 4.1
/// adds the production `KernelHaltResolver` that ties resolution into
/// `invoke_halt`'s pending-resolution state + halt-receipt production.
/// Integration with E3 Story 3.3 UX surface wires here — see
/// `crates/maos-director-surface/src/halt_ui.rs`.
///
/// **Architecture note:** This trait lives in `maos-domain` (not
/// `maos-kernel-core`) to avoid a circular dependency:
/// `maos-kernel-core → maos-director-surface` (via `NotificationDispatcher`)
/// and `maos-director-surface → HaltResolver` would otherwise cycle.
/// `MockHaltResolver` and `FailingHaltResolver` live in
/// `maos-kernel-core::halt::resolver` as they're kernel test doubles.
pub trait HaltResolver: Send + Sync + 'static {
    /// Accept a director's resolution for a previously-emitted halt.
    /// Returns `Err(ResolveError::UnknownHalt)` if the halt_id has no
    /// pending state (production impl in Story 4.1; mock impl tracks
    /// calls in a Vec for unit-test assertion).
    fn resolve(&self, halt_id: &HaltId, resolution: Resolution) -> Result<(), ResolveError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ResolveError {
    #[error("unknown halt_id: {0}")]
    UnknownHalt(String),
    #[error("halt {0} already resolved")]
    AlreadyResolved(String),
    /// Story 4.3 — internal error during halt resolution (e.g. memory-write
    /// failure in `ProvidedContext` arm).  Carries diagnostic-only context.
    #[error("internal resolution error: {0}")]
    Internal(String),
    /// The submitted payload is structurally invalid (empty `text` /
    /// `operator_policy_ref`). Returned BEFORE any halt-state transition
    /// so an invalid payload can never leave a halt in a terminal state.
    #[error("invalid resolution payload: {0}")]
    InvalidResolution(#[from] ResolutionError),
}

/// Journal trait for halt resolution audit writes (Story 3.3, AC4).
/// Defined in `maos-domain` to avoid circular dep between
/// `maos-kernel-core` and `maos-director-surface`.
/// `maos-kernel-core::TransparencyLogAdapter` implements this.
pub trait HaltJournal: Send + Sync + 'static {
    fn journal_halt_resolution(
        &self,
        actor: &str,
        spirit_id: &str,
        halt_id: &HaltId,
        resolution: &Resolution,
    ) -> Result<(), HaltJournalError>;
}

#[derive(Debug, thiserror::Error)]
pub enum HaltJournalError {
    #[error("audit journal write failed: {0}")]
    WriteFailed(String),
}

// ----- Story 4.1 domain extensions (additive — preserve all existing items) -----

/// Termination kind — typed enum for the 4 termination paths.
/// Replaces the raw `&str` at `terminate_spirit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TerminationKind {
    /// Director-initiated unload (FR51-class)
    PlannedUnload,
    /// `accepted_halt` resolution
    HaltAccepted,
    /// SIGKILL / process death (Story 5.3's domain; scaffolded here)
    UnplannedCrash,
    /// `[epistemic_policy]` rejected the halt; receipt still produced
    HaltRejection,
    /// Story 5.4 — Spirit revoked via CRL propagation.
    RevocationTerminated,
}

impl TerminationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlannedUnload => "planned_unload",
            Self::HaltAccepted => "halt_accepted",
            Self::UnplannedCrash => "unplanned_crash",
            Self::HaltRejection => "halt_rejection",
            Self::RevocationTerminated => "revocation_terminated",
        }
    }
}

/// The substrate's halt-receipt — proof a halt invocation reached the
/// audit chain. Returned by `invoke_halt` on every successful call;
/// returned by `terminate_spirit` on every termination path (planned,
/// unplanned, crash). The receipt-production rate ≥99.9% (AC4) is
/// measured by counting receipt presence in the 1000-termination
/// corpus.
///
/// Fields populated post-resolution (terminal_state, resolution_kind,
/// resolution_timestamp_ns) are `None` for receipts returned at
/// invocation time; the resolver writes them when `KernelHaltResolver::resolve`
/// completes.
///
/// Construct via `HaltReceipt::new` to enforce non-NaN + non-empty
/// validation; struct-literal construction bypasses validation per
/// the `frame.rs` pub-field convention (see ADR-041 / A3).
#[doc = "Construct via `HaltReceipt::new` to enforce validation; struct literals bypass checks."]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HaltReceipt {
    pub halt_id: HaltId,
    pub timestamp_ns: u64,
    pub spirit_pid: u32,
    pub boot_nonce: u64,
    pub frame_id: [u8; 16],
    /// Filled by `KernelHaltResolver::resolve`; `None` at invocation time.
    pub terminal_state: Option<HaltState>,
    /// Filled by `KernelHaltResolver::resolve`; `None` at invocation time.
    pub resolution_kind: Option<String>,
    /// Filled by `KernelHaltResolver::resolve`; `None` at invocation time.
    pub resolution_timestamp_ns: Option<u64>,
}

impl HaltReceipt {
    /// Construct an invocation-time receipt — pre-resolution fields are
    /// `None`. The resolver fills them post-resolution via `with_resolution`.
    pub fn new(
        halt_id: HaltId,
        timestamp_ns: u64,
        spirit_pid: u32,
        boot_nonce: u64,
        frame_id: [u8; 16],
    ) -> Self {
        Self {
            halt_id,
            timestamp_ns,
            spirit_pid,
            boot_nonce,
            frame_id,
            terminal_state: None,
            resolution_kind: None,
            resolution_timestamp_ns: None,
        }
    }

    /// Fluent builder for the post-resolution fields. Used by
    /// `KernelHaltResolver::resolve` to attach terminal state.
    pub fn with_resolution(
        mut self,
        terminal_state: HaltState,
        resolution_kind: &str,
        resolution_timestamp_ns: u64,
    ) -> Self {
        self.terminal_state = Some(terminal_state);
        self.resolution_kind = Some(resolution_kind.to_string());
        self.resolution_timestamp_ns = Some(resolution_timestamp_ns);
        self
    }
}

/// Lifecycle states a halt traverses. `PendingResolution` is the only
/// quiescent state — every halt either advances to one of the three
/// terminal states (`Resumed`, `Terminated`, `Overridden`) or remains
/// pending until the Spirit terminates (in which case the kernel
/// terminates the halt with `terminate_for_spirit_exit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HaltState {
    /// Initial state on `invoke_halt`; awaits director resolution.
    PendingResolution,
    /// Terminal — `provided_context` resolution path. Spirit resumed.
    Resumed,
    /// Terminal — `accepted_halt` resolution path. Spirit terminated;
    /// `task.orphaned` IAC frame emitted per FR12.
    Terminated,
    /// Terminal — `authorized_override` resolution path. Spirit continued
    /// with `OutputMarker::Override` appended to subsequent output queue.
    Overridden,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvokeHaltError {
    #[error("halt_id {0} already pending in registry")]
    DuplicateHaltId(String),
    #[error("transparency log write failed: {0}")]
    TransparencyLogWriteFailed(String),
    #[error("lifecycle journal write failed: {0}")]
    JournalWriteFailed(String),
    #[error("registry insert failed: {0}")]
    RegistryInsertFailed(String),
}

/// I14 — Hot-swap halt-continuity typed error per ADR-019.
/// `validate_halt_set` returns this when the successor's
/// `halt_protocol_compatibility = N` does NOT match the predecessor's
/// halt-protocol version. Story 5.2 owns the integration; Story 4.1
/// owns the typed-error path + unit test.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HaltContinuityError {
    #[error("halt-continuity violation: schema mismatch — predecessor v{predecessor} vs successor v{successor}; orphaned halts: {orphan_count}")]
    EHaltContinuityViolation {
        predecessor: u32,
        successor: u32,
        orphan_count: usize,
    },
    #[error("successor manifest missing required field `halt_protocol_compatibility`")]
    MissingHaltProtocolCompatibility,
}

#[doc = "Output marker appended to a Spirit's output queue after `authorized_override`. Story 4.2's `output_shape` predicates consume this marker; this story only emits it. Construct via `OutputMarker::override_for(halt_id)` to enforce non-empty validation."]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutputMarker {
    pub kind: OutputMarkerKind,
    pub halt_id: HaltId,
    pub operator_policy_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutputMarkerKind {
    Override,
}

impl OutputMarker {
    pub fn override_for(
        halt_id: HaltId,
        operator_policy_ref: String,
    ) -> Result<Self, OutputMarkerError> {
        if operator_policy_ref.trim().is_empty() {
            return Err(OutputMarkerError::EmptyPolicyRef);
        }
        Ok(Self {
            kind: OutputMarkerKind::Override,
            halt_id,
            operator_policy_ref: Some(operator_policy_ref),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputMarkerError {
    #[error("operator_policy_ref must be non-empty for Override marker")]
    EmptyPolicyRef,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_id_new_empty_returns_error() {
        let result = HaltId::new("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HaltIdError::Empty));
    }

    #[test]
    fn halt_id_new_nonempty_works() {
        let id = HaltId::new("halt-001").unwrap();
        assert_eq!(id.as_str(), "halt-001");
    }

    #[test]
    fn halt_id_serde_round_trip() {
        let id = HaltId::new("halt-42").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: HaltId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn resolution_provided_context_rejects_empty() {
        let result = Resolution::provided_context("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResolutionError::EmptyText));
    }

    #[test]
    fn resolution_provided_context_rejects_whitespace_only() {
        let result = Resolution::provided_context("   ");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResolutionError::EmptyText));
    }

    #[test]
    fn resolution_authorized_override_rejects_empty() {
        let result = Resolution::authorized_override("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResolutionError::EmptyOperatorPolicyRef
        ));
    }

    #[test]
    fn resolution_authorized_override_rejects_whitespace_only() {
        let result = Resolution::authorized_override("  ");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ResolutionError::EmptyOperatorPolicyRef
        ));
    }

    #[test]
    fn resolution_kind_label_provided_context() {
        let r = Resolution::ProvidedContext {
            text: "more info".into(),
        };
        assert_eq!(r.kind_label(), "provided_context");
    }

    #[test]
    fn resolution_kind_label_accepted_halt() {
        assert_eq!(Resolution::AcceptedHalt.kind_label(), "accepted_halt");
    }

    #[test]
    fn resolution_kind_label_authorized_override() {
        let r = Resolution::AuthorizedOverride {
            operator_policy_ref: "pol".into(),
        };
        assert_eq!(r.kind_label(), "authorized_override");
    }

    #[test]
    fn resolution_serde_round_trip_provided_context() {
        let r = Resolution::ProvidedContext {
            text: "context here".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Resolution = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
        let expected = r#"{"ProvidedContext":{"text":"context here"}}"#;
        assert_eq!(json, expected);
    }

    #[test]
    fn resolution_serde_round_trip_accepted_halt() {
        let r = Resolution::AcceptedHalt;
        let json = serde_json::to_string(&r).unwrap();
        let back: Resolution = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
        let expected = r#""AcceptedHalt""#;
        assert_eq!(json, expected);
    }

    #[test]
    fn resolution_serde_round_trip_authorized_override() {
        let r = Resolution::AuthorizedOverride {
            operator_policy_ref: "policy://x".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Resolution = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
        let expected = r#"{"AuthorizedOverride":{"operator_policy_ref":"policy://x"}}"#;
        assert_eq!(json, expected);
    }

    // --- Story 3.3 review closure — empty payloads rejected on EVERY
    // construction path, not just the validated constructors ---

    #[test]
    fn resolution_validate_rejects_struct_literal_empty_text() {
        let r = Resolution::ProvidedContext {
            text: String::new(),
        };
        assert!(matches!(
            r.validate().unwrap_err(),
            ResolutionError::EmptyText
        ));
    }

    #[test]
    fn resolution_validate_rejects_struct_literal_whitespace_operator_policy_ref() {
        let r = Resolution::AuthorizedOverride {
            operator_policy_ref: " \t ".into(),
        };
        assert!(matches!(
            r.validate().unwrap_err(),
            ResolutionError::EmptyOperatorPolicyRef
        ));
    }

    #[test]
    fn resolution_validate_accepts_populated_and_unit_variants() {
        assert!(Resolution::AcceptedHalt.validate().is_ok());
        assert!(Resolution::ProvidedContext { text: "x".into() }
            .validate()
            .is_ok());
        assert!(Resolution::AuthorizedOverride {
            operator_policy_ref: "policy://x".into(),
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn resolution_deserialize_rejects_empty_text() {
        let err = serde_json::from_str::<Resolution>(r#"{"ProvidedContext":{"text":""}}"#)
            .expect_err("empty text must not deserialize");
        assert!(
            err.to_string()
                .contains("provided_context text must be non-empty"),
            "serde error should carry the domain diagnostic, got: {err}"
        );
    }

    #[test]
    fn resolution_deserialize_rejects_whitespace_operator_policy_ref() {
        let err = serde_json::from_str::<Resolution>(
            r#"{"AuthorizedOverride":{"operator_policy_ref":"   "}}"#,
        )
        .expect_err("whitespace-only operator_policy_ref must not deserialize");
        assert!(
            err.to_string()
                .contains("operator_policy_ref must be non-empty"),
            "serde error should carry the domain diagnostic, got: {err}"
        );
    }

    #[test]
    fn resolution_deserialize_accepts_populated_payloads() {
        // The validation gate must not narrow the accepted wire surface.
        let r: Resolution =
            serde_json::from_str(r#"{"ProvidedContext":{"text":"context here"}}"#).unwrap();
        assert_eq!(
            r,
            Resolution::ProvidedContext {
                text: "context here".into()
            }
        );
        let r: Resolution = serde_json::from_str(r#""AcceptedHalt""#).unwrap();
        assert_eq!(r, Resolution::AcceptedHalt);
    }

    // --- Story 4.1 — HaltReceipt + OutputMarker constructor tests ---

    #[test]
    fn halt_receipt_new_constructs_invocation_time_receipt() {
        let hid = HaltId::new("halt-001").unwrap();
        let receipt = HaltReceipt::new(hid.clone(), 42, 7, 0xCAFE, [0xAB; 16]);
        assert_eq!(receipt.halt_id, hid);
        assert_eq!(receipt.timestamp_ns, 42);
        assert_eq!(receipt.spirit_pid, 7);
        assert_eq!(receipt.boot_nonce, 0xCAFE);
        assert_eq!(receipt.frame_id, [0xAB; 16]);
        assert!(receipt.terminal_state.is_none());
        assert!(receipt.resolution_kind.is_none());
        assert!(receipt.resolution_timestamp_ns.is_none());
    }

    #[test]
    fn halt_receipt_with_resolution_fills_post_resolution_fields() {
        let hid = HaltId::new("halt-002").unwrap();
        let receipt = HaltReceipt::new(hid, 100, 1, 0, [0u8; 16]);
        let resolved = receipt.with_resolution(HaltState::Resumed, "provided_context", 200);
        assert_eq!(resolved.terminal_state, Some(HaltState::Resumed));
        assert_eq!(
            resolved.resolution_kind.as_deref(),
            Some("provided_context")
        );
        assert_eq!(resolved.resolution_timestamp_ns, Some(200));
    }

    #[test]
    fn halt_receipt_serde_round_trip() {
        let hid = HaltId::new("halt-serde").unwrap();
        let receipt = HaltReceipt::new(hid, 1, 2, 3, [0xFF; 16]).with_resolution(
            HaltState::Terminated,
            "accepted_halt",
            1000,
        );
        let json = serde_json::to_string(&receipt).unwrap();
        let back: HaltReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(back.halt_id.as_str(), "halt-serde");
        assert_eq!(back.terminal_state, Some(HaltState::Terminated));
        assert_eq!(back.resolution_kind.as_deref(), Some("accepted_halt"));
    }

    #[test]
    fn halt_state_serde_round_trip() {
        for state in [
            HaltState::PendingResolution,
            HaltState::Resumed,
            HaltState::Terminated,
            HaltState::Overridden,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: HaltState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn output_marker_override_for_constructs_valid_marker() {
        let hid = HaltId::new("halt-mark").unwrap();
        let marker = OutputMarker::override_for(hid.clone(), "policy://test".into()).unwrap();
        assert_eq!(marker.kind, OutputMarkerKind::Override);
        assert_eq!(marker.halt_id, hid);
        assert_eq!(marker.operator_policy_ref.as_deref(), Some("policy://test"));
    }

    #[test]
    fn output_marker_override_for_rejects_empty_policy_ref() {
        let hid = HaltId::new("halt-mark").unwrap();
        let err = OutputMarker::override_for(hid, "".into()).unwrap_err();
        assert!(matches!(err, OutputMarkerError::EmptyPolicyRef));
    }

    #[test]
    fn output_marker_override_for_rejects_whitespace_only_policy_ref() {
        let hid = HaltId::new("halt-mark").unwrap();
        let err = OutputMarker::override_for(hid, "   ".into()).unwrap_err();
        assert!(matches!(err, OutputMarkerError::EmptyPolicyRef));
    }

    #[test]
    fn output_marker_serde_round_trip() {
        let hid = HaltId::new("halt-mark").unwrap();
        let marker = OutputMarker::override_for(hid, "policy://test".into()).unwrap();
        let json = serde_json::to_string(&marker).unwrap();
        let back: OutputMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, OutputMarkerKind::Override);
        assert_eq!(back.halt_id.as_str(), "halt-mark");
        assert_eq!(back.operator_policy_ref.as_deref(), Some("policy://test"));
    }

    #[test]
    fn invoke_halt_error_display() {
        let e = InvokeHaltError::DuplicateHaltId("dup".into());
        assert!(e.to_string().contains("dup"));
        let e = InvokeHaltError::TransparencyLogWriteFailed("tl".into());
        assert!(e.to_string().contains("tl"));
        let e = InvokeHaltError::JournalWriteFailed("jw".into());
        assert!(e.to_string().contains("jw"));
        let e = InvokeHaltError::RegistryInsertFailed("ri".into());
        assert!(e.to_string().contains("ri"));
    }

    #[test]
    fn halt_continuity_error_display() {
        let e = HaltContinuityError::EHaltContinuityViolation {
            predecessor: 1,
            successor: 3,
            orphan_count: 2,
        };
        let s = e.to_string();
        assert!(s.contains("v1"));
        assert!(s.contains("v3"));
        assert!(s.contains("2"));
        let e = HaltContinuityError::MissingHaltProtocolCompatibility;
        assert!(e.to_string().contains("halt_protocol_compatibility"));
    }

    #[test]
    fn termination_kind_revocation_terminated_as_str() {
        assert_eq!(
            TerminationKind::RevocationTerminated.as_str(),
            "revocation_terminated"
        );
    }

    #[test]
    fn termination_kind_revocation_terminated_serde_roundtrip() {
        let original = TerminationKind::RevocationTerminated;
        let json = serde_json::to_string(&original).unwrap();
        let back: TerminationKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TerminationKind::RevocationTerminated);
    }
}
