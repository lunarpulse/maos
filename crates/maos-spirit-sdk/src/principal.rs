//! Story 18-4 (AC3.1, D-18-4-02 A) — the Spirit-facing write port into the
//! FR31 principal namespace (ADR-026). The host builds one per Spirit, FUSED
//! to the pid the scheduler resolved: the Spirit names the principal, the
//! schema and the key, never a pid, so it cannot read or write as another
//! Spirit. Values are Markdown (the durable Private-tier kind), so a record
//! survives a process restart and `maosctl forget --principal` erases it.

/// A pid-fused principal-namespace store.
pub trait PrincipalMemoryPort: Send + Sync {
    /// The pid the host fused this port to (diagnostic only).
    fn pid(&self) -> u32;

    /// Write `markdown` under `principal:<principal_id>:<schema>` / `key`.
    fn write(
        &self,
        principal_id: &str,
        schema: &str,
        key: &str,
        markdown: &str,
    ) -> Result<(), String>;

    /// Read the value under `principal:<principal_id>:<schema>` / `key`.
    fn read(&self, principal_id: &str, schema: &str, key: &str) -> Result<Option<String>, String>;
}
