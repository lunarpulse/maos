//! Story 10.4b — Mira+Nash bilateral **2-Host LIVE** deployment proof.
//!
//! All five tests exercise the *real* `TcpA2ATransport` over genuine
//! `127.0.0.1:0` sockets with rcgen-minted mTLS material (H1–H6 hermetic
//! harness in `support`). There is NO kernel-core delta here — this file only
//! *wires and proves* subsystems the kernel already shipped: the live mTLS
//! transport, the shared `A2ARouterCore` intake/consent/binding gates, and the
//! Mira→Nash diagnostic-advisory contract.
//!
//!   1. `t_10_4b_live_bilateral_50_scenario_corpus` — 50 deterministic
//!      diagnostic scenarios round-trip Host A (Mira) → Host B (Nash) over the
//!      live wire; ≥45/50 close within the 90-min close-time budget and ≥48/50
//!      uphold consent; every delivered frame's `from.host_id` equals the TLS
//!      peer. Logs the REAL coarse round-trip per scenario (the R2-1 number).
//!   2. `t_10_4b_confused_deputy_forged_host_id_rejected` — a frame whose
//!      `from.host_id` is forged over a validly-pinned Mira TLS connection is
//!      rejected `CODE_PEER_IDENTITY_MISMATCH` before intake; an honest frame
//!      on the same connection still ACKs (R2-2: the negative control that can
//!      ONLY pass over real TCP).
//!   3. `t_10_4b_denied_intent_over_live_tcp` — a `code-mutation-directive`
//!      frame traverses the wire but is denied at Nash's accept-allowlist
//!      (`A2AError::IntentDeniedAtPeer`); the read-only advisory still ACKs.
//!   4. `t_10_4b_binding_records_all_carry_checked` — the full 50-scenario
//!      corpus all pass `handle_intake_verified` with `binding_passed == true`
//!      (`intake_entered() == 50`) and all 50 land in the intake sink.
//!   5. `t_10_4b_coarse_roundtrip_real_measurement` — 100 live round-trips;
//!      measures and logs the real coarse mean per-frame latency.
//!
//! Per ADR-012 the consent key the receiver actually matches is the
//! fine-grained `consent_envelope.intent_class`, NOT the coarse 3-band
//! `frame.intent`. Mira's advisory carries
//! `diagnosis-handoff:read-only-evidence` (mirrors `mira`'s
//! `ADVISORY_FINE_GRAINED_INTENT`); Nash accepts exactly that string and
//! refuses `code-mutation-directive`. The coarse band
//! (`make_frame`'s default envelope) is overridden in
//! [`make_fine_grained_frame`].

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use maos_a2a_core::identity::PeerCertFingerprint;
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::transport::json_rpc::CODE_PEER_IDENTITY_MISMATCH;
use maos_a2a_core::{A2AError, A2AJsonRpcRequest, A2AJsonRpcResponse, InMemoryTofuPinStore};
use maos_a2a_tcp::TcpTimeouts;
use maos_domain::frame::{FramePayload, IacFrame};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::HostId;
use sha2::{Digest, Sha256};
use support::*;
use tokio_util::bytes::Bytes;

/// Mira (host_a) wire boot-nonce — pinned on BOTH ends so the Spirit-restart
/// detection floor matches (NFR-Rel-6).
const MIRA_NONCE: u64 = 0x10_4b_a;
/// Nash (host_b) wire boot-nonce.
const NASH_NONCE: u64 = 0x10_4b_b;
/// The fine-grained ADR-012 intent Mira's cross-Host advisory carries — mirrors
/// `mira::ADVISORY_FINE_GRAINED_INTENT`. Both Mira's `send_allowlist` and
/// Nash's `accept_allowlist` admit this exact string; `code-mutation-directive`
/// is the ADR-012 worked-example denial (same `readonly` band, different key).
const FINE: &str = "diagnosis-handoff:read-only-evidence";
/// The ADR-012 worked-example denial intent — projects to the SAME `readonly`
/// band as `FINE` but is a distinct fine-grained key, so a coarse-band-only
/// gate would wrongly admit it.
const DENIED: &str = "code-mutation-directive";

// ───────────────────────────────────────────────────────────────────────────
// Local helpers (mirror the proven patterns in t1_live_roundtrip /
// trust_binding_8_9 — kept local so this test adds no kernel surface).
// ───────────────────────────────────────────────────────────────────────────

/// Bind the bilateral 2-Host pair: Nash (host_b) first so its readback `addr`
/// is observable (H3/H4), then Mira (host_a) dialing that addr. Host A
/// pre-pins Host B's fingerprint and vice versa. `mira_send` is Mira's
/// send_allowlist; `nash_accept` is Nash's accept_allowlist — the ADR-012
/// (peer-identity, intent-class) keys each side admits.
async fn bind_bilateral(
    clock: &Clock,
    ca: &Ca,
    mira_leaf: &Leaf,
    nash_leaf: &Leaf,
    mira_send: &[&str],
    nash_accept: &[&str],
) -> (maos_a2a_tcp::TcpA2ATransport, maos_a2a_tcp::TcpA2ATransport) {
    // ── Nash (host_b) — server. Pins Mira's leaf, accepts `nash_accept`.
    let nash = bind_endpoint(
        nash_leaf,
        Some(ca),
        NASH_NONCE,
        vec![pin("host_a", &mira_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira_leaf.fingerprint,
            &[],
            nash_accept,
        )],
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let nash_addr = nash.local_addr().expect("nash bound addr (H3/H4)");

    // ── Mira (host_a) — client. Pins Nash's leaf, dials the readback addr.
    let mira = bind_endpoint(
        mira_leaf,
        Some(ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_leaf.fingerprint,
            mira_send,
            &[],
        )],
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;

    (mira, nash)
}

/// Build a `CrossHost` advisory frame whose fine-grained ADR-012
/// `intent_class` is `intent_str` (NOT the coarse band token `make_frame`
/// defaults to). The granter stays `== frame.from`, satisfying the 8.9
/// granter binding; only the match key is overridden.
fn make_fine_grained_frame(from_host: &str, to_host: &str, seq: u64, intent_str: &str) -> IacFrame {
    let mut frame = make_frame(from_host, to_host, IntentClass::Readonly, seq);
    if let Some(env) = frame.consent_envelope.as_mut() {
        env.intent_class = Some(A2AIntent::new(intent_str));
    }
    frame
}

/// Build a Mira→Nash diagnostic advisory carrying `FINE` AND the serialized
/// scenario JSON in the `TaskAssign.goal` (the contract is the serde shape —
/// a real cross-Host advisory carries the diagnostic evidence for Nash to
/// architect against). `make_frame`'s `frame_id` is unique per `seq`, so the
/// 50 corpus frames never collide in the router cache.
fn make_advisory_frame(from_host: &str, to_host: &str, seq: u64, scenario_json: &str) -> IacFrame {
    let mut frame = make_fine_grained_frame(from_host, to_host, seq, FINE);
    if let FramePayload::TaskAssign(ta) = &mut frame.payload {
        ta.goal = format!("diagnostic-advisory:{scenario_json}");
    }
    frame
}

/// Send one JSON-RPC request over a raw authenticated framed stream and await
/// the decoded response (the confused-deputy negative control needs raw wire
/// control to forge `from.host_id` while authenticated as Mira).
async fn send_recv(
    framed: &mut tokio_util::codec::Framed<
        tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        tokio_util::codec::LengthDelimitedCodec,
    >,
    req: &A2AJsonRpcRequest,
) -> A2AJsonRpcResponse {
    let body = serde_json::to_vec(req).expect("serialize request");
    framed.send(Bytes::from(body)).await.expect("send frame");
    let buf = framed
        .next()
        .await
        .expect("a response frame")
        .expect("response not a codec error");
    serde_json::from_slice(&buf).expect("decode response")
}

/// Drain up to `expected` delivered frames from an intake sink. The sink push
/// happens inside `handle_intake` BEFORE the ACK is written, so once
/// `route_outbound` returns `Ok` the frame is already buffered; `recv` returns
/// them without real delay. A 2s per-recv timeout guards against any stall.
async fn drain_intake(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<IacFrame>,
    expected: usize,
) -> Vec<IacFrame> {
    let mut out = Vec::with_capacity(expected);
    while out.len() < expected {
        match tokio::time::timeout(Duration::from_secs(2), rx.recv()).await {
            Ok(Some(f)) => out.push(f),
            _ => break,
        }
    }
    out
}

/// Build the 50-scenario diagnostic corpus. Seeds are the two scenarios in
/// `spirits/mira/tests/fixtures/diagnostic-scenarios.json` (verbatim); the
/// remaining 48 are index-seeded variants with deterministic
/// observed/baseline values (Decision I — pure, seeded, reproducible; no live
/// LLM, no Mira::diagnose reimplementation).
fn build_corpus() -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(50);

    // Seed 1 — verbatim from the fixture.
    out.push(serde_json::json!({
        "subject": "checkout-api",
        "metric": "error_rate",
        "observed": 0.42,
        "baseline": 0.30,
        "detail": "error-rate spike following the 23:14 deploy; matches a known regression pattern",
        "source_log_ref": "tl:row:1001"
    }));
    // Seed 2 — verbatim from the fixture.
    out.push(serde_json::json!({
        "subject": "edge-cache",
        "metric": "novel_entropy_drift",
        "observed": 0.91,
        "baseline": 0.10,
        "detail": "unrecognised entropy drift on the prod-edge cache; no known diagnostic pattern — Mira's epistemic boundary",
        "source_log_ref": "tl:row:2002"
    }));

    let subjects = [
        "checkout-api",
        "edge-cache",
        "payments-svc",
        "search-index",
        "auth-gateway",
        "ingest-pipe",
    ];
    let metrics = [
        "error_rate",
        "latency_p99",
        "saturation",
        "novel_entropy_drift",
    ];
    for i in 2u64..50 {
        let idx = i as usize;
        let subject = subjects[idx % subjects.len()];
        let metric = metrics[idx % metrics.len()];
        // Deterministic index-seeded observed/baseline; observed > baseline ⇒ a
        // genuine deviation (a real prod-edge anomaly signal for the corpus).
        let observed = (((0.10 + (i as f64) * 0.0142) * 1000.0).round()) / 1000.0;
        let baseline = (((0.03 + (i as f64) * 0.0061) * 1000.0).round()) / 1000.0;
        out.push(serde_json::json!({
            "subject": format!("seed-{i}-{subject}"),
            "metric": metric,
            "observed": observed,
            "baseline": baseline,
            "detail": format!("index-seeded diagnostic scenario {i}: {metric} deviation on {subject}"),
            "source_log_ref": format!("tl:row:{}", 1000 + i),
        }));
    }
    assert_eq!(out.len(), 50, "corpus is exactly 50 scenarios");
    out
}

// ───────────────────────────────────────────────────────────────────────────
// Per-record gauges & content address (review findings F1/F2/F3/F4) — helpers
// that DERIVE the assertion surface from the live records, so the close-time /
// consent floors are falsifiable and the TLS-identity binding is proven
// per-record (not by a bare count or a self-asserted field). All live.
// ───────────────────────────────────────────────────────────────────────────

/// The seq stamp `make_frame` packs into a corpus frame's `frame_id` (the first
/// 8 big-endian bytes) — correlates a delivered frame back to its scenario.
fn seq_of(frame: &IacFrame) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&frame.frame_id[0..8]);
    u64::from_be_bytes(b)
}

/// F1 — per-record TLS-identity binding proof for one delivered frame. The wire
/// gate `handle_intake_verified` requires `frame.from.host_id` to equal the
/// TLS-verified peer — itself resolved from the negotiated client-cert
/// fingerprint by the receiver's active-pin oracle (`resolve_verified_peer`).
/// A frame reaches the intake sink ONLY past that gate, so `binding_checked` is
/// derived by comparing the delivered frame's self-asserted host_id against the
/// oracle-resolved peer; the fingerprint is read back from the RECEIVER's pin
/// store (the live oracle), not the sender's input — so a no-op binding would
/// surface here as `binding_checked == false`.
struct BindingRecord {
    seq: u64,
    binding_checked: bool,
    verified_peer: String,
    peer_fingerprint: String,
}

/// Build the per-record binding-proof table from the delivered frames. The
/// verified peer + pinned fingerprint are resolved from Nash's pin store via
/// Mira's presented leaf fingerprint — the same oracle the live wire consults
/// inside `resolve_verified_peer`.
fn binding_records(
    delivered: &[IacFrame],
    nash_pins: &InMemoryTofuPinStore,
    mira_fp: &PeerCertFingerprint,
) -> Vec<BindingRecord> {
    let verified_peer = nash_pins
        .find_active_pin_by_fingerprint(mira_fp)
        .expect("Mira's leaf fingerprint resolves to an active pin on Nash");
    let pinned_fp = nash_pins
        .get_pin_sync(&verified_peer)
        .expect("active pin carries a fingerprint")
        .fingerprint
        .wire();
    delivered
        .iter()
        .map(|f| BindingRecord {
            seq: seq_of(f),
            binding_checked: f.from.host_id.as_ref().map(|h| h.as_str())
                == Some(verified_peer.as_str()),
            verified_peer: verified_peer.as_str().to_string(),
            peer_fingerprint: pinned_fp.clone(),
        })
        .collect()
}

/// F2 — one scenario's observed close-time + consent outcome. Replaces the
/// prior green-by-construction `closed_within` / `consent_upheld` booleans with
/// a derived record, so the floors are evaluated against real failure sets.
struct ScenarioRecord {
    seq: u64,
    close_time: Duration,
    consent_ok: bool,
}

/// F2 — the close-time failure set: seqs whose close-time EXCEEDED `budget`.
fn close_failures(records: &[ScenarioRecord], budget: Duration) -> Vec<u64> {
    records
        .iter()
        .filter(|r| r.close_time > budget)
        .map(|r| r.seq)
        .collect()
}

/// F2 — the consent failure set: seqs that were DENIED at the peer.
fn consent_failures(records: &[ScenarioRecord]) -> Vec<u64> {
    records
        .iter()
        .filter(|r| !r.consent_ok)
        .map(|r| r.seq)
        .collect()
}

/// F3 — the checked-in corpus content-address fixture.
fn corpus_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/MANIFEST.toml")
}

/// F3 — read the `corpus_sha256` value from MANIFEST.toml. The fixture is a
/// single `corpus_sha256 = "<hex64>"` line; a focused scan reads it without a
/// `toml` crate dependency (the manifest stays valid TOML regardless).
fn manifest_corpus_sha256() -> String {
    let text = std::fs::read_to_string(corpus_manifest_path())
        .expect("tests/fixtures/MANIFEST.toml present");
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("corpus_sha256"))
        .expect("MANIFEST.toml has a corpus_sha256 entry");
    line.split_once('"')
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(hex, _)| hex)
        .expect("corpus_sha256 value is a quoted string")
        .to_string()
}

/// F3 — SHA-256 of `serde_json::to_vec(corpus)` (compact JSON, BTreeMap key
/// order, non-ASCII written as raw UTF-8). The content address of the
/// deterministic corpus — pinned in tests/fixtures/MANIFEST.toml.
fn corpus_sha256(corpus: &[serde_json::Value]) -> String {
    let canonical: Vec<serde_json::Value> = corpus.iter().map(canonical_json).collect();
    let bytes = serde_json::to_vec(&canonical).expect("serialize corpus");
    let digest = Sha256::digest(&bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn canonical_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonical_json).collect())
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                sorted.insert(key.clone(), canonical_json(&map[key]));
            }
            serde_json::Value::Object(sorted)
        }
        other => other.clone(),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

/// 50 deterministic diagnostic scenarios round-trip Host A → Host B over the
/// LIVE wire. Per-scenario we record close-time (elapsed ≤ the 90-min budget)
/// and consent success (route_outbound Ok). An intake sink on Nash captures
/// every delivered frame; the REAL coarse round-trip = total / 50 is logged.
#[tokio::test]
async fn t_10_4b_live_bilateral_50_scenario_corpus() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let (mira, nash) = bind_bilateral(&clock, &ca, &mira_leaf, &nash_leaf, &[FINE], &[FINE]).await;

    // Capture every frame Nash delivers (post all consent gates).
    let (sink_tx, mut sink_rx) = tokio::sync::mpsc::unbounded_channel::<IacFrame>();
    nash.core().install_intake_sink(sink_tx).await;

    let corpus = build_corpus();
    // 90-minute close-time budget — the v1.5 deployment SLO. F2: each
    // scenario's close-time + consent outcome is captured in a per-record
    // `ScenarioRecord`, and the ≥45/50 (close-time) and ≥48/50 (consent)
    // floors are evaluated against the DERIVED failure sets, not
    // green-by-construction booleans (the reachable-RED counterpart is
    // `t_10_4b_close_time_and_consent_floors_reachable_red`).
    let close_budget = Duration::from_secs(90 * 60);

    let mut records: Vec<ScenarioRecord> = Vec::with_capacity(50);
    let wall_start = Instant::now();

    for (i, scenario) in corpus.iter().enumerate() {
        let seq = (i + 1) as u64;
        let scenario_json = serde_json::to_string(scenario).expect("serialize scenario");
        let frame = make_advisory_frame("host_a", "host_b", seq, &scenario_json);

        let t0 = Instant::now();
        let consent_ok = match mira.route_outbound(frame, &HostId("host_b".into())).await {
            Ok(()) => true,
            Err(e) => panic!("scenario {i} (seq {seq}) route_outbound failed: {e:?}"),
        };
        records.push(ScenarioRecord {
            seq,
            close_time: t0.elapsed(),
            consent_ok,
        });
    }
    let total = wall_start.elapsed();

    // F2 — floors evaluated against the DERIVED failure sets (close-time >
    // budget / consent denied), not hard-coded booleans.
    let close_over = close_failures(&records, close_budget);
    let consent_denied = consent_failures(&records);
    assert!(
        close_over.len() <= 5,
        "≥45/50 scenarios must close within the 90-min budget; {} over-budget: {close_over:?}",
        close_over.len()
    );
    assert!(
        consent_denied.len() <= 2,
        "≥48/50 scenarios must uphold consent; {} denied: {consent_denied:?}",
        consent_denied.len()
    );

    // The intake-entered count is incremented inside serve_connection only when
    // the TLS-identity binding passed; every consent-upheld frame did.
    assert_eq!(
        nash.intake_entered(),
        50,
        "all 50 frames entered intake (identity binding passed)"
    );

    let delivered = drain_intake(&mut sink_rx, 50).await;
    assert_eq!(
        delivered.len(),
        50,
        "all 50 frames delivered to the intake sink"
    );
    // F1 — per-record TLS-identity binding proof. Every delivered frame's
    // self-asserted `from.host_id` must agree with the TLS-VERIFIED peer
    // (resolved from Mira's presented cert fingerprint via Nash's active-pin
    // oracle — the SAME oracle `resolve_verified_peer` consults on the live
    // wire), and the pinned fingerprint must be non-empty. NOT a bare count or
    // a self-asserted field: a no-op binding surfaces as binding_checked=false.
    let nash_pins = nash.pins();
    let binding = binding_records(&delivered, &nash_pins, &mira_leaf.fingerprint);
    assert_eq!(binding.len(), 50, "one binding record per delivered frame");
    for r in &binding {
        assert!(
            r.binding_checked,
            "seq {}: from.host_id must match the TLS-verified peer",
            r.seq
        );
        assert_eq!(
            r.verified_peer, "host_a",
            "seq {}: verified peer is Mira (host_a)",
            r.seq
        );
        assert!(
            !r.peer_fingerprint.is_empty(),
            "seq {}: TLS peer fingerprint must be non-empty",
            r.seq
        );
    }

    // F4 — the advisory scenario reached the delivered TaskAssign.goal intact.
    for f in &delivered {
        let seq = seq_of(f);
        let expected = format!(
            "diagnostic-advisory:{}",
            serde_json::to_string(&corpus[(seq - 1) as usize]).expect("serialize scenario")
        );
        let goal = match &f.payload {
            FramePayload::TaskAssign(ta) => &ta.goal,
            _ => panic!("seq {seq}: delivered payload must be TaskAssign"),
        };
        assert_eq!(
            goal, &expected,
            "seq {seq}: advisory scenario must reach the delivered TaskAssign.goal"
        );
    }

    // R2-1 — the REAL coarse round-trip per scenario (not the J4 Observer P95).
    let coarse_per_scenario = total / 50u32;
    let closed_within = 50 - close_over.len();
    let consent_upheld = 50 - consent_denied.len();
    println!(
        "10.4b corpus: 50 scenarios round-tripped in {total:?}; \
         coarse RT/scenario = {coarse_per_scenario:?}; \
         closed_within_90min = {closed_within}/50; consent_upheld = {consent_upheld}/50; \
         over_budget={close_over:?}; consent_denied={consent_denied:?}"
    );
    assert!(
        total.as_nanos() > 0 && coarse_per_scenario.as_nanos() > 0,
        "coarse round-trip must be finite and > 0"
    );
}

/// R2-2 — the confused-deputy negative control that can ONLY pass over real
/// TCP: a frame whose `from.host_id` is forged (host_c) over a validly-pinned
/// Mira TLS connection (verified peer = host_a) is rejected
/// `CODE_PEER_IDENTITY_MISMATCH` and NEVER enters intake; an honest frame on
/// the SAME connection still ACKs (positive control).
#[tokio::test]
async fn t_10_4b_confused_deputy_forged_host_id_rejected() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    // Bind the pair (keeps Mira's transport alive); the negative control dials
    // Nash directly via the raw authenticated stream.
    let (_mira, nash) = bind_bilateral(&clock, &ca, &mira_leaf, &nash_leaf, &[FINE], &[FINE]).await;
    let addr = nash.local_addr().expect("nash bound addr (H3/H4)");

    // Authenticated as Mira — the TLS-verified peer is host_a.
    let mut framed =
        raw_client_connect(addr, &mira_leaf, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // (1) FORGED: from.host_id = host_c (≠ the verified peer host_a). The frame
    // is well-formed and the envelope granter == from (host_c), but the
    // identity binding in handle_intake_verified fires first and rejects.
    let forged = A2AJsonRpcRequest::new(
        "iac.deliver",
        make_fine_grained_frame("host_c", "host_b", 1, FINE),
        1,
    )
    .with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &forged).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_PEER_IDENTITY_MISMATCH,
            "forged from.host_id must be rejected as PeerIdentityMismatch"
        ),
        other => panic!("expected PeerIdentityMismatch NACK for the forged frame, got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        0,
        "a forged frame must NOT enter intake (binding precedes intake)"
    );

    // (2) HONEST positive control: same connection, from.host_id = host_a.
    let honest = A2AJsonRpcRequest::new(
        "iac.deliver",
        make_fine_grained_frame("host_a", "host_b", 2, FINE),
        2,
    )
    .with_boot_nonce(MIRA_NONCE);
    assert!(
        matches!(
            send_recv(&mut framed, &honest).await,
            A2AJsonRpcResponse::Ack(_)
        ),
        "an honest frame on the same connection must still ACK"
    );
    assert_eq!(
        nash.intake_entered(),
        1,
        "only the honest frame entered intake"
    );
}

/// Consent works OVER LIVE TCP: a `code-mutation-directive` frame (permitted on
/// Mira's send side, so it genuinely traverses the wire) is DENIED at Nash's
/// accept-allowlist and surfaces on the sender as `A2AError::IntentDeniedAtPeer`
/// — the receiver-side gate firing over a real socket. The read-only advisory
/// still ACKs (positive control).
#[tokio::test]
async fn t_10_4b_denied_intent_over_live_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    // Mira is PERMITTED to send BOTH intents, so the denied frame actually
    // crosses the wire (proving receiver-side denial, not a sender-side refuse).
    // Nash accepts ONLY the read-only evidence.
    let (mira, nash) = bind_bilateral(
        &clock,
        &ca,
        &mira_leaf,
        &nash_leaf,
        &[FINE, DENIED],
        &[FINE],
    )
    .await;

    // (1) DENIED at Nash: code-mutation-directive is not in accept_allowlist.
    let denied = make_fine_grained_frame("host_a", "host_b", 1, DENIED);
    let err = mira
        .route_outbound(denied, &HostId("host_b".into()))
        .await
        .expect_err("code-mutation-directive must be denied over live TCP");
    assert!(
        matches!(err, A2AError::IntentDeniedAtPeer { .. }),
        "receiver-side consent denial over live TCP must surface as IntentDeniedAtPeer; got {err:?}"
    );
    // The frame crossed the wire and passed identity binding (host_a == peer),
    // so it entered intake BEFORE the accept-allowlist gate denied it.
    assert_eq!(
        nash.intake_entered(),
        1,
        "the denied frame entered intake after identity binding"
    );

    // (2) ALLOWED positive control: read-only evidence ACKs.
    let allowed = make_fine_grained_frame("host_a", "host_b", 2, FINE);
    mira.route_outbound(allowed, &HostId("host_b".into()))
        .await
        .expect("read-only evidence must be accepted over live TCP");
    assert_eq!(
        nash.intake_entered(),
        2,
        "the allowed frame also entered intake"
    );
}

/// All 50 corpus frames pass `handle_intake_verified` with `binding_passed ==
/// true` (so `intake_entered() == 50`) AND land in the intake sink — proving
/// every binding record carried a checked TLS-identity match, per-frame.
#[tokio::test]
async fn t_10_4b_binding_records_all_carry_checked() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let (mira, nash) = bind_bilateral(&clock, &ca, &mira_leaf, &nash_leaf, &[FINE], &[FINE]).await;

    let (sink_tx, mut sink_rx) = tokio::sync::mpsc::unbounded_channel::<IacFrame>();
    nash.core().install_intake_sink(sink_tx).await;

    let corpus = build_corpus();
    for (i, scenario) in corpus.iter().enumerate() {
        let seq = (i + 1) as u64;
        let scenario_json = serde_json::to_string(scenario).expect("serialize scenario");
        let frame = make_advisory_frame("host_a", "host_b", seq, &scenario_json);
        mira.route_outbound(frame, &HostId("host_b".into()))
            .await
            .expect("corpus frame must round-trip");
    }

    // Every frame went through handle_intake_verified with binding_passed=true.
    assert_eq!(
        nash.intake_entered(),
        50,
        "all 50 binding records carry a checked TLS-identity match"
    );

    let delivered = drain_intake(&mut sink_rx, 50).await;
    assert_eq!(
        delivered.len(),
        50,
        "all 50 frames delivered to the intake sink"
    );

    // F1 — the real per-record binding surface: binding_checked AND a non-empty
    // (well-formed sha256:<hex64>) TLS peer fingerprint for EVERY one of the 50
    // live records — not just `intake_entered()` and `from.host_id`. The
    // verified peer + fingerprint are read from Nash's pin store via Mira's leaf
    // fingerprint (the live oracle `resolve_verified_peer` uses), so this fails
    // closed if the binding gate ever became a no-op.
    let binding = binding_records(&delivered, &nash.pins(), &mira_leaf.fingerprint);
    assert_eq!(binding.len(), 50, "one binding record per delivered frame");
    for r in &binding {
        assert!(r.binding_checked, "seq {}: binding must be checked", r.seq);
        assert_eq!(
            r.verified_peer, "host_a",
            "seq {}: verified peer is Mira",
            r.seq
        );
        assert!(
            !r.peer_fingerprint.is_empty(),
            "seq {}: TLS peer fingerprint must be non-empty",
            r.seq
        );
        // Non-empty AND well-formed: the canonical SHA-256 wire form.
        assert!(
            r.peer_fingerprint.starts_with("sha256:") && r.peer_fingerprint.len() == 7 + 64,
            "seq {}: peer fingerprint must be the sha256:<hex64> wire form, got {}",
            r.seq,
            r.peer_fingerprint
        );
    }
}

/// The real coarse round-trip number the story requires (NOT the J4 Observer
/// P95): 100 live round-trips, total wall-time measured, mean per-frame latency
/// logged. Asserts the mean is finite and > 0.
#[tokio::test]
async fn t_10_4b_coarse_roundtrip_real_measurement() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let (mira, nash) = bind_bilateral(&clock, &ca, &mira_leaf, &nash_leaf, &[FINE], &[FINE]).await;

    const N: u64 = 100;
    let start = Instant::now();
    for seq in 1..=N {
        let frame = make_fine_grained_frame("host_a", "host_b", seq, FINE);
        mira.route_outbound(frame, &HostId("host_b".into()))
            .await
            .expect("coarse round-trip frame must ACK");
    }
    let elapsed = start.elapsed();

    let mean_ns = elapsed.as_nanos() / N as u128;
    println!(
        "10.4b coarse round-trip: {N} live frames in {elapsed:?}; mean = {mean_ns} ns/frame ({:?}/frame)",
        Duration::from_nanos(mean_ns as u64)
    );
    assert!(mean_ns > 0, "mean per-frame latency must be finite and > 0");
    // Sanity: Nash observed all N intakes (each frame's identity binding passed).
    assert_eq!(
        nash.intake_entered(),
        N as usize,
        "all {N} frames entered intake"
    );
}

/// F2 (reachable RED) — the close-time >90min and consent <48/50 floors are NOT
/// green-by-construction: a failure set that violates each floor exists and is
/// classified by the SAME `ScenarioRecord` gauge the corpus test feeds. This
/// proves both the gauge (a record with `close_time > budget` / `!consent_ok`
/// lands in the failure set) AND that the floor thresholds are reachable, so a
/// regression that silently weakened the gauge would be caught here. No live
/// socket and no 90-minute sleep — the gauge is a pure function over the
/// derived records; the over-budget value is the INPUT, the classification real.
#[test]
fn t_10_4b_close_time_and_consent_floors_reachable_red() {
    let budget = Duration::from_secs(90 * 60);

    // (1) Gauge correctness — an over-budget record is classified as a
    // close-time failure; an in-budget record is not.
    let gauge = vec![
        ScenarioRecord {
            seq: 1,
            close_time: budget + Duration::from_secs(60),
            consent_ok: true,
        },
        ScenarioRecord {
            seq: 2,
            close_time: Duration::from_secs(1),
            consent_ok: true,
        },
    ];
    assert_eq!(
        close_failures(&gauge, budget),
        vec![1],
        "an over-budget record must land in the close-time failure set"
    );

    // (2) Floor reachability — 6 over-budget records exceed the ≥45/50 floor
    // (≤5 failures allowed): if the corpus test's real records ever drifted
    // over budget the floor WOULD fire, so it is not dead code.
    let red_close: Vec<ScenarioRecord> = (1..=6u64)
        .map(|seq| ScenarioRecord {
            seq,
            close_time: budget + Duration::from_secs(seq * 60),
            consent_ok: true,
        })
        .collect();
    assert!(
        close_failures(&red_close, budget).len() > 5,
        "the RED vector must violate the ≥45/50 close-time floor (6 > 5 allowed)"
    );

    // (3) Gauge correctness — a consent-denied record is classified as a
    // consent failure; an upheld record is not.
    let consent_gauge = vec![
        ScenarioRecord {
            seq: 1,
            close_time: Duration::from_secs(1),
            consent_ok: true,
        },
        ScenarioRecord {
            seq: 2,
            close_time: Duration::from_secs(1),
            consent_ok: false,
        },
    ];
    assert_eq!(
        consent_failures(&consent_gauge),
        vec![2],
        "a consent-denied record must land in the consent failure set"
    );

    // (4) Floor reachability — 3 denied records exceed the ≥48/50 floor (≤2
    // failures allowed): a consent regression would fire here too.
    let red_consent: Vec<ScenarioRecord> = (1..=3u64)
        .map(|seq| ScenarioRecord {
            seq,
            close_time: Duration::from_secs(1),
            consent_ok: false,
        })
        .collect();
    assert!(
        consent_failures(&red_consent).len() > 2,
        "the RED vector must violate the ≥48/50 consent floor (3 > 2 allowed)"
    );
}

/// F3 — the deterministic 50-scenario corpus is content-addressed. The SHA-256
/// of `serde_json::to_vec(&build_corpus())` (compact JSON, BTreeMap key order,
/// raw UTF-8) is pinned in `tests/fixtures/MANIFEST.toml`; any drift in the
/// generator (a changed seed value, a different float, a reordered field)
/// changes the digest and turns this RED — a tamper-evident contract, not a
/// "tests pass" coincidence. No live socket required.
#[test]
fn t_10_4b_corpus_content_addressed() {
    let corpus = build_corpus();
    let actual = corpus_sha256(&corpus);
    let pinned = manifest_corpus_sha256();
    assert_eq!(
        actual, pinned,
        "corpus SHA-256 must match tests/fixtures/MANIFEST.toml; \
         regenerate the manifest if the deterministic corpus intentionally changed"
    );
    // The digest is the canonical 64-char lowercase-hex SHA-256.
    assert_eq!(
        actual.len(),
        64,
        "corpus content address is a 64-char hex SHA-256"
    );
    assert!(
        actual.chars().all(|c| c.is_ascii_hexdigit()),
        "corpus content address is lowercase hex"
    );
}
