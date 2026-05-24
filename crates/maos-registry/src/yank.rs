//! Yank propagation — kernel-side 5-min poll task.
//!
//! The `YankPoller` piggybacks on Story 5.4's CRL poll loop, calling
//! `registry.yanks_since` and emitting `FrameKind::RegistryYank` TL rows
//! for each new yank entry.
//!
//! Yank vs CRL distinction (per ADR-008 + FR59):
//! - CRL (FR13, Story 5.4) — revoked Spirit identity → running instances TERMINATED.
//! - YANK (FR59, Story 5.5d) — registry retracts a version → new installs BLOCKED;
//!   running instances of the yanked version ARE NOT terminated.

use std::sync::{Arc, Mutex};

use maos_domain::ports::registry::{RegistryError, YankEntry, YankList};

/// Abstraction over yank fetching — allows `YankPoller` to work with both
/// the production `McpSpiritRegistryClient` and test doubles without
/// requiring the internal `yanks_since` method on the public
/// `SpiritRegistryClient` trait.
pub trait YankSource: Send + Sync {
    fn fetch_yanks(&self, since_ns: u64) -> Result<YankList, RegistryError>;
}

impl YankSource for crate::client::McpSpiritRegistryClient {
    fn fetch_yanks(&self, since_ns: u64) -> Result<YankList, RegistryError> {
        self.yanks_since(since_ns)
    }
}

/// In-memory cache of the last-seen yank timestamp.
#[derive(Debug, Clone)]
pub struct YankCache {
    /// The maximum `yanked_at_ns` observed across all poll iterations.
    pub last_seen_ns: u64,
}

impl YankCache {
    pub fn new() -> Self {
        Self { last_seen_ns: 0 }
    }

    /// Apply a yank list — advance `last_seen_ns` to the max entry timestamp.
    pub fn apply(&mut self, list: &YankList) {
        let max = list
            .entries
            .iter()
            .map(|e| e.yanked_at_ns)
            .max()
            .unwrap_or(self.last_seen_ns);
        if max > self.last_seen_ns {
            self.last_seen_ns = max;
        }
    }
}

/// Per-yank callback trait — allows the poller to emit TL rows without
/// depending on kernel-core types.
pub trait YankObserver: Send + Sync {
    fn on_yank(&self, entry: &YankEntry);
}

/// The kernel-side 5-min yank polling task.
pub struct YankPoller {
    source: Arc<dyn YankSource>,
    cache: Mutex<YankCache>,
    observer: Arc<dyn YankObserver>,
}

impl YankPoller {
    pub fn new(source: Arc<dyn YankSource>, observer: Arc<dyn YankObserver>) -> Self {
        Self {
            source,
            cache: Mutex::new(YankCache::new()),
            observer,
        }
    }

    /// Called every 5 min by the kernel's polling task.
    /// Returns the number of new yank entries observed.
    pub fn poll_once(&self) -> Result<usize, RegistryError> {
        let since_ns = self.cache.lock().unwrap().last_seen_ns;
        let list = self.source.fetch_yanks(since_ns)?;
        let count = list.entries.len();

        for entry in &list.entries {
            self.observer.on_yank(entry);
        }

        self.cache.lock().unwrap().apply(&list);
        Ok(count)
    }

    /// Expose the cache's `last_seen_ns` for testing.
    pub fn last_seen_ns(&self) -> u64 {
        self.cache.lock().unwrap().last_seen_ns
    }
}

// The McpSpiritRegistryClient-specific poller (production impl)
#[cfg(feature = "fixture_replay")]
impl crate::client::McpSpiritRegistryClient {
    /// Poll yanks from the registry.
    pub fn yanks_poll_once(&self, cache: &mut YankCache) -> Result<usize, RegistryError> {
        let since_ns = cache.last_seen_ns;
        let list = self.yanks_since(since_ns)?;
        let count = list.entries.len();
        cache.apply(&list);
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::registry::SpiritId;

    struct TestObserver {
        seen: Mutex<Vec<YankEntry>>,
    }

    impl TestObserver {
        fn new() -> Self {
            Self {
                seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl YankObserver for TestObserver {
        fn on_yank(&self, entry: &YankEntry) {
            self.seen.lock().unwrap().push(entry.clone());
        }
    }

    struct FixedYankSource {
        entries: Vec<YankEntry>,
    }

    impl FixedYankSource {
        fn new(entries: Vec<YankEntry>) -> Self {
            Self { entries }
        }
    }

    impl YankSource for FixedYankSource {
        fn fetch_yanks(&self, _since_ns: u64) -> Result<YankList, RegistryError> {
            Ok(YankList::new(self.entries.clone()))
        }
    }

    #[test]
    fn cache_advances_to_max_yanked_at_ns_on_apply() {
        let mut cache = YankCache::new();
        let list = YankList::new(vec![
            YankEntry::new(SpiritId::from("s1"), "1.0.0".into(), 100, "reason".into()),
            YankEntry::new(SpiritId::from("s2"), "2.0.0".into(), 200, "reason".into()),
        ]);
        cache.apply(&list);
        assert_eq!(cache.last_seen_ns, 200);
    }

    #[test]
    fn cache_does_not_regress_on_empty_list() {
        let mut cache = YankCache::new();
        cache.last_seen_ns = 42;
        let list = YankList::new(vec![]);
        cache.apply(&list);
        assert_eq!(cache.last_seen_ns, 42);
    }

    #[test]
    fn poll_once_with_no_yanks_returns_zero() {
        let source = Arc::new(FixedYankSource::new(vec![]));
        let observer = Arc::new(TestObserver::new());
        let poller = YankPoller::new(source, observer.clone());
        let count = poller.poll_once().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn poll_once_with_two_yanks_emits_two_tl_rows() {
        let entries = vec![
            YankEntry::new(SpiritId::from("s1"), "1.0.0".into(), 100, "critical vuln".into()),
            YankEntry::new(SpiritId::from("s2"), "2.0.0".into(), 200, "data loss bug".into()),
        ];
        let source = Arc::new(FixedYankSource::new(entries.clone()));
        let observer = Arc::new(TestObserver::new());
        let poller = YankPoller::new(source, observer.clone());

        let count = poller.poll_once().unwrap();
        assert_eq!(count, 2);

        let seen = observer.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].spirit_id, SpiritId::from("s1"));
        assert_eq!(seen[0].version, "1.0.0");
        assert_eq!(seen[1].spirit_id, SpiritId::from("s2"));
        assert_eq!(seen[1].version, "2.0.0");
    }

    #[test]
    fn poll_once_with_monotonic_now_ns_used() {
        let entries = vec![
            YankEntry::new(SpiritId::from("s1"), "1.0.0".into(), 300, "reason".into()),
            YankEntry::new(SpiritId::from("s2"), "2.0.0".into(), 500, "reason".into()),
        ];
        let source = Arc::new(FixedYankSource::new(entries));
        let observer = Arc::new(TestObserver::new());
        let poller = YankPoller::new(source, observer);

        assert_eq!(poller.last_seen_ns(), 0);
        poller.poll_once().unwrap();
        assert_eq!(poller.last_seen_ns(), 500);
    }
}
