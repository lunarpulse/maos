#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 14-2a — THE RUNTIME LEG. The one that proves the deliverable.
//!
//! # Why this test exists in this shape
//!
//! The obvious gate for "a production caller exists" is an AST probe, and for
//! this story an AST probe is a SPELLING CHECK. The only such primitive in the
//! repo (`xtask/src/check_reza_production_path.rs:49-71`) implements
//! `visit_expr_call` only — free functions — while every call this story makes
//! is a method call, so copied as-is that leg is green from birth. Even with
//! `visit_expr_method_call` implemented it answers only *"does this file contain
//! a method call spelled `install_cert_rotation`"*: it stays green if the call
//! moves into a `#[cfg(test)]` block or into a function nothing reaches.
//!
//! So the CONTROL is this test:
//!
//! * a REAL `maos` child process in `MAOS_ONE_SHOT=cohort-a2a-daemon` mode,
//!   running the full production composition root;
//! * a REAL second mTLS endpoint in the test process, pinned to the daemon and
//!   pinned BY the daemon through its operator TOML;
//! * a REAL signed, version-monotonic `cohort:manifest-reissue` delivered over
//!   the wire by the production courier (`CohortDistributor::push_to`);
//! * the pin-set change OBSERVED from outside the process through the
//!   production operator HTTP surface, and the transitions read back out of the
//!   daemon's own Transparency Log.
//!
//! Nothing here is a re-implementation of the production path, so **deleting the
//! `rotation_state.install_cert_rotation(...)` call in `main.rs` reds this leg**
//! while the daemon still boots, still binds, still accepts the signed reissue
//! and still advances its manifest version. That mutation — not the deletion of
//! a line of text — is the proven-red vector for this story's deliverable.

#[path = "support/rotation_daemon_14_2a.rs"]
mod rotation_daemon;

use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind as TlFrameKind};
use rotation_daemon::{drive_a_signed_rotation, fixture_grace, operator_get};

/// Story 14-2a / AC1.1, AC1.2, AC1.5, AC2.1, AC2.2 — THE CONTROL: an operator
/// rotates a live daemon's trust in a peer's certificate by signing a new
/// cohort manifest, and the change is observable from outside the process.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_a_signed_reissue_rotates_a_live_daemons_peer_trust() {
    let rotation = drive_a_signed_rotation("live").await;

    assert!(
        rotation.windows.contains(&rotation.rotated.wire()),
        "the live daemon's open-window set must name the incoming generation: {}",
        rotation.windows
    );
    assert!(
        rotation
            .windows
            .contains(&rotation.fixture.host_b.fingerprint.wire()),
        "and the generation still serving, so the overlap is visible: {}",
        rotation.windows
    );
    // AC2.2 — plane A, not just plane B. `declared` is read from the LIVE router
    // core: a rotation wired to a detached core would still open and still
    // report this window, and only this assertion notices.
    assert!(
        rotation
            .windows
            .contains(&format!(r#""declared":"{}""#, rotation.rotated.wire())),
        "the live router must DECLARE the incoming generation, not merely accept it: {}",
        rotation.windows
    );
    assert!(
        rotation.windows.contains(r#""state":"open""#),
        "and the row must be reported as an OPEN overlap: {}",
        rotation.windows
    );
}

/// Story 14-2a / AC4.3, AC4.5, AC4.6(b) — the rotation timeline round-trips
/// through the REAL audit surface under one stable, greppable intent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_rotation_rows_round_trip_through_the_real_audit_surface() {
    let rotation = drive_a_signed_rotation("audit").await;
    let fixture = &rotation.fixture;
    let rotated = &rotation.rotated;

    // ── 5. And every transition is a row an auditor can query back ──────────
    let log = maos_iac::adapter::TransparencyLogAdapter::open(&fixture.audit_db, 0)
        .expect("the daemon's own Transparency Log opens for reading");
    let rows = log
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::TelemetryEvent),
            ..Default::default()
        })
        .expect("the Transparency Log is queryable");
    let rotation_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.intent == maos_cohort::CERT_ROTATION_INTENT)
        .collect();
    assert!(
        !rotation_rows.is_empty(),
        "the rotation timeline must be queryable under the stable intent \
         `{}`; TelemetryEvent intents present: {:?}",
        maos_cohort::CERT_ROTATION_INTENT,
        rows.iter().map(|row| &row.intent).collect::<Vec<_>>()
    );
    let opened = rotation_rows
        .iter()
        .map(|row| String::from_utf8_lossy(&row.payload_redacted).to_string())
        .find(|payload| payload.contains("cert_rotation_window_opened"))
        .unwrap_or_else(|| {
            panic!(
                "no window-opened row: {:?}",
                rotation_rows
                    .iter()
                    .map(|row| String::from_utf8_lossy(&row.payload_redacted).to_string())
                    .collect::<Vec<_>>()
            )
        });

    // ⚠ MEASURED: the I2 redaction filter rewrites any hex run of ≥32 chars as
    // `<REDACTED:type=capability_token,…>`, so a full 64-hex fingerprint never
    // lands on disk — not in THIS row and not in the shipped
    // `member_reissue_accepted` row either. What survives, and what an auditor
    // therefore gets, is asserted here explicitly rather than wished away: the
    // peer, the version, per-value redaction stamps that stay DISTINCT, and the
    // 8-hex correlation prefix of each generation.
    assert!(
        opened.contains(&format!(
            r#""retiring_short":"{}""#,
            &fixture.host_b.fingerprint.hex[..8]
        )) && opened.contains(&format!(r#""next_short":"{}""#, &rotated.hex[..8])),
        "the row must carry a correlation prefix for BOTH generations: {opened}"
    );
    let stamps: Vec<&str> = opened.match_indices("hash=").map(|(_, m)| m).collect();
    assert_eq!(
        stamps.len(),
        2,
        "both fingerprints are redacted, and both stamps are present: {opened}"
    );
    let retiring_stamp = opened
        .split("\"retiring\":")
        .nth(1)
        .and_then(|rest| rest.split("hash=").nth(1))
        .and_then(|rest| rest.split('>').next())
        .unwrap_or_default();
    let next_stamp = opened
        .split("\"next\":")
        .nth(1)
        .and_then(|rest| rest.split("hash=").nth(1))
        .and_then(|rest| rest.split('>').next())
        .unwrap_or_default();
    assert_ne!(
        retiring_stamp, next_stamp,
        "a redacted row that could not distinguish the two generations would be \
         an audit surface in name only: {opened}"
    );
    println!("\nROTATION_AUDIT_ROWS={}", rotation_rows.len());
}

/// Story 14-2a / AC3.2 — the PRODUCTION grace timer, not the test fixture.
///
/// Every other leg installs a manual timer and fires it by hand, so a
/// `TokioGraceTimer::schedule` that did nothing at all would leave them all
/// green and leave every widened trust set open forever. This drives the real
/// implementation. It uses a short injected grace rather than Tokio's paused
/// clock because `test-util` is not enabled on this crate and this story does
/// not change its dependencies. The margins are sized for a loaded CI runner:
/// a 2 s injected grace with a CONTINUOUS 250 ms early-fire guard (a stall can
/// only end the guard, never fail it — the old 20 ms point check against a
/// 200 ms deadline false-red the moment a stall outlived the deadline), and a
/// 15 s eventual-fire budget that scales with the grace. Bounded and
/// deterministic: the leg can never run longer than ~15.8 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_the_production_grace_timer_fires_its_terminal_action_exactly_once() {
    use maos_cohort::rotation::RotationGraceTimer;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let fired = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&fired);
    maos_bin::cert_rotation::TokioGraceTimer.schedule(
        Duration::from_secs(2),
        Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }),
    );

    // Early-fire, asserted CONTINUOUSLY for the first 250 ms instead of at a
    // single instant: if the runner stalls past the injected grace, the guard
    // simply ends — a correct timer can no longer be made to look like an
    // early fire, but an early-firing timer is still caught on every sample
    // the runner actually executes.
    let early_deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < early_deadline {
        assert_eq!(
            fired.load(Ordering::SeqCst),
            0,
            "the terminal action must not run before the grace elapses — the overlap is what \
             keeps a mid-rotation mesh reachable"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Eventual-fire, on a budget that scales with the injected grace.
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && fired.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fired.load(Ordering::SeqCst),
        1,
        "and it must run EXACTLY once after it: a no-op scheduler leaves the trust set \
         widened forever, and a double-fire would double-promote"
    );
    // Belt: a further wait must not produce a second firing.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(fired.load(Ordering::SeqCst), 1);
    assert!(
        maos_cohort::cold_deployment_t_grace() >= Duration::from_secs(5),
        "§7.2.1.a floors the PRODUCTION T_grace at 5 s on either branch"
    );
}

/// Story 14-2a / AC3.2 — the INSTALLED production closer, not the timer type.
///
/// The test above drives `TokioGraceTimer::schedule` directly, so it proves
/// the timer TYPE fires while saying nothing about what `main.rs` actually
/// INSTALLS at the composition root. This closes that gap end to end against
/// a real daemon: the signed reissue opens a window on the live pin store,
/// the composition root's own
/// `rotation_state.install_cert_rotation(.., TokioGraceTimer,
/// cold_deployment_t_grace())` arms the production closer, and after the
/// five-second grace the window must be GONE from the operator surface and a
/// `cert_rotation_window_closed` row must be queryable from the daemon's own
/// Transparency Log.
///
/// Proven-red vectors, at the install rather than in isolation: a NO-OP timer
/// leaves the window open forever and reds on the close deadline; an EARLY or
/// wrong-grace closer reds on the elapsed-time floor. Timing margins are
/// stall-proof by construction: the close is measured from the reissue ACK,
/// whose own delay also delays the closer (both live in the daemon), so a
/// runner stall can only PUSH the observed close later — it can never shrink
/// the measured gap below the production grace minus a fixed 1.5 s slop for
/// observation granularity. The close deadline is bounded at
/// `T_grace + 45 s` and the audit row gets its own bounded wait, because the
/// closer removes the ledger row BEFORE it appends the terminal row.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_the_installed_production_closer_closes_the_window_and_journals_the_close() {
    let rotation = drive_a_signed_rotation("installed-closer").await;
    let grace = fixture_grace();

    // ── 5. The window must survive until the grace and be gone after it ─────
    let mut seen = rotation.windows.clone();
    let mut closed_observed_at: Option<Instant> = None;
    let deadline = rotation.acked_at + grace + Duration::from_secs(45);
    while closed_observed_at.is_none() {
        assert!(
            Instant::now() < deadline,
            "the window must close within T_grace + 45 s of the reissue ACK; the operator \
             surface still reports {seen}\n(a no-op timer installed at the composition root \
             leaves every widened trust set open forever)"
        );
        seen = operator_get(rotation.operator_port, "/v1/a2a/rotation-windows");
        if seen.contains(r#""open_windows":[]"#) {
            closed_observed_at = Some(Instant::now());
        } else {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    let closed_observed_at = closed_observed_at.expect("the loop only exits with a timestamp");
    let observed_gap = closed_observed_at.saturating_duration_since(rotation.acked_at);
    assert!(
        observed_gap + Duration::from_millis(1500) >= grace,
        "the window closed {observed_gap:?} after the reissue ACK (first seen open \
         {:?} after the ACK) but the production grace is {grace:?}: something fired the \
         closer EARLY (a wrong timer or wrong grace at the install)",
        rotation
            .opened_observed_at
            .saturating_duration_since(rotation.acked_at)
    );

    // ── 6. And the close is a row in the daemon's own Transparency Log ──────
    // ⚠ MEASURED (see the audit leg above): the I2 redaction filter rewrites
    // every ≥32-char hex run, so what an auditor correlates on is the 8-hex
    // prefix each row carries — `retired_short` (the generation that was
    // serving) and `promoted_short` (the generation the closer moved in).
    let retired_short = rotation.fixture.host_b.fingerprint.hex[..8].to_string();
    let promoted_short = rotation.rotated.hex[..8].to_string();
    let mut found_closed_row = false;
    let audit_deadline = Instant::now() + Duration::from_secs(20);
    while !found_closed_row {
        assert!(
            Instant::now() < audit_deadline,
            "no `cert_rotation_window_closed` row for host-b under intent {} within 20 s of \
             the close; an audit surface that reports a window's start but never its end is \
             the AC4.3 failure",
            maos_cohort::CERT_ROTATION_INTENT
        );
        let log = maos_iac::adapter::TransparencyLogAdapter::open(&rotation.fixture.audit_db, 0)
            .expect("the daemon's own Transparency Log opens for reading");
        let rows = log
            .query_frames(FrameFilter {
                kind: Some(TlFrameKind::TelemetryEvent),
                ..Default::default()
            })
            .expect("the Transparency Log is queryable");
        let closes: Vec<String> = rows
            .iter()
            .filter(|row| row.intent == maos_cohort::CERT_ROTATION_INTENT)
            .map(|row| String::from_utf8_lossy(&row.payload_redacted).to_string())
            .filter(|payload| payload.contains(r#""event":"cert_rotation_window_closed""#))
            .collect();
        let matching: Vec<&String> = closes
            .iter()
            .filter(|payload| {
                payload.contains(r#""peer":"host-b""#)
                    && payload.contains(r#""version":2"#)
                    && payload.contains(&format!(r#""retired_short":"{retired_short}""#))
                    && payload.contains(&format!(r#""promoted_short":"{promoted_short}""#))
            })
            .collect();
        match matching.len() {
            1 => found_closed_row = true,
            0 => tokio::time::sleep(Duration::from_millis(250)).await,
            n => panic!(
                "exactly ONE terminal row is written per opened window, found {n}: {matching:?}"
            ),
        }
    }

    // The marker is newline-prefixed for the same libtest `--nocapture` reason
    // as ROTATION_WINDOWS_OBSERVED above: the gate matches it as an exact
    // trimmed line, so an early return that skips the close cannot go green.
    println!("\nROTATION_WINDOW_CLOSED_OBSERVED=1");
}
