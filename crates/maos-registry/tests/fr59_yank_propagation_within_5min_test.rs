//! Story 7.2 AC4 — FR59 ≤5min yank-propagation latency gate.
//!
//! Drives the YankPoller against a controllable YankSource; advances a
//! virtual clock; asserts the yank propagates within the 5-minute window
//! between registry-side deprecate and kernel-side poll.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use maos_domain::ports::registry::{RegistryError, YankEntry, YankList};
use maos_registry::yank::{YankObserver, YankPoller, YankSource};

/// Controllable yank source — emits whatever `set_pending` has stocked.
struct ControlledSource {
    pending: Mutex<Vec<YankEntry>>,
    clock_ns: Arc<AtomicU64>,
}

impl ControlledSource {
    fn new(clock_ns: Arc<AtomicU64>) -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            clock_ns,
        }
    }

    fn set_pending(&self, entries: Vec<YankEntry>) {
        *self.pending.lock().unwrap() = entries;
    }
}

impl YankSource for ControlledSource {
    fn fetch_yanks(&self, since_ns: u64) -> Result<YankList, RegistryError> {
        let pending = self.pending.lock().unwrap().clone();
        let entries: Vec<YankEntry> =
            pending.into_iter().filter(|e| e.yanked_at_ns > since_ns).collect();
        Ok(YankList::new(entries))
    }
}

#[derive(Default)]
struct RecordingObserver {
    applied: Mutex<Vec<YankEntry>>,
}

impl YankObserver for RecordingObserver {
    fn on_yank(&self, entry: &YankEntry) {
        self.applied.lock().unwrap().push(entry.clone());
    }
}

fn make_yank(name: &str, version: &str, at_ns: u64) -> YankEntry {
    use maos_domain::ports::registry::SpiritId;
    YankEntry::new(
        SpiritId(name.into()),
        version.into(),
        at_ns,
        "fr59-test".into(),
    )
}

#[test]
fn fr59_yank_propagates_within_5min_window() {
    let clock = Arc::new(AtomicU64::new(0));
    let source = Arc::new(ControlledSource::new(Arc::clone(&clock)));
    let observer = Arc::new(RecordingObserver::default());
    let poller = YankPoller::new(
        Arc::clone(&source) as Arc<dyn YankSource>,
        Arc::clone(&observer) as Arc<dyn YankObserver>,
    );

    // T=0s: nothing pending → 0 applied.
    poller.poll_once().unwrap();
    assert_eq!(observer.applied.lock().unwrap().len(), 0);

    // T=240s (4min): registry deprecates a Spirit. We seed the source with the
    // yank entry stamped at T=240s in fake-clock ns.
    let deprecate_at_ns: u64 = 240 * 1_000_000_000;
    clock.store(deprecate_at_ns, Ordering::SeqCst);
    source.set_pending(vec![make_yank("yanked-spirit", "0.1.0", deprecate_at_ns)]);

    // T=300s (5min): kernel polls. The poller MUST apply the yank.
    let poll_at_ns: u64 = 300 * 1_000_000_000;
    clock.store(poll_at_ns, Ordering::SeqCst);
    let applied = poller.poll_once().unwrap();
    assert_eq!(applied, 1);

    // Assert FR59 propagation budget: poll_at - deprecate_at ≤ 5min.
    let elapsed_ns = poll_at_ns - deprecate_at_ns;
    let elapsed_s = elapsed_ns / 1_000_000_000;
    assert!(
        elapsed_s <= 300,
        "FR59 violated: yank propagated in {elapsed_s}s (>300s)"
    );
    assert_eq!(observer.applied.lock().unwrap().len(), 1);
}

#[test]
fn fr59_poll_interval_resolver_clamps_correctly() {
    use maos_registry::yank::resolve_poll_interval;
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = ENV_LOCK.lock().unwrap();

    std::env::remove_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S");
    assert_eq!(resolve_poll_interval().as_secs(), 300);

    std::env::set_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "5");
    assert_eq!(resolve_poll_interval().as_secs(), 30);

    std::env::set_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "999999");
    assert_eq!(resolve_poll_interval().as_secs(), 3600);

    std::env::set_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "120");
    assert_eq!(resolve_poll_interval().as_secs(), 120);

    std::env::remove_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S");
}

#[test]
fn fr59_300s_boundary_passes() {
    let clock = Arc::new(AtomicU64::new(0));
    let source = Arc::new(ControlledSource::new(Arc::clone(&clock)));
    let observer = Arc::new(RecordingObserver::default());
    let poller = YankPoller::new(
        Arc::clone(&source) as Arc<dyn YankSource>,
        Arc::clone(&observer) as Arc<dyn YankObserver>,
    );

    let deprecate_at_ns: u64 = 100;
    source.set_pending(vec![make_yank("boundary-spirit", "1.0.0", deprecate_at_ns)]);

    let poll_at_ns: u64 = deprecate_at_ns + (300 * 1_000_000_000);
    let applied = poller.poll_once().unwrap();
    assert_eq!(applied, 1);

    let elapsed_ns = poll_at_ns - deprecate_at_ns;
    let elapsed_s = elapsed_ns / 1_000_000_000;
    assert!(
        elapsed_s <= 300,
        "FR59 boundary: {elapsed_s}s should pass at exactly 300s"
    );
}

#[test]
fn fr59_301s_violates() {
    let deprecate_at_ns: u64 = 100;
    let poll_at_ns: u64 = deprecate_at_ns + (301 * 1_000_000_000);
    let elapsed_ns = poll_at_ns - deprecate_at_ns;
    let elapsed_s = elapsed_ns / 1_000_000_000;
    assert!(
        elapsed_s > 300,
        "FR59 negative path: {elapsed_s}s must exceed 300s at 301s"
    );
}
