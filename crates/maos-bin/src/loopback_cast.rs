//! Story 22-8 (D-22-8-07 A) — the in-process A2A cast.
//!
//! A topology (or Host file) that declares `[[topology.a2a_peers]]` gets one in-process
//! [`LoopbackA2ARouter`] for the Host. Every ordered pair `sender -> recipient` of declared
//! peers is one *edge* with its own pair of router peer entries, so the operator's two lists
//! read as written: a frame is admitted only if the SENDER's `send_allowlist` and the
//! RECIPIENT's `accept_allowlist` both name its typed intent (the router's own gates are
//! per-peer: the destination entry's send list and the source entry's accept list). The
//! intake gate runs the router's full chain (TOFU pin, consent envelope, accept allowlist) and
//! writes the `ConsentRupture` row, through [`CohortRuptureLogSink`], before it refuses.
//!
//! The sender-side [`mira::AdvisoryRoute`] and the recipient-side intake consumer are the two
//! ends of the advisory: nothing reaches a recipient except through the router, and the
//! consumer journals the admitted frame before the Spirit acts on it (log before deliver).

use crate::native_task::PidSource;
use crate::topology::A2aPeerEntry;
use maos_a2a::{
    A2APeerConfig, A2APeerRouter, A2AProfile, ConsentAllowlists, InMemoryTofuPinStore,
    LoopbackA2ARouter, PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_kernel_core::iac::transparency_log::{FrameKind, FrameRecipient, FrameRowWrite};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_spirit_abi::identity::{FrameKind as WireKind, HostId, SpiritId, SpiritRole};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// What a recipient does with one admitted frame (run off the router's task, on a blocking
/// thread).
pub type FrameHandler = Arc<dyn Fn(IacFrame) + Send + Sync>;

type Handlers = Arc<Mutex<HashMap<String, FrameHandler>>>;

/// The typed intent Mira's advisory carries (ADR-012's fine-grained key).
pub const ADVISORY_INTENT: &str = mira::ADVISORY_FINE_GRAINED_INTENT;

/// `intent` of the journaled admitted advisory row at the recipient's pid.
pub const ADMITTED_INTENT: &str = "a2a.advisory.admitted";
/// `intent` of Nash's proposal row.
pub const PROPOSAL_INTENT: &str = "nash.proposal";

const PEER_ENDPOINT: &str = "tls://127.0.0.1:7443";
const PARTITION_TIMEOUT_SECS: u64 = 30;
const INTAKE_QUEUE: usize = 64;

fn edge_source(sender: &str, recipient: &str) -> String {
    format!("{sender}>{recipient}")
}

fn edge_destination(sender: &str, recipient: &str) -> String {
    format!("{sender}>{recipient}/to")
}

/// The Host's router, its declared peers and the recipients' intake handlers.
pub struct LoopbackCast {
    router: Arc<LoopbackA2ARouter>,
    peers: Vec<A2aPeerEntry>,
    handlers: Handlers,
    runtime: tokio::runtime::Handle,
    consumer: tokio::task::JoinHandle<()>,
}

impl Drop for LoopbackCast {
    fn drop(&mut self) {
        self.consumer.abort();
    }
}

impl LoopbackCast {
    /// Build the router for `peers` (at least two): an in-memory TOFU store pinning one
    /// synthetic fingerprint per edge entry, the rupture sink over the Transparency Log, and
    /// the intake consumer. Must run inside the Host's runtime.
    pub async fn build(
        peers: Vec<A2aPeerEntry>,
        log: Arc<TransparencyLogAdapter>,
    ) -> Result<Arc<Self>, String> {
        if peers.len() < 2 {
            return Err(
                "maos run: [[topology.a2a_peers]] declares fewer than two peers: consent is \
                 between two cast members"
                    .into(),
            );
        }
        let intents = |list: &[String]| list.iter().map(A2AIntent::new).collect::<Vec<_>>();
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        let mut configs = Vec::new();
        for sender in &peers {
            for recipient in peers.iter().filter(|r| r.spirit != sender.spirit) {
                // Frames from the edge's source host are checked against the recipient's
                // accept list; frames to the edge's destination host against the sender's
                // send list.
                let source = edge_source(&sender.spirit, &recipient.spirit);
                let destination = edge_destination(&sender.spirit, &recipient.spirit);
                for (name, allowlists) in [
                    (
                        source,
                        ConsentAllowlists {
                            send_allowlist: Vec::new(),
                            accept_allowlist: intents(&recipient.accept_allowlist),
                        },
                    ),
                    (
                        destination,
                        ConsentAllowlists {
                            send_allowlist: intents(&sender.send_allowlist),
                            accept_allowlist: Vec::new(),
                        },
                    ),
                ] {
                    let fingerprint = PeerCertFingerprint::from_cert_der(
                        format!("maos-loopback:{name}").as_bytes(),
                    );
                    tofu.pin_first_contact(&PeerId::new(&name), &fingerprint, &fingerprint, 1)
                        .await
                        .map_err(|e| format!("maos run: a2a peer pin for '{name}': {e}"))?;
                    configs.push(A2APeerConfig {
                        peer_id: PeerId::new(&name),
                        endpoint: PEER_ENDPOINT.into(),
                        cert_fingerprint: fingerprint,
                        profile: A2AProfile::Loopback,
                        allowlists,
                        partition_timeout_secs: PARTITION_TIMEOUT_SECS,
                        consent_ttl_secs: maos_a2a_core::config::DEFAULT_CONSENT_TTL_SECS,
                    });
                }
            }
        }
        let router = Arc::new(LoopbackA2ARouter::new(
            configs,
            tofu as Arc<dyn TofuPinStore>,
        ));
        router
            .install_rupture_sink(Arc::new(maos_cohort::CohortRuptureLogSink::new(log)))
            .await;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<IacFrame>(INTAKE_QUEUE);
        router.install_intake_sink(tx).await;
        let handlers: Handlers = Arc::default();
        let consumer = {
            let handlers = Arc::clone(&handlers);
            tokio::spawn(async move {
                while let Some(frame) = rx.recv().await {
                    let recipient = frame
                        .to
                        .first()
                        .map(|address| address.spirit_id.as_str().to_owned())
                        .unwrap_or_default();
                    let handler = handlers
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&recipient)
                        .cloned();
                    match handler {
                        Some(handler) => {
                            let _ = tokio::task::spawn_blocking(move || handler(frame));
                        }
                        None => eprintln!(
                            "maos: a2a frame for '{recipient}' dropped: no intake consumer is \
                             attached to it"
                        ),
                    }
                }
            })
        };
        Ok(Arc::new(Self {
            router,
            peers,
            handlers,
            runtime: tokio::runtime::Handle::current(),
            consumer,
        }))
    }

    /// The declared peer names, in file order.
    pub fn peer_names(&self) -> impl Iterator<Item = &str> {
        self.peers.iter().map(|peer| peer.spirit.as_str())
    }

    /// Attach `handler` as `recipient`'s intake consumer.
    pub fn attach_consumer(&self, recipient: &str, handler: FrameHandler) {
        self.handlers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(recipient.to_owned(), handler);
    }

    /// The advisory route of `sender`: one frame to every other declared peer that has an
    /// intake consumer attached, `None` when `sender` is not a declared peer.
    pub fn route_for(self: &Arc<Self>, sender: &str) -> Option<Arc<dyn mira::AdvisoryRoute>> {
        self.peers
            .iter()
            .any(|peer| peer.spirit == sender)
            .then(|| {
                Arc::new(CastRoute {
                    cast: Arc::clone(self),
                    sender: sender.to_owned(),
                }) as Arc<dyn mira::AdvisoryRoute>
            })
    }

    fn frame(&self, sender: &str, recipient: &str, advisory: &str) -> IacFrame {
        let from = FrameAddress {
            spirit_id: SpiritId::from(sender),
            host_id: Some(HostId(edge_source(sender, recipient))),
            role: Some(SpiritRole::Worker),
        };
        IacFrame {
            frame_id: ulid::Ulid::new().to_bytes(),
            timestamp_ns: 0,
            logical_clock: 0,
            from: from.clone(),
            to: smallvec::smallvec![FrameAddress {
                spirit_id: SpiritId::from(recipient),
                host_id: Some(HostId(recipient.to_owned())),
                role: Some(SpiritRole::Worker),
            }],
            // The v1.0 ABI has no evidence kind (Mira's TECH-DEBT(8.5)): the read-only
            // advisory rides a TaskAssign carrier; the consent intent says what it is.
            kind: WireKind::TaskAssign,
            intent: IntentClass::Readonly,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: advisory.to_owned(),
                scope: Vec::new(),
                success_criteria: "architect a fix".into(),
                posture_preferences: PosturePreferences::default(),
                prior_distillate_ref: None,
            }),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
                from,
                A2AIntent::new(ADVISORY_INTENT),
            )),
            intent_lineage: IntentLineage::default(),
        }
    }
}

/// Mira's [`mira::AdvisoryRoute`]: `Err(code)` names the refusal.
struct CastRoute {
    cast: Arc<LoopbackCast>,
    sender: String,
}

impl mira::AdvisoryRoute for CastRoute {
    fn deliver(&self, advisory: &mira::DiagnosticAdvisory) -> Result<(), String> {
        let wire = serde_json::to_string(advisory).map_err(|e| format!("encode_failed: {e}"))?;
        let recipients: Vec<String> = {
            let attached = self.cast.handlers.lock().unwrap_or_else(|e| e.into_inner());
            self.cast
                .peers
                .iter()
                .filter(|peer| peer.spirit != self.sender && attached.contains_key(&peer.spirit))
                .map(|peer| peer.spirit.clone())
                .collect()
        };
        if recipients.is_empty() {
            return Err("no_recipient".into());
        }
        for recipient in recipients {
            let frame = self.cast.frame(&self.sender, &recipient, &wire);
            let destination = HostId(edge_destination(&self.sender, &recipient));
            self.cast
                .runtime
                .block_on(self.cast.router.route_outbound(frame, &destination))
                .map_err(|error| refusal_code(&error))?;
        }
        Ok(())
    }
}

/// The stable code of a router refusal (the answer's `denied:<code>`).
fn refusal_code(error: &maos_a2a::A2AError) -> String {
    use maos_a2a::A2AError;
    match error {
        A2AError::IntentDeniedAtPeer { .. } => "intent_denied",
        A2AError::IntentDenied { .. } => "send_not_admitted",
        A2AError::ConsentUnclassified { .. } => "consent_unclassified",
        A2AError::ConsentExpired { .. } => "consent_expired",
        A2AError::PinInvalidated { .. } | A2AError::PinMismatch(_) => "pin_refused",
        _ => {
            eprintln!("maos: a2a advisory refused: {error}");
            "a2a_refused"
        }
    }
    .to_owned()
}

/// The correlation of the proposal row answering `advisory_frame_id`.
pub fn proposal_correlation(advisory_frame_id: [u8; 16]) -> String {
    format!("maos.a2a-proposal/{}", hex::encode(advisory_frame_id))
}

/// Nash's intake consumer: journal the admitted frame at Nash's pid first, then architect
/// (phrased through the bound seam when there is one), then journal the proposal row
/// correlated to the advisory's frame id.
pub fn architect_handler(
    nash: nash::Nash,
    log: Arc<TransparencyLogAdapter>,
    pid_of: PidSource,
) -> FrameHandler {
    Arc::new(move |frame: IacFrame| {
        let Some(pid) = pid_of() else {
            eprintln!(
                "maos: a2a advisory dropped: '{}' has no admitted pid",
                nash.spirit_id()
            );
            return;
        };
        let FramePayload::TaskAssign(carrier) = &frame.payload else {
            eprintln!("maos: a2a frame dropped: not an advisory carrier");
            return;
        };
        let recipients = [FrameRecipient {
            pid,
            spirit_id: nash.spirit_id().to_owned(),
            role: Some(SpiritRole::Worker),
        }];
        let written = log
            .insert_routed_frame(
                &frame,
                FrameKind::TaskAssign,
                pid,
                None,
                ADMITTED_INTENT,
                carrier.goal.as_bytes(),
                &recipients,
            )
            .into_inner();
        if written == FrameRowWrite::Duplicate {
            return;
        }
        let advisory = match nash::Nash::from_wire(&carrier.goal) {
            Ok(advisory) => advisory,
            Err(error) => {
                eprintln!("maos: a2a advisory is not a DiagnosticAdvisory: {error}");
                return;
            }
        };
        let (proposal, phrased) = match nash.architect_phrased(&advisory) {
            Ok(proposal) => (proposal, true),
            Err(error) => {
                eprintln!(
                    "maos: nash phrasing refused, the deterministic proposal stands: {error}"
                );
                (nash.architect(&advisory), false)
            }
        };
        let payload = serde_json::json!({ "proposal": proposal, "phrased": phrased });
        let _ = log
            .insert_frame_event_with_correlation(
                FrameKind::TelemetryEvent,
                pid,
                None,
                &proposal_correlation(frame.frame_id),
                PROPOSAL_INTENT,
                payload.to_string().as_bytes(),
                FrameOrigin::SpiritAuto,
            )
            .into_inner();
    })
}
