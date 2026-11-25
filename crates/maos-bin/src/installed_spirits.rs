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
    /// Boot installed roots; a `shell` boot, or a boot under a door that serves directed tasks
    /// (`directed`, Story 22-4), gives each loaded root its turn port and shell-session wrapper.
    pub async fn boot(
        manifests: Vec<(String, PathBuf)>,
        deps: &AdmissionDeps<'_>,
        crypto: &crate::crypto_provider::SelectedCrypto,
        boot_nonce: u64,
        shell: bool,
        directed: Option<&std::sync::Arc<crate::directed_task::DirectedTasks>>,
        #[cfg(feature = "wasm-host")] host: Option<&dyn maos_host::SpiritHostPort>,
        #[cfg(feature = "wasm-host")] session_deps: Option<crate::spirit_session::SessionDeps>,
    ) -> Self {
        #[cfg(not(feature = "wasm-host"))]
        let _ = (shell, directed);
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
            if let Err(refusal) = check_pin(&name, &publisher, deps.transparency_log) {
                eprintln!(
                    "maos: installed spirit '{name}' refused at boot: {} ({refusal})",
                    refusal.code()
                );
                result.refusals.insert(name, refusal);
                continue;
            }
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
                    let port = (shell || directed.is_some())
                        .then(|| crate::spirit_session::ShellPort::new(launch.turn_budget));
                    if let Some(port) = &port {
                        result
                            .ports
                            .insert(name.clone(), std::sync::Arc::clone(port));
                        // Story 22-4 — the door's directed tasks share the shell's one port.
                        if let Some(directed) = directed {
                            directed.register(
                                &name,
                                std::sync::Arc::new(PortRunner {
                                    port: std::sync::Arc::clone(port),
                                    name: name.clone(),
                                    iac: std::sync::Arc::clone(&session_deps.iac),
                                    runtime: tokio::runtime::Handle::current(),
                                }),
                            );
                        }
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
                            let end = session_end(&error);
                            eprintln!("maos: installed spirit '{id}' session {end}: {error}");
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
    /// The stable admission code that refused `name` at this boot, if it was refused.
    pub fn refusal_code(&self, name: &str) -> Option<&'static str> {
        self.refusals.get(name).map(AdmissionRefusal::code)
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
            run_turn(
                port,
                name,
                ulid::Ulid::new().to_bytes(),
                message,
                maos_iac::adapter::mailbox::ReplyLabel::Shell,
                iac,
                &runtime,
                None,
            )
        })
        .map_err(|failure| match failure {
            TurnFailure::Turn(error) => format!("spirit '{name}': {error}"),
            TurnFailure::DispatchRefused(error) => {
                format!("spirit '{name}' dispatch refused: {error}")
            }
        })
    }
    pub async fn join(self) {
        #[cfg(feature = "wasm-host")]
        for session in self.sessions {
            let _ = session.await;
        }
    }
}

/// Story 22-9 (D-22-9-11 A) — the verified package's publisher must be the durable pin's; a
/// mismatch (or an unreadable pin) refuses this Spirit and journals it, an absent pin (an
/// install made before pins) admits and says so.
fn check_pin(
    name: &str,
    publisher: &[u8; 32],
    tl: &maos_kernel_core::iac::TransparencyLogAdapter,
) -> Result<(), AdmissionRefusal> {
    let note = |admitted: bool, text: &str| {
        crate::admission::emit_vetter_key_event(tl, name, "unknown", admitted, "N/A", text)
    };
    match maos_registry::storage::LocalFsRegistryStorage::default_installed_publisher(name) {
        Ok(Some(pin)) if pin == *publisher => Ok(()),
        Ok(None) => {
            note(
                true,
                "installed_publisher_pin_absent: install predates pins",
            );
            Ok(())
        }
        outcome => {
            let detail = match outcome {
                Err(error) => format!("the durable publisher pin is unreadable: {error}"),
                _ => "the package publisher differs from the durable pin".to_string(),
            };
            note(false, &format!("installed_publisher_mismatch: {detail}"));
            Err(AdmissionRefusal::InstalledPublisherMismatch { detail })
        }
    }
}
/// An orderly supervisor stop (the shell left) is an end, not a refusal.
#[cfg(feature = "wasm-host")]
fn session_end(error: &crate::spirit_session::SessionError) -> &'static str {
    use crate::spirit_session::SessionError::Stopped;
    if matches!(error, Stopped) {
        "ended"
    } else {
        "refused"
    }
}

/// Why one turn produced no answer (Story 22-4): the typed port error, or the bus refusing the
/// request frame.
#[cfg(feature = "wasm-host")]
pub enum TurnFailure {
    Turn(crate::spirit_session::TurnError),
    DispatchRefused(String),
}

#[cfg(feature = "wasm-host")]
impl TurnFailure {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Turn(error) => error.reason(),
            Self::DispatchRefused(_) => "dispatch_refused",
        }
    }
}

/// The one turn: wait for the session's readiness, queue `message` under the host-minted
/// `frame_id`, deliver its `HumanAuthored` `TaskAssign` from the host pseudo-address, and hold
/// the routed answer until the matching clean turn completion. Blocks; call it off the async
/// workers. The port's `serialize()` makes the shell REPL and the door take turns.
#[cfg(feature = "wasm-host")]
pub fn run_turn(
    port: &std::sync::Arc<crate::spirit_session::ShellPort>,
    name: &str,
    frame_id: [u8; 16],
    message: &str,
    label: maos_iac::adapter::mailbox::ReplyLabel,
    iac: &maos_iac::adapter::IacBusAdapter,
    runtime: &tokio::runtime::Handle,
    assertion: Option<&maos_control::Assertion>,
) -> Result<String, TurnFailure> {
    use crate::spirit_session::TurnError;
    let _serial = port.serialize();
    let (pid, generation) = port.await_ready().map_err(TurnFailure::Turn)?;
    // The task's assertion governs the mints for this pid for exactly this turn (Story 23-2,
    // D-23-2-04 A): `_scope` is dropped on every exit path below.
    let _scope = crate::assertion_supply::enter(pid, assertion).map_err(|_| {
        TurnFailure::DispatchRefused(
            "an assertion scope is already active for this Spirit".to_owned(),
        )
    })?;
    let deadline = std::time::Instant::now() + port.turn_budget();
    let turn = port
        .arm_labelled(label, frame_id, name, pid, generation, deadline)
        .map_err(TurnFailure::Turn)?;
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
    // The timer is built inside the future: a pump thread has no ambient runtime until `block_on`.
    match runtime.block_on(async {
        tokio::time::timeout(remaining, iac.deliver_typed(frame, 0, None)).await
    }) {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => return Err(TurnFailure::DispatchRefused(error.to_string())),
        Err(_) => return Err(TurnFailure::Turn(TurnError::DeadlineExceeded)),
    }
    turn.wait().map_err(TurnFailure::Turn)
}

/// A loaded installed Spirit's door-task runner: one directed task is one [`run_turn`].
#[cfg(feature = "wasm-host")]
struct PortRunner {
    port: std::sync::Arc<crate::spirit_session::ShellPort>,
    name: String,
    iac: std::sync::Arc<maos_iac::adapter::IacBusAdapter>,
    runtime: tokio::runtime::Handle,
}

#[cfg(feature = "wasm-host")]
impl crate::directed_task::TaskRunner for PortRunner {
    fn run(
        &self,
        task_id: [u8; 16],
        text: &str,
        assertion: Option<&maos_control::Assertion>,
    ) -> Result<String, crate::directed_task::RunFailure> {
        run_turn(
            &self.port,
            &self.name,
            task_id,
            text,
            maos_iac::adapter::mailbox::ReplyLabel::Task,
            &self.iac,
            &self.runtime,
            assertion,
        )
        .map_err(|failure| crate::directed_task::RunFailure {
            reason: failure.reason(),
            detail: match failure {
                TurnFailure::Turn(error) => error.to_string(),
                TurnFailure::DispatchRefused(error) => error,
            },
        })
    }
}

#[cfg(all(test, feature = "wasm-host"))]
mod session_end_tests {
    use super::session_end;
    use crate::spirit_session::SessionError;

    #[test]
    fn only_a_supervisor_stop_is_not_a_refusal() {
        assert_eq!(session_end(&SessionError::Stopped), "ended");
        assert_eq!(session_end(&SessionError::UnexpectedEof), "refused");
        assert_eq!(session_end(&SessionError::Deadline("delivery")), "refused");
    }
}
