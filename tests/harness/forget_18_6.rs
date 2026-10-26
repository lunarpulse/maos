//! Shared real-store/process oracles for Story 18-6 (no cargo subprocesses).
#![allow(dead_code)]
use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::{DistillationPort, MemoryManagerPort};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
pub fn db(home: &Path) -> PathBuf {
    home.join("audit/transparency.sqlite")
}
pub fn command(binary: &Path, home: &Path) -> Command {
    let mut c = Command::new(binary);
    c.current_dir(root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"))
        .env("MAOS_MEMORY_ROOT", home.join("memory"))
        .env("MAOS_SECRETS_BACKEND", "env")
        .env("MAOS_NOTIFY_DISABLE", "1");
    c
}
pub fn checked(mut c: Command) -> Output {
    let output = c.output().expect("spawn MAOS process");
    assert!(
        output.status.success(),
        "status {:?}; stdout {}; stderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
pub fn ctl(binary: &Path, home: &Path, args: &[&str]) -> Output {
    let mut c = command(binary, home);
    c.args(args);
    c.output().expect("maosctl")
}
pub fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "terminal JSON: {e}; {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}
pub fn rows(home: &Path) -> Vec<(String, u32, String, Value)> {
    if !db(home).exists() {
        return vec![];
    }
    let c = rusqlite::Connection::open(db(home)).unwrap();
    let mut s = c.prepare("SELECT lower(hex(frame_id)), spirit_pid, intent, payload_redacted FROM transparency_log ORDER BY timestamp_ns").unwrap();
    let rows = s
        .query_map([], |r| {
            let bytes: Vec<u8> = r.get(3)?;
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                serde_json::from_slice(&bytes)
                    .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into_owned())),
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    rows
}
pub fn wait(what: &str, daemon: &Daemon, mut predicate: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(15);
    while !predicate() {
        assert!(
            Instant::now() < end,
            "waiting for {what}: {}",
            daemon.diagnostics()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub struct Daemon {
    child: Child,
    stderr: Arc<Mutex<String>>,
}
impl Daemon {
    pub fn spawn(mut command: Command) -> Self {
        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut pipe = child.stderr.take().unwrap();
        let stderr = Arc::new(Mutex::new(String::new()));
        let sink = stderr.clone();
        std::thread::spawn(move || {
            let mut b = [0; 4096];
            while let Ok(n) = pipe.read(&mut b) {
                if n == 0 {
                    break;
                }
                sink.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&b[..n]));
            }
        });
        Self { child, stderr }
    }
    pub fn diagnostics(&self) -> String {
        self.stderr.lock().unwrap().clone()
    }
}
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct Stores {
    pub memory: MemoryManagerAdapter,
    pub tl: Arc<TransparencyLogAdapter>,
}
impl Stores {
    pub fn open(home: &Path, nonce: u64) -> Self {
        std::fs::create_dir_all(home.join("audit")).unwrap();
        let private = Arc::new(PrivateMemoryStore::new(home.join("memory"), 4096));
        let shared = Arc::new(SharedMemoryStore::open(&db(home)).unwrap());
        let index = Arc::new(PrincipalNamespaceIndex::open(&db(home)).unwrap());
        let tl = Arc::new(
            TransparencyLogAdapter::open_with_global_legal_holds(&db(home), &db(home), nonce)
                .unwrap(),
        );
        let memory = MemoryManagerAdapter::new(private, shared, index, tl.clone());
        Self { memory, tl }
    }
    pub fn private(&self, pid: u32, principal: &str, bytes: &str) {
        self.memory
            .write(
                pid,
                MemoryTier::Private,
                &MemoryNamespace::Principal {
                    principal_id: principal.into(),
                    schema: "forget-18-6".into(),
                },
                "record",
                MemoryValue::Blob(bytes.as_bytes().to_vec()),
            )
            .unwrap();
    }
    pub fn frame(&self, pid: u32, principal: &str, canary: &str) -> [u8; 16] {
        self.tl.insert_frame_event(
            FrameKind::McpInvocation,
            pid,
            None,
            "mcp:forget-witness",
            serde_json::json!({"principal": principal, "canary": canary})
                .to_string()
                .as_bytes(),
            FrameOrigin::SpiritAuto,
        );
        self.tl.last_frame_id()
    }
    pub fn digest(&self, pid: u32, sources: Vec<[u8; 16]>, body: &str) -> [u8; 16] {
        let writer =
            maos_kernel_core::iac::distillate::DistillateWriter::new(self.tl.clone(), Arc::new(()));
        writer
            .write_distillate(
                pid,
                DistillationRequest::new(sources, 1, DigestPayload::Text(body.into()), None)
                    .unwrap(),
            )
            .unwrap()
            .digest_frame_id
    }
}
fn private_bytes(path: &Path, out: &mut Vec<u8>) {
    if !path.exists() {
        return;
    }
    for e in std::fs::read_dir(path).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            private_bytes(&p, out);
        } else {
            out.extend(std::fs::read(p).unwrap());
        }
    }
}
/// Queryable bytes, not SQLite free pages; lifecycle intent exceptions apply only to the id.
pub fn assert_absent(home: &Path, principal: &str, canary: &str) {
    assert!(maos_audit::subject_access_query(&db(home), principal)
        .unwrap()
        .is_empty());
    let c = rusqlite::Connection::open(db(home)).unwrap();
    let mut s = c
        .prepare("SELECT intent,payload_redacted FROM transparency_log")
        .unwrap();
    for row in s
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
        })
        .unwrap()
    {
        let (intent, bytes) = row.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(canary), "canary leakage in {intent}: {text}");
        if ![
            "principal.forget",
            "principal.forget.held",
            "distillate.redacted",
        ]
        .contains(&intent.as_str())
        {
            assert!(
                !text.contains(principal),
                "principal leakage in {intent}: {text}"
            );
        }
    }
    for table in ["shared_memory", "principal_index"] {
        let mut s = c.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns = s.column_count();
        let mut cursor = s.query([]).unwrap();
        while let Some(row) = cursor.next().unwrap() {
            for i in 0..columns {
                let text = match row.get_ref(i).unwrap() {
                    rusqlite::types::ValueRef::Text(b) | rusqlite::types::ValueRef::Blob(b) => {
                        String::from_utf8_lossy(b)
                    }
                    _ => continue,
                };
                assert!(
                    !text.contains(principal) && !text.contains(canary),
                    "{table} leakage: {text}"
                );
            }
        }
    }
    let mut bytes = Vec::new();
    private_bytes(&home.join("memory"), &mut bytes);
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        !text.contains(principal) && !text.contains(canary),
        "Private spill leakage"
    );
}
pub fn assert_pending(terminal: &Value) {
    let outcomes = terminal["redistill_outcomes"]
        .as_array()
        .expect("typed outcomes");
    for row in outcomes {
        assert_eq!(row["outcome"]["status"], "pending");
        assert_eq!(row["outcome"]["reason"], "writer-not-live");
    }
    assert_eq!(
        terminal["pending_count"].as_u64().unwrap() as usize,
        outcomes.len()
    );
}
