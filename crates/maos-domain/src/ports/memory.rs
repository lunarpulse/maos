//! Memory Manager port trait per architecture §4.2.
//!
//! Provides three named memory tiers (`private`, `shared`, `collective`)
//! and enforces I5 namespace scopes. At v0.1-α this was an empty hexagonal
//! adapter shell; Story 4.3 lands the three-tier mechanics with Principal
//! Namespace (ADR-026) and Self-Telemetry (FR56).

use crate::invariants::i5::{MemoryScope, NamespaceKey};
use crate::memory::{
    ExportEntry, ForgetReceipt, MemoryEntry, MemoryError, MemoryNamespace, MemoryTier, MemoryValue,
    PrincipalIndexRow,
};

/// Memory Manager — namespace scope enforcement and tiered memory.
///
/// Per §4.2: "The kernel enforces three memory tiers: private,
/// shared, and collective. Spirits cannot read outside their scope."
pub trait MemoryManagerPort {
    /// Class: data-movement
    ///
    /// Validates that a read from the given namespace key is permitted
    /// under the provided memory scope. Returns `true` if the read
    /// would not cross a scope boundary.
    fn validate_namespace_read(&self, key: &NamespaceKey<MemoryScope>) -> bool;

    /// Class: data-movement
    ///
    /// Validates that a write to the given namespace key is permitted
    /// under the provided memory scope. Returns `true` if the write
    /// would not cross a scope boundary.
    fn validate_namespace_write(&self, key: &NamespaceKey<MemoryScope>) -> bool;

    // ---- Story 4.3 additive methods (ADR-010 sync-trait rule) ----

    /// Class: data-movement
    ///
    /// Write a value to the given tier + namespace.  `spirit_pid` is
    /// kernel-set from the calling context — not Spirit-supplied.
    /// The `Private` tier routes to the per-Spirit in-memory map + optional
    /// filesystem spill; `Shared` routes to the Host-wide SQLite kv;
    /// `Collective` returns [`MemoryError::CollectiveNotYetAvailable`].
    fn write(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), MemoryError>;

    /// Class: data-movement
    ///
    /// Read a value from the given tier + namespace.  Returns `Ok(None)`
    /// when no write has been recorded for this key.
    fn read(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, MemoryError>;

    /// Class: data-movement
    ///
    /// Scan entries matching a key prefix within the given tier + namespace,
    /// up to `limit` entries.  Scan order is NOT guaranteed stable (HashMap
    /// iteration is non-deterministic).
    fn scan(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError>;

    /// Class: data-movement
    ///
    /// Subject-access query per ADR-026 — returns every `(writer_spirit_pid,
    /// schema, key, timestamp_ns)` indexed for the given `principal_id`
    /// across ALL Spirits on this Host.  Carries NO content.
    fn subject_access(&self, principal_id: &str) -> Result<Vec<PrincipalIndexRow>, MemoryError>;

    /// Class: supervision
    ///
    /// Right-to-be-forgotten cascade per ADR-026 — deletes every
    /// principal-namespaced entry for the given `principal_id` from the
    /// in-memory map, filesystem area, and index table.  Journals a
    /// Transparency Log frame carrying the receipt.
    fn forget(&self, principal_id: &str) -> Result<ForgetReceipt, MemoryError>;

    /// Class: data-movement
    ///
    /// Export entries for a principal with optional redaction.  When
    /// `include_principal` is false, content fields are replaced with
    /// `<REDACTED:type=principal-namespace>` placeholders per ADR-028.
    fn export_redactable(
        &self,
        principal_id: &str,
        include_principal: bool,
    ) -> Result<Vec<ExportEntry>, MemoryError>;
}
