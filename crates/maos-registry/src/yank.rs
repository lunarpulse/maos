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

#[derive(Debug, Default)]
struct YankCache {
    last_seen_ns: u64,
    resync: bool,
}

pub enum YankDisposition {
    /// Authority was checked with `yank_auth::verify_entry`, even if not actionable.
    Verified,
    /// Permanent refusal is journalled, never used to advance the trusted cursor.
    Rejected(String),
}

pub trait YankObserver: Send + Sync {
    fn on_yank(&self, entry: &YankEntry) -> Result<YankDisposition, RegistryError>;
    fn on_rejected(&self, entry: &YankEntry, reason: &str) -> Result<(), RegistryError> {
        eprintln!("yank-rejected sequence={}: {reason}", entry.sequence);
        Ok(())
    }
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
    registry_uri: String,
    persistent: std::sync::atomic::AtomicBool,
}
impl YankPoller {
    pub fn new(
        source: Arc<dyn YankSource>,
        observer: Arc<dyn YankObserver>,
        registry_uri: String,
    ) -> Self {
        Self {
            source,
            cache: Mutex::new(YankCache::default()),
            observer,
            registry_uri,
            persistent: std::sync::atomic::AtomicBool::new(false),
        }
    }
    pub fn poll_once(&self) -> Result<usize, RegistryError> {
        let since = self.begin_poll()?;
        self.apply_list(self.source.fetch_yanks(since)?)
    }
    fn begin_poll(&self) -> Result<u64, RegistryError> {
        if let Err(error) = self.observer.on_poll() {
            eprintln!("local vetting re-evaluation failed; still fetching yanks: {error}");
        }
        let cache = self
            .cache
            .lock()
            .map_err(|e| RegistryError::Transport(e.to_string()))?;
        Ok(if cache.resync { 0 } else { cache.last_seen_ns })
    }
    fn apply_list(&self, list: YankList) -> Result<usize, RegistryError> {
        // Serialize authenticated delivery; overlapping fetches cannot replay rows.
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| RegistryError::Transport(e.to_string()))?;
        if let Some(head) = list.head.filter(|head| *head < cache.last_seen_ns) {
            cache.resync = false;
            let error = RegistryError::HeadBehindCursor {
                head,
                cursor: cache.last_seen_ns,
            };
            eprintln!("yank-head-rollback: {error}");
            return Err(error);
        }
        cache.resync = false;
        let mut count = 0;
        let mut expected = cache.last_seen_ns.checked_add(1);
        for entry in &list.entries {
            if entry.sequence <= cache.last_seen_ns {
                continue;
            }
            if Some(entry.sequence) != expected {
                self.observer.on_rejected(
                    entry,
                    &format!(
                        "yank-sequence-anomaly: expected {expected:?}, got {}",
                        entry.sequence
                    ),
                )?;
                cache.resync = true;
                continue;
            }
            expected = entry.sequence.checked_add(1);
            match self.observer.on_yank(entry)? {
                YankDisposition::Rejected(reason) => self.observer.on_rejected(entry, &reason)?,
                YankDisposition::Verified => {
                    if self.persistent.load(std::sync::atomic::Ordering::SeqCst) {
                        save_cursor(&self.registry_uri, entry.sequence)
                            .map_err(|e| RegistryError::Transport(e.to_string()))?;
                    }
                    cache.last_seen_ns = entry.sequence;
                    count += 1;
                }
            }
        }
        Ok(count)
    }
    pub fn last_seen_ns(&self) -> u64 {
        self.cache.lock().map(|g| g.last_seen_ns).unwrap_or(0)
    }
    /// Enable per-row persistence only after resolving an operator-owned path.
    pub fn load_cursor(&self) -> std::io::Result<()> {
        let cursor = load_cursor(&self.registry_uri)?;
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        cache.last_seen_ns = cursor.map_or(0, |cursor| cursor.last_seen_ns);
        self.persistent
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    pub fn save_cursor(&self) -> std::io::Result<()> {
        save_cursor(&self.registry_uri, self.last_seen_ns())
    }
}

pub async fn yank_poller_production_loop(
    poller: Arc<YankPoller>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    interval: std::time::Duration,
) {
    if let Err(error) = poller.load_cursor() {
        eprintln!("yank cursor unavailable; not using a shared fallback: {error}");
    }
    type Reply = tokio::sync::oneshot::Sender<Result<YankList, RegistryError>>;
    let (requests, incoming) = std::sync::mpsc::sync_channel::<(u64, Reply)>(1);
    let source = poller.source.clone();
    // Only the transport crosses this boundary, never the observer/scheduler/
    // applier. A blocked DNS/connect cannot retain kernel owners during drain.
    let worker = std::thread::spawn(move || {
        while let Ok((since, reply)) = incoming.recv() {
            let _ = reply.send(source.fetch_yanks(since));
        }
    });
    let mut in_flight = false;
    loop {
        if *shutdown.borrow() {
            break;
        }
        let deadline = tokio::time::Instant::now() + interval;
        match poller.begin_poll() {
            Err(error) => eprintln!("yank poll failed: {error}"),
            Ok(since) => {
                let (reply, result) = tokio::sync::oneshot::channel();
                if requests.send((since, reply)).is_err() {
                    break;
                }
                in_flight = true;
                let response = tokio::select! {
                    response = result => Some(response),
                    _ = shutdown.changed() => None,
                };
                let Some(response) = response else {
                    break;
                };
                in_flight = false;
                match response {
                    Ok(Ok(list)) => {
                        if let Err(error) = poller.apply_list(list) {
                            eprintln!("yank poll failed: {error}");
                        }
                    }
                    Ok(Err(error)) => eprintln!("yank fetch failed: {error}"),
                    Err(error) => {
                        eprintln!("yank fetch worker failed: {error}");
                        break;
                    }
                }
            }
        }
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {},
            _ = shutdown.changed() => break,
        }
    }
    drop(requests);
    // Idle workers are joined. An in-flight transport retires independently;
    // its reply is cancelled and can no longer deliver a kernel policy effect.
    if !in_flight && worker.join().is_err() {
        eprintln!("yank transport worker panicked");
    }
}

pub fn resolve_poll_interval() -> Result<std::time::Duration, String> {
    let secs = std::env::var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(300);
    if secs == 0 {
        Err(
            "MAOS_REGISTRY_YANK_POLL_INTERVAL_S=0 would disable local vetting; use 30..300 seconds"
                .into(),
        )
    } else {
        Ok(std::time::Duration::from_secs(secs.clamp(30, 300)))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YankCursorFile {
    pub schema_version: u32,
    pub registry_uri: String,
    pub last_seen_ns: u64,
}
pub fn cursor_file_path() -> std::io::Result<std::path::PathBuf> {
    if let Some(path) = std::env::var_os("MAOS_REGISTRY_YANK_CURSOR_PATH") {
        if path.is_empty() {
            return Err(std::io::Error::other("empty yank cursor path"));
        }
        return Ok(path.into());
    }
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .ok_or_else(|| std::io::Error::other("HOME or explicit yank cursor path is required"))?;
    Ok(std::path::PathBuf::from(home).join(".local/share/maos/registry/yank_cursor.json"))
}
pub fn save_cursor(registry_uri: &str, last_seen_ns: u64) -> std::io::Result<()> {
    use std::io::Write;
    let path = cursor_file_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("cursor has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(
        &mut file,
        &YankCursorFile {
            schema_version: 2,
            registry_uri: registry_uri.into(),
            last_seen_ns,
        },
    )
    .map_err(std::io::Error::other)?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(&path).map_err(std::io::Error::other)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn load_cursor(registry_uri: &str) -> std::io::Result<Option<YankCursorFile>> {
    let bytes = match std::fs::read(cursor_file_path()?) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    match serde_json::from_slice::<YankCursorFile>(&bytes) {
        Ok(cursor) if cursor.schema_version == 2 && cursor.registry_uri == registry_uri => {
            Ok(Some(cursor))
        }
        _ => {
            eprintln!("yank cursor obsolete or for another registry; replaying signed history");
            Ok(None)
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
        fn on_yank(&self, entry: &YankEntry) -> Result<YankDisposition, RegistryError> {
            self.0.lock().unwrap().push(entry.clone());
            Ok(YankDisposition::Verified)
        }
    }
    fn entry(sequence: u64) -> YankEntry {
        let mut entry = YankEntry::new("vetted".into(), "1.0.0".into(), 0, "withdrawn".into());
        entry.sequence = sequence;
        entry
    }
    #[test]
    fn authenticated_head_rollback_never_mutates_or_resets_cursor_d3_f5() {
        let poller = YankPoller::new(
            Arc::new(Source(Vec::new())),
            Arc::new(Observer::default()),
            "https://registry.example/mcp".into(),
        );
        poller.cache.lock().unwrap().last_seen_ns = 5;
        let mut list = YankList::new(Vec::new());
        list.head = Some(2);
        let result = poller.apply_list(list);
        assert!(
            matches!(
                result,
                Err(RegistryError::HeadBehindCursor { head: 2, cursor: 5 })
            ),
            "{result:?}"
        );
        assert_eq!(poller.last_seen_ns(), 5);
        assert_eq!(
            poller.begin_poll().unwrap(),
            5,
            "rollback must not resync from zero"
        );
    }
    #[test]
    fn repeated_source_does_not_replay_journal_rows() {
        let observer = Arc::new(Observer::default());
        let poller = YankPoller::new(
            Arc::new(Source(vec![entry(1), entry(2)])),
            observer.clone(),
            "fixture://sequence".into(),
        );
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
