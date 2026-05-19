#![forbid(unsafe_code)]

//! `SpiritMemoryView` — pid-fused reborrow surface for the Memory Manager
//! (Story 4.3).
//!
//! The wire-protocol handler constructs a `SpiritMemoryView` once per
//! request from the kernel-set `spirit_pid`, fusing it into every
//! memory call.  This is the I5 enforcement substrate — the Spirit
//! never supplies `spirit_pid` directly.

use maos_domain::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;

use crate::memory::MemoryManagerAdapter;

/// A memory surface scoped to a single Spirit, with the pid fused at
/// construction time.  All methods forward to the underlying adapter.
pub struct SpiritMemoryView<'a> {
    adapter: &'a MemoryManagerAdapter,
    spirit_pid: u32,
}

impl<'a> SpiritMemoryView<'a> {
    pub fn new(adapter: &'a MemoryManagerAdapter, spirit_pid: u32) -> Self {
        Self {
            adapter,
            spirit_pid,
        }
    }

    pub fn write(
        &self,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), MemoryError> {
        self.adapter
            .write(self.spirit_pid, tier, namespace, key, value)
    }

    pub fn read(
        &self,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, MemoryError> {
        self.adapter
            .read(self.spirit_pid, tier, namespace, key)
    }

    pub fn scan(
        &self,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.adapter
            .scan(self.spirit_pid, tier, namespace, prefix, limit)
    }
}
