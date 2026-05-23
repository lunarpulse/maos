#![forbid(unsafe_code)]

//! Revocation poller — periodic task polling the `RegistryClient` trait.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::revocation::{RegistryClient, RevocationError};

use crate::revocation::applier::RevocationApplier;
use crate::supervision::watchdog_common;
use crate::telemetry::iac_rt::{ErrorKind, IacRtMetrics, Service};

/// Periodic CRL fetch + apply task.
#[maos_attrs::i9_exempt(reason = "revocation poller composite; holds exempt adapter Arcs")]
pub struct RevocationPoller {
    applier: Arc<RevocationApplier>,
    registry_client: Arc<dyn RegistryClient>,
    crypto: Arc<dyn CryptoProvider>,
    telemetry: Arc<IacRtMetrics>,
}

impl RevocationPoller {
    pub fn new(
        applier: Arc<RevocationApplier>,
        registry_client: Arc<dyn RegistryClient>,
        crypto: Arc<dyn CryptoProvider>,
        telemetry: Arc<IacRtMetrics>,
    ) -> Self {
        Self {
            applier,
            registry_client,
            crypto,
            telemetry,
        }
    }

    pub fn spawn(self: Arc<Self>, cancel: CancellationToken) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let cadence = pick_poll_cadence();
            let mut interval = tokio::time::interval(cadence);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = interval.tick() => {
                        if let Err(e) = self.poll_once().await {
                            eprintln!("revocation poller: poll_once failed: {e}");
                            self.telemetry.record_iac_error(
                                Service::RevocationApplier,
                                ErrorKind::App,
                            );
                        }
                    }
                }
            }
        })
    }

    async fn poll_once(&self) -> Result<(), RevocationError> {
        let bytes = self.registry_client.fetch_signed_crl()?;
        let trust_anchor = self.registry_client.trust_anchor_pub()?;
        let crl =
            crate::revocation::parser::parse_signed_crl(&bytes, &trust_anchor, &*self.crypto)?;
        let _report = self.applier.apply_crl(crl).await?;
        Ok(())
    }
}

/// Poll cadence: uses the shared watchdog_common baseline, but allows
/// `MAOS_REVOCATION_FAST` to collapse to 100ms independently of
/// `MAOS_SUPERVISION_FAST`.
fn pick_poll_cadence() -> std::time::Duration {
    let base = watchdog_common::pick_poll_cadence();
    if std::env::var_os("MAOS_REVOCATION_FAST").is_some() {
        std::time::Duration::from_millis(100)
    } else {
        base
    }
}
