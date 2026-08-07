#![allow(dead_code)]

//! Story 13.6e (AC3) — the HARNESS side of the evidence ledger.
//!
//! # Why this lives in the harness and not in the gate (trap 2)
//!
//! A gate that signed a transcript after reading it would attest "the gate saw
//! this text", not "the test produced it" — the judge grading its own code. So
//! the LIVE TEST signs its own record and the gate only verifies. This module
//! is the signer, shared by every live test file a ledger-set gate names.
//!
//! # Why it is a `#[path]`-included module and not a crate
//!
//! Story 13.6e adds no new crate and no new dependency. Every including crate
//! (`maos-loom-lite`, `maos-bin`, `maos-bench`) already reaches `maos-audit`
//! and `maos-domain`, and this file lives under a `tests/` directory so it
//! contributes zero lines to any kloc ceiling. Include it with:
//!
//! ```ignore
//! #[path = "../../../tests/harness/evidence_record.rs"]
//! mod evidence_record;
//! ```
//!
//! # The contract
//!
//! The gate exports four variables to the `cargo test` child and creates an
//! empty sink file:
//!
//! * `MAOS_EVIDENCE_GATE`   — the gate whose ledger this record belongs to
//! * `MAOS_EVIDENCE_COMMIT` — the commit under test
//! * `MAOS_EVIDENCE_NONCE`  — a nonce minted once per gate invocation
//! * `MAOS_EVIDENCE_SINK`   — where to append the record
//!
//! A passing test appends
//! `MAOS-EVIDENCE-V1 {"commit":…,"gate":…,"nonce":…,"outcome":"PASSED",`
//! `"signature":…,"test":…}`. The signed outcome and exact test identity bind
//! the record to the leg it proves.
//!
//! Nothing is emitted when the variables are unset (an ordinary `cargo test`)
//! or when the operator audit key is unavailable. **There is no dev-key
//! fallback**: a dev-key-forgible artifact stamped `PROVEN` is the 13.2
//! trusted-registry category error replayed. Without a key the gate records
//! `INDETERMINATE` with a written reason, which is the honest outcome.

pub const RECORD_PREFIX: &str = "MAOS-EVIDENCE-V1 ";

/// Emit the record when the test finishes WITHOUT panicking.
///
/// Held as `let _evidence = evidence_record::attest("test_name");` at the top of
/// a live test. `Drop` checks `std::thread::panicking()`, so a failing test
/// signs nothing — the signature attests a run that reached its own end, not
/// merely a run that started.
pub struct Attestation {
    test: &'static str,
}

pub fn attest(test: &'static str) -> Attestation {
    Attestation { test }
}

impl Drop for Attestation {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        emit(self.test);
    }
}

/// Sign and write one transcript record. A no-op outside a gate invocation.
pub fn emit(test: &str) {
    let (Ok(gate), Ok(commit), Ok(nonce)) = (
        std::env::var("MAOS_EVIDENCE_GATE"),
        std::env::var("MAOS_EVIDENCE_COMMIT"),
        std::env::var("MAOS_EVIDENCE_NONCE"),
    ) else {
        // Not running under a ledger-set gate — an ordinary `cargo test`.
        return;
    };

    // The operator-pinned key, by the shipped precedence: explicit path →
    // MAOS_AUDIT_KEY → ~/.config/maos/audit-signing.key, 0600, fail-loud.
    let Ok(seed) = maos_domain::audit_key::load_audit_key_seed(&None) else {
        // CI holds no operator key by ratified design. Emit nothing; the gate
        // downgrades to INDETERMINATE with a written reason.
        return;
    };

    let payload = serde_json::json!({
        "commit": commit,
        "gate": gate,
        "nonce": nonce,
        "test": test,
        "outcome": "PASSED",
    });
    // ADR-028 D5b: the ONE canonicalizer, the same call the gate makes.
    let Ok(bytes) = maos_audit::sealed_export::canonicalize_value(&payload) else {
        return;
    };
    let signature = hex_encode(&maos_audit::release_verify::sign_sha256sums(&bytes, &seed));

    let mut record = match payload {
        serde_json::Value::Object(map) => map,
        _ => return,
    };
    record.insert(
        "signature".to_string(),
        serde_json::Value::String(signature),
    );
    let line = format!("{RECORD_PREFIX}{}", serde_json::Value::Object(record));

    // libtest swallows a PASSING test's stdout unless `--nocapture`, so the
    // sink file is the reliable channel; the printed copy is for humans and for
    // the gates that do pass `--nocapture`.
    println!("{line}");
    if let Ok(sink) = std::env::var("MAOS_EVIDENCE_SINK") {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(sink)
        {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
