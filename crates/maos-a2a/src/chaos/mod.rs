//! NFR-Sec-13 mTLS cert rotation chaos + NFR-Rel-7 A2A churn scaffolding.
//!
//! Per architecture §7.2.1 + §7.2.1.a + §7.2.1.b. v0.5 ships in CALIBRATION
//! PHASE — metrics are MEASURED and REPORTED but NOT enforced. v0.7 flips
//! revocation propagation + re-handshake to hard-fail; v1.0 flips
//! `cert_post_grace_reject` ≤0.1%.
//!
//! **Calibration mode = report, not enforce.** Per
//! `[[feedback_lunarpulse_observability_preference]]`, the harness output IS
//! the observable evidence. The calibration window is bounded — the dev
//! record documents the v0.7 hard-fail flip date.

pub mod churn;
pub mod harness_3_host;
pub mod metrics;
pub mod report;
pub mod rotation;
