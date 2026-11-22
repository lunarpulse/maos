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
            // D-LRQC-09 A: print each error change and the recovery, count the repeats.
            let mut report = PollReport::default();
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = interval.tick() => {
                        let polled = self.poll_once().await;
                        if let Err((_, kind, _)) = &polled {
                            self.telemetry.record_iac_error(Service::RevocationApplier, *kind);
                        }
                        if let Some(line) = report.observe(polled) {
                            eprintln!("{line}");
                        }
                    }
                }
            }
        })
    }

    /// An unchanged CRL (`AlreadyApplied`) is the steady state, not a failure.
    async fn poll_once(&self) -> Result<(), PollFault> {
        let fault =
            |stage: &'static str, kind| move |e: RevocationError| (stage, kind, e.to_string());
        let bytes = self
            .registry_client
            .fetch_signed_crl()
            .map_err(fault("fetch", ErrorKind::Transport))?;
        let trust_anchor = self
            .registry_client
            .trust_anchor_pub()
            .map_err(fault("trust anchor", ErrorKind::Transport))?;
        let crl = crate::revocation::parser::parse_signed_crl(&bytes, &trust_anchor, &*self.crypto)
            .map_err(fault("parse", ErrorKind::Decode))?;
        match self.applier.apply_crl(crl).await {
            Ok(_) | Err(RevocationError::AlreadyApplied { .. }) => Ok(()),
            Err(e) => Err(("apply", ErrorKind::App, e.to_string())),
        }
    }
}

/// One failed poll: the stage that failed, its telemetry bucket and the error text.
type PollFault = (&'static str, ErrorKind, String);

/// The poller's failure state: one stderr line per error change (stage or text), repeats
/// counted, and a recovery line that names the last error.
#[derive(Default)]
struct PollReport {
    last: Option<(&'static str, String)>,
    failures: u64,
}

impl PollReport {
    fn observe(&mut self, polled: Result<(), PollFault>) -> Option<String> {
        let Err((stage, _, text)) = polled else {
            let (stage, text) = self.last.take()?;
            let failures = std::mem::take(&mut self.failures);
            return Some(format!(
                "revocation poller: recovered after {failures} failed poll(s); last error ({stage}): {text}"
            ));
        };
        self.failures += 1;
        if self
            .last
            .as_ref()
            .is_some_and(|(s, t)| *s == stage && *t == text)
        {
            return None;
        }
        let line = format!(
            "revocation poller: poll_once failed ({stage}): {text} (repeats counted until recovery)"
        );
        self.last = Some((stage, text));
        Some(line)
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
    use super::{pick_poll_cadence_from_flags, ErrorKind, PollFault, PollReport};
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

    /// AC7 (D-LRQC-09 A): an error-class or text change prints once, repeats are counted, and
    /// the recovery line names the last error; steady success prints nothing.
    #[test]
    fn poller_reports_an_error_class_change_once_and_counts_repeats() {
        let fault = |stage: &'static str, text: &str| -> Result<(), PollFault> {
            Err((stage, ErrorKind::App, text.to_string()))
        };
        let mut report = PollReport::default();
        let first = report
            .observe(fault("fetch", "no file"))
            .expect("first failure prints");
        assert!(first.contains("(fetch): no file"), "{first}");
        assert!(first.contains("repeats counted until recovery"), "{first}");
        assert!(report.observe(fault("fetch", "no file")).is_none());
        assert!(report.observe(fault("fetch", "no file")).is_none());
        let changed = report
            .observe(fault("parse", "bad json"))
            .expect("class change prints");
        assert!(changed.contains("(parse): bad json"), "{changed}");
        assert!(
            report.observe(fault("parse", "other")).is_some(),
            "text change prints"
        );
        assert!(report.observe(fault("parse", "other")).is_none());
        let recovered = report.observe(Ok(())).expect("recovery prints");
        assert!(
            recovered.contains("recovered after 6 failed poll(s)"),
            "{recovered}"
        );
        assert!(
            recovered.contains("last error (parse): other"),
            "{recovered}"
        );
        assert!(
            report.observe(Ok(())).is_none(),
            "steady success prints nothing"
        );
    }
}
