#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{
    FrameKindLabel, LogFetchResponse, LogRecallCursor, LogRecallError, LogRecallFilter,
    LogRecallPage,
};
use maos_domain::ports::LogRecallPort;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_spirit_sdk::spirit_test::SpiritTest;
use researcher::{
    __maos_spirit_vtable_Researcher, ClaimPayload, FetchedClaim, Researcher, ResearcherMcpError,
    ResearcherMcpPort, MAX_RECALL_PAGES,
};

const PID: u32 = 17;

// Match Researcher's poison recovery without adding a Spirit dependency.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

#[derive(Default)]
struct Claims(Mutex<Vec<FetchedClaim>>);

impl ResearcherMcpPort for Claims {
    fn survey_literature(&self, _query: &str) -> Result<Vec<FetchedClaim>, ResearcherMcpError> {
        Ok(lock(&self.0).clone())
    }
}

struct CountingRecall {
    adapter: LogRecallAdapter,
    filters: Mutex<Vec<LogRecallFilter>>,
    returned_rows: AtomicUsize,
    fetches: Mutex<BTreeMap<[u8; 16], usize>>,
    fail_fetch_once: Mutex<Option<[u8; 16]>>,
}

impl CountingRecall {
    fn new(log: Arc<TransparencyLogAdapter>) -> Self {
        Self {
            adapter: LogRecallAdapter::new(log),
            filters: Mutex::new(Vec::new()),
            returned_rows: AtomicUsize::new(0),
            fetches: Mutex::new(BTreeMap::new()),
            fail_fetch_once: Mutex::new(None),
        }
    }
}

impl LogRecallPort for CountingRecall {
    fn recall(&self, pid: u32, filter: LogRecallFilter) -> Result<LogRecallPage, LogRecallError> {
        assert_eq!(pid, PID);
        assert_eq!(filter.kind, Some(FrameKindLabel::McpInvocation));
        lock(&self.filters).push(filter.clone());
        let page = self.adapter.recall(pid, filter)?;
        self.returned_rows
            .fetch_add(page.entries.len(), Ordering::Relaxed);
        Ok(page)
    }

    fn fetch(&self, pid: u32, id: [u8; 16]) -> Result<LogFetchResponse, LogRecallError> {
        assert_eq!(pid, PID);
        *lock(&self.fetches).entry(id).or_default() += 1;
        let mut failure = lock(&self.fail_fetch_once);
        if *failure == Some(id) {
            *failure = None;
            return Err(LogRecallError::Storage("one fetch unavailable".into()));
        }
        drop(failure);
        self.adapter.fetch(pid, id)
    }
}

fn seed(log: &TransparencyLogAdapter, claims: &Claims, index: usize) -> [u8; 16] {
    let source_key = format!("paper-{index}");
    log.insert_frame_event(
        FrameKind::McpInvocation,
        PID,
        None,
        "mcp:arxiv/get_paper",
        &serde_json::to_vec(&serde_json::json!({"arxiv_id": source_key})).unwrap(),
        FrameOrigin::SpiritAuto,
    );
    let id = log.last_frame_id();
    lock(&claims.0).push(FetchedClaim {
        source_key,
        claim: ClaimPayload {
            claim_id: format!("claim-{index}"),
            statement: "the effect is likely present".into(),
            topic: "fusion".into(),
            methodology_strength: 0.9,
            confidence: 0.9,
            load_bearing: true,
            polarity: true,
            hedges: vec!["likely".into()],
        },
    });
    id
}

fn idle(researcher: &Researcher) {
    let vtable = __maos_spirit_vtable_Researcher();
    let mut harness = SpiritTest::new(researcher, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    let report = harness.run();
    assert_eq!(report.base.hooks_fired.get("on_idle"), Some(&1));
}

#[test]
fn on_idle_recall_and_fetch_scale_with_new_rows_across_ticks() {
    const TICKS: usize = 8;
    const INITIAL_ROWS: usize = 1025;
    const NEW_ROWS: usize = 3;
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x17_3d_01));
    let claims = Arc::new(Claims::default());
    let port = Arc::new(CountingRecall::new(Arc::clone(&log)));
    let researcher = Researcher::new()
        .with_mcp_port(claims.clone())
        .with_log_recall_port(port.clone(), Arc::new(AtomicU32::new(PID)));
    let first_id = seed(&log, &claims, 0);
    for index in 1..INITIAL_ROWS {
        seed(&log, &claims, index);
    }
    let mut rows = INITIAL_ROWS;
    for tick in 0..TICKS {
        if tick > 0 {
            for _ in 0..NEW_ROWS {
                seed(&log, &claims, rows);
                rows += 1;
            }
        }
        idle(&researcher);
        let output = researcher
            .last_output()
            .expect("successful incremental survey");
        assert_eq!(
            output.findings.len(),
            rows,
            "cached claims still join on tick {tick}"
        );
        assert!(output
            .findings
            .iter()
            .any(|finding| finding.claim_id == "claim-0"));
        assert_eq!(port.returned_rows.load(Ordering::Relaxed), rows);
        let fetches = lock(&port.fetches);
        assert_eq!(fetches.len(), rows);
        assert!(
            fetches.values().all(|count| *count == 1),
            "no historical refetch"
        );
        assert_eq!(fetches.get(&first_id), Some(&1));
    }
    // First tick spans two pages; later ticks need exactly one recall each.
    assert_eq!(lock(&port.filters).len(), TICKS + 1);
    for filter in lock(&port.filters).iter().skip(1) {
        assert!(
            filter.cursor.is_some(),
            "even a terminal page retains a high-water mark"
        );
    }
    // Empty ticks keep the cursor instead of resetting to a historical walk.
    idle(&researcher);
    idle(&researcher);
    assert_eq!(port.returned_rows.load(Ordering::Relaxed), rows);
    assert!(lock(&port.fetches).values().all(|count| *count == 1));
    let filters = lock(&port.filters);
    assert_eq!(filters.len(), TICKS + 3);
    assert_eq!(
        filters[filters.len() - 1].cursor,
        filters[filters.len() - 2].cursor
    );
    drop(filters);
    seed(&log, &claims, rows);
    idle(&researcher);
    assert_eq!(port.returned_rows.load(Ordering::Relaxed), rows + 1);
    assert_eq!(researcher.last_output().unwrap().findings.len(), rows + 1);
}

#[test]
fn on_idle_fetch_failure_keeps_successful_prefix_and_retries_remaining_rows() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x17_3d_02));
    let claims = Arc::new(Claims::default());
    let first = seed(&log, &claims, 0);
    let second = seed(&log, &claims, 1);
    let third = seed(&log, &claims, 2);
    let port = Arc::new(CountingRecall::new(log));
    *lock(&port.fail_fetch_once) = Some(second);
    let researcher = Researcher::new()
        .with_mcp_port(claims)
        .with_log_recall_port(port.clone(), Arc::new(AtomicU32::new(PID)));
    idle(&researcher);
    assert!(researcher.last_output().is_none());
    idle(&researcher);
    assert_eq!(researcher.last_output().unwrap().findings.len(), 3);
    let fetches = lock(&port.fetches);
    assert_eq!(fetches.get(&first), Some(&1));
    assert_eq!(fetches.get(&second), Some(&2));
    assert_eq!(fetches.get(&third), Some(&1));
    assert_eq!(
        lock(&port.filters)[1]
            .cursor
            .as_ref()
            .unwrap()
            .last_frame_id,
        first
    );
}

#[test]
fn on_idle_recalls_later_inserts_across_backward_clock_adjustments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recall.sqlite");
    let log = Arc::new(TransparencyLogAdapter::open(&path, 0x17_3d_03).unwrap());
    let claims = Arc::new(Claims::default());
    let port = Arc::new(CountingRecall::new(log.clone()));
    let researcher = Researcher::new()
        .with_mcp_port(claims.clone())
        .with_log_recall_port(port.clone(), Arc::new(AtomicU32::new(PID)));
    let first = seed(&log, &claims, 0);
    idle(&researcher);
    let timestamp = log.query_frame_by_id(first).unwrap().unwrap().timestamp_ns;
    let second = seed(&log, &claims, 1);
    // Inject the timestamp a new write receives after a backward clock step;
    // never change the host clock or depend on timing between test operations.
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE transparency_log SET timestamp_ns = ?1 WHERE frame_id = ?2",
        rusqlite::params![(timestamp - 1) as i64, &second[..]],
    )
    .unwrap();
    idle(&researcher);
    assert_eq!(researcher.last_output().unwrap().findings.len(), 2);
    idle(&researcher);
    let third = seed(&log, &claims, 2);
    idle(&researcher);
    let output = researcher.last_output().unwrap();
    assert_eq!(output.findings.len(), 3);
    assert!(output
        .findings
        .iter()
        .any(|finding| finding.claim_id == "claim-1"));
    for id in [first, second, third] {
        assert_eq!(lock(&port.fetches).get(&id), Some(&1));
    }
}

struct EndlessRecall(AtomicUsize);

impl LogRecallPort for EndlessRecall {
    fn recall(&self, _pid: u32, _filter: LogRecallFilter) -> Result<LogRecallPage, LogRecallError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(LogRecallPage::new(
            Vec::new(),
            Some(LogRecallCursor::new(1, [1; 16])),
            maos_domain::log_recall::RecallScope::Participant,
        ))
    }

    fn fetch(&self, _pid: u32, _id: [u8; 16]) -> Result<LogFetchResponse, LogRecallError> {
        panic!("nonterminating metadata walk must not fetch payloads")
    }
}

#[test]
fn on_idle_incremental_walk_preserves_recall_all_page_cap() {
    let port = Arc::new(EndlessRecall(AtomicUsize::new(0)));
    let researcher = Researcher::new()
        .with_mcp_port(Arc::new(Claims::default()))
        .with_log_recall_port(port.clone(), Arc::new(AtomicU32::new(PID)));
    idle(&researcher);
    assert_eq!(port.0.load(Ordering::Relaxed), MAX_RECALL_PAGES);
    assert!(researcher.last_output().is_none());
}
