//! Daemon custody and root ownership of per-Worker egress listeners.

use maos_domain::{host_grant::EgressDestination, invariants::i3::FrameOrigin};
use maos_egress::{
    Allowlist, BearerTable, Credential, Limits, ProxyConfig, ProxyGuard, Refusal, RefusalSink,
};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use parking_lot::Mutex;
use std::{
    collections::{HashMap, HashSet},
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
    unconfirmed: HashSet<u32>,
    reported: HashSet<u32>,
}
struct Shared {
    run_dir: PathBuf,
    table: Arc<BearerTable>,
    sink: Arc<JournalSink>,
    cancel: CancellationToken,
    state: Mutex<OwnerState>,
}

impl Shared {
    /// Stop a root's proxies without holding the owner registry lock over
    /// revocation, joins or filesystem operations.
    fn stop_and_join(&self) -> io::Result<()> {
        let (proxies, unreported) = {
            let mut state = self.state.lock();
            state.closed = true;
            let unreported = state
                .unconfirmed
                .difference(&state.reported)
                .copied()
                .collect::<Vec<_>>();
            state.reported.extend(unreported.iter().copied());
            (
                state
                    .proxies
                    .iter()
                    .map(|(pid, proxy)| (*pid, Arc::clone(proxy)))
                    .collect::<Vec<_>>(),
                unreported,
            )
        };
        for pid in unreported {
            eprintln!(
                "maos: Worker egress lease for pid {pid} remains termination_unconfirmed residue"
            );
        }
        let mut failure = None;
        // Keep every proxy in the owner map until its container termination
        // has been confirmed; root cancellation must not erase lease residue.
        // Never hold the registry lock while a proxy is stopped or joined.
        for (pid, proxy) in proxies {
            if let Err(error) = proxy.lock().stop_and_join() {
                eprintln!("maos: Worker egress proxy cleanup failed for pid {pid}: {error}");
                failure.get_or_insert(error);
            }
        }
        // Proxies remove their own Worker directories. A non-empty root is
        // intentionally left for inspection rather than making shutdown fail.
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
                unconfirmed: HashSet::new(),
                reported: HashSet::new(),
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

    /// A fail-closed hook for a stop whose termination stays unconfirmed:
    /// revoke the bearer now and mark the lease root-owned residue, before
    /// the binding (blocked on a held pipe) can ever reach `finish`.
    pub fn revoker(&self) -> Arc<dyn Fn() + Send + Sync> {
        let (owner, pid) = (Arc::clone(&self.owner), self.pid);
        Arc::new(move || retain_unconfirmed(&owner, pid))
    }

    pub fn finish(&mut self, termination_confirmed: bool) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        self.finished = true;
        if !termination_confirmed {
            retain_unconfirmed(&self.owner, self.pid);
        }
        let result = self.proxy.lock().stop_and_join();
        if result.is_ok() && termination_confirmed {
            let mut state = self.owner.state.lock();
            if state
                .proxies
                .get(&self.pid)
                .is_some_and(|proxy| Arc::ptr_eq(proxy, &self.proxy))
            {
                state.proxies.remove(&self.pid);
            }
        }
        result
    }
}

fn retain_unconfirmed(owner: &Shared, pid: u32) {
    use std::io::Write as _;
    owner.table.revoke(pid);
    let report = {
        let mut state = owner.state.lock();
        state.unconfirmed.insert(pid);
        state.reported.insert(pid)
    };
    if report {
        let _ = writeln!(
            std::io::stderr(),
            "maos: Worker egress lease for pid {pid} retained as termination_unconfirmed residue"
        );
    }
}

impl Drop for WorkerEgress {
    fn drop(&mut self) {
        if let Err(error) = self.finish(false) {
            eprintln!("maos: Worker egress cleanup failed: {error}");
        }
    }
}

/// T3 state is volatile root-owned custody, not durable user data: this keeps
/// private pid/CID receipts outside MAOS_HOME and ADR-067's purge inventory.
pub fn create_worker_state_root() -> io::Result<PathBuf> {
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let runtime_dir = PathBuf::from(runtime_dir);
        if !runtime_dir.as_os_str().is_empty() && validate_private_dir(&runtime_dir).is_ok() {
            if let Ok(root) = create_root_dir(&runtime_dir.join("maos-worker-state")) {
                return Ok(root);
            }
        }
    }
    create_root_dir(&std::env::temp_dir().join(format!(
        "maos-worker-state-{}",
        rustix::process::geteuid().as_raw()
    )))
}
/// Allocate one fresh state directory beneath this process root.
pub fn create_worker_state_dir(root: &Path) -> io::Result<PathBuf> {
    validate_private_dir(root)?;
    create_private_child_dir(root)
}

/// Create a private root below `parent`, then allocate its fresh process-owned
/// child. Non-empty children are retained as evidence during cleanup.
fn create_root_dir(parent: &Path) -> io::Result<PathBuf> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)?;
    validate_private_dir(parent)?;
    create_private_child_dir(parent)
}

fn validate_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    let metadata = std::fs::symlink_metadata(path)?;
    let effective_uid = rustix::process::geteuid().as_raw();
    if !metadata.file_type().is_dir()
        || metadata.uid() != effective_uid
        || metadata.permissions().mode() & 0o7777 != 0o700
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "directory must be owned by the effective uid and mode 0700",
        ));
    }
    Ok(())
}

fn create_private_child_dir(parent: &Path) -> io::Result<PathBuf> {
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
        "no unique private directory",
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
