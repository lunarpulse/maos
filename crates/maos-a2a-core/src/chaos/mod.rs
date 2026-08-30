//! NFR-Sec-13 mTLS cert rotation + NFR-Rel-7 churn scaffolding.
//!
//! Per architecture §7.2.1 + §7.2.1.a + §7.2.1.b. The v0.5 CALIBRATION
//! window this module once advertised is CLOSED, and its promised v0.7
//! hard-fail flip date was **never written** — recorded here in plain words
//! because the previous doc pointed at a date that never existed (the
//! project passed v0.7 on both owning NFRs with nothing noticing; Story
//! 14-2 Blocking condition 6). Enforcement no longer lives here at all:
//! the real-socket drill (`crates/maos-a2a-tcp/tests/
//! t_10_4b_rotation_real_timing.rs`) measures on a live mesh, `rotation.rs`
//! holds the floors, and the `check-rotation-real-timing` gate owns
//! execution — skipped ≠ passed. The synthetic `harness_3_host` twin and
//! its `metrics` collector were DELETED by Story 14-2 (AC6.1): they seeded
//! timestamps from `DrillConfig` constants and asserted their own floors
//! green — 11.3's `run_scaffold` defect wearing rotation's hat.

pub mod churn;
pub mod report;
pub mod rotation;
