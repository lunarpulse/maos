//! Installed component boot admission and shell name routing (20-1a AC4).
use crate::admission::{AdmissionDeps, AdmissionRefusal};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
#[cfg(feature = "wasm-host")]
use std::sync::Arc;

#[derive(Default)]
pub struct InstalledSpirits {
    refusals: BTreeMap<String, AdmissionRefusal>,
    loaded: BTreeSet<String>,
    #[cfg(feature = "wasm-host")]
    sessions: Vec<tokio::task::JoinHandle<()>>,
}
impl InstalledSpirits {
    pub async fn boot(
        manifests: Vec<(String, PathBuf)>,
        deps: &AdmissionDeps<'_>,
        boot_nonce: u64,
        #[cfg(feature = "wasm-host")] host: Option<&dyn maos_host::SpiritHostPort>,
        #[cfg(feature = "wasm-host")] session_deps: Option<crate::spirit_session::SessionDeps>,
    ) -> Self {
        let mut result = Self::default();
        for (name, path) in manifests {
            let text = match maos_registry::install::verify_installed(&name, &path) {
                Ok(text) => text,
                Err(error) => {
                    crate::admission::emit_vetter_key_event(
                        deps.transparency_log,
                        &name,
                        "unknown",
                        false,
                        "N/A",
                        &format!("installed_package_invalid: {error}"),
                    );
                    let refusal = AdmissionRefusal::Admission {
                        detail: error.to_string(),
                    };
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
                gated.bind_registry_package_at(&path)?;
                if gated.spirit_id != name {
                    return Err(AdmissionRefusal::ClassBuild {
                        detail: "installed name differs from signed manifest".into(),
                    });
                }
                #[cfg(not(feature = "wasm-host"))]
                {
                    let _ = (&gated, boot_nonce);
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
                    let launch = crate::spirit_session::manifest_launch(host, &gated, &path, false)
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
                    result.sessions.push(tokio::spawn(async move {
                        if let Err(error) = crate::spirit_session::run_admitted_session(
                            session_deps,
                            admitted,
                            core,
                            launch,
                        )
                        .await
                        {
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
    /// None alone means the name was neither loaded nor refused at this boot.
    pub fn dispatch(
        &self,
        name: &str,
        message: &str,
        iac: &maos_iac::adapter::IacBusAdapter,
    ) -> Option<String> {
        if let Some(refusal) = self.refusals.get(name) {
            return Some(format!(
                "maos: spirit '{name}' refused at boot: {}",
                refusal.code()
            ));
        }
        if !self.loaded.contains(name) {
            return None;
        }
        let frame = maos_domain::frame::IacFrame {
            frame_id: ulid::Ulid::new().to_bytes(),
            timestamp_ns: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
            logical_clock: 0,
            from: maos_domain::frame::FrameAddress {
                spirit_id: "operator".into(),
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
        let runtime = tokio::runtime::Handle::current();
        Some(tokio::task::block_in_place(|| {
            match runtime.block_on(iac.deliver_typed(frame, 0, None)) {
                Ok(_) => format!("maos: message delivered to spirit '{name}'"),
                Err(error) => format!("maos: spirit '{name}' dispatch refused: {error}"),
            }
        }))
    }
    pub async fn join(self) {
        #[cfg(feature = "wasm-host")]
        for session in self.sessions {
            let _ = session.await;
        }
    }
}
