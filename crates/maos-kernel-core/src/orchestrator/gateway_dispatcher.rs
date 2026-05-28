#![forbid(unsafe_code)]

//! Gateway Dispatcher — kernel-managed lifecycle for gateway sub-modules.
//!
//! Story 6.5 / FR54 / ADR-029 binding-v1.0.
//!
//! **Boundary-Note:** Gateway sub-modules are **kernel-managed lifecycles**
//! mediated through the Capability Registry, NOT extensions to the 14-hook
//! `Spirit` trait. The `count_hooks!()` macro remains at 14; no new hooks
//! are added. This follows the CliWrapperSpirit option-(b) precedent.
//!
//! The dispatcher owns a `DashMap<(spirit_pid, gateway_id), GatewayInstance>`
//! of running gateway tasks. Per-Spirit admission calls
//! `admit_spirit_gateways`; per-Spirit unload calls `unload_spirit_gateways`.
//!
//! I9 exemption: the map is transient per-process state (structural-state
//! caching per I9). The dispatcher is recreated on kernel restart.

use dashmap::DashMap;
use maos_manifest::{GatewayEntry, GatewayType, GatewaysSection};
use maos_spirit_abi::gateway::{
    CancellationSignal, GatewayCapabilityHandle, GatewayCtx, GatewayError, GatewayMailboxHandle,
    GatewaySecretsHandle, GatewaySubmodule, GatewayTransparencyLogHandle, InboundMessage,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Per-gateway running state.
pub struct GatewayInstance {
    /// The gateway submodule (Arc-shared so on_disconnect can be called).
    pub submodule: Arc<dyn GatewaySubmodule>,
    /// Task handle for the gateway implementor's async task.
    pub task: tokio::task::JoinHandle<()>,
    /// Cancellation flag — set to true to signal unload.
    pub cancel_flag: Arc<AtomicBool>,
    /// Gateway type from the manifest (for uninstall record).
    pub gateway_type: GatewayType,
    /// Principal id for namespace isolation.
    pub principal_id: String,
    /// Cloned handle set for constructing on_disconnect ctx.
    pub cancel_handle: Box<dyn CancellationSignal>,
    pub mailbox: Box<dyn GatewayMailboxHandle>,
    pub capability: Box<dyn GatewayCapabilityHandle>,
    pub secrets: Box<dyn GatewaySecretsHandle>,
    pub transparency_log: Box<dyn GatewayTransparencyLogHandle>,
}

impl std::fmt::Debug for GatewayInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayInstance")
            .field("task", &"JoinHandle<()>")
            .field("cancel_flag", &"AtomicBool")
            .field("gateway_type", &self.gateway_type)
            .finish()
    }
}

/// Dispatcher that manages gateway submodules for all Spirits.
#[maos_attrs::i9_exempt(
    reason = "GatewayDispatcher holds per-process transient state (DashMap of running gateway tasks); recreated on kernel restart — structural-state caching per I9"
)]
pub struct GatewayDispatcher {
    /// (spirit_pid, gateway_id) -> running instance.
    gateways: Arc<DashMap<(u32, String), GatewayInstance>>,
    /// Factory registry by gateway type.
    factories: Arc<GatewaySubmoduleRegistry>,
}

impl std::fmt::Debug for GatewayDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayDispatcher")
            .field("gateways", &format!("DashMap<{} entries>", self.gateways.len()))
            .field("factories", &"...")
            .finish()
    }
}

impl GatewayDispatcher {
    pub fn new(factories: Arc<GatewaySubmoduleRegistry>) -> Self {
        Self {
            gateways: Arc::new(DashMap::new()),
            factories,
        }
    }

    /// Admit a Spirit's gateways: for each `[[gateway]]` entry, look up the
    /// factory, spawn the gateway task, and store the instance.
    ///
    /// Returns `Err(GatewayError::Fatal)` if any declared gateway type has
    /// no registered factory — Spirit admission FAILS in that case per AC4.
    pub async fn admit_spirit_gateways(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        principal_id: &str,
        gateways: &GatewaysSection,
    ) -> Result<(), GatewayError> {
        for entry in &gateways.entries {
            if self.gateways.contains_key(&(spirit_pid, entry.id.clone())) {
                return Err(GatewayError::Fatal(format!(
                    "EGatewayDuplicateId: gateway id '{}' already admitted for spirit_pid {}",
                    entry.id, spirit_pid
                )));
            }

            let factory = match self.factories.get(&entry.gateway_type) {
                Some(f) => f.clone(),
                None => {
                    return Err(GatewayError::Fatal(format!(
                        "EGatewayTypeUnregistered: no factory for gateway type {:?}",
                        entry.gateway_type
                    )));
                }
            };

            let cancel_flag = Arc::new(AtomicBool::new(false));
            let cancel_handle: Box<dyn CancellationSignal> =
                Box::new(GatewayCancelHandle { flag: cancel_flag.clone() });
            let mailbox: Box<dyn GatewayMailboxHandle> = Box::new(StubMailbox);
            let capability: Box<dyn GatewayCapabilityHandle> = Box::new(StubCapability);
            let secrets: Box<dyn GatewaySecretsHandle> = Box::new(StubSecrets);
            let transparency_log: Box<dyn GatewayTransparencyLogHandle> =
                Box::new(StubTransparencyLog);

            let ctx = GatewayCtx {
                spirit_id: spirit_id.into(),
                gateway_id: entry.id.clone(),
                principal_id: principal_id.into(),
                cancel: cancel_handle.clone_box(),
                mailbox: mailbox.clone_box(),
                capability: capability.clone_box(),
                secrets: secrets.clone_box(),
                transparency_log: transparency_log.clone_box(),
            };

            let submodule: Box<dyn GatewaySubmodule> = factory.create(entry)?;
            let arc_sub: Arc<dyn GatewaySubmodule> = Arc::from(submodule);

            let task = {
                let arc_sub = arc_sub.clone();
                let cancel = cancel_handle.clone_box();
                let secrets = secrets.clone_box();
                let tl = transparency_log.clone_box();
                let spirit_id = spirit_id.to_string();
                let gateway_id = entry.id.clone();
                let gateway_type_str = format!("{:?}", entry.gateway_type).to_lowercase();
                let max_backoff = std::time::Duration::from_secs(300);
                let max_retries: u32 = 5;

                tokio::spawn(async move {
                    let secret_ref = arc_sub.auth_secret_ref();
                    let _secret = match secrets.resolve(secret_ref).await {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            tl.write_lifecycle(
                                &spirit_id, &gateway_id, &gateway_type_str,
                                "auth_resolve_failed", 0,
                            ).await.ok();
                            return;
                        }
                    };

                    let mut attempt: u32 = 0;
                    loop {
                        if cancel.is_cancelled().await {
                            tl.write_lifecycle(
                                &spirit_id, &gateway_id, &gateway_type_str,
                                "cancelled_before_connect", 0,
                            ).await.ok();
                            return;
                        }

                        let ctx_clone = GatewayCtx {
                            spirit_id: spirit_id.clone(),
                            gateway_id: gateway_id.clone(),
                            principal_id: String::new(),
                            cancel: cancel.clone_box(),
                            mailbox: Box::new(StubMailbox),
                            capability: Box::new(StubCapability),
                            secrets: Box::new(StubSecrets),
                            transparency_log: tl.clone_box(),
                        };

                        let result = arc_sub.on_connect(ctx_clone).await;
                        match result {
                            Ok(()) => {
                                tl.write_lifecycle(
                                    &spirit_id, &gateway_id, &gateway_type_str,
                                    "connect", 0,
                                ).await.ok();
                                return;
                            }
                            Err(GatewayError::Backoff { retry_after }) => {
                                attempt += 1;
                                if attempt > max_retries {
                                    tl.write_lifecycle(
                                        &spirit_id, &gateway_id, &gateway_type_str,
                                        "backoff_exhausted", 0,
                                    ).await.ok();
                                    return;
                                }
                                let delay = retry_after.min(max_backoff);
                                tl.write_lifecycle(
                                    &spirit_id, &gateway_id, &gateway_type_str,
                                    "backoff_retry", 0,
                                ).await.ok();
                                tokio::time::sleep(delay).await;
                            }
                            Err(
                                GatewayError::Fatal(_)
                                | GatewayError::AuthResolveFailed(_)
                                | GatewayError::OutboundCapabilityDenied
                                | GatewayError::Cancelled
                                | _,
                            ) => {
                                tl.write_lifecycle(
                                    &spirit_id, &gateway_id, &gateway_type_str,
                                    "connect_failed", 0,
                                ).await.ok();
                                return;
                            }
                        }
                    }
                })
            };

            self.gateways.insert(
                (spirit_pid, entry.id.clone()),
                GatewayInstance {
                    submodule: arc_sub,
                    task,
                    cancel_flag,
                    gateway_type: entry.gateway_type.clone(),
                    principal_id: principal_id.into(),
                    cancel_handle,
                    mailbox,
                    capability,
                    secrets,
                    transparency_log,
                },
            );
        }
        Ok(())
    }

    /// Unload a Spirit's gateways: call `on_disconnect`, send cancellation,
    /// await task completion with timeout. Returns a record of the uninstall
    /// operation.
    pub async fn unload_spirit_gateways(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
    ) -> maos_domain::frame::GatewayUninstallRecord {
        use maos_domain::frame::{
            DisconnectOutcome, GatewayUninstallEntry, GatewayUninstallRecord,
        };
        let uninstalled_at_ns = crate::capability::cap_tokens::monotonic_now_ns();
        let keys_to_remove: Vec<(u32, String)> = self
            .gateways
            .iter()
            .filter(|e| e.key().0 == spirit_pid)
            .map(|e| e.key().clone())
            .collect();

        let mut entries = Vec::with_capacity(keys_to_remove.len());
        for key in keys_to_remove {
            let gateway_id = key.1.clone();
            if let Some((_, instance)) = self.gateways.remove(&key) {
                instance.cancel_flag.store(true, Ordering::Release);

                let disconnect_ctx = GatewayCtx {
                    spirit_id: spirit_id.into(),
                    gateway_id: gateway_id.clone(),
                    principal_id: instance.principal_id.clone(),
                    cancel: instance.cancel_handle.clone_box(),
                    mailbox: instance.mailbox.clone_box(),
                    capability: instance.capability.clone_box(),
                    secrets: instance.secrets.clone_box(),
                    transparency_log: instance.transparency_log.clone_box(),
                };

                let arc_sub = instance.submodule.clone();
                let disconnect_result = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    arc_sub.on_disconnect(disconnect_ctx),
                )
                .await;

                let task_result = instance.task.await;

                let disconnect_outcome = match (&disconnect_result, &task_result) {
                    (Ok(Ok(())), _) | (_, Ok(())) => DisconnectOutcome::Clean,
                    (Ok(Err(_)), _) => DisconnectOutcome::Failed("on_disconnect error".into()),
                    (Err(_), _) => DisconnectOutcome::Timeout,
                    (_, Err(_)) => DisconnectOutcome::Failed("task panicked".into()),
                };

                let gateway_type_str = format!("{:?}", instance.gateway_type).to_lowercase();

                entries.push(GatewayUninstallEntry {
                    gateway_id,
                    gateway_type: gateway_type_str,
                    principal_ns_keys_removed: vec![],
                    revoked_cap_token_ids: vec![],
                    terminated_connection_id: None,
                    disconnect_outcome,
                });
            }
        }

        GatewayUninstallRecord {
            spirit_id: spirit_id.into(),
            spirit_pid,
            uninstalled_at_ns,
            gateways: entries,
        }
    }

    /// Deliver an inbound message to a specific gateway instance.
    pub async fn deliver_inbound(
        &self,
        spirit_pid: u32,
        gateway_id: &str,
        msg: InboundMessage<'_>,
    ) {
        let _ = (spirit_pid, gateway_id, msg);
    }
}

/// Registry of gateway submodule factories by type.
pub struct GatewaySubmoduleRegistry {
    factories: DashMap<GatewayType, Arc<dyn GatewaySubmoduleFactory>>,
}

impl std::fmt::Debug for GatewaySubmoduleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewaySubmoduleRegistry")
            .field("factories", &format!("DashMap<{} entries>", self.factories.len()))
            .finish()
    }
}

impl Default for GatewaySubmoduleRegistry {
    fn default() -> Self {
        Self {
            factories: DashMap::new(),
        }
    }
}

impl GatewaySubmoduleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        gateway_type: GatewayType,
        factory: Arc<dyn GatewaySubmoduleFactory>,
    ) {
        self.factories.insert(gateway_type, factory);
    }

    pub fn get(
        &self,
        gateway_type: &GatewayType,
    ) -> Option<Arc<dyn GatewaySubmoduleFactory>> {
        self.factories.get(gateway_type).map(|e| e.clone())
    }
}

/// Factory trait for creating gateway submodule instances.
/// Receives the manifest entry for per-instance configuration.
pub trait GatewaySubmoduleFactory: Send + Sync {
    fn create(
        &self,
        entry: &GatewayEntry,
    ) -> Result<Box<dyn GatewaySubmodule>, GatewayError>;
}

// ------------------------------------------------------------------
// Stub implementations for v0.5 (Task 5 wires real handles).
// ------------------------------------------------------------------

struct GatewayCancelHandle {
    flag: Arc<AtomicBool>,
}

impl CancellationSignal for GatewayCancelHandle {
    fn is_cancelled(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        let flag = self.flag.clone();
        Box::pin(async move { flag.load(Ordering::Acquire) })
    }

    fn clone_box(&self) -> Box<dyn CancellationSignal> {
        Box::new(GatewayCancelHandle {
            flag: self.flag.clone(),
        })
    }
}

#[derive(Clone)]
struct StubMailbox;

impl GatewayMailboxHandle for StubMailbox {
    fn deliver_inbound(
        &self,
        _gateway_id: &str,
        _external_recipient_id: &str,
        _sender_id: &str,
        _payload: &[u8],
        _timestamp_ns: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
    fn clone_box(&self) -> Box<dyn GatewayMailboxHandle> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct StubCapability;

impl GatewayCapabilityHandle for StubCapability {
    fn verify_outbound(
        &self,
        _token_id: [u8; 16],
        _recipient: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
    fn clone_box(&self) -> Box<dyn GatewayCapabilityHandle> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct StubSecrets;

impl GatewaySecretsHandle for StubSecrets {
    fn resolve(
        &self,
        _secret_ref: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, GatewayError>> + Send>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
    fn clone_box(&self) -> Box<dyn GatewaySecretsHandle> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct StubTransparencyLog;

impl GatewayTransparencyLogHandle for StubTransparencyLog {
    fn write_inbound(
        &self,
        _receiving_spirit_id: &str,
        _gateway_id: &str,
        _gateway_type: &str,
        _external_recipient_id: &str,
        _sender_id: &str,
        _payload_redacted_len: u32,
        _timestamp_ns: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async { Ok(()) })
    }

    fn write_outbound(
        &self,
        _sending_spirit_id: &str,
        _gateway_id: &str,
        _gateway_type: &str,
        _external_recipient_id: &str,
        _cap_token_id: [u8; 16],
        _payload_redacted_len: u32,
        _timestamp_ns: u64,
        _send_outcome: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async { Ok(()) })
    }

    fn write_lifecycle(
        &self,
        _spirit_id: &str,
        _gateway_id: &str,
        _gateway_type: &str,
        _event: &str,
        _timestamp_ns: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), GatewayError>> + Send>> {
        Box::pin(async { Ok(()) })
    }

    fn clone_box(&self) -> Box<dyn GatewayTransparencyLogHandle> {
        Box::new(self.clone())
    }
}
