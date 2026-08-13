#![forbid(unsafe_code)]

//! Kernel-side implementation of the `LifecycleResolver` trait.
//!
//! Architecture §4.0.9 + Story 5.1 Task 8.

use std::sync::Arc;

use maos_domain::lifecycle::{LifecycleError, LifecycleReceipt, LifecycleResolver, LifecycleVerb};

use crate::iac::transparency_log::TransparencyLogAdapter;
use crate::scheduler::scheduler_loop::SpiritSchedulerAdapter;

/// Kernel-side lifecycle resolver — routes operator verbs through
/// the Spirit Scheduler and journals FR42 director-action audit rows.
#[maos_attrs::i9_exempt(
    reason = "kernel lifecycle verb resolver holding Arc handles to the already-exempt scheduler + transparency-log adapters; supervised composite per I9 (Story 7.1.7 baseline-reset)"
)]
pub struct KernelLifecycleResolver {
    scheduler: Arc<SpiritSchedulerAdapter>,
    transparency_log: Arc<TransparencyLogAdapter>,
    director_identity: String,
}

impl KernelLifecycleResolver {
    /// Construct a resolver for an authenticated, nonblank director identity.
    pub fn new(
        scheduler: Arc<SpiritSchedulerAdapter>,
        transparency_log: Arc<TransparencyLogAdapter>,
        director_identity: String,
    ) -> Result<Self, LifecycleError> {
        if director_identity.trim().is_empty() {
            return Err(LifecycleError::Admission(
                "director identity must not be blank".into(),
            ));
        }
        Ok(Self {
            scheduler,
            transparency_log,
            director_identity,
        })
    }
}

impl LifecycleResolver for KernelLifecycleResolver {
    fn resolve_verb(
        &self,
        spirit_id: &str,
        verb: LifecycleVerb,
    ) -> Result<LifecycleReceipt, LifecycleError> {
        let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
        let scheduler = Arc::clone(&self.scheduler);
        let tl = Arc::clone(&self.transparency_log);
        let director = self.director_identity.clone();
        let sid = spirit_id.to_string();
        let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);

        // The public resolver is synchronous, but it must be callable both
        // without Tokio and from a current-thread Tokio runtime.  Own the
        // async bridge on a dedicated thread instead of nesting/blocking the
        // caller's runtime, which would panic in either unsupported context.
        std::thread::Builder::new()
            .name("maos-lifecycle-resolver".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|err| {
                        LifecycleError::Internal(format!(
                            "unable to build lifecycle runtime: {err}"
                        ))
                    })
                    .and_then(|runtime| {
                        runtime.block_on(async move {
                            let receipt = match verb {
                                LifecycleVerb::Load => {
                                    return Err(LifecycleError::Admission(
                                        format!("load for '{sid}' must be called through SpiritSchedulerAdapter::load() directly — LifecycleResolver does not carry manifest/Spirit object")
                                    ));
                                }
                                LifecycleVerb::Start => {
                                    let pid = scheduler.resolve_pid(&sid).ok_or_else(|| {
                                        LifecycleError::NotLoaded {
                                            spirit_id: sid.clone(),
                                        }
                                    })?;
                                    scheduler.start(pid).await?;
                                    LifecycleReceipt::new(pid, verb, now_ns, None)?
                                }
                                LifecycleVerb::Pause => {
                                    let pid = scheduler.resolve_pid(&sid).ok_or_else(|| {
                                        LifecycleError::NotLoaded {
                                            spirit_id: sid.clone(),
                                        }
                                    })?;
                                    scheduler.pause(pid).await?;
                                    LifecycleReceipt::new(pid, verb, now_ns, None)?
                                }
                                LifecycleVerb::Resume => {
                                    let pid = scheduler.resolve_pid(&sid).ok_or_else(|| {
                                        LifecycleError::NotLoaded {
                                            spirit_id: sid.clone(),
                                        }
                                    })?;
                                    scheduler.resume(pid).await?;
                                    LifecycleReceipt::new(pid, verb, now_ns, None)?
                                }
                                LifecycleVerb::Unload => {
                                    let pid = scheduler.resolve_pid(&sid).ok_or_else(|| {
                                        LifecycleError::NotLoaded {
                                            spirit_id: sid.clone(),
                                        }
                                    })?;
                                    scheduler.unload(pid).await?;
                                    LifecycleReceipt::new(pid, verb, now_ns, None)?
                                }
                                _ => {
                                    return Err(LifecycleError::Internal(format!(
                                        "verb {verb:?} not yet implemented at v0.3-β"
                                    )));
                                }
                            };

                            let _ = tl.insert_frame_event(
                                crate::iac::transparency_log::FrameKind::DecisionDispatch,
                                receipt.spirit_pid,
                                None,
                                &format!("lifecycle.{:?}", verb),
                                serde_json::json!({
                                    "director": director,
                                    "spirit_id": sid,
                                    "verb": format!("{:?}", verb),
                                })
                                .to_string()
                                .as_bytes(),
                                maos_domain::invariants::i3::FrameOrigin::HumanAuthored,
                            );
                            Ok(receipt)
                        })
                    });
                let _ = result_tx.send(result);
            })
            .map_err(|err| {
                LifecycleError::Internal(format!("unable to start lifecycle bridge: {err}"))
            })?;

        result_rx.recv().map_err(|err| {
            LifecycleError::Internal(format!(
                "lifecycle bridge terminated before returning: {err}"
            ))
        })?
    }
}

/// Test double — captures every call to `resolve_verb`.
/// NOT under `#[cfg(test)]` so director-surface tests can consume.
pub mod test_double {
    use super::*;
    use std::sync::Mutex;

    #[maos_attrs::i9_exempt(
        reason = "public test double (director-surface test support, intentionally not #[cfg(test)]-gated so external tests can consume it); the Mutex<Vec<(String, LifecycleVerb)>> captures resolve_verb calls for assertions — test-only capture state, never production runtime state, per I9 (Story 7.1.7 baseline-reset)"
    )]
    pub struct MockLifecycleResolver {
        calls: Mutex<Vec<(String, LifecycleVerb)>>,
    }

    impl MockLifecycleResolver {
        pub fn new() -> Self {
            Self {
                calls: Mutex::new(vec![]),
            }
        }

        pub fn calls(&self) -> Vec<(String, LifecycleVerb)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl LifecycleResolver for MockLifecycleResolver {
        fn resolve_verb(
            &self,
            spirit_id: &str,
            verb: LifecycleVerb,
        ) -> Result<LifecycleReceipt, LifecycleError> {
            self.calls.lock().unwrap().push((spirit_id.into(), verb));
            Ok(LifecycleReceipt {
                spirit_pid: 1,
                verb,
                timestamp_ns: 0,
                journal_offset_bytes: None,
            })
        }
    }
}
