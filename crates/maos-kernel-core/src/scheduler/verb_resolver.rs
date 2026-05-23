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
pub struct KernelLifecycleResolver {
    scheduler: Arc<SpiritSchedulerAdapter>,
    transparency_log: Arc<TransparencyLogAdapter>,
    director_identity: String,
}

impl KernelLifecycleResolver {
    pub fn new(
        scheduler: Arc<SpiritSchedulerAdapter>,
        transparency_log: Arc<TransparencyLogAdapter>,
        director_identity: String,
    ) -> Self {
        Self {
            scheduler,
            transparency_log,
            director_identity,
        }
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

        // Use block_in_place to run async from sync context without panic.
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let receipt = match verb {
                    LifecycleVerb::Load => {
                        // Load requires a manifest + Spirit object, which the
                        // LifecycleResolver trait (by design per architecture §4.0.9)
                        // does not carry.  The operator loads a Spirit via
                        // `scheduler.load(spirit_id, manifest, spirit, boot_nonce)`
                        // directly; LifecycleResolver handles start/pause/resume/unload
                        // only.  v0.3-β: return a clear error directing the caller.
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

                // FR42 director-action audit row.
                let _ = tl.insert_frame_event(
                    crate::iac::transparency_log::FrameKind::DecisionDispatch,
                    receipt.spirit_pid,
                    None,
                    &format!("lifecycle.{:?}", verb),
                    serde_json::json!({
                        "director": director,
                        "spirit_id": sid,
                        "verb": format!("{:?}", verb),
                    }).to_string().as_bytes(),
                    maos_domain::invariants::i3::FrameOrigin::HumanAuthored,
                );

                Ok(receipt)
            })
        })?;

        Ok(result)
    }
}

/// Test double — captures every call to `resolve_verb`.
/// NOT under `#[cfg(test)]` so director-surface tests can consume.
pub mod test_double {
    use super::*;
    use std::sync::Mutex;

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
