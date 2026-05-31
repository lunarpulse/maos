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
        let since_ns = self
            .cache
            .lock()
            .map_err(|e| RegistryError::Transport(format!("yank cache lock poisoned: {e}")))?
            .last_seen_ns;
        let list = self.source.fetch_yanks(since_ns)?;
        let count = list.entries.len();

        for entry in &list.entries {
            self.observer.on_yank(entry);
        }

        self.cache
            .lock()
            .map_err(|e| RegistryError::Transport(format!("yank cache lock poisoned: {e}")))?
            .apply(&list);
        Ok(count)
    }

    /// Expose the cache's `last_seen_ns` for testing.
    pub fn last_seen_ns(&self) -> u64 {
        self.cache.lock().map(|g| g.last_seen_ns).unwrap_or(0)
    }

    /// Seed the cache from a persisted cursor file.
    ///
    /// Implements wall-clock remapping (Story 7.2 AC5 Option B): the cursor
    /// stores both monotonic-ns and a wall-clock anchor. On restart, the
    /// monotonic clock has reset, so we compute:
    ///   `new_since_ns = max(current_monotonic_ns - wall_clock_elapsed_ns, 0)`
    /// This approximates the old cursor position in the new monotonic timeline.
    /// If the remapping would underflow (system time went backward), we fall
    /// back to 0 (full historical replay — safe but may process duplicates).
    pub fn load_cursor(&self) {
        if let Some(cursor) = load_cursor() {
            if let Ok(mut guard) = self.cache.lock() {
                let remapped_ns = remap_cursor_to_current_monotonic(&cursor, guard.last_seen_ns);
                guard.last_seen_ns = remapped_ns;
            }
        }
    }

    /// Persist the current cache state to disk.
    pub fn save_cursor(&self) -> std::io::Result<()> {
        let last_seen_ns = self.last_seen_ns();
        save_cursor(last_seen_ns, 0)
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

/// Story 7.2 AC4 — production yank-poller loop wired into the kernel
/// composition root.
///
/// Owns the poll cadence (default 5min per FR59; configurable via
/// `MAOS_REGISTRY_YANK_POLL_INTERVAL_S`, clamped `[30s, 3600s]`), respects
/// the `shutdown` flag via Story 5.5c's JoinHandle discipline, and emits a
/// `tracing::info` row each iteration for operator observability.
pub async fn yank_poller_production_loop(
    poller: std::sync::Arc<YankPoller>,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    interval: std::time::Duration,
) {
    use std::sync::atomic::Ordering;

    // Load persisted cursor before first poll.
    poller.load_cursor();

    let mut iter: u64 = 0;
    while !shutdown.load(Ordering::SeqCst) {
        iter = iter.wrapping_add(1);
        match poller.poll_once() {
            Ok(applied) => {
                #[cfg(feature = "tracing")]
                tracing::info!(
                    "yank poller iteration {iter} — applied {applied} yanks; last_seen_ns={}",
                    poller.last_seen_ns()
                );
                // Persist cursor after each successful poll.
                if let Err(e) = poller.save_cursor() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("yank poller: failed to save cursor: {e}");
                }
            }
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::warn!("yank poller iteration {iter} failed: {e}");
            }
        }
        let mut slept = std::time::Duration::ZERO;
        let tick = std::time::Duration::from_millis(250);
        while slept < interval && !shutdown.load(Ordering::SeqCst) {
            tokio::time::sleep(tick).await;
            slept += tick;
        }
    }
    // Final save on graceful shutdown.
    if let Err(e) = poller.save_cursor() {
        #[cfg(feature = "tracing")]
        tracing::warn!("yank poller: failed to save cursor on shutdown: {e}");
    }
    #[cfg(feature = "tracing")]
    tracing::info!("yank poller loop exiting on shutdown signal");
}

/// Story 7.2 — resolve the poll interval from `MAOS_REGISTRY_YANK_POLL_INTERVAL_S`
/// env var. Returns `Duration::ZERO` to disable the poller. Otherwise clamped
/// to `[30s, 3600s]` with default 300s (5min) per FR59.
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

// Story 7.2 (closes 5.5d Low #32) — yank cursor persistence across kernel
// restarts.
//
// Dev chose Option A (the smaller server-side change): the cursor file at
// `~/.local/share/maos/registry/yank_cursor.json` stores both the last-seen
// monotonic nanoseconds AND a wall-clock anchor. On restart the kernel loads
// the cursor, uses the wall-clock anchor to map back into the new
// monotonic-now timeline, and seeds the in-memory cache. Servers continue to
// accept `since_ns: u64` unchanged — no JSON-RPC schema change.

/// On-disk cursor shape. Cross-restart-safe via the wall-clock anchor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YankCursorFile {
    /// Wall-clock timestamp at the moment the cursor was last saved.
    pub last_saved_iso8601: String,
    /// Monotonic-nanos snapshot at the moment the cursor was last saved.
    pub last_seen_ns: u64,
    /// Number of yanks observed across the kernel's lifetime (informational).
    #[serde(default)]
    pub last_seen_yank_count: u64,
}

/// Compute the default cursor file path: `~/.local/share/maos/registry/yank_cursor.json`.
/// Test code may override via `MAOS_REGISTRY_YANK_CURSOR_PATH`.
pub fn cursor_file_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("MAOS_REGISTRY_YANK_CURSOR_PATH") {
        return std::path::PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(home).join(".local/share/maos/registry/yank_cursor.json")
}

/// Save the cursor to disk. Stamps `last_saved_iso8601` with the current UTC
/// wall-clock time formatted as RFC 3339 (using `std::time::SystemTime` so we
/// don't introduce a `chrono` dep just for the cursor).
pub fn save_cursor(last_seen_ns: u64, last_seen_yank_count: u64) -> std::io::Result<()> {
    let path = cursor_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let last_saved_iso8601 = current_iso8601_utc();
    let cursor = YankCursorFile {
        last_saved_iso8601,
        last_seen_ns,
        last_seen_yank_count,
    };
    let bytes = serde_json::to_vec_pretty(&cursor)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    std::fs::write(&path, bytes)
}

/// Load the cursor from disk, returning `None` if the file does not exist
/// (first boot or after a cache wipe — full historical replay is acceptable).
pub fn load_cursor() -> Option<YankCursorFile> {
    let path = cursor_file_path();
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return None,
    };
    match serde_json::from_slice::<YankCursorFile>(&bytes) {
        Ok(c) => Some(c),
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "yank cursor file {:?} is malformed, starting from zero: {e}",
                path
            );
            let _ = &e;
            None
        }
    }
}

/// Remap a persisted cursor's `last_seen_ns` to the current monotonic timeline.
///
/// Uses the wall-clock anchor to compute elapsed time since the cursor was saved,
/// then subtracts that from the current monotonic time. This handles kernel
/// restarts where the monotonic clock resets.
fn remap_cursor_to_current_monotonic(cursor: &YankCursorFile, current_monotonic_ns: u64) -> u64 {
    // Parse the wall-clock anchor.
    let saved_wall_clock_ns = parse_iso8601_to_ns(&cursor.last_saved_iso8601).unwrap_or(0);
    let now_wall_clock_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    if saved_wall_clock_ns == 0 || now_wall_clock_ns <= saved_wall_clock_ns {
        // Can't compute delta — fall back to 0 (full replay).
        return 0;
    }

    let wall_clock_elapsed_ns = now_wall_clock_ns - saved_wall_clock_ns;
    current_monotonic_ns.saturating_sub(wall_clock_elapsed_ns)
}

/// Parse a limited ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SS.mmmZ) to nanoseconds since epoch.
/// Returns None if parsing fails or format is unexpected.
fn parse_iso8601_to_ns(s: &str) -> Option<u64> {
    // Expected format: 2026-05-29T12:34:56.789Z
    // Validate separators at fixed positions.
    if s.len() < 24
        || s.as_bytes().get(4) != Some(&b'-')
        || s.as_bytes().get(7) != Some(&b'-')
        || s.as_bytes().get(10) != Some(&b'T')
        || s.as_bytes().get(13) != Some(&b':')
        || s.as_bytes().get(16) != Some(&b':')
        || s.as_bytes().get(19) != Some(&b'.')
        || s.as_bytes().last() != Some(&b'Z')
    {
        return None;
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let min: u32 = s[14..16].parse().ok()?;
    let sec: u32 = s[17..19].parse().ok()?;
    let ms: u32 = s[20..23].parse().ok()?;

    // Convert to days since epoch (simplified — assumes year >= 1970).
    let mut days = 0i64;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    let month_days_arr = month_days(is_leap_year(year));
    for m in 1..month {
        days += month_days_arr[(m - 1) as usize] as i64;
    }
    days += (day - 1) as i64;

    let total_secs = days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64);
    Some((total_secs as u64) * 1_000_000_000 + (ms as u64) * 1_000_000)
}

fn current_iso8601_utc() -> String {
    let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d,
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::warn!("system clock appears before UNIX epoch: {e}; using fallback");
            let _ = &e;
            std::time::Duration::from_secs(1)
        }
    };
    // YYYY-MM-DDTHH:MM:SS.mmmZ — compute via integer division to avoid chrono.
    let total_secs = now.as_secs();
    let ms = now.subsec_millis();
    let (year, month, day, hour, min, sec) = epoch_to_components(total_secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hour, min, sec, ms
    )
}

/// Convert UNIX epoch seconds to UTC calendar components. Self-contained
/// to avoid adding a chrono dep just for the cursor sidecar.
fn epoch_to_components(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let secs_in_day = (secs % 86400) as u32;
    let hour = secs_in_day / 3600;
    let min = (secs_in_day % 3600) / 60;
    let sec = secs_in_day % 60;

    // Days since 1970-01-01.
    let mut year = 1970i64;
    let mut remaining = days;
    loop {
        let leap = is_leap_year(year);
        let yd = if leap { 366 } else { 365 };
        if remaining < yd {
            break;
        }
        remaining -= yd;
        year += 1;
    }
    let mut month = 1u32;
    for &md in &month_days(is_leap_year(year)) {
        if remaining < md as i64 {
            break;
        }
        remaining -= md as i64;
        month += 1;
    }
    let day = remaining as u32 + 1;
    (year as i32, month, day, hour, min, sec)
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn month_days(leap: bool) -> [u32; 12] {
    [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ]
}

#[cfg(test)]
mod cursor_tests {
    use super::*;

    // Both tests mutate the same env var; serialize them to avoid the
    // cargo-test parallel-runner race.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn cursor_round_trips_through_disk() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("maos-yank-cursor-rt-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        std::env::set_var("MAOS_REGISTRY_YANK_CURSOR_PATH", &tmp);

        save_cursor(123_456_789, 7).unwrap();
        let loaded = load_cursor().expect("cursor must load");
        assert_eq!(loaded.last_seen_ns, 123_456_789);
        assert_eq!(loaded.last_seen_yank_count, 7);
        assert!(
            loaded.last_saved_iso8601.contains('T'),
            "iso8601 expected, got: {}",
            loaded.last_saved_iso8601
        );

        let _ = std::fs::remove_file(&tmp);
        std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
    }

    #[test]
    fn missing_cursor_returns_none() {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "maos-yank-cursor-missing-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&tmp);
        std::env::set_var("MAOS_REGISTRY_YANK_CURSOR_PATH", &tmp);
        assert!(load_cursor().is_none());
        std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
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
            YankEntry::new(
                SpiritId::from("s1"),
                "1.0.0".into(),
                100,
                "critical vuln".into(),
            ),
            YankEntry::new(
                SpiritId::from("s2"),
                "2.0.0".into(),
                200,
                "data loss bug".into(),
            ),
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
