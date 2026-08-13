#![forbid(unsafe_code)]

//! Revocation poller — periodic task polling the `RegistryClient` trait.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::revocation::{RegistryClient, RevocationError};

use crate::revocation::applier::RevocationApplier;
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

/// Poll cadence has explicit precedence: its own test override wins; otherwise
/// it follows the supervision test override; production default is one second.
fn pick_poll_cadence() -> std::time::Duration {
    pick_poll_cadence_from_flags(
        std::env::var_os("MAOS_REVOCATION_FAST").is_some(),
        std::env::var_os("MAOS_SUPERVISION_FAST").is_some(),
    )
}

fn pick_poll_cadence_from_flags(
    revocation_fast: bool,
    supervision_fast: bool,
) -> std::time::Duration {
    if revocation_fast {
        std::time::Duration::from_millis(100)
    } else if supervision_fast {
        std::time::Duration::from_millis(100)
    } else {
        std::time::Duration::from_secs(1)
    }
}

#[cfg(test)]
mod tests {
    use super::pick_poll_cadence_from_flags;
    use std::time::Duration;

    #[test]
    fn cadence_precedence_is_revocation_then_supervision_then_default() {
        assert_eq!(
            pick_poll_cadence_from_flags(true, true),
            Duration::from_millis(100)
        );
        assert_eq!(
            pick_poll_cadence_from_flags(true, false),
            Duration::from_millis(100)
        );
        assert_eq!(
            pick_poll_cadence_from_flags(false, true),
            Duration::from_millis(100)
        );
        assert_eq!(
            pick_poll_cadence_from_flags(false, false),
            Duration::from_secs(1)
        );
    }
}
