//! Daemon custody and root ownership of per-Worker egress listeners.

use maos_domain::{host_grant::EgressDestination, invariants::i3::FrameOrigin};
use maos_egress::{
    Allowlist, BearerTable, Credential, Limits, ProxyConfig, ProxyGuard, Refusal, RefusalSink,
};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    io,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

type Proxy = Arc<Mutex<ProxyGuard>>;

struct OwnerState {
    closed: bool,
    proxies: HashMap<u32, Proxy>,
}

struct Shared {
    run_dir: PathBuf,
    table: Arc<BearerTable>,
    sink: Arc<JournalSink>,
    cancel: CancellationToken,
    state: Mutex<OwnerState>,
}

impl Shared {
    fn stop_and_join(&self) -> io::Result<()> {
        let proxies = {
            let mut state = self.state.lock();
            state.closed = true;
            std::mem::take(&mut state.proxies)
        };
        let mut failure = None;
        // Keep every proxy until its join completes, including one a binding
        // is concurrently finishing. Never hold the registry lock while joining.
        for proxy in proxies.into_values() {
            if let Err(error) = proxy.lock().stop_and_join() {
                failure.get_or_insert(error);
            }
        }
        // Proxies remove their own Worker directories; an empty root directory
        // is now inert. A failed proxy leaves it non-empty for inspection.
        let _ = std::fs::remove_dir(&self.run_dir);
        failure.map_or(Ok(()), Err)
    }
}

#[derive(Clone)]
pub struct EgressHandle(Arc<Shared>);

impl EgressHandle {
    pub fn start(
        home: PathBuf,
        log: Arc<TransparencyLogAdapter>,
        cancel: CancellationToken,
    ) -> io::Result<(Self, tokio::task::JoinHandle<io::Result<()>>)> {
        let run_dir = create_root_dir(&home.join("egress"))?;
        let shared = Arc::new(Shared {
            run_dir,
            table: Arc::new(BearerTable::default()),
            sink: Arc::new(JournalSink(log)),
            cancel,
            state: Mutex::new(OwnerState {
                closed: false,
                proxies: HashMap::new(),
            }),
        });
        let task_owner = Arc::clone(&shared);
        let task = tokio::spawn(async move {
            task_owner.cancel.cancelled().await;
            tokio::task::spawn_blocking(move || task_owner.stop_and_join())
                .await
                .map_err(|_| io::Error::other("egress root cleanup task panicked"))?
        });
        Ok((Self(shared), task))
    }

    pub fn is_available(&self) -> bool {
        !self.0.cancel.is_cancelled() && !self.0.state.lock().closed
    }

    pub fn spawn(
        &self,
        pid: u32,
        spirit_id: &str,
        requested: &[EgressDestination],
        granted: &[EgressDestination],
        credential: Credential,
        ttl: Duration,
    ) -> io::Result<WorkerEgress> {
        let mut state = self.0.state.lock();
        if state.closed || self.0.cancel.is_cancelled() {
            return Err(io::Error::other("egress root is stopped"));
        }
        if state.proxies.contains_key(&pid) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Worker egress is already bound",
            ));
        }
        let policy = Arc::new(Allowlist::intersect(requested, granted));
        let bearer = self.0.table.mint(pid, spirit_id, policy, ttl)?;
        let guard = match ProxyGuard::bind(
            &self.0.run_dir,
            ProxyConfig {
                spirit_pid: pid,
                spirit_id: spirit_id.to_owned(),
                table: Arc::clone(&self.0.table),
                credential,
                sink: self.0.sink.clone(),
                limits: Limits::default(),
            },
        ) {
            Ok(guard) => guard,
            Err(error) => {
                self.0.table.revoke(pid);
                return Err(error);
            }
        };
        let socket = guard.socket_path().to_owned();
        let proxy = Arc::new(Mutex::new(guard));
        state.proxies.insert(pid, Arc::clone(&proxy));
        Ok(WorkerEgress {
            pid,
            bearer,
            socket,
            proxy,
            owner: Arc::clone(&self.0),
            finished: false,
        })
    }
}

/// The binding's secret lease. No Debug/Display or serialization implementation.
pub struct WorkerEgress {
    pid: u32,
    bearer: String,
    socket: PathBuf,
    proxy: Proxy,
    owner: Arc<Shared>,
    finished: bool,
}

impl WorkerEgress {
    pub fn bearer(&self) -> &str {
        &self.bearer
    }
    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    pub fn finish(&mut self) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        self.finished = true;
        let result = self.proxy.lock().stop_and_join();
        if result.is_ok() {
            let mut state = self.owner.state.lock();
            if state
                .proxies
                .get(&self.pid)
                .is_some_and(|proxy| Arc::ptr_eq(proxy, &self.proxy))
            {
                state.proxies.remove(&self.pid);
            }
        }
        // A failed confirmation remains root-owned so root teardown reports it.
        result
    }
}

impl Drop for WorkerEgress {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("maos: Worker egress cleanup failed: {error}");
        }
    }
}

/// Scheduler pids restart at 1 in every root, and roots may share one data
/// home. Each root therefore binds Worker sockets beneath its own exclusively
/// created private directory; a crashed root's leftovers cannot collide.
fn create_root_dir(parent: &Path) -> io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)?;
    let metadata = std::fs::symlink_metadata(parent)?;
    if !metadata.file_type().is_dir() || metadata.permissions().mode() & 0o777 != 0o700 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "egress directory must be a private directory",
        ));
    }
    for _ in 0..8 {
        let mut nonce = [0u8; 4];
        getrandom::fill(&mut nonce).map_err(io::Error::other)?;
        let path = parent.join(format!("{:08x}", u32::from_ne_bytes(nonce)));
        match std::fs::DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no unique egress root directory",
    ))
}

struct JournalSink(Arc<TransparencyLogAdapter>);

#[derive(serde::Serialize)]
struct BlockPayload<'a> {
    spirit_id: &'a str,
    reason: &'a str,
}

impl RefusalSink for JournalSink {
    fn confirm(&self, refusal: &Refusal<'_>) -> io::Result<()> {
        let intent = match refusal.destination {
            Some(destination) => format!("sandbox.block.egress.{destination}"),
            None => "sandbox.block.egress.invalid-request".to_owned(),
        };
        let payload = serde_json::to_vec(&BlockPayload {
            spirit_id: refusal.spirit_id,
            reason: refusal.reason,
        })
        .map_err(io::Error::other)?;
        // The return value is the TL's confirmed LogBeforeDeliver proof. A
        // failed insert cannot return success, so the proxy cannot send a 403.
        let _confirmed = self.0.insert_frame_event(
            FrameKind::SandboxBlock,
            refusal.spirit_pid,
            None,
            &intent,
            &payload,
            FrameOrigin::Kernel,
        );
        Ok(())
    }
}
