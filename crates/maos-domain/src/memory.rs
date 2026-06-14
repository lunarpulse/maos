#![forbid(unsafe_code)]

//! Memory tier, namespace, value, and error types for the three-tier
//! Memory Manager surface (Story 4.3).
//!
//! Per architecture §4.2: three memory tiers (`private`, `shared`,
//! `collective`), a typed Principal Memory Namespace (ADR-026),
//! namespace-grammar-locked closed enum (NFR-Test-11), and typed error
//! taxonomy. All types are pure domain — no I/O, no SQLite, no async.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// MemoryTier
// ---------------------------------------------------------------------------

/// Closed enum for the three memory tiers per architecture §4.2.
///
/// `#[repr(u8)]` for ABI stability — matches the `MemoryScope` pattern
/// at `invariants/i5.rs:26-35`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MemoryTier {
    /// Private to a single Spirit instance (per-Spirit HashMap + filesystem).
    Private = 0,
    /// Shared among Spirits within the same Host (Host-wide SQLite kv).
    Shared = 1,
    /// Collective across pre-paired Hosts (scaffold — v0.5 returns typed error).
    Collective = 2,
}

// ---------------------------------------------------------------------------
// MemoryNamespace
// ---------------------------------------------------------------------------

/// Namespace-grammar-locked closed enum (NFR-Test-11).
///
/// New variants land via ABI-additive amendment only.  The `Forgotten`
/// variant is stubbed here; Story 5.2 wires the GC sweep.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryNamespace {
    /// Default per-Spirit namespace — carries no additional semantics.
    Default,
    /// Cross-Spirit coordination namespace.
    Coordination,
    /// GC variant — stub; wired in Story 5.2 Hot-Swap.
    Forgotten,
    /// ADR-026 Principal Memory Namespace — typed namespace within the
    /// private tier.  Writes tagged `principal:<principal_id>:<schema>`
    /// inherit subject-access query, right-to-be-forgotten, and
    /// redaction-on-export.
    Principal {
        principal_id: String,
        schema: String,
    },
}

impl MemoryNamespace {
    /// Construct a `Principal` namespace with validation.
    ///
    /// Rejects empty `principal_id`, empty `schema`, and any of the
    /// forbidden characters: `:`, NUL, ASCII control chars.
    pub fn principal(
        principal_id: impl Into<String>,
        schema: impl Into<String>,
    ) -> Result<Self, NamespaceError> {
        let principal_id: String = principal_id.into();
        let schema: String = schema.into();

        if principal_id.is_empty() {
            return Err(NamespaceError::EmptyPrincipalId);
        }
        if schema.is_empty() {
            return Err(NamespaceError::EmptySchema);
        }
        for (field_name, field_value) in [("principal_id", &principal_id), ("schema", &schema)] {
            for ch in field_value.chars() {
                if ch == ':' {
                    return Err(NamespaceError::ForbiddenCharacter {
                        field: field_name.into(),
                        ch: ':',
                    });
                }
                if ch == '\0' || ch.is_ascii_control() {
                    return Err(NamespaceError::ForbiddenCharacter {
                        field: field_name.into(),
                        ch,
                    });
                }
            }
        }

        Ok(Self::Principal {
            principal_id,
            schema,
        })
    }

    /// Stable label for storage routing.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Coordination => "coordination",
            Self::Forgotten => "forgotten",
            Self::Principal { .. } => "principal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NamespaceError {
    #[error("principal_id must be non-empty")]
    EmptyPrincipalId,
    #[error("schema must be non-empty")]
    EmptySchema,
    #[error("forbidden character {ch:?} in field '{field}'")]
    ForbiddenCharacter { field: String, ch: char },
}

// ---------------------------------------------------------------------------
// PrincipalKey
// ---------------------------------------------------------------------------

/// Typed wrapper matching the `Principal` namespace shape.
///
/// `PrincipalKey::new` is the validated constructor — rejects empty
/// `principal_id`, empty `schema`, and forbidden characters.
#[doc = "Construct via [`PrincipalKey::new`] to enforce validation; struct literals bypass namespace-grammar / non-empty checks."]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrincipalKey {
    pub principal_id: String,
    pub schema: String,
}

impl PrincipalKey {
    pub fn new(
        principal_id: impl Into<String>,
        schema: impl Into<String>,
    ) -> Result<Self, NamespaceError> {
        let principal_id: String = principal_id.into();
        let schema: String = schema.into();

        if principal_id.is_empty() {
            return Err(NamespaceError::EmptyPrincipalId);
        }
        if schema.is_empty() {
            return Err(NamespaceError::EmptySchema);
        }
        for (field_name, field_value) in [("principal_id", &principal_id), ("schema", &schema)] {
            for ch in field_value.chars() {
                if ch == ':' {
                    return Err(NamespaceError::ForbiddenCharacter {
                        field: field_name.into(),
                        ch: ':',
                    });
                }
                if ch == '\0' || ch.is_ascii_control() {
                    return Err(NamespaceError::ForbiddenCharacter {
                        field: field_name.into(),
                        ch,
                    });
                }
            }
        }

        Ok(Self {
            principal_id,
            schema,
        })
    }
}

// ---------------------------------------------------------------------------
// ValueKind / MemoryValue / MemoryEntry
// ---------------------------------------------------------------------------

/// Narrow type tag for storage-routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueKind {
    Json,
    Markdown,
    Blob,
    Text,
}

/// Typed content variants — the kernel routes by `kind()` but does NOT
/// parse or summarize the bytes (§4.0.7).  `PartialEq` only (no `Eq`
/// because `serde_json::Value` is not `Eq`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryValue {
    Json(serde_json::Value),
    Markdown(String),
    Blob(Vec<u8>),
    Text(String),
}

impl MemoryValue {
    pub fn kind(&self) -> ValueKind {
        match self {
            Self::Json(_) => ValueKind::Json,
            Self::Markdown(_) => ValueKind::Markdown,
            Self::Blob(_) => ValueKind::Blob,
            Self::Text(_) => ValueKind::Text,
        }
    }

    /// Estimate the byte size for spill-threshold decisions.
    pub fn approximate_len(&self) -> Result<usize, MemoryError> {
        match self {
            Self::Json(v) => {
                let bytes: Vec<u8> = serde_json::to_vec(v)
                    .map_err(|e| MemoryError::Storage(format!("serde: {e}")))?;
                Ok(bytes.len())
            }
            Self::Markdown(s) => Ok(s.len()),
            Self::Blob(b) => Ok(b.len()),
            Self::Text(s) => Ok(s.len()),
        }
    }
}

/// Return shape for `scan`.
#[doc = "Construct via [`MemoryEntry::new`] to enforce validation; struct literals bypass key-traversal / non-empty checks."]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub namespace: MemoryNamespace,
    pub key: String,
    pub value: MemoryValue,
    pub timestamp_ns: u64,
}

impl MemoryEntry {
    pub fn new(
        namespace: MemoryNamespace,
        key: impl Into<String>,
        value: MemoryValue,
        timestamp_ns: u64,
    ) -> Result<Self, MemoryError> {
        let key: String = key.into();
        if key.is_empty() {
            return Err(MemoryError::InvalidKey { key });
        }
        Ok(Self {
            namespace,
            key,
            value,
            timestamp_ns,
        })
    }
}

// ---------------------------------------------------------------------------
// MemoryError
// ---------------------------------------------------------------------------

/// Typed error taxonomy for Memory Manager operations.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("namespace violation: {0}")]
    NamespaceViolation(String),
    #[error("key traversal rejected: {key}")]
    KeyTraversalRejected { key: String },
    #[error("key too long: {len} bytes (max {max})")]
    KeyTooLong { len: usize, max: usize },
    #[error("collective tier not yet available — ships at {ship_target} via {landing_story}")]
    CollectiveNotYetAvailable {
        ship_target: &'static str,
        landing_story: &'static str,
    },
    #[error("invalid key: {key}")]
    InvalidKey { key: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("value too large: {len} bytes (max {max})")]
    ValueTooLarge { len: usize, max: usize },
}

// ---------------------------------------------------------------------------
// PrincipalIndexRow
// ---------------------------------------------------------------------------

/// Return shape for `subject_access` — one row per `(principal_id,
/// writer_spirit_pid, schema, key)` quartet.  Carries NO content (kernel
/// does NOT interpret per §4.0.7).
#[doc = "Construct via [`PrincipalIndexRow::new`] to enforce validation; struct literals bypass non-empty checks."]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalIndexRow {
    pub principal_id: String,
    pub writer_spirit_pid: u32,
    pub schema: String,
    pub key: String,
    pub timestamp_ns: u64,
}

impl PrincipalIndexRow {
    pub fn new(
        principal_id: impl Into<String>,
        writer_spirit_pid: u32,
        schema: impl Into<String>,
        key: impl Into<String>,
        timestamp_ns: u64,
    ) -> Result<Self, NamespaceError> {
        let principal_id: String = principal_id.into();
        let schema: String = schema.into();
        let key: String = key.into();
        if principal_id.is_empty() {
            return Err(NamespaceError::EmptyPrincipalId);
        }
        if schema.is_empty() {
            return Err(NamespaceError::EmptySchema);
        }
        if key.is_empty() {
            return Err(NamespaceError::EmptySchema);
        }
        Ok(Self {
            principal_id,
            writer_spirit_pid,
            schema,
            key,
            timestamp_ns,
        })
    }
}

// ---------------------------------------------------------------------------
// ForgetReceipt
// ---------------------------------------------------------------------------

/// Return shape for `forget` — proof of the cascade having executed.
#[doc = "Construct via [`ForgetReceipt::new`] to enforce validation; struct literals bypass non-empty checks."]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetReceipt {
    pub principal_id: String,
    pub deleted_entries: u64,
    pub deleted_index_rows: u64,
    pub timestamp_ns: u64,
    pub frame_id: [u8; 16],
    /// Story 9.3b (AC6 / R12) — schema in force at erasure-execution time.
    /// Additive; bytes-identical for pre-9.3b entries via `skip_serializing_if`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
    /// Story 9.3b (AC6 / R12) — schema version in force at erasure-execution time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
}

impl ForgetReceipt {
    pub fn new(
        principal_id: impl Into<String>,
        deleted_entries: u64,
        deleted_index_rows: u64,
        timestamp_ns: u64,
        frame_id: [u8; 16],
    ) -> Result<Self, NamespaceError> {
        let principal_id: String = principal_id.into();
        if principal_id.is_empty() {
            return Err(NamespaceError::EmptyPrincipalId);
        }
        Ok(Self {
            principal_id,
            deleted_entries,
            deleted_index_rows,
            timestamp_ns,
            frame_id,
            schema_id: None,
            schema_version: None,
        })
    }
}
// ---------------------------------------------------------------------------
// Story 9.2 — GDPR Art.17 forget outcome with legal-hold suspension.
// ---------------------------------------------------------------------------

/// Record of a lawful legal-hold that blocked a forget cascade.
/// Per Decision E: scope is per-principal-global in v1.0; the schema
/// reserves `scope` so per-Spirit holds can be added later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalHoldRecord {
    pub principal_id: String,
    pub scope: String,
    pub reason: String,
    pub case_ref: Option<String>,
    pub requested_at_ns: u64,
    pub status: String,
}

/// Result of `MemoryManagerAdapter::forget_with_reason`.
/// Either the cascade erased the principal, or a legal hold suspended it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome")]
pub enum ForgetOutcome {
    Erased {
        receipt: ForgetReceipt,
        redacted_distillate_frame_ids: Vec<String>,
        redacted_principal_frame_ids: Vec<String>,
    },
    Suspended {
        hold: LegalHoldRecord,
    },
}

// ---------------------------------------------------------------------------
// ExportEntry / ExportPayload
// ---------------------------------------------------------------------------

/// Return shape for `export_redactable`.  `PartialEq` only (payload
/// carries `MemoryValue` which is not `Eq`).
#[doc = "Construct via the named constructor; struct literals bypass redaction discipline."]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportEntry {
    pub namespace: MemoryNamespace,
    pub key: String,
    pub payload: ExportPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExportPayload {
    Redacted {
        content_type: String,
        principal_id: String,
        schema: String,
    },
    Raw(MemoryValue),
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- MemoryTier ---

    #[test]
    fn memory_tier_discriminants_are_distinct() {
        assert_ne!(MemoryTier::Private as u8, MemoryTier::Shared as u8);
        assert_ne!(MemoryTier::Shared as u8, MemoryTier::Collective as u8);
    }

    #[test]
    fn memory_tier_serde_round_trip() {
        for tier in [
            MemoryTier::Private,
            MemoryTier::Shared,
            MemoryTier::Collective,
        ] {
            let json = serde_json::to_string(&tier).unwrap();
            let back: MemoryTier = serde_json::from_str(&json).unwrap();
            assert_eq!(tier, back);
        }
    }

    // --- MemoryNamespace ---

    #[test]
    fn namespace_principal_rejects_empty_principal_id() {
        let err = MemoryNamespace::principal("", "calendar").unwrap_err();
        assert!(matches!(err, NamespaceError::EmptyPrincipalId));
    }

    #[test]
    fn namespace_principal_rejects_empty_schema() {
        let err = MemoryNamespace::principal("alice@example.org", "").unwrap_err();
        assert!(matches!(err, NamespaceError::EmptySchema));
    }

    #[test]
    fn namespace_principal_rejects_colon_in_principal_id() {
        let err = MemoryNamespace::principal("ali:ce", "calendar").unwrap_err();
        match err {
            NamespaceError::ForbiddenCharacter { field, ch } => {
                assert_eq!(field, "principal_id");
                assert_eq!(ch, ':');
            }
            _ => panic!("expected ForbiddenCharacter"),
        }
    }

    #[test]
    fn namespace_principal_rejects_colon_in_schema() {
        let err = MemoryNamespace::principal("alice@example.org", "cal:endar").unwrap_err();
        match err {
            NamespaceError::ForbiddenCharacter { field, ch } => {
                assert_eq!(field, "schema");
                assert_eq!(ch, ':');
            }
            _ => panic!("expected ForbiddenCharacter"),
        }
    }

    #[test]
    fn namespace_principal_rejects_nul() {
        let err = MemoryNamespace::principal("alice\0bad", "calendar").unwrap_err();
        assert!(matches!(err, NamespaceError::ForbiddenCharacter { .. }));
    }

    #[test]
    fn namespace_principal_rejects_control_char() {
        let err = MemoryNamespace::principal("alice", "cal\x01endar").unwrap_err();
        assert!(matches!(err, NamespaceError::ForbiddenCharacter { .. }));
    }

    #[test]
    fn namespace_principal_round_trips() {
        let ns = MemoryNamespace::principal("alice@example.org", "calendar").unwrap();
        match &ns {
            MemoryNamespace::Principal {
                principal_id,
                schema,
            } => {
                assert_eq!(principal_id, "alice@example.org");
                assert_eq!(schema, "calendar");
            }
            _ => panic!("expected Principal variant"),
        }
    }

    #[test]
    fn namespace_kind_labels_are_distinct() {
        assert_eq!(MemoryNamespace::Default.kind_label(), "default");
        assert_eq!(MemoryNamespace::Coordination.kind_label(), "coordination");
        assert_eq!(MemoryNamespace::Forgotten.kind_label(), "forgotten");
        assert_eq!(
            MemoryNamespace::principal("a", "b").unwrap().kind_label(),
            "principal"
        );
    }

    // --- PrincipalKey ---

    #[test]
    fn principal_key_new_rejects_empty_principal_id() {
        let err = PrincipalKey::new("", "schema").unwrap_err();
        assert!(matches!(err, NamespaceError::EmptyPrincipalId));
    }

    #[test]
    fn principal_key_new_rejects_empty_schema() {
        let err = PrincipalKey::new("alice@example.org", "").unwrap_err();
        assert!(matches!(err, NamespaceError::EmptySchema));
    }

    #[test]
    fn principal_key_new_rejects_colon_in_principal_id() {
        let err = PrincipalKey::new("alice:bad", "schema").unwrap_err();
        assert!(matches!(err, NamespaceError::ForbiddenCharacter { .. }));
    }

    #[test]
    fn principal_key_new_round_trips() {
        let pk = PrincipalKey::new("alice@example.org", "calendar").unwrap();
        assert_eq!(pk.principal_id, "alice@example.org");
        assert_eq!(pk.schema, "calendar");
    }

    // --- MemoryValue ---

    #[test]
    fn memory_value_kind_json() {
        let v = MemoryValue::Json(serde_json::json!({"a": 1}));
        assert_eq!(v.kind(), ValueKind::Json);
    }

    #[test]
    fn memory_value_kind_markdown() {
        let v = MemoryValue::Markdown("# Hello".into());
        assert_eq!(v.kind(), ValueKind::Markdown);
    }

    #[test]
    fn memory_value_kind_blob() {
        let v = MemoryValue::Blob(vec![0, 1, 2]);
        assert_eq!(v.kind(), ValueKind::Blob);
    }

    #[test]
    fn memory_value_kind_text() {
        let v = MemoryValue::Text("hello".into());
        assert_eq!(v.kind(), ValueKind::Text);
    }

    #[test]
    fn memory_value_serde_round_trip_json() {
        let v = MemoryValue::Json(serde_json::json!({"x": 42}));
        let json = serde_json::to_string(&v).unwrap();
        let back: MemoryValue = serde_json::from_str(&json).unwrap();
        match back {
            MemoryValue::Json(j) => assert_eq!(j, serde_json::json!({"x": 42})),
            _ => panic!("expected Json"),
        }
    }

    #[test]
    fn memory_value_serde_round_trip_markdown() {
        let v = MemoryValue::Markdown("# Title\n\nbody".into());
        let json = serde_json::to_string(&v).unwrap();
        let back: MemoryValue = serde_json::from_str(&json).unwrap();
        assert_eq!(back, MemoryValue::Markdown("# Title\n\nbody".into()));
    }

    // --- MemoryError ---

    #[test]
    fn memory_error_display_key_traversal() {
        let e = MemoryError::KeyTraversalRejected {
            key: "../bad".into(),
        };
        assert!(e.to_string().contains("../bad"));
    }

    #[test]
    fn memory_error_display_collective() {
        let e = MemoryError::CollectiveNotYetAvailable {
            ship_target: "v1.5",
            landing_story: "E10 Story 10.4",
        };
        let s = e.to_string();
        assert!(s.contains("v1.5"));
        assert!(s.contains("E10 Story 10.4"));
    }

    #[test]
    fn memory_error_display_invalid_key() {
        let e = MemoryError::InvalidKey {
            key: "bad/key".into(),
        };
        assert!(e.to_string().contains("bad/key"));
    }

    #[test]
    fn memory_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let e: MemoryError = io_err.into();
        match e {
            MemoryError::Io(_) => {}
            _ => panic!("expected Io variant"),
        }
    }
}
