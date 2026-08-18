#![forbid(unsafe_code)]

//! Gate — `check-j1-two-host-signed-run` (story `j1-crosshost-2c`, AC5.1/AC5.2).
//!
//! `2a` made one host able to tell the truth about its worker. `2b` made a second
//! host actually do the work. This gate is **the judge**: it holds the controls
//! that decide what a signed two-host artifact is allowed to assert.
//!
//! ## Why ONE always-`Blocking` job, and no `AdvisorySubstrate` sibling
//!
//! The obvious shape was two jobs: hermetic legs `Blocking`, and the paid-run leg
//! `AdvisorySubstrate`. Ask what substrate that second job needs — an operator, two
//! hosts and a funded API key. CI has never had those and never will. The job
//! would take the substrate-ABSENT branch on every run for its entire lifetime,
//! printing WOULD-HAVE-BLOCKED into the void and blocking nothing, ever. **A gate
//! whose substrate cannot exist is a monument, not a control.**
//!
//! So the paid run's evidence is a **capture artifact** on the T6 model: this gate
//! validates it when present, and refuses to let anything claim it when absent.
//! One job, one binding class, no never-firing sibling. The absent case is
//! expressed by the demo's beat staying ABSENT, which is already its honest model
//! — never by a binding class that cannot fire.
//!
//! ## Legs
//!
//! 1. **`signing-identity-repaired`** (AC1) — both `sealed-export` sites print the
//!    key that actually SIGNED, the stdout arm prints one at all, and
//!    `verify-bundle` has a derivation path. This is the bug that would have burned
//!    the paid run *after* the agent was billed.
//! 2. **`host-discriminator-signed`** (AC2.1) — the host field exists, is
//!    `skip_serializing_if` (byte-identity), and is copied into the struct
//!    `verify_bundle` re-canonicalizes, which is what binds it to the signature.
//! 3. **`reconciliation-refuses-one-root`** (AC2.2/AC2.4) — two halves under one
//!    root are REFUSED. Without this the host field proves nothing: one seed holder
//!    could legitimately sign both halves of a "two-host" bundle.
//! 4. **`bundle-schema-enforced`** (AC2.6) — the schema is validated against a real
//!    bundle AND corrected to match the struct. A planted extra field REDs this.
//! 5. **`fault-typing-and-bounds`** (AC3.1/3.2/3.3) — `CODE_INTERNAL` and
//!    `CODE_TIMEOUT` are typed, `connect` and `framed.send` are bounded, and the
//!    TCP path reads `partition_timeout_secs`.
//! 6. **`duplicate-after-durable`** (AC3.5) — the digest-reply commit guard runs
//!    BEFORE the dedup record is published. Nothing is `Duplicate` until something
//!    is durable.
//! 7. **`pin-refusal-journaled`** (AC3.6/3.7) — both sides journal, and
//!    `maos-a2a-tcp` still carries no `maos-kernel-core` production dependency.
//! 8. **`stored-row-scan`** (AC4.1) — the read-path scan exists and reports BOTH
//!    classes distinctly.
//! 9. **`paid-run-capture`** (AC5.1/AC5.5) — validate the capture when present;
//!    refuse any claim of a two-host signed run when absent.
//! 10. **`two-host-vectors-enrolled`** (AC5.2) — every `_2c.rs` target is named in
//!    this gate's job, DERIVED from the filesystem. A hand-listed set is one
//!    forgotten line from a dead test behind a green gate.
//!
//! ## Vacuity and hermeticity
//!
//! Every leg records a [`LegAudit`]; a leg reporting `!ran || checks == 0`
//! hard-FAILs, because `findings.is_empty()` cannot tell a leg that passed from a
//! leg that read nothing. Every read is `root.join(rel)` so the proven-red harness
//! can point the gate at a fixture tree — a hardcoded path or a shelled `cargo`
//! would vacuum every planted vector and report green.

use crate::gate_common::{dev_enforced_red_blocks, vacuous_legs, BindingClass, LegAudit};
use std::fs;
use std::path::Path;

const SUBCOMMANDS_RS: &str = "crates/maos-cli/src/subcommands.rs";
const CLI_RS: &str = "crates/maos-cli/src/cli.rs";
const SEALED_EXPORT_RS: &str = "crates/maos-audit/src/sealed_export.rs";
const ROUTER_RS: &str = "crates/maos-a2a-core/src/router.rs";
const COHORT_RS: &str = "crates/maos-a2a-core/src/cohort.rs";
const TRANSPORT_RS: &str = "crates/maos-a2a-tcp/src/transport.rs";
const A2A_TCP_MANIFEST: &str = "crates/maos-a2a-tcp/Cargo.toml";
const COHORT_STATE_RS: &str = "crates/maos-cohort/src/state.rs";
const REDACTION_RS: &str = "crates/maos-iac/src/adapter/redaction.rs";
const BUNDLE_SCHEMA: &str = "schemas/audit-bundle.schema.json";
const WORKFLOW: &str = ".github/workflows/discipline.yml";

/// The DIRECTORY the enrolled test set is DERIVED from, and the suffix that marks
/// a target as this story's. Never a hand-maintained const list: add a `_2c.rs`
/// file, forget the line, and the file is dead in CI while the gate stays green.
const TESTS_DIRS: &[&str] = &[
    "crates/maos-cli/tests",
    "crates/maos-audit/tests",
    "crates/maos-a2a-core/tests",
    "crates/maos-a2a-tcp/tests",
];
const STORY_TEST_SUFFIX: &str = "_2c.rs";
/// `crates/maos-a2a-tcp/tests/` names its targets `t_<story>_<topic>.rs`
/// (`t_12_4a_digest_read.rs`, `t_12_3_cohort_halt_receipt.rs`), so a suffix-only
/// derivation would silently miss every file in that directory — the exact
/// hand-maintained-list failure this derivation exists to prevent, re-created by
/// assuming one naming convention.
const STORY_TEST_PREFIX: &str = "t_2c_";

/// The paid run's capture artifact — validated when present, unclaimed when absent.
const CAPTURE: &str = "_bmad-output/test-artifacts/j1-two-host-evidence/two-host-capture.json";
/// The operator-signed `MAOS-EVIDENCE-V1` transcript for the paid run.
const CAPTURE_TRANSCRIPT: &str =
    "_bmad-output/test-artifacts/j1-two-host-evidence/two-host-evidence.txt";
/// The signed bundle halves the capture attests, if the paid run produced them.
const CAPTURE_BUNDLE_A: &str =
    "_bmad-output/test-artifacts/j1-two-host-evidence/host-a-bundle.json";
const CAPTURE_BUNDLE_B: &str =
    "_bmad-output/test-artifacts/j1-two-host-evidence/host-b-bundle.json";

/// The EXACT words the artifact is allowed to use for what two signatures prove.
/// Anything stronger is a claim standing in for a control.
pub const CLAIM_SCOPE: &str =
    "two keyed identities signed; not two machines, two processes, or two operators";

pub fn ledger_leg_names() -> Vec<&'static str> {
    vec![
        "signing-identity-repaired",
        "host-discriminator-signed",
        "reconciliation-refuses-one-root",
        "bundle-schema-enforced",
        "fault-typing-and-bounds",
        "duplicate-after-durable",
        "pin-refusal-journaled",
        "stored-row-scan",
        "paid-run-capture",
        "two-host-vectors-enrolled",
    ]
}

#[derive(Debug)]
pub struct Finding {
    pub check: &'static str,
    pub detail: String,
}

/// Read a governed file. A missing file is a FINDING, never a skip: a gate that
/// silently passes when its subject is absent is the null control this story
/// exists to stop.
fn read(
    root: &Path,
    rel: &str,
    findings: &mut Vec<Finding>,
    check: &'static str,
) -> Option<String> {
    match fs::read_to_string(root.join(rel)) {
        Ok(s) => Some(s),
        Err(e) => {
            findings.push(Finding {
                check,
                detail: format!("cannot read {rel}: {e}"),
            });
            None
        }
    }
}

/// Lines that are not comments — so an invariant cannot be satisfied by prose
/// describing it.
///
/// `#` starts a comment in TOML and YAML, both of which this gate reads, so a
/// leading `#` is filtered. `#[…]` is a Rust ATTRIBUTE and therefore live code:
/// `#[serde(...)]` and `#[command(group(...))]` are exactly the kind of fact these
/// legs need to see, and filtering them would make those checks unsatisfiable.
fn live_lines(src: &str) -> impl Iterator<Item = &str> {
    src.lines().filter(|l| {
        let t = l.trim_start();
        if t.is_empty() || t.starts_with("//") || t.starts_with('*') {
            return false;
        }
        !(t.starts_with('#') && !t.starts_with("#["))
    })
}

fn contains_live(src: &str, needle: &str) -> bool {
    live_lines(src).any(|l| l.contains(needle))
}

/// Assert one live needle, recording the check either way.
fn require(
    src: &str,
    needle: &str,
    audit: &mut LegAudit,
    findings: &mut Vec<Finding>,
    check: &'static str,
    why: &str,
) {
    audit.checked();
    if !contains_live(src, needle) {
        findings.push(Finding {
            check,
            detail: format!("`{needle}` is absent from live code — {why}"),
        });
    }
}

pub struct Judgement {
    pub findings: Vec<Finding>,
    pub audits: Vec<LegAudit>,
    /// Whether a paid-run capture was present this run. `false` is honest and
    /// expected in CI; it is NOT a substrate gate, it is the absence of a claim.
    pub capture_present: bool,
    /// Enrolled `_2c.rs` targets, derived from the filesystem.
    pub enrolled: Vec<String>,
}

impl Judgement {
    pub fn leg_green(&self, leg: &str) -> Option<bool> {
        self.audits
            .iter()
            .find(|a| a.leg() == leg)
            .map(|a| !a.is_vacuous() && !self.findings.iter().any(|f| f.check == leg))
    }
}

// ── Leg 1 — AC1 ────────────────────────────────────────────────────────────

fn leg_signing_identity(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "signing-identity-repaired";
    let mut audit = LegAudit::new(check);
    let Some(sub) = read(root, SUBCOMMANDS_RS, findings, check) else {
        return audit;
    };
    audit.entered();

    // Both sites must bind the resolved region to a local BEFORE the match
    // consumes it, and derive the printed key from THAT.
    audit.checked();
    let region_locals = sub
        .matches("let region_home = match resolve_region_home()")
        .count();
    if region_locals < 2 {
        findings.push(Finding {
            check,
            detail: format!(
                "expected BOTH sealed-export sites to bind the resolved region to a local \
                 before printing; found {region_locals}. Fixing only one leaves the bug live \
                 on the other subcommand"
            ),
        });
    }
    audit.checked();
    let derived_prints = sub.matches("derive_region_pubkey(&seed, r)").count();
    if derived_prints < 2 {
        findings.push(Finding {
            check,
            detail: format!(
                "expected BOTH sites to print `derive_region_pubkey`; found {derived_prints}. \
                 A printed key that is not the signing key makes the bundle unverifiable, and \
                 demo-j1 scrapes that key straight into verify-bundle"
            ),
        });
    }
    // The stdout arm must print a pubkey too, in the established line shape.
    require(
        &sub,
        "written to stdout ({} entries, pubkey {})",
        &mut audit,
        findings,
        check,
        "a stdout-mode export with no printed key is an unverifiable artifact you can \
         produce by accident",
    );
    // And nothing may print the BASE key while a region resolved.
    audit.checked();
    if contains_live(
        &sub,
        "let pubkey = maos_audit::sealed_export::derive_pubkey(&seed);",
    ) {
        findings.push(Finding {
            check,
            detail: "an unconditional `derive_pubkey(&seed)` print survives — that is the \
                     P12 bug"
                .to_string(),
        });
    }

    if let Some(cli) = read(root, CLI_RS, findings, check) {
        require(
            &cli,
            "seed: Option<std::path::PathBuf>",
            &mut audit,
            findings,
            check,
            "verify-bundle needs a derivation path: a region-pinned bundle could otherwise \
             only be checked by someone who already knew the derived key",
        );
        require(
            &cli,
            "ArgGroup::new(\"verify_key\").required(true)",
            &mut audit,
            findings,
            check,
            "exactly one key source must be required, and the refusal must name both",
        );
    }
    audit
}

// ── Leg 2 — AC2.1 ──────────────────────────────────────────────────────────

fn leg_host_discriminator(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "host-discriminator-signed";
    let mut audit = LegAudit::new(check);
    let Some(src) = read(root, SEALED_EXPORT_RS, findings, check) else {
        return audit;
    };
    audit.entered();

    // The field exists on BOTH the signed and the for-signing struct. Only the
    // second one binds it to the signature.
    audit.checked();
    let declarations = src.matches("pub host: Option<String>").count();
    if declarations < 2 {
        findings.push(Finding {
            check,
            detail: format!(
                "`host` must be declared on BOTH `AuditBundle` and `BundleForSigning`; found \
                 {declarations}. A field only on `AuditBundle` is a LABEL a forger can \
                 rewrite, because `verify_bundle` would never re-canonicalize it"
            ),
        });
    }
    require(
        &src,
        "host: bundle.host.clone()",
        &mut audit,
        findings,
        check,
        "verify_bundle must copy the host into the struct it re-canonicalizes, or the field \
         is not under the signature",
    );
    require(
        &src,
        "host: bundle_for_signing.host",
        &mut audit,
        findings,
        check,
        "sign_bundle must carry the host through to the signed bundle",
    );
    // Byte-identity: the field must be omitted when absent.
    audit.checked();
    let host_decl_idx = src.find("pub host: Option<String>");
    let omitted = host_decl_idx
        .and_then(|i| src[..i].rfind("skip_serializing_if"))
        .map(|j| host_decl_idx.unwrap() - j < 200)
        .unwrap_or(false);
    if !omitted {
        findings.push(Finding {
            check,
            detail: "`host` must be `skip_serializing_if = \"Option::is_none\"` — otherwise \
                     every pre-2c bundle stops replaying byte-identically"
                .to_string(),
        });
    }
    require(
        &src,
        "pub fn with_host",
        &mut audit,
        findings,
        check,
        "the stamping builder must exist",
    );
    // The claim scope, in the ratified words, in the artifact rather than a story.
    audit.checked();
    if !src.contains(CLAIM_SCOPE) {
        findings.push(Finding {
            check,
            detail: format!(
                "the bounded claim must travel with the format, verbatim: {CLAIM_SCOPE:?}"
            ),
        });
    }
    if let Some(cli) = read(root, CLI_RS, findings, check) {
        require(
            &cli,
            "host: Option<String>",
            &mut audit,
            findings,
            check,
            "sealed-export needs a --host flag or no bundle can ever be stamped",
        );
    }
    audit
}

// ── Leg 3 — AC2.2 / AC2.4 ──────────────────────────────────────────────────

fn leg_reconciliation(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "reconciliation-refuses-one-root";
    let mut audit = LegAudit::new(check);
    let Some(src) = read(root, SEALED_EXPORT_RS, findings, check) else {
        return audit;
    };
    audit.entered();

    for (needle, why) in [
        (
            "pub fn reconcile_two_host_bundles",
            "the two-bundle verb must exist; the single-`--pubkey` surface cannot express two \
             independent roots",
        ),
        (
            "SharedAttesterRoot",
            "one seed holder signing BOTH halves is the exact forgery the host field exists \
             to stop, and under a shared root the field proves nothing at all",
        ),
        (
            "if key_a == key_b {",
            "the shared-root refusal must be checked BEFORE anything else succeeds",
        ),
        (
            "MissingHostClaim",
            "a bundle with no host claim cannot be half of a two-host run",
        ),
        (
            "DuplicateHostClaim",
            "two halves claiming one host are one host, whatever the keys say",
        ),
        (
            "NoSharedFrames",
            "logs that share no frame_id did not witness one run",
        ),
        (
            "pub fn build_two_host_receipt",
            "the ported receipt shape is the two-host claim",
        ),
        (
            "pub fn verify_two_host_receipt",
            "a receipt nobody can verify is a note",
        ),
    ] {
        require(&src, needle, &mut audit, findings, check, why);
    }

    // The join is on frame_id, and `correlation_id` is NOT projected.
    require(
        &src,
        "e.frame_id_hex.as_str()",
        &mut audit,
        findings,
        check,
        "the join key is frame_id: both hosts provably carry the same sixteen bytes",
    );
    audit.checked();
    if contains_live(&src, "correlation_id") {
        findings.push(Finding {
            check,
            detail: "`correlation_id` must NOT be projected — it is not the join key".to_string(),
        });
    }
    // R-RG1: the verifier must never read the key the artifact carries.
    audit.checked();
    let reconcile_body = src
        .split_once("pub fn reconcile_two_host_bundles")
        .map(|(_, rest)| rest.split_once("\n}").map_or(rest, |(b, _)| b))
        .unwrap_or("");
    if reconcile_body.contains("attester_pubkey") {
        findings.push(Finding {
            check,
            detail: "reconciliation must never read `signature_block.attester_pubkey` to decide \
                     anything (R-RG1) — derive from the claimed identity"
                .to_string(),
        });
    }
    // maos-audit must NOT reach for maos-loom-lite: that closes a cycle.
    if let Some(manifest) = read(root, "crates/maos-audit/Cargo.toml", findings, check) {
        audit.checked();
        if manifest.contains("maos-loom-lite") {
            findings.push(Finding {
                check,
                detail: "maos-audit must not depend on maos-loom-lite — `maos-loom-lite -> \
                         maos-audit` already exists and the reverse edge closes a CYCLE. The \
                         pattern is reimplemented natively here instead"
                    .to_string(),
            });
        }
    }
    audit
}

// ── Leg 4 — AC2.6 ──────────────────────────────────────────────────────────

/// Minimal `additionalProperties: false` + `required` validator.
///
/// Deliberately small and dependency-free: the schema's own contract is exactly
/// these two keywords plus nested objects, and a JSON-Schema crate in `xtask`
/// would be a larger surface than the thing it validates.
fn validate_against(
    schema: &serde_json::Value,
    value: &serde_json::Value,
    path: &str,
) -> Vec<String> {
    let mut errs = Vec::new();
    let Some(obj) = value.as_object() else {
        return errs;
    };
    let props = schema.get("properties").and_then(|p| p.as_object());
    if schema
        .get("additionalProperties")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        if let Some(props) = props {
            for key in obj.keys() {
                if !props.contains_key(key) {
                    errs.push(format!("{path}{key}: not declared by the schema"));
                }
            }
        }
    }
    for req in schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .unwrap_or(&Vec::new())
    {
        if let Some(name) = req.as_str() {
            if !obj.contains_key(name) {
                errs.push(format!("{path}{name}: required but absent"));
            }
        }
    }
    if let Some(props) = props {
        for (key, sub) in obj {
            let Some(sub_schema) = props.get(key) else {
                continue;
            };
            let child = format!("{path}{key}.");
            match sub {
                serde_json::Value::Object(_) => {
                    errs.extend(validate_against(sub_schema, sub, &child))
                }
                serde_json::Value::Array(items) => {
                    if let Some(item_schema) = sub_schema.get("items") {
                        for item in items {
                            errs.extend(validate_against(item_schema, item, &child));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    errs
}

fn leg_bundle_schema(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "bundle-schema-enforced";
    let mut audit = LegAudit::new(check);
    let Some(schema_text) = read(root, BUNDLE_SCHEMA, findings, check) else {
        return audit;
    };
    audit.entered();
    let schema: serde_json::Value = match serde_json::from_str(&schema_text) {
        Ok(v) => v,
        Err(e) => {
            findings.push(Finding {
                check,
                detail: format!("{BUNDLE_SCHEMA} is not valid JSON: {e}"),
            });
            return audit;
        }
    };

    // The schema must declare every field the struct can emit. A schema with
    // `additionalProperties: false` that omits an emitted field is not
    // documentation — it is a FALSE SPECIFICATION.
    audit.checked();
    let declared: Vec<&String> = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|o| o.keys().collect())
        .unwrap_or_default();
    for field in [
        "schema_version",
        "entries",
        "i12_digest_refs",
        "i11_distilled_content",
        "freshness",
        "applied_redaction",
        "redaction_policy",
        "region",
        "host",
        "signature_block",
    ] {
        audit.checked();
        if !declared.iter().any(|d| d.as_str() == field) {
            findings.push(Finding {
                check,
                detail: format!(
                    "{BUNDLE_SCHEMA} omits `{field}`, which the struct emits, while declaring \
                     `additionalProperties: false`. That is a false specification, not \
                     documentation"
                ),
            });
        }
    }

    // And it must actually VALIDATE a bundle. Any bundle present in the tree is
    // fair game — the capture halves when the paid run produced them.
    for rel in [CAPTURE_BUNDLE_A, CAPTURE_BUNDLE_B] {
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        audit.checked();
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(bundle) => {
                let errs = validate_against(&schema, &bundle, "");
                if !errs.is_empty() {
                    findings.push(Finding {
                        check,
                        detail: format!("{rel} violates {BUNDLE_SCHEMA}: {}", errs.join("; ")),
                    });
                }
            }
            Err(e) => findings.push(Finding {
                check,
                detail: format!("{rel} is not valid JSON: {e}"),
            }),
        }
    }
    audit
}

// ── Leg 5 — AC3.1 / AC3.2 / AC3.3 ──────────────────────────────────────────

fn leg_fault_typing(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "fault-typing-and-bounds";
    let mut audit = LegAudit::new(check);
    if let Some(router) = read(root, ROUTER_RS, findings, check) {
        audit.entered();
        for (needle, why) in [
            (
                "CODE_INTERNAL => Err(A2AError::PeerInternalFailure",
                "a dropped-receiver internal NACK and a genuine partition were byte-identical \
                 at the sender; AC3's fault windows cannot assert on faults they cannot tell \
                 apart",
            ),
            (
                "CODE_TIMEOUT => Err(A2AError::PeerIntakeTimeout",
                "the receiver's own intake timeout is not a wire partition",
            ),
        ] {
            require(&router, needle, &mut audit, findings, check, why);
        }
    }
    if let Some(transport) = read(root, TRANSPORT_RS, findings, check) {
        audit.entered();
        for (needle, why) in [
            (
                "tokio::time::timeout(partition, TcpStream::connect(addr))",
                "a bare `TcpStream::connect` hangs on the ~130s OS SYN-retry backstop",
            ),
            (
                "tokio::time::timeout(partition, framed.send(",
                "`framed.send` was ALSO unbounded, and it is the cheaper real partition: a \
                 peer that accepts and stops reading hangs route_outbound forever with NO OS \
                 backstop",
            ),
            (
                "peer_cfg.partition_timeout_secs",
                "the operator-configured partition window must reach the TCP path, or the \
                 §7.2 claim is not true of the wire",
            ),
            (
                "A2AError::PartitionTimeout {",
                "a partition must be minted typed where the frame id is in scope",
            ),
        ] {
            require(&transport, needle, &mut audit, findings, check, why);
        }
    }
    audit
}

// ── Leg 6 — AC3.5 ──────────────────────────────────────────────────────────

fn leg_duplicate_after_durable(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "duplicate-after-durable";
    let mut audit = LegAudit::new(check);
    if let Some(cohort) = read(root, COHORT_RS, findings, check) {
        audit.entered();
        require(
            &cohort,
            "fn observe_reply_guarded",
            &mut audit,
            findings,
            check,
            "the reply path needs a commit guard, mirroring note_admitted_request_guarded",
        );
    }
    if let Some(router) = read(root, ROUTER_RS, findings, check) {
        audit.entered();
        require(
            &router,
            "observe_reply_guarded(&peer_host, frame, &mut push)",
            &mut audit,
            findings,
            check,
            "the router must hand the intake push in as the guard, not push afterwards",
        );
        // The old ordering must be gone from the digest-reply path.
        audit.checked();
        let reply_window = router
            .split_once("DigestFrameClass::Reply { .. }")
            .map(|(_, rest)| rest.split_once("// (3) ADR-012").map_or(rest, |(b, _)| b))
            .unwrap_or("");
        if reply_window.contains("push_to_intake_sink") {
            findings.push(Finding {
                check,
                detail: "the digest-reply path must not call `push_to_intake_sink` after \
                         observing: that ordering is exactly the `Duplicate`-before-durable lie"
                    .to_string(),
            });
        }
    }
    if let Some(state) = read(root, COHORT_STATE_RS, findings, check) {
        audit.entered();
        audit.checked();
        let body = state
            .split_once("fn observe_reply_guarded")
            .map(|(_, rest)| rest.split_once("\n    }").map_or(rest, |(b, _)| b))
            .unwrap_or("");
        let guard_at = body.find("before_commit()?");
        let publish_at = body.find("received.insert");
        match (guard_at, publish_at) {
            (Some(g), Some(p)) if g < p => {}
            (Some(_), Some(_)) => findings.push(Finding {
                check,
                detail: "`before_commit()` must run BEFORE `received.insert` — publishing the \
                         dedup first is the whole defect"
                    .to_string(),
            }),
            _ => findings.push(Finding {
                check,
                detail: "the impl must call `before_commit()?` and then publish the dedup record"
                    .to_string(),
            }),
        }
    }
    audit
}

// ── Leg 7 — AC3.6 / AC3.7 ──────────────────────────────────────────────────

fn leg_pin_refusal_journaled(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "pin-refusal-journaled";
    let mut audit = LegAudit::new(check);
    if let Some(transport) = read(root, TRANSPORT_RS, findings, check) {
        audit.entered();
        audit.checked();
        let calls = transport.matches("journal_peer_identity_refusal").count();
        if calls < 3 {
            findings.push(Finding {
                check,
                detail: format!(
                    "expected the listen-side handshake arm, the unresolved-peer arm and the \
                     dial-side arm to journal; found {calls} call(s). The listen side used to \
                     take a blanket `_ => return` and leave ZERO trace"
                ),
            });
        }
        require(
            &transport,
            "PeerRefusalDirection::Listen",
            &mut audit,
            findings,
            check,
            "the journal must record WHICH side spoke: the listen side accepts any active pin \
             while per-peer scoping exists only on the dial side",
        );
        require(
            &transport,
            "PeerRefusalDirection::Dial",
            &mut audit,
            findings,
            check,
            "both sides must journal, or `journal on both sides` is half true",
        );
        audit.checked();
        if contains_live(&transport, "Ok(Err(e)) => {") && !transport.contains("classify_handshake")
        {
            findings.push(Finding {
                check,
                detail: "the handshake error arm must classify before deciding to journal"
                    .to_string(),
            });
        }
    }
    if let Some(manifest) = read(root, A2A_TCP_MANIFEST, findings, check) {
        audit.entered();
        audit.checked();
        let production = manifest
            .split_once("[dev-dependencies]")
            .map_or(manifest.as_str(), |(p, _)| p);
        if production.contains("maos-kernel-core") {
            findings.push(Finding {
                check,
                detail: "maos-a2a-tcp must not gain a `maos-kernel-core` PRODUCTION dependency \
                         — the chaos-absence barrier greps this manifest inside its 50x loop"
                    .to_string(),
            });
        }
    }
    audit
}

// ── Leg 8 — AC4.1 ──────────────────────────────────────────────────────────

fn leg_stored_row_scan(root: &Path, findings: &mut Vec<Finding>) -> LegAudit {
    let check = "stored-row-scan";
    let mut audit = LegAudit::new(check);
    if let Some(redaction) = read(root, REDACTION_RS, findings, check) {
        audit.entered();
        for (needle, why) in [
            (
                "pub fn scan_stored_payload",
                "the read-path scan exists nowhere else: every redaction call site is pre-write",
            ),
            (
                "CredentialShape::HexRun",
                "asserting only the prefix half leaves the scan blind to exactly the class the \
                 write path handles SILENTLY",
            ),
            (
                "CredentialShape::Prefix",
                "the two classes must be reported distinctly so the escape's class is visible",
            ),
        ] {
            require(&redaction, needle, &mut audit, findings, check, why);
        }
        // Single source of truth: the scan must reuse RULES, not re-derive them.
        audit.checked();
        let body = redaction
            .split_once("pub fn scan_stored_payload")
            .map(|(_, rest)| rest.split_once("\n}").map_or(rest, |(b, _)| b))
            .unwrap_or("");
        if !body.contains("RULES") {
            findings.push(Finding {
                check,
                detail: "the scan must reuse `RULES` — a scan that re-derives the rules is a \
                         second source of truth waiting to drift"
                    .to_string(),
            });
        }
    }
    if let Some(cli) = read(root, CLI_RS, findings, check) {
        audit.entered();
        require(
            &cli,
            "ScanCredentials {",
            &mut audit,
            findings,
            check,
            "an operator needs a way to run the scan",
        );
    }
    audit
}

// ── Leg 9 — AC5.1 / AC5.5 ──────────────────────────────────────────────────

fn leg_paid_run_capture(root: &Path, findings: &mut Vec<Finding>) -> (LegAudit, bool) {
    let check = "paid-run-capture";
    let mut audit = LegAudit::new(check);
    audit.entered();
    let capture_text = fs::read_to_string(root.join(CAPTURE)).ok();
    let present = capture_text.is_some();

    let Some(text) = capture_text else {
        // ABSENT is the honest CI state. The control is that nothing may CLAIM the
        // run — so the gate checks the demo's beat is still allowed to be ABSENT
        // and that no artifact asserts a completed two-host signed run.
        audit.checked();
        if let Some(demo) = read(root, "xtask/src/demo_j1.rs", findings, check) {
            audit.checked();
            if !demo.contains("j1-crosshost-2d-paid-two-host-run") {
                findings.push(Finding {
                    check,
                    detail: "with no capture present, the demo beat must still name \
                             `j1-crosshost-2d-paid-two-host-run` as its owner so the claim stays ABSENT rather \
                             than unattributed"
                        .to_string(),
                });
            }
        }
        return (audit, present);
    };

    // PRESENT: validate it. A capture that cannot be parsed is worse than absent.
    audit.checked();
    let capture: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            findings.push(Finding {
                check,
                detail: format!("{CAPTURE} is present but not valid JSON: {e}"),
            });
            return (audit, present);
        }
    };
    for field in [
        "host_a",
        "host_b",
        "shape",
        "claim_scope",
        "trust_anchor_established_out_of_band",
        "host_b_audit_key_provisioned_separately",
        "stranger_verification",
    ] {
        audit.checked();
        if capture.get(field).is_none() {
            findings.push(Finding {
                check,
                detail: format!(
                    "{CAPTURE} omits `{field}` — the capture must state the SYSTEM properties \
                     that bound its claim, not leave them to a story file"
                ),
            });
        }
    }
    // The claim may not be stronger than the control.
    audit.checked();
    if capture.get("claim_scope").and_then(|v| v.as_str()) != Some(CLAIM_SCOPE) {
        findings.push(Finding {
            check,
            detail: format!(
                "{CAPTURE} must carry the ratified claim scope verbatim: {CLAIM_SCOPE:?}"
            ),
        });
    }
    audit.checked();
    let blob = text.to_lowercase().replace(&CLAIM_SCOPE.to_lowercase(), "");
    for overclaim in ["two machines", "two operators", "fully automated pairing"] {
        if blob.contains(overclaim) && !blob.contains(&format!("not {overclaim}")) {
            findings.push(Finding {
                check,
                detail: format!(
                    "{CAPTURE} asserts `{overclaim}`, which no control in this story proves"
                ),
            });
        }
    }
    // Both halves must be present when the capture claims a two-host run.
    for rel in [CAPTURE_BUNDLE_A, CAPTURE_BUNDLE_B] {
        audit.checked();
        if !root.join(rel).exists() {
            findings.push(Finding {
                check,
                detail: format!(
                    "the capture claims a two-host signed run but {rel} is absent — a claim \
                     without its artifact is the thing this gate exists to refuse"
                ),
            });
        }
    }
    (audit, present)
}

// ── Leg 10 — AC5.2 ─────────────────────────────────────────────────────────

fn leg_vectors_enrolled(root: &Path, findings: &mut Vec<Finding>) -> (LegAudit, Vec<String>) {
    let check = "two-host-vectors-enrolled";
    let mut audit = LegAudit::new(check);
    audit.entered();

    let mut derived: Vec<String> = Vec::new();
    for dir in TESTS_DIRS {
        let Ok(entries) = fs::read_dir(root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(STORY_TEST_SUFFIX)
                || (name.starts_with(STORY_TEST_PREFIX) && name.ends_with(".rs"))
            {
                derived.push(format!(
                    "{}::{}",
                    dir.split('/').nth(1).unwrap_or(dir),
                    name.trim_end_matches(".rs")
                ));
            }
        }
    }
    derived.sort();

    audit.checked();
    if derived.is_empty() {
        findings.push(Finding {
            check,
            detail: format!(
                "no `*{STORY_TEST_SUFFIX}` or `{STORY_TEST_PREFIX}*.rs` targets were derived \
                 from {TESTS_DIRS:?}. A \
                 derivation that comes back empty is how a filesystem-derived leg goes \
                 quietly decorative"
            ),
        });
        return (audit, derived);
    }

    let Some(workflow) = read(root, WORKFLOW, findings, check) else {
        return (audit, derived);
    };
    let job = workflow
        .split_once("check-j1-two-host-signed-run:")
        .map(|(_, rest)| rest.split_once("\n  check-").map_or(rest, |(body, _)| body))
        .unwrap_or("");
    audit.checked();
    if job.is_empty() {
        findings.push(Finding {
            check,
            detail: format!(
                "{WORKFLOW} has no `check-j1-two-host-signed-run` job — a test file that is \
                 not `--test`-enrolled is a suggestion, not a control"
            ),
        });
        return (audit, derived);
    }
    for target in &derived {
        audit.checked();
        let (crate_name, test_name) = target.split_once("::").unwrap_or(("", target.as_str()));
        let enrolled = job.contains(&format!("--test {test_name}"))
            && job.contains(&format!("-p {crate_name}"));
        if !enrolled {
            findings.push(Finding {
                check,
                detail: format!(
                    "`{target}` exists but is not named in this gate's job: add \
                     `cargo test -p {crate_name} --test {test_name}`"
                ),
            });
        }
    }
    // The job must not carry a `services:` block.
    audit.checked();
    if job.contains("services:") {
        findings.push(Finding {
            check,
            detail: "this job must not declare a `services:` block — the substrate-drift gate \
                     rejects an unregistered service-bearing gate job, and this gate needs no \
                     substrate"
                .to_string(),
        });
    }
    (audit, derived)
}

// ── Judgement ──────────────────────────────────────────────────────────────

pub fn judge(root: &Path) -> Judgement {
    let mut findings = Vec::new();
    let mut audits = Vec::new();

    audits.push(leg_signing_identity(root, &mut findings));
    audits.push(leg_host_discriminator(root, &mut findings));
    audits.push(leg_reconciliation(root, &mut findings));
    audits.push(leg_bundle_schema(root, &mut findings));
    audits.push(leg_fault_typing(root, &mut findings));
    audits.push(leg_duplicate_after_durable(root, &mut findings));
    audits.push(leg_pin_refusal_journaled(root, &mut findings));
    audits.push(leg_stored_row_scan(root, &mut findings));
    let (capture_audit, capture_present) = leg_paid_run_capture(root, &mut findings);
    audits.push(capture_audit);
    let (enrolled_audit, enrolled) = leg_vectors_enrolled(root, &mut findings);
    audits.push(enrolled_audit);

    // A vacuous leg is invisible to `findings.is_empty()`, so it becomes a finding.
    for leg in vacuous_legs(&audits) {
        findings.push(Finding {
            check: "leg-vacuity",
            detail: format!(
                "leg `{leg}` reported no executed check. A leg that reads nothing is \
                 indistinguishable from a leg that passed, so this gate treats it as RED — fix \
                 the leg or its input, never the guard"
            ),
        });
    }

    Judgement {
        findings,
        audits,
        capture_present,
        enrolled,
    }
}

/// The gate name evidence records must be bound to.
pub const GATE: &str = "check-j1-two-host-signed-run";

/// `j1-crosshost-2c` AC5.4 — `PROVEN_LIVE_SIGNED` under Reza's posture, in the
/// real vocabulary.
///
/// The key is `MAOS_AUDIT_KEY` — a **filesystem path**, resolved by
/// `maos_domain::audit_key::load_audit_key_seed`. There is no
/// `MAOS_AUDIT_KEY_SEED`: its only occurrence in this workspace is an error
/// string, and code written against it would not compile.
///
/// The evidence is a `MAOS-EVIDENCE-V1` record bound to this build's commit and
/// nonce, verified through `verify_release_signature`. `outcome == "PASSED"`, the
/// commit and the nonce are all checked by [`crate::evidence_ledger`]'s verifier;
/// this function only supplies the transcript.
///
/// **CI holds no operator key by ratified design**, with a written reason and no
/// dev-key fallback. So in CI this leg is `INDETERMINATE` — not failed, and NOT
/// "no leg has ever reached this state": 27 legs have, on the operator lane. The
/// operator lane is what produces the signed claim.
pub fn verify_capture_signature(root: &Path) -> Result<String, String> {
    let transcript = fs::read_to_string(root.join(CAPTURE_TRANSCRIPT))
        .map_err(|e| format!("no signed evidence transcript at {CAPTURE_TRANSCRIPT}: {e}"))?;
    let binding = crate::evidence_ledger::BuildBinding::for_run(GATE)?;
    let verifier = crate::evidence_ledger::EvidenceVerifier::load(binding)?;
    if !verifier.key_available() {
        return Err(verifier.key_reason().to_string());
    }
    let records: Vec<_> = crate::evidence_ledger::parse_records(&transcript)
        .into_iter()
        .filter(|r| r.payload.gate == GATE)
        .collect();
    let record = records.first().ok_or_else(|| {
        format!("{CAPTURE_TRANSCRIPT} carries no MAOS-EVIDENCE-V1 record for {GATE}")
    })?;
    verifier.verify(record)?;
    Ok(record.payload.test.clone())
}

pub fn run(json: bool) -> Result<(), String> {
    run_with_root(json, Path::new("."))
}

pub fn run_with_root(json: bool, root: &Path) -> Result<(), String> {
    let judgement = judge(root);
    let findings = &judgement.findings;
    let oracle_green = findings.is_empty();
    // ONE binding class for the whole gate. Hermetic: a RED oracle hard-fails at
    // HEAD regardless of CURRENT_PHASE. The paid run is a validated capture, never
    // a substrate-gated leg — see the module docs.
    let dev_blocks = dev_enforced_red_blocks(BindingClass::Blocking, true);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": "check-j1-two-host-signed-run",
                "passed": oracle_green,
                "oracle_green": oracle_green,
                "binding": "Blocking",
                "legs": ledger_leg_names(),
                "leg_audits": judgement.audits,
                // ABSENT is honest: CI holds no operator key, two hosts or funded
                // API key by ratified design. The control is that nothing may CLAIM
                // the run while this is false — never a binding class that cannot fire.
                "paid_run_capture_present": judgement.capture_present,
                "two_host_signed_run_claimed": judgement.capture_present && oracle_green,
                "claim_scope": CLAIM_SCOPE,
                "enrolled_vectors": judgement.enrolled,
                "findings": findings.iter().map(|f| serde_json::json!({
                    "check": f.check, "detail": f.detail,
                })).collect::<Vec<_>>(),
            })
        );
    } else if oracle_green {
        eprintln!(
            "check-j1-two-host-signed-run: PASS — signing identity repaired, host \
             discriminator under the signature, reconciliation refuses a shared root, bundle \
             schema enforced, faults typed and bounded, nothing Duplicate until durable, pin \
             refusals journaled both sides, stored rows scanned; paid-run capture present = \
             {} (absent is honest — the claim is refused, not assumed)",
            judgement.capture_present
        );
    } else {
        eprintln!(
            "check-j1-two-host-signed-run: BLOCKING — {} finding(s):",
            findings.len()
        );
        for f in findings {
            eprintln!("  [FAIL] {} — {}", f.check, f.detail);
        }
    }

    if oracle_green || !dev_blocks {
        Ok(())
    } else {
        Err(format!(
            "check-j1-two-host-signed-run: {} finding(s) — the two-host signed run is not \
             provably bounded by what it proves",
            findings.len()
        ))
    }
}
