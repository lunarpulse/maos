#![cfg(feature = "network")]
#![forbid(unsafe_code)]
//! Story 17-4 AC4: authored activity corpus, production delivery, file-backed oracle.

use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
use maos_bin::hook_pump::{HookPump, PumpStats};
use maos_domain::invariants::i7::ScalarTapEvent;
use maos_kernel_core::capability::{
    cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::control_block::ScbLifecycleState;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use observer::{ActivitySink, ObservedActivity};
use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

const CORPUS: &str = include_str!("../../../tests/corpora/observer-activity-v1.jsonl");
const MANIFEST: &str = include_str!("../../../spirits/observer/manifest.toml");
const NONCE: u64 = 0x17_04;
const HARD_DEADLINE: Duration = Duration::from_secs(30);
const UNDECLARED_TAG: &str = "activity.undeclared.17_4";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Class {
    Steady,
    InterleaveTags,
    InterleaveEmitters,
    Burst,
    RampToHaltBand,
    Mixed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Schedule {
    Sequential,
    RoundRobin,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Emitter {
    spirit_id: String,
    pid: u32,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Segment {
    spirit_id: String,
    tag: String,
    start: f64,
    step: f64,
    count: usize,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    id: String,
    class: Class,
    description: String,
    emitters: Vec<Emitter>,
    segments: Vec<Segment>,
    schedule: Schedule,
    pause_every: usize,
    pause_us: u64,
}

#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error("line {line}: scenario JSON: {source}")]
    Json {
        line: usize,
        source: serde_json::Error,
    },
    #[error("line {line}: malformed scenario id {id:?}")]
    Id { line: usize, id: String },
    #[error("line {line}: duplicate scenario id or ordinal {id}")]
    Duplicate { line: usize, id: String },
    #[error("scenario {id}: invalid emitter declarations")]
    Emitters { id: String },
    #[error("scenario {id}: segment names undeclared emitter {spirit_id}")]
    Undeclared { id: String, spirit_id: String },
    #[error("scenario {id}: zero segment count")]
    ZeroCount { id: String },
    #[error("scenario {id}: non-finite segment value")]
    NonFinite { id: String },
    #[error("scenario {id}: empty description, segments or tag")]
    Empty { id: String },
}

fn load_scenarios(text: &str) -> Result<Vec<Scenario>, LoadError> {
    let mut ids = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            let line_no = index + 1;
            let scenario: Scenario =
                serde_json::from_str(line).map_err(|source| LoadError::Json {
                    line: line_no,
                    source,
                })?;
            let bytes = scenario.id.as_bytes();
            let valid = bytes.len() >= 7
                && bytes.starts_with(b"oa-")
                && bytes[3..5].iter().all(u8::is_ascii_digit)
                && bytes[5] == b'-'
                && bytes[6..]
                    .iter()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
                && bytes[6].is_ascii_alphanumeric()
                && bytes.last().unwrap().is_ascii_alphanumeric();
            if !valid {
                return Err(LoadError::Id {
                    line: line_no,
                    id: scenario.id,
                });
            }
            let ordinal: u8 = scenario.id[3..5].parse().unwrap();
            if !(1..=20).contains(&ordinal) {
                return Err(LoadError::Id {
                    line: line_no,
                    id: scenario.id,
                });
            }
            if !ids.insert(scenario.id.clone()) || !ordinals.insert(ordinal) {
                return Err(LoadError::Duplicate {
                    line: line_no,
                    id: scenario.id,
                });
            }
            let emitters: BTreeSet<_> = scenario
                .emitters
                .iter()
                .map(|e| e.spirit_id.as_str())
                .collect();
            let pids: BTreeSet<_> = scenario.emitters.iter().map(|e| e.pid).collect();
            if emitters.is_empty()
                || emitters.len() != scenario.emitters.len()
                || pids.len() != scenario.emitters.len()
                || scenario
                    .emitters
                    .iter()
                    .any(|e| e.spirit_id.is_empty() || e.pid == 0)
            {
                return Err(LoadError::Emitters { id: scenario.id });
            }
            if scenario.description.is_empty()
                || scenario.segments.is_empty()
                || scenario.segments.iter().any(|s| s.tag.is_empty())
            {
                return Err(LoadError::Empty { id: scenario.id });
            }
            for segment in &scenario.segments {
                if !emitters.contains(segment.spirit_id.as_str()) {
                    return Err(LoadError::Undeclared {
                        id: scenario.id.clone(),
                        spirit_id: segment.spirit_id.clone(),
                    });
                }
                if segment.count == 0 {
                    return Err(LoadError::ZeroCount { id: scenario.id });
                }
                if !segment.start.is_finite()
                    || !segment.step.is_finite()
                    || (0..segment.count)
                        .any(|i| !(segment.start + segment.step * i as f64).is_finite())
                {
                    return Err(LoadError::NonFinite { id: scenario.id });
                }
            }
            Ok(scenario)
        })
        .collect()
}

/// Visit program order, never seq order. Exhausted round-robin segments are skipped.
fn schedule(scenario: &Scenario) -> Vec<(&Segment, usize)> {
    match scenario.schedule {
        Schedule::Sequential => scenario
            .segments
            .iter()
            .flat_map(|s| (0..s.count).map(move |i| (s, i)))
            .collect(),
        Schedule::RoundRobin => (0..scenario.segments.iter().map(|s| s.count).max().unwrap())
            .flat_map(|i| {
                scenario
                    .segments
                    .iter()
                    .filter(move |s| i < s.count)
                    .map(move |s| (s, i))
            })
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    seq: u64,
    spirit_id: String,
    tag: String,
    value_bits: u64,
}
impl From<&ScalarTapEvent> for Record {
    fn from(event: &ScalarTapEvent) -> Self {
        Self {
            seq: event.seq,
            spirit_id: event.spirit_id.clone(),
            tag: event.tag.clone(),
            value_bits: event.value.to_bits(),
        }
    }
}
fn scalar_record(activity: ObservedActivity) -> Option<Record> {
    if let ObservedActivity::Scalar {
        seq,
        spirit_id,
        tag,
        value,
        ..
    } = activity
    {
        Some(Record {
            seq,
            spirit_id,
            tag,
            value_bits: value.to_bits(),
        })
    } else {
        None
    }
}
fn append(writer: &mut impl Write, record: &Record) {
    serde_json::to_writer(&mut *writer, record).expect("harness: write JSON record");
    writer
        .write_all(b"\n")
        .expect("harness: write record newline");
}
fn sync(writer: &mut BufWriter<File>) {
    writer.flush().expect("harness: flush journal");
    writer.get_ref().sync_all().expect("harness: sync journal");
}
fn read_records(path: &Path) -> Vec<Record> {
    BufReader::new(File::open(path).expect("harness: open oracle input"))
        .lines()
        .map(|line| {
            serde_json::from_str(&line.expect("harness: read oracle input"))
                .expect("harness: parse oracle input")
        })
        .collect()
}
#[derive(Debug)]
struct FileSink(Mutex<BufWriter<File>>);
impl ActivitySink for FileSink {
    fn emit(&self, activity: ObservedActivity) {
        if let Some(record) = scalar_record(activity) {
            append(&mut *self.0.lock(), &record);
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
struct CounterDelta {
    delivered: u64,
    lagged_skipped: u64,
    ineligible: u64,
    failed: u64,
    skipped_manifest: u64,
}
impl CounterDelta {
    fn between(before: PumpStats, after: PumpStats) -> Self {
        Self {
            delivered: after.delivered - before.delivered,
            lagged_skipped: after.lagged_skipped - before.lagged_skipped,
            ineligible: after.ineligible - before.ineligible,
            failed: after.failed - before.failed,
            skipped_manifest: after.skipped_manifest - before.skipped_manifest,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, Serialize)]
struct Metrics {
    emitted: u64,
    received: u64,
    missed: u64,
    causal_pairs: u64,
    correct_pairs: u64,
    cross_emitter_pairs: u64,
}
impl Metrics {
    fn missed_rate(self) -> f64 {
        assert!(self.emitted > 0, "oracle: emitted-denominator");
        self.missed as f64 / self.emitted as f64
    }
    fn causal_correctness(self) -> f64 {
        assert!(self.causal_pairs > 0, "oracle: causal-denominator");
        self.correct_pairs as f64 / self.causal_pairs as f64
    }
    fn add(&mut self, other: Self) {
        self.emitted += other.emitted;
        self.received += other.received;
        self.missed += other.missed;
        self.causal_pairs += other.causal_pairs;
        self.correct_pairs += other.correct_pairs;
        self.cross_emitter_pairs += other.cross_emitter_pairs;
    }
}

/// Pure identity and causal analysis. seq is an identity label, never a sort key.
fn analyze(emitted: &[Record], received: &[Record]) -> Metrics {
    assert!(!emitted.is_empty(), "oracle: emitted-denominator");
    let mut expected = BTreeMap::new();
    for record in emitted {
        assert!(
            expected.insert(record.seq, record).is_none(),
            "oracle: emitted-duplicate seq={}",
            record.seq
        );
    }
    let mut positions = BTreeMap::new();
    for (position, record) in received.iter().enumerate() {
        assert!(
            positions.insert(record.seq, position).is_none(),
            "oracle: duplicate seq={}",
            record.seq
        );
        let source = expected.get(&record.seq);
        assert!(source.is_some(), "oracle: unknown seq={}", record.seq);
        assert_eq!(
            *source.unwrap(),
            record,
            "oracle: field-mismatch seq={}",
            record.seq
        );
    }
    let mut metrics = Metrics {
        emitted: emitted.len() as u64,
        received: received.len() as u64,
        missed: (emitted.len() - received.len()) as u64,
        ..Metrics::default()
    };
    let mut previous = BTreeMap::<&str, u64>::new();
    for record in emitted {
        if let Some(seq) = previous.insert(&record.spirit_id, record.seq) {
            if let (Some(left), Some(right)) = (positions.get(&seq), positions.get(&record.seq)) {
                metrics.causal_pairs += 1;
                metrics.correct_pairs += u64::from(left < right);
            }
        }
    }
    metrics.cross_emitter_pairs = emitted
        .windows(2)
        .filter(|pair| pair[0].spirit_id != pair[1].spirit_id)
        .count() as u64;
    assert!(metrics.causal_pairs > 0, "oracle: causal-denominator");
    metrics
}
fn assert_quality(metrics: Metrics) {
    assert!(
        metrics.missed_rate() <= 0.02,
        "oracle: missed-rate {} > 0.02",
        metrics.missed_rate()
    );
    assert!(
        metrics.causal_correctness() >= 0.99,
        "oracle: causal-correctness {} < 0.99",
        metrics.causal_correctness()
    );
}
fn assert_no_silent_loss(metrics: Metrics, counters: CounterDelta) {
    assert_eq!(
        metrics.missed, counters.lagged_skipped,
        "oracle: silent-loss missed != lagged_skipped"
    );
    assert_eq!(counters.ineligible, 0, "oracle: silent-loss ineligible");
    assert_eq!(counters.failed, 0, "oracle: silent-loss failed");
    assert_eq!(
        counters.skipped_manifest, 0,
        "oracle: silent-loss skipped_manifest"
    );
}

struct World {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    security: Arc<SecurityManagerAdapter>,
    memory: Arc<MemoryManagerAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    lookup: Arc<RwLock<BTreeMap<String, u32>>>,
    pump: HookPump,
    audit_writer: Option<tokio::task::JoinHandle<()>>,
    observer_pid: Option<u32>,
}
impl World {
    fn new(dir: &Path, telemetry: Arc<TelemetryStreamAdapter>) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(
            TransparencyLogAdapter::open(&dir.join("transparency.sqlite"), NONCE).unwrap(),
        );
        let policy = Arc::new(PolicyTable::new());
        let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
        let (audit_tx, audit_rx) = cap_audit::channel();
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            NONCE,
            policy,
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(WorkingMemoryStore::new()),
            Arc::clone(&telemetry),
        ));
        let database = dir.join("memory.sqlite");
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.join("private"),
                4,
            )),
            Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&database).unwrap()),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&database)
                    .unwrap(),
            ),
            Arc::clone(&tl),
        ));
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let iac = Arc::new(IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
            Arc::clone(&tl),
        ));
        let scheduler = Arc::new(SpiritSchedulerAdapter::new(
            Arc::clone(&tl),
            Arc::clone(&capability),
            Arc::clone(&memory),
            iac,
            Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
            metrics,
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        let journal = Arc::new(
            maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson")).unwrap(),
        );
        let pump = HookPump::new(&scheduler, &telemetry, Arc::clone(&tl));
        Self {
            tl: Arc::clone(&tl),
            capability,
            security,
            memory,
            scheduler,
            journal,
            pump,
            lookup: Arc::new(RwLock::new(BTreeMap::new())),
            audit_writer: Some(cap_audit::CapAuditWriter::spawn(audit_rx, Arc::clone(&tl))),
            observer_pid: None,
        }
    }
    async fn admit(&mut self, sink: Arc<dyn ActivitySink>) {
        let gated = gate_manifest(MANIFEST, &|name| name == "observer")
            .expect("harness: gate real Observer manifest");
        let admitted = load_admit_start(
            &AdmissionDeps {
                scheduler: &self.scheduler,
                security: &self.security,
                transparency_log: &self.tl,
                journal: &self.journal,
                memory: &self.memory,
                pid_by_spirit_id: &self.lookup,
                hooks: Some(&self.pump),
            },
            &gated,
            maos_kernel_core::scheduler::make_spirit_obj(
                observer::Observer::default()
                    .with_id("observer")
                    .with_activity_sink(sink),
            ),
            NONCE,
            StartPolicy::StartNow,
            &mut |_| {},
        )
        .await
        .expect("harness: admit and start Observer");
        self.observer_pid = Some(admitted.pid);
        assert_eq!(
            self.scheduler
                .scbs()
                .read()
                .unwrap()
                .get(&admitted.pid)
                .unwrap()
                .current_state(),
            ScbLifecycleState::Running,
            "harness: Observer must be Running before emission"
        );
    }
    async fn finish(mut self) {
        self.pump.shutdown().await;
        self.scheduler
            .unload(self.observer_pid.unwrap())
            .await
            .expect("harness: unload Observer");
        let audit = self.audit_writer.take().unwrap();
        drop(self);
        tokio::time::timeout(HARD_DEADLINE, audit)
            .await
            .expect("harness timeout: audit drain")
            .expect("harness: join audit writer");
    }
}
async fn flush(pump: &HookPump) {
    tokio::time::timeout(HARD_DEADLINE, pump.flush())
        .await
        .expect("harness timeout: telemetry pump flush hard deadline expired")
        .expect("harness: telemetry pump stopped before flush");
}
fn output_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("observer-activity-17-4")
}

#[derive(Serialize)]
struct ScenarioRow {
    id: String,
    metrics: Metrics,
    counters: CounterDelta,
    missed_rate: f64,
    causal_correctness: f64,
}
#[derive(Serialize)]
struct Aggregate {
    metrics: Metrics,
    missed_rate: f64,
    causal_correctness: f64,
}
#[derive(Serialize)]
struct Summary {
    commit_sha: String,
    corpus_sha256: String,
    scenarios: Vec<ScenarioRow>,
    aggregate: Aggregate,
}
fn commit_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|sha| sha.trim().to_owned())
        .filter(|sha| !sha.is_empty())
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .expect("harness: commit SHA unavailable from git rev-parse HEAD and GITHUB_SHA")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn observer_activity_corpus_production_path() {
    let scenarios = load_scenarios(CORPUS).expect("harness: load real corpus");
    let output = output_dir();
    fs::create_dir_all(&output).unwrap();
    let summary_path = output.join("summary.json");
    if summary_path.exists() {
        fs::remove_file(&summary_path).unwrap();
    }
    let mut rows = Vec::new();
    let mut aggregate = Metrics::default();
    println!("scenario | emitted | received | missed | missed-rate | causal correct/total | correctness | cross-emitter (excluded)");
    for scenario in scenarios {
        let emitted_path = output.join(format!("{}.emitted.jsonl", scenario.id));
        let received_path = output.join(format!("{}.received.jsonl", scenario.id));
        // File::create truncates each scenario's previous run before its first write.
        let mut emitted = BufWriter::new(File::create(&emitted_path).unwrap());
        let sink = Arc::new(FileSink(Mutex::new(BufWriter::new(
            File::create(&received_path).unwrap(),
        ))));
        let root = tempfile::tempdir().unwrap();
        let mut world = World::new(root.path(), Arc::new(TelemetryStreamAdapter::default()));
        world.admit(sink.clone()).await;
        let before = world.pump.stats();
        let emitters: BTreeMap<_, _> = scenario
            .emitters
            .iter()
            .map(|e| (e.spirit_id.as_str(), e.pid))
            .collect();
        for (position, (segment, i)) in schedule(&scenario).into_iter().enumerate() {
            let value = segment.start + segment.step * i as f64;
            let event = world
                .capability
                .set_scalar(
                    emitters[segment.spirit_id.as_str()],
                    &segment.spirit_id,
                    &segment.tag,
                    value,
                    "corpus",
                )
                .expect("harness: production set_scalar");
            append(&mut emitted, &Record::from(&event));
            if scenario.pause_every > 0
                && (position + 1) % scenario.pause_every == 0
                && scenario.pause_us > 0
            {
                tokio::time::sleep(Duration::from_micros(scenario.pause_us)).await;
            }
        }
        flush(&world.pump).await;
        let counters = CounterDelta::between(before, world.pump.stats());
        sync(&mut emitted);
        sync(&mut sink.0.lock());
        let metrics = analyze(&read_records(&emitted_path), &read_records(&received_path));
        assert_no_silent_loss(metrics, counters);
        println!(
            "{} | {} | {} | {} | {:.8} | {}/{} | {:.8} | {}",
            scenario.id,
            metrics.emitted,
            metrics.received,
            metrics.missed,
            metrics.missed_rate(),
            metrics.correct_pairs,
            metrics.causal_pairs,
            metrics.causal_correctness(),
            metrics.cross_emitter_pairs
        );
        aggregate.add(metrics);
        rows.push(ScenarioRow {
            id: scenario.id,
            metrics,
            counters,
            missed_rate: metrics.missed_rate(),
            causal_correctness: metrics.causal_correctness(),
        });
        world.finish().await;
    }
    println!(
        "aggregate | {} | {} | {} | {:.8} | {}/{} | {:.8} | {}",
        aggregate.emitted,
        aggregate.received,
        aggregate.missed,
        aggregate.missed_rate(),
        aggregate.correct_pairs,
        aggregate.causal_pairs,
        aggregate.causal_correctness(),
        aggregate.cross_emitter_pairs
    );
    let summary = Summary {
        commit_sha: commit_sha(),
        corpus_sha256: hex::encode(Sha256::digest(CORPUS.as_bytes())),
        scenarios: rows,
        aggregate: Aggregate {
            metrics: aggregate,
            missed_rate: aggregate.missed_rate(),
            causal_correctness: aggregate.causal_correctness(),
        },
    };
    let mut summary_file = BufWriter::new(File::create(&summary_path).unwrap());
    serde_json::to_writer_pretty(&mut summary_file, &summary).unwrap();
    sync(&mut summary_file);
    println!("summary.json: {}", summary_path.display());
    assert_quality(aggregate);
}

#[derive(Debug, Default)]
struct Release {
    open: Mutex<bool>,
    changed: Condvar,
}
impl Release {
    fn release(&self) {
        *self.open.lock() = true;
        self.changed.notify_all();
    }
}
struct ReleaseOnDrop(Arc<Release>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}
#[derive(Debug)]
struct BlockingSink {
    first: AtomicBool,
    entered: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: Arc<Release>,
    records: Mutex<Vec<Record>>,
}
impl ActivitySink for BlockingSink {
    fn emit(&self, activity: ObservedActivity) {
        if let Some(record) = scalar_record(activity) {
            self.records.lock().push(record);
            if !self.first.swap(true, Ordering::SeqCst) {
                self.entered.lock().take().unwrap().send(()).unwrap();
                let mut open = self.release.open.lock();
                while !*open {
                    self.release.changed.wait(&mut open);
                }
            }
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn overflow_counts_exact_loss_and_preserves_first_plus_suffix() {
    const CAPACITY: usize = 4; // Power of two: tokio does not round this upward.
    const EXTRA: usize = 11;
    let root = tempfile::tempdir().unwrap();
    let mut world = World::new(root.path(), Arc::new(TelemetryStreamAdapter::new(CAPACITY)));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let release = Arc::new(Release::default());
    let _release_on_failure = ReleaseOnDrop(Arc::clone(&release));
    let sink = Arc::new(BlockingSink {
        first: AtomicBool::new(false),
        entered: Mutex::new(Some(entered_tx)),
        release: Arc::clone(&release),
        records: Mutex::new(Vec::new()),
    });
    world.admit(sink.clone()).await;
    let before = world.pump.stats();
    let mut emitted = vec![Record::from(
        &world
            .capability
            .set_scalar(100, "butler", "belief_variance", 0.0, "corpus")
            .unwrap(),
    )];
    tokio::time::timeout(HARD_DEADLINE, entered_rx)
        .await
        .expect("harness timeout: first sink record not reached")
        .unwrap();
    for i in 1..=EXTRA {
        emitted.push(Record::from(
            &world
                .capability
                .set_scalar(100, "butler", "belief_variance", i as f64, "corpus")
                .unwrap(),
        ));
    }
    release.release();
    flush(&world.pump).await;
    let counters = CounterDelta::between(before, world.pump.stats());
    assert_eq!(
        counters.lagged_skipped,
        (EXTRA - CAPACITY) as u64,
        "overflow: exact lag count"
    );
    let expected: Vec<_> = std::iter::once(emitted[0].clone())
        .chain(emitted[emitted.len() - CAPACITY..].iter().cloned())
        .collect();
    assert_eq!(
        *sink.records.lock(),
        expected,
        "overflow: received must be first record plus exact retained suffix"
    );
    assert_no_silent_loss(analyze(&emitted, &expected), counters);
    assert_eq!(
        counters.delivered,
        (CAPACITY + 1) as u64,
        "overflow: exact delivered count"
    );
    world.finish().await;
}

#[test]
fn real_corpus_meets_all_scenario_floors() {
    let scenarios = load_scenarios(CORPUS).unwrap();
    assert_eq!(CORPUS.lines().count(), 20);
    assert_eq!(scenarios.len(), 20);
    let classes = [
        Class::Steady,
        Class::InterleaveTags,
        Class::InterleaveEmitters,
        Class::Burst,
        Class::RampToHaltBand,
        Class::Mixed,
    ];
    for class in classes {
        assert!(
            scenarios.iter().filter(|s| s.class == class).count() >= 2,
            "floor: {class:?}"
        );
    }
    assert!(
        scenarios
            .iter()
            .flat_map(|s| &s.segments)
            .map(|s| s.count)
            .sum::<usize>()
            >= 4000
    );
    let no_pause = |s: &Scenario| s.pause_every == 0 || s.pause_us == 0;
    assert!(scenarios.iter().any(|s| s.class == Class::Burst
        && no_pause(s)
        && s.segments.iter().map(|s| s.count).sum::<usize>() >= 1500));
    assert!(scenarios.iter().any(|s| s.class == Class::InterleaveTags
        && s.emitters.len() == 1
        && no_pause(s)
        && s.schedule == Schedule::RoundRobin
        && s.segments.len() == 2
        && s.segments[0].tag != s.segments[1].tag
        && s.segments[0].count == s.segments[1].count));
    assert!(scenarios.iter().any(|s| s.emitters.len() == 4
        && s.emitters.iter().all(|e| s
            .segments
            .iter()
            .filter(|segment| segment.spirit_id == e.spirit_id)
            .map(|segment| &segment.tag)
            .collect::<BTreeSet<_>>()
            .len()
            >= 3)));
    assert!(scenarios.iter().any(|s| s
        .segments
        .iter()
        .map(|s| &s.tag)
        .collect::<BTreeSet<_>>()
        .len()
        >= 8));
    assert!(scenarios
        .iter()
        .flat_map(|s| &s.segments)
        .any(|s| s.tag == "belief_variance"));
    assert!(scenarios
        .iter()
        .flat_map(|s| &s.segments)
        .any(|s| s.tag == UNDECLARED_TAG));
    let spirits = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spirits");
    let mut policies_checked = 0;
    for directory in fs::read_dir(spirits).unwrap() {
        let path = directory.unwrap().path().join("manifest.toml");
        if !path.is_file() {
            continue;
        }
        let manifest: toml::Value = toml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        if let Some(policy) = manifest.get("epistemic_policy") {
            policies_checked += 1;
            for rule in policy
                .get("rules")
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
            {
                assert_ne!(
                    rule.get("tag").and_then(toml::Value::as_str),
                    Some(UNDECLARED_TAG),
                    "floor: undeclared tag acquired a policy declaration"
                );
            }
        }
    }
    assert!(policies_checked > 0, "floor: no real policies checked");
}

fn synthetic(seq: u64, spirit_id: &str, tag: &str) -> Record {
    Record {
        seq,
        spirit_id: spirit_id.into(),
        tag: tag.into(),
        value_bits: 0.25_f64.to_bits(),
    }
}
fn sample() -> Vec<Record> {
    vec![
        synthetic(9, "a", "x"),
        synthetic(2, "b", "x"),
        synthetic(7, "a", "y"),
        synthetic(1, "b", "y"),
    ]
}
fn panic_named(name: &str, action: impl FnOnce() + std::panic::UnwindSafe) {
    let panic = std::panic::catch_unwind(action).expect_err("synthetic negative case must fail");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("");
    assert!(message.contains(name), "expected {name}, got {message}");
}
#[test]
fn oracle_identity_and_cross_tag_causality_positive() {
    let emitted = sample();
    // Reordering independent emitters is allowed; seq's numeric order is irrelevant.
    let received = vec![
        emitted[1].clone(),
        emitted[0].clone(),
        emitted[3].clone(),
        emitted[2].clone(),
    ];
    let metrics = analyze(&emitted, &received);
    assert_eq!(metrics.missed, 0);
    assert_eq!(metrics.causal_pairs, 2);
    assert_eq!(metrics.correct_pairs, 2);
    assert_eq!(metrics.cross_emitter_pairs, 3);
    assert_quality(metrics);
    assert_no_silent_loss(metrics, CounterDelta::default());
}
#[test]
fn oracle_duplicate_including_final_is_rejected() {
    for index in [0, 3] {
        let emitted = sample();
        let mut received = emitted.clone();
        received.push(emitted[index].clone());
        panic_named("oracle: duplicate", || {
            analyze(&emitted, &received);
        });
    }
}
#[test]
fn oracle_unknown_is_rejected() {
    let emitted = sample();
    let mut received = emitted.clone();
    received.push(synthetic(100, "a", "x"));
    panic_named("oracle: unknown", || {
        analyze(&emitted, &received);
    });
}
#[test]
fn oracle_each_field_mismatch_is_rejected() {
    for field in 0..3 {
        let emitted = sample();
        let mut received = emitted.clone();
        match field {
            0 => received[0].spirit_id = "other".into(),
            1 => received[0].tag = "other".into(),
            _ => received[0].value_bits ^= 1,
        }
        panic_named("oracle: field-mismatch", || {
            analyze(&emitted, &received);
        });
    }
}
#[test]
fn oracle_missed_identity_and_original_adjacency() {
    let emitted = vec![
        synthetic(8, "a", "x"),
        synthetic(3, "a", "y"),
        synthetic(6, "a", "z"),
        synthetic(2, "a", "x"),
    ];
    let metrics = analyze(
        &emitted,
        &[emitted[0].clone(), emitted[2].clone(), emitted[3].clone()],
    );
    assert_eq!(metrics.missed, 1);
    assert_eq!(metrics.causal_pairs, 1);
    assert_eq!(metrics.correct_pairs, 1);
    assert_no_silent_loss(
        metrics,
        CounterDelta {
            lagged_skipped: 1,
            ..CounterDelta::default()
        },
    );
}
#[test]
fn oracle_missed_rate_boundary_and_negative() {
    let base = Metrics {
        emitted: 100,
        received: 98,
        missed: 2,
        causal_pairs: 100,
        correct_pairs: 100,
        cross_emitter_pairs: 0,
    };
    assert_quality(base);
    panic_named("oracle: missed-rate", || {
        assert_quality(Metrics { missed: 3, ..base })
    });
}
#[test]
fn oracle_causal_boundary_and_reversal_negative() {
    let base = Metrics {
        emitted: 101,
        received: 101,
        causal_pairs: 100,
        correct_pairs: 99,
        ..Metrics::default()
    };
    assert_quality(base);
    panic_named("oracle: causal-correctness", || {
        assert_quality(Metrics {
            correct_pairs: 98,
            ..base
        })
    });
    let emitted = sample();
    let mut received = emitted.clone();
    received.swap(0, 2);
    let metrics = analyze(&emitted, &received);
    assert_eq!(metrics.correct_pairs, 1);
    assert_eq!(metrics.causal_pairs, 2);
    panic_named("oracle: causal-correctness", || assert_quality(metrics));
}
#[test]
fn oracle_silent_loss_and_each_counter_are_rejected() {
    let metrics = analyze(&sample(), &sample());
    for counters in [
        CounterDelta {
            lagged_skipped: 1,
            ..CounterDelta::default()
        },
        CounterDelta {
            ineligible: 1,
            ..CounterDelta::default()
        },
        CounterDelta {
            failed: 1,
            ..CounterDelta::default()
        },
        CounterDelta {
            skipped_manifest: 1,
            ..CounterDelta::default()
        },
    ] {
        panic_named("oracle: silent-loss", || {
            assert_no_silent_loss(metrics, counters)
        });
    }
    panic_named("oracle: silent-loss", || {
        assert_no_silent_loss(
            Metrics {
                missed: 1,
                ..metrics
            },
            CounterDelta::default(),
        )
    });
}
#[test]
fn oracle_zero_denominators_are_rejected() {
    panic_named("oracle: emitted-denominator", || {
        analyze(&[], &[]);
    });
    panic_named("oracle: causal-denominator", || {
        analyze(&[synthetic(1, "a", "x")], &[synthetic(1, "a", "x")]);
    });
    panic_named("oracle: causal-denominator", || {
        analyze(&sample(), &[]);
    });
    panic_named("oracle: emitted-denominator", || {
        Metrics::default().missed_rate();
    });
    panic_named("oracle: causal-denominator", || {
        Metrics::default().causal_correctness();
    });
}
#[test]
fn oracle_aggregate_is_weighted_by_events_and_pairs() {
    let mut aggregate = Metrics {
        emitted: 10,
        received: 9,
        missed: 1,
        causal_pairs: 5,
        correct_pairs: 4,
        cross_emitter_pairs: 2,
    };
    aggregate.add(Metrics {
        emitted: 990,
        received: 990,
        missed: 0,
        causal_pairs: 995,
        correct_pairs: 995,
        cross_emitter_pairs: 3,
    });
    assert_eq!(aggregate.missed_rate(), 0.001);
    assert_eq!(aggregate.causal_correctness(), 0.999);
    assert_eq!(aggregate.cross_emitter_pairs, 5);
    assert_quality(aggregate);
}
#[test]
fn loader_rejects_unknown_fields_at_every_level() {
    for level in ["scenario", "emitter", "segment"] {
        let mut value: serde_json::Value =
            serde_json::from_str(CORPUS.lines().next().unwrap()).unwrap();
        let object = match level {
            "scenario" => &mut value,
            "emitter" => &mut value["emitters"][0],
            _ => &mut value["segments"][0],
        };
        object
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::json!(true));
        assert!(matches!(
            load_scenarios(&value.to_string()),
            Err(LoadError::Json { .. })
        ));
    }
}
#[test]
fn loader_validates_ids_emitters_counts_finiteness_and_enums() {
    let source = CORPUS.lines().next().unwrap();
    for id in [
        "oa-00-bad",
        "oa-21-bad",
        "oa-1-bad",
        "oa-01-",
        "oa-01-UPPER",
        "oa-01-bad_underscore",
        "oa-01--bad",
    ] {
        let mut value: serde_json::Value = serde_json::from_str(source).unwrap();
        value["id"] = id.into();
        assert!(
            matches!(
                load_scenarios(&value.to_string()),
                Err(LoadError::Id { .. })
            ),
            "{id}"
        );
    }
    assert!(matches!(
        load_scenarios(&format!("{source}\n{source}")),
        Err(LoadError::Duplicate { .. })
    ));
    let mut other: serde_json::Value = serde_json::from_str(source).unwrap();
    other["id"] = "oa-01-different-slug".into();
    assert!(matches!(
        load_scenarios(&format!("{source}\n{other}")),
        Err(LoadError::Duplicate { .. })
    ));
    let mut value: serde_json::Value = serde_json::from_str(source).unwrap();
    value["segments"][0]["spirit_id"] = "missing".into();
    assert!(matches!(
        load_scenarios(&value.to_string()),
        Err(LoadError::Undeclared { .. })
    ));
    let mut value: serde_json::Value = serde_json::from_str(source).unwrap();
    value["segments"][0]["count"] = 0.into();
    assert!(matches!(
        load_scenarios(&value.to_string()),
        Err(LoadError::ZeroCount { .. })
    ));
    let mut value: serde_json::Value = serde_json::from_str(source).unwrap();
    value["segments"][0]["start"] = serde_json::json!(1e308);
    value["segments"][0]["step"] = serde_json::json!(1e308);
    assert!(matches!(
        load_scenarios(&value.to_string()),
        Err(LoadError::NonFinite { .. })
    ));
    for key in ["class", "schedule"] {
        let mut value: serde_json::Value = serde_json::from_str(source).unwrap();
        value[key] = "unsupported".into();
        assert!(matches!(
            load_scenarios(&value.to_string()),
            Err(LoadError::Json { .. })
        ));
    }
    for (key, original) in [("start", "0.2"), ("step", "0")] {
        let invalid = source.replace(
            &format!("\"{key}\":{original}"),
            &format!("\"{key}\":1e999"),
        );
        assert_ne!(
            invalid, source,
            "synthetic non-finite input must modify {key}"
        );
        assert!(matches!(
            load_scenarios(&invalid),
            Err(LoadError::Json { .. })
        ));
    }
    let mut value: serde_json::Value = serde_json::from_str(source).unwrap();
    value["emitters"] = serde_json::json!([]);
    assert!(matches!(
        load_scenarios(&value.to_string()),
        Err(LoadError::Emitters { .. })
    ));
}
#[test]
fn schedule_sequential_and_round_robin_with_exhaustion() {
    let mut scenario = load_scenarios(CORPUS).unwrap().remove(0);
    scenario.segments[0].count = 3;
    let mut other = scenario.segments[0].clone();
    other.tag = "other".into();
    other.count = 2;
    scenario.segments.push(other);
    let view = |s: &Scenario| {
        schedule(s)
            .iter()
            .map(|(segment, i)| (segment.tag.clone(), *i))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        view(&scenario),
        vec![
            ("belief_variance".into(), 0),
            ("belief_variance".into(), 1),
            ("belief_variance".into(), 2),
            ("other".into(), 0),
            ("other".into(), 1)
        ]
    );
    scenario.schedule = Schedule::RoundRobin;
    assert_eq!(
        view(&scenario),
        vec![
            ("belief_variance".into(), 0),
            ("other".into(), 0),
            ("belief_variance".into(), 1),
            ("other".into(), 1),
            ("belief_variance".into(), 2)
        ]
    );
}
