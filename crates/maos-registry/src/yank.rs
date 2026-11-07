//! Restart-safe server-sequence yank polling. Wall time is display-only.
use maos_domain::ports::registry::{RegistryError, YankEntry, YankList};
use std::sync::{Arc, Mutex};

pub trait YankSource: Send + Sync {
    fn fetch_yanks(&self, since_ns: u64) -> Result<YankList, RegistryError>;
}
impl YankSource for crate::client::McpSpiritRegistryClient {
    fn fetch_yanks(&self, since_ns: u64) -> Result<YankList, RegistryError> {
        self.yanks_since(since_ns)
    }
}

impl YankSource for crate::client::NullSpiritRegistryClient {
    fn fetch_yanks(&self, _: u64) -> Result<YankList, RegistryError> {
        Ok(YankList::new(Vec::new()))
    }
}

/// The legacy wire field `since_ns` now carries a durable server sequence.
#[derive(Debug, Clone, Default)]
pub struct YankCache {
    pub last_seen_ns: u64,
}
impl YankCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn apply(&mut self, list: &YankList) {
        self.last_seen_ns = list
            .entries
            .iter()
            .map(|e| e.sequence)
            .max()
            .unwrap_or(self.last_seen_ns)
            .max(self.last_seen_ns);
    }
}

pub trait YankObserver: Send + Sync {
    /// Authenticate and journal before returning success and advancing the cursor.
    fn on_yank(&self, entry: &YankEntry) -> Result<(), RegistryError>;
    /// Running-vetting re-evaluation shares the yank cadence, even on an empty list.
    fn on_poll(&self) -> Result<(), RegistryError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum YankPolicy {
    #[default]
    Warn,
    Quarantine,
    AutoRevoke,
}
impl std::str::FromStr for YankPolicy {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "warn" => Ok(Self::Warn),
            "quarantine" => Ok(Self::Quarantine),
            "auto-revoke" => Ok(Self::AutoRevoke),
            _ => Err(format!("invalid yank policy '{value}'")),
        }
    }
}
/// Typed request at the non-kernel boundary. Quarantine effects require final KQ1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum YankPolicyOutcome {
    Warned,
    QuarantineRequested,
    AutoRevokeRequested,
}
impl YankPolicy {
    pub fn outcome(self) -> YankPolicyOutcome {
        match self {
            Self::Warn => YankPolicyOutcome::Warned,
            Self::Quarantine => YankPolicyOutcome::QuarantineRequested,
            Self::AutoRevoke => YankPolicyOutcome::AutoRevokeRequested,
        }
    }
}

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
    pub fn poll_once(&self) -> Result<usize, RegistryError> {
        // Serialize polls through callbacks; a concurrent poll cannot replay rows.
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| RegistryError::Transport(e.to_string()))?;
        self.observer.on_poll()?;
        let list = self.source.fetch_yanks(cache.last_seen_ns)?;
        let mut count = 0;
        for entry in &list.entries {
            if entry.sequence <= cache.last_seen_ns {
                continue;
            }
            self.observer.on_yank(entry)?;
            cache.last_seen_ns = entry.sequence;
            count += 1;
        }
        Ok(count)
    }
    pub fn last_seen_ns(&self) -> u64 {
        self.cache.lock().map(|g| g.last_seen_ns).unwrap_or(0)
    }
    pub fn load_cursor(&self) {
        if let Some(cursor) = load_cursor() {
            if let Ok(mut cache) = self.cache.lock() {
                cache.last_seen_ns = cursor.last_seen_ns;
            }
        }
    }
    pub fn save_cursor(&self) -> std::io::Result<()> {
        save_cursor(self.last_seen_ns(), 0)
    }
}

#[cfg(feature = "fixture_replay")]
impl crate::client::McpSpiritRegistryClient {
    pub fn yanks_poll_once(&self, cache: &mut YankCache) -> Result<usize, RegistryError> {
        let list = self.yanks_since(cache.last_seen_ns)?;
        let count = list.entries.len();
        cache.apply(&list);
        Ok(count)
    }
}

pub async fn yank_poller_production_loop(
    poller: Arc<YankPoller>,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
    interval: std::time::Duration,
) {
    use std::sync::atomic::Ordering;
    poller.load_cursor();
    while !shutdown.load(Ordering::SeqCst) {
        match poller.poll_once() {
            Ok(_) => {
                if let Err(error) = poller.save_cursor() {
                    eprintln!("yank cursor save failed: {error}");
                }
            }
            Err(error) => eprintln!("yank poll failed: {error}"),
        }
        let deadline = tokio::time::Instant::now() + interval;
        while tokio::time::Instant::now() < deadline && !shutdown.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }
    if let Err(error) = poller.save_cursor() {
        eprintln!("yank cursor shutdown save failed: {error}");
    }
}

pub fn resolve_poll_interval() -> std::time::Duration {
    let secs = std::env::var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(300);
    if secs == 0 {
        std::time::Duration::ZERO
    } else {
        std::time::Duration::from_secs(secs.clamp(30, 3600))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YankCursorFile {
    pub schema_version: u32,
    pub last_seen_ns: u64,
    pub last_seen_yank_count: u64,
}
pub fn cursor_file_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("MAOS_REGISTRY_YANK_CURSOR_PATH") {
        return path.into();
    }
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
        .join(".local/share/maos/registry/yank_cursor.json")
}
pub fn save_cursor(last_seen_ns: u64, last_seen_yank_count: u64) -> std::io::Result<()> {
    use std::io::Write;
    let path = cursor_file_path();
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("cursor has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(
        &mut file,
        &YankCursorFile {
            schema_version: 1,
            last_seen_ns,
            last_seen_yank_count,
        },
    )
    .map_err(std::io::Error::other)?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(std::io::Error::other)?;
    Ok(())
}
pub fn load_cursor() -> Option<YankCursorFile> {
    let bytes = std::fs::read(cursor_file_path()).ok()?;
    match serde_json::from_slice::<YankCursorFile>(&bytes) {
        Ok(cursor) if cursor.schema_version == 1 => Some(cursor),
        _ => {
            eprintln!("yank cursor invalid or obsolete; replaying signed history");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source(Vec<YankEntry>);
    impl YankSource for Source {
        fn fetch_yanks(&self, _: u64) -> Result<YankList, RegistryError> {
            Ok(YankList::new(self.0.clone()))
        }
    }
    #[derive(Default)]
    struct Observer(Mutex<Vec<YankEntry>>);
    impl YankObserver for Observer {
        fn on_yank(&self, entry: &YankEntry) -> Result<(), RegistryError> {
            self.0.lock().unwrap().push(entry.clone());
            Ok(())
        }
    }
    fn entry(sequence: u64) -> YankEntry {
        let mut entry = YankEntry::new("vetted".into(), "1.0.0".into(), 0, "withdrawn".into());
        entry.sequence = sequence;
        entry
    }
    #[test]
    fn sequence_does_not_depend_on_display_time() {
        let mut cache = YankCache::new();
        cache.apply(&YankList::new(vec![entry(7)]));
        assert_eq!(cache.last_seen_ns, 7);
        cache.apply(&YankList::new(vec![]));
        assert_eq!(cache.last_seen_ns, 7);
    }
    #[test]
    fn repeated_source_does_not_replay_journal_rows() {
        let observer = Arc::new(Observer::default());
        let poller = YankPoller::new(Arc::new(Source(vec![entry(1), entry(2)])), observer.clone());
        assert_eq!(poller.poll_once().unwrap(), 2);
        assert_eq!(poller.poll_once().unwrap(), 0);
        assert_eq!(observer.0.lock().unwrap().len(), 2);
    }
    #[test]
    fn policy_outcomes_are_distinct_ac8() {
        for (name, expected) in [
            ("warn", YankPolicyOutcome::Warned),
            ("quarantine", YankPolicyOutcome::QuarantineRequested),
            ("auto-revoke", YankPolicyOutcome::AutoRevokeRequested),
        ] {
            assert_eq!(name.parse::<YankPolicy>().unwrap().outcome(), expected);
        }
        assert!("terminate".parse::<YankPolicy>().is_err());
    }
}
