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
        let text = text.into();
        if text.trim().is_empty() {
            return Err(ResolutionError::EmptyText);
        }
        Ok(Self::ProvidedContext { text })
    }

    /// Construct an `AuthorizedOverride` resolution with non-empty validation.
    pub fn authorized_override(operator_policy_ref: impl Into<String>) -> Result<Self, ResolutionError> {
        let operator_policy_ref = operator_policy_ref.into();
        if operator_policy_ref.trim().is_empty() {
            return Err(ResolutionError::EmptyOperatorPolicyRef);
        }
        Ok(Self::AuthorizedOverride { operator_policy_ref })
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
pub enum ResolveError {
    #[error("unknown halt_id: {0}")]
    UnknownHalt(String),
    #[error("halt {0} already resolved")]
    AlreadyResolved(String),
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
        assert!(matches!(result.unwrap_err(), ResolutionError::EmptyOperatorPolicyRef));
    }

    #[test]
    fn resolution_authorized_override_rejects_whitespace_only() {
        let result = Resolution::authorized_override("  ");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResolutionError::EmptyOperatorPolicyRef));
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
}
