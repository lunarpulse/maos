//! Installed component boot admission and shell name routing (20-1a AC4).
use crate::admission::{AdmissionDeps, AdmissionRefusal};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Default)]
pub struct InstalledSpirits {
    refusals: BTreeMap<String, AdmissionRefusal>,
    loaded: BTreeSet<String>,
    #[cfg(feature = "wasm-host")]
    sessions: Vec<tokio::task::JoinHandle<()>>,
    /// Each loaded Spirit's session-owned shell port: readiness, turn binding and turn end.
    #[cfg(feature = "wasm-host")]
    ports: BTreeMap<String, std::sync::Arc<crate::spirit_session::ShellPort>>,
}
impl InstalledSpirits {
    /// Boot installed roots; only a `shell` boot gives each loaded root its shell port and
    /// shell-session wrapper.
    pub async fn boot(
        manifests: Vec<(String, PathBuf)>,
        deps: &AdmissionDeps<'_>,
        crypto: &crate::crypto_provider::SelectedCrypto,
        boot_nonce: u64,
        shell: bool,
        #[cfg(feature = "wasm-host")] host: Option<&dyn maos_host::SpiritHostPort>,
        #[cfg(feature = "wasm-host")] session_deps: Option<crate::spirit_session::SessionDeps>,
    ) -> Self {
        #[cfg(not(feature = "wasm-host"))]
        let _ = shell;
        let mut result = Self::default();
        for (name, path) in manifests {
            let (text, publisher, held) = match maos_registry::install::verify_installed_launch(
                &name,
                &path,
                crypto.provider.as_ref(),
                crypto.name,
                &|id| {
                    deps.providers
                        .and_then(|catalog| catalog.get(id))
                        .map(|provider| provider.endpoint.as_str())
                },
            ) {
                Ok(launch) => (launch.manifest_toml, launch.publisher, launch.component),
                Err(error) => {
                    crate::admission::emit_vetter_key_event(
                        deps.transparency_log,
                        &name,
                        "unknown",
                        false,
                        "N/A",
                        &format!("installed_package_invalid: {error}"),
                    );
                    let refusal = AdmissionRefusal::from_registry_error(&*error);
                    eprintln!(
                        "maos: installed spirit '{name}' refused at boot: {} ({refusal})",
                        refusal.code()
                    );
                    result.refusals.insert(name, refusal);
                    continue;
                }
            };
            let admission = async {
                // Registry installs never introduce compiled-in first-party classes.
                let mut gated = crate::admission::gate_manifest(&text, &|_| false)?;
                gated.bind_registry_publisher(&publisher);
                if gated.spirit_id != name {
                    return Err(AdmissionRefusal::ClassBuild {
                        detail: "installed name differs from signed manifest".into(),
                    });
                }
                #[cfg(not(feature = "wasm-host"))]
                {
                    let _ = (&gated, boot_nonce, &held);
                    Err::<(), _>(AdmissionRefusal::WasmEngineOff)
                }
                #[cfg(feature = "wasm-host")]
                {
                    let session_deps = session_deps.as_ref().ok_or(
                        AdmissionRefusal::SpawnedSurfaceUnsupported {
                            surface: "installed component host",
                        },
                    )?;
                    let host = host.ok_or(AdmissionRefusal::WasmEngineOff)?;
                    // The component the verifier compared against the signed bytes travels
                    // open, with its signed digest, through admission into the runner: the
                    // install pathname is never reopened.
                    let component = held.ok_or_else(|| AdmissionRefusal::ClassBuild {
                        detail: "installed component has no verified held artifact".into(),
                    })?;
                    let artifact =
                        crate::spirit_session::LaunchArtifact::from_verified(component, &gated)
                            .map_err(|e| AdmissionRefusal::ClassBuild {
                                detail: e.to_string(),
                            })?;
                    let launch = crate::spirit_session::manifest_launch_held(
                        host,
                        &gated,
                        &path,
                        path.parent().unwrap_or(std::path::Path::new(".")),
                        false,
                        artifact,
                    )
                    .map_err(|e| AdmissionRefusal::ClassBuild {
                        detail: e.to_string(),
                    })?;
                    let (core, obj) = session_deps
                        .supervisor
                        .prepare_spawned(&name, &gated.bundle())
                        .map_err(|e| AdmissionRefusal::ClassBuild {
                            detail: e.to_string(),
                        })?;
                    let admitted = crate::admission::load_admit_start(
                        deps,
                        &gated,
                        obj,
                        boot_nonce,
                        crate::admission::StartPolicy::StartNow,
                        &mut |pid| core.bind_scheduler_pid(pid),
                    )
                    .await?;
                    let session_deps = session_deps.clone();
                    let id = name.clone();
                    let port =
                        shell.then(|| crate::spirit_session::ShellPort::new(launch.turn_budget));
                    if let Some(port) = &port {
                        result
                            .ports
                            .insert(name.clone(), std::sync::Arc::clone(port));
                    }
                    result.sessions.push(tokio::spawn(async move {
                        let ran = match port {
                            Some(port) => {
                                crate::spirit_session::run_admitted_shell_session(
                                    session_deps,
                                    admitted,
                                    core,
                                    launch,
                                    port,
                                )
                                .await
                            }
                            None => {
                                crate::spirit_session::run_admitted_session(
                                    session_deps,
                                    admitted,
                                    core,
                                    launch,
                                )
                                .await
                            }
                        };
                        if let Err(error) = ran {
                            eprintln!("maos: installed spirit '{id}' session refused: {error}");
                        }
                    }));
                    Ok(())
                }
            }
            .await;
            match admission {
                Ok(()) => {
                    result.loaded.insert(name);
                }
                Err(refusal) => {
                    crate::admission::emit_structural_rejection(
                        deps.transparency_log,
                        &text,
                        &refusal,
                    );
                    eprintln!(
                        "maos: installed spirit '{name}' refused at boot: {} ({refusal})",
                        refusal.code()
                    );
                    result.refusals.insert(name, refusal);
                }
            }
        }
        result
    }
    /// `None` alone means the name was neither loaded nor refused at this boot. A dispatched
    /// turn returns the Spirit's own answer, or the typed reason it produced none.
    pub fn dispatch(
        &self,
        name: &str,
        message: &str,
        iac: &maos_iac::adapter::IacBusAdapter,
    ) -> Option<Result<String, String>> {
        if let Some(refusal) = self.refusals.get(name) {
            return Some(Err(format!(
                "maos: spirit '{name}' refused at boot: {}",
                refusal.code()
            )));
        }
        if !self.loaded.contains(name) {
            return None;
        }
        #[cfg(feature = "wasm-host")]
        {
            Some(match self.ports.get(name) {
                Some(port) => Self::ask(port, name, message, iac),
                None => Err(format!("maos: spirit '{name}' has no shell turn port")),
            })
        }
        #[cfg(not(feature = "wasm-host"))]
        {
            let _ = (message, iac);
            Some(Err(format!(
                "maos: spirit '{name}' refused at boot: {}",
                AdmissionRefusal::WasmEngineOff.code()
            )))
        }
    }
    /// One shell turn: wait for the session's own readiness, queue the request, deliver its
    /// `TaskAssign`, and hold the routed answer until the matching clean turn completion.
    /// One absolute deadline, set once startup is done, bounds queueing through the reply.
    #[cfg(feature = "wasm-host")]
    fn ask(
        port: &std::sync::Arc<crate::spirit_session::ShellPort>,
        name: &str,
        message: &str,
        iac: &maos_iac::adapter::IacBusAdapter,
    ) -> Result<String, String> {
        let runtime = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            let _serial = port.serialize();
            let (pid, generation) = port
                .await_ready()
                .map_err(|error| format!("spirit '{name}': {error}"))?;
            let deadline = std::time::Instant::now() + port.turn_budget();
            let frame_id = ulid::Ulid::new().to_bytes();
            let turn = port
                .arm(frame_id, name, pid, generation, deadline)
                .map_err(|error| format!("spirit '{name}': {error}"))?;
            let frame = maos_domain::frame::IacFrame {
                frame_id,
                timestamp_ns: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                logical_clock: 0,
                from: maos_domain::frame::FrameAddress {
                    spirit_id: maos_iac::adapter::mailbox::SHELL_REPLY_DESTINATION.into(),
                    host_id: None,
                    role: None,
                },
                to: smallvec::smallvec![maos_domain::frame::FrameAddress {
                    spirit_id: name.into(),
                    host_id: None,
                    role: None
                }],
                kind: maos_spirit_abi::identity::FrameKind::TaskAssign,
                intent: maos_domain::invariants::i1::IntentClass::Standard,
                payload: maos_domain::frame::FramePayload::TaskAssign(
                    maos_domain::frame::TaskAssignPayload {
                        goal: message.to_owned(),
                        scope: Vec::new(),
                        success_criteria: "Answer the operator's shell message".into(),
                        posture_preferences: Default::default(),
                        prior_distillate_ref: None,
                    },
                ),
                auto_marker: maos_domain::invariants::i3::FrameOrigin::HumanAuthored,
                consent_envelope: None,
                intent_lineage: Default::default(),
            };
            let remaining = turn
                .deadline()
                .saturating_duration_since(std::time::Instant::now());
            match runtime.block_on(tokio::time::timeout(
                remaining,
                iac.deliver_typed(frame, 0, None),
            )) {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => return Err(format!("spirit '{name}' dispatch refused: {error}")),
                Err(_) => return Err(format!("spirit '{name}': turn deadline exceeded")),
            }
            turn.wait()
                .map_err(|error| format!("spirit '{name}': {error}"))
        })
    }
    pub async fn join(self) {
        #[cfg(feature = "wasm-host")]
        for session in self.sessions {
            let _ = session.await;
        }
    }
}
