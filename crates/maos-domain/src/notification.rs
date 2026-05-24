//! Notification surface types — domain-level per architecture §7.4.
//!
//! These types are shared between `maos-kernel-core` (Approval Manager
//! emits events) and `maos-director-surface` (dispatcher sends them to
//! channels). Putting them in `maos-domain` avoids a circular dependency.

/// The three notification levels from §7.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NotificationLevel {
    Immediate,
    Queue,
    Digest,
}

/// The notification surface a kernel event dispatches into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSurface {
    Terminal,
    AcpEditor,
    MobilePush,
}

/// What the kernel hands to a NotificationChannel.
///
/// Story 3.1 ships `TaskAssigned` + `ApprovalPrompt`;
/// Story 3.3 adds `Halt`; Story 3.4 adds `AnomalyFlagged`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum NotificationEvent {
    TaskAssigned {
        frame_id: [u8; 16],
        from: String,
        goal: String,
    },
    ApprovalPrompt {
        decision_id: u64,
        class: ApprovalClass,
        capability: String,
        reasoning: Option<String>,
    },
    /// Story 3.3 — halt surfaced to the director for resolution.
    /// halt_id is read from `payload.halt_id` — single source of truth.
    Halt {
        payload: crate::frame::EpistemicHaltPayload,
    },
    /// Story 3.4 — anomaly surfaced to the director by an Observer-class
    /// Spirit (full Observer wiring at Story 8.3). The director's surface
    /// renders the anomaly with confidence + originating Spirit so the
    /// director decides whether to pause/revoke/intervene.
    AnomalyFlagged {
        /// SpiritId of the Observer that flagged the anomaly (string at
        /// v0.3-β; Story 8.3 may promote to typed SpiritId).
        #[doc = "Construct via [`NotificationEvent::anomaly_flagged`] to enforce validation; struct literals bypass NaN / empty / range checks."]
        observer: String,
        /// SpiritId of the Spirit the anomaly was observed on.
        #[doc = "Construct via [`NotificationEvent::anomaly_flagged`] to enforce validation; struct literals bypass NaN / empty / range checks."]
        subject: String,
        /// Free-form human-readable anomaly summary.
        #[doc = "Construct via [`NotificationEvent::anomaly_flagged`] to enforce validation; struct literals bypass NaN / empty / range checks."]
        summary: String,
        /// Observer-supplied confidence in [0.0, 1.0]. Rendered as a percentage.
        /// f32 to match Story 4.2's tagged-scalar shape; NaN rejected at construction.
        #[doc = "Construct via [`NotificationEvent::anomaly_flagged`] to enforce validation; struct literals bypass NaN / empty / range checks."]
        confidence: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NotificationEventError {
    #[error("anomaly confidence must not be NaN")]
    NanConfidence,
    #[error("anomaly summary must be non-empty")]
    EmptySummary,
    #[error("anomaly confidence must be in [0.0, 1.0]")]
    ConfidenceOutOfRange,
}

impl NotificationEvent {
    /// Construct an AnomalyFlagged event with NaN + empty-summary validation.
    /// Mirrors `EpistemicHaltPayload::new` validation shape from Story 3.3 AC1.
    pub fn anomaly_flagged(
        observer: impl Into<String>,
        subject: impl Into<String>,
        summary: impl Into<String>,
        confidence: f32,
    ) -> Result<Self, NotificationEventError> {
        let summary = summary.into();
        if summary.trim().is_empty() {
            return Err(NotificationEventError::EmptySummary);
        }
        if confidence.is_nan() {
            return Err(NotificationEventError::NanConfidence);
        }
        if confidence < 0.0 || confidence > 1.0 {
            return Err(NotificationEventError::ConfidenceOutOfRange);
        }
        Ok(Self::AnomalyFlagged {
            observer: observer.into(),
            subject: subject.into(),
            summary,
            confidence,
        })
    }
}

/// Maps to architecture §4.3.3's 6-class taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApprovalClass {
    ReadonlyScoped,
    ReadonlySearch,
    Mutating,
    ExecCapable,
    ControlPlane,
    Interactive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anomaly_flagged_rejects_nan_confidence() {
        let result =
            NotificationEvent::anomaly_flagged("observer-1", "subject-1", "test anomaly", f32::NAN);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            NotificationEventError::NanConfidence
        ));
    }

    #[test]
    fn anomaly_flagged_rejects_empty_summary() {
        let result = NotificationEvent::anomaly_flagged("observer-1", "subject-1", "", 0.5);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            NotificationEventError::EmptySummary
        ));
    }

    #[test]
    fn anomaly_flagged_rejects_whitespace_only_summary() {
        let result = NotificationEvent::anomaly_flagged("observer-1", "subject-1", "   ", 0.5);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            NotificationEventError::EmptySummary
        ));
    }

    #[test]
    fn anomaly_flagged_accepts_valid_input() {
        let event = NotificationEvent::anomaly_flagged(
            "observer-1",
            "subject-1",
            "resource exhaustion detected",
            0.85,
        )
        .unwrap();
        match event {
            NotificationEvent::AnomalyFlagged {
                observer,
                subject,
                summary,
                confidence,
            } => {
                assert_eq!(observer, "observer-1");
                assert_eq!(subject, "subject-1");
                assert_eq!(summary, "resource exhaustion detected");
                assert!((confidence - 0.85).abs() < f32::EPSILON);
            }
            _ => panic!("expected AnomalyFlagged variant"),
        }
    }

    #[test]
    fn anomaly_flagged_rejects_out_of_range_confidence() {
        let cases = [-1.0f32, 1.5, f32::INFINITY, f32::NEG_INFINITY, 100.0];
        for bad_confidence in cases {
            let result = NotificationEvent::anomaly_flagged("obs", "sub", "test", bad_confidence);
            assert!(
                matches!(result, Err(NotificationEventError::ConfidenceOutOfRange)),
                "expected ConfidenceOutOfRange for {bad_confidence}, got {:?}",
                result
            );
        }
    }

    #[test]
    fn anomaly_flagged_accepts_boundary_confidence() {
        let lo = NotificationEvent::anomaly_flagged("o", "s", "test", 0.0).unwrap();
        let hi = NotificationEvent::anomaly_flagged("o", "s", "test", 1.0).unwrap();
        match lo {
            NotificationEvent::AnomalyFlagged { confidence, .. } => assert_eq!(confidence, 0.0),
            _ => panic!("expected AnomalyFlagged"),
        }
        match hi {
            NotificationEvent::AnomalyFlagged { confidence, .. } => assert_eq!(confidence, 1.0),
            _ => panic!("expected AnomalyFlagged"),
        }
    }
}
