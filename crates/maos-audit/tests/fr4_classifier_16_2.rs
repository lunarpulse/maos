#![forbid(unsafe_code)]

//! Story 16-2 / AC5 (D-16-2-G) — the FR4 classifier's vectors, and the
//! INVENTORY DOORBELL that keeps "every kernel row is classified" true.
//!
//! Every vector builds rows from the MEASURED writer payloads (T0's table),
//! never from the classifier's own table read back: a row the real writer
//! would have written, classified the way the audit read renders it.

use maos_audit::fr4_classifier::{
    classify_fr4_row, Fr4RowDisposition, TokenColumn, WriterIntent, NON_CALL_KINDS, WRITER_SHAPES,
};
use maos_audit::{to_fr4_ndjson, to_fr4_plain, to_plain, AuditEntry, Fr4SchemaError};

fn entry(kind: &str, intent: &str, pid: u32, token: Option<&str>, payload: &str) -> AuditEntry {
    AuditEntry {
        frame_id_hex: format!("{:032x}", pid),
        timestamp_ns: 1,
        spirit_pid: pid,
        boot_nonce: 7,
        capability_token_hex: token.map(|t| t.to_string()),
        kind: kind.to_string(),
        intent: intent.to_string(),
        payload: payload.to_string(),
        redaction: None,
    }
}

fn tokenless(kind: &str, intent: &str, pid: u32, payload: &str) -> AuditEntry {
    entry(kind, intent, pid, None, payload)
}

fn call_with_token(kind: &str, intent: &str, pid: u32) -> AuditEntry {
    entry(kind, intent, pid, Some(&"a".repeat(64)), "{}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Violations — a call with no token stays a violation (fail-closed)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn tokenless_inference_call_is_a_violation() {
    let row = tokenless("inference.call", "infer:ollama->replay:default", 1, "{}");
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
    assert!(matches!(
        maos_audit::project_to_fr4(&row),
        Err(Fr4SchemaError::MissingCapabilityToken)
    ));
}

#[test]
fn lifecycle_load_without_spirit_id_is_a_violation() {
    // The measured `lifecycle.load` shape minus its `spirit_id` — a payload
    // the writer never writes. EXACT match refused.
    let row = tokenless(
        "capability.invocation",
        "lifecycle.load",
        1,
        r#"{"lifecycle_event":"Load","spirit_pid":1}"#,
    );
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
}

#[test]
fn lifecycle_start_with_foreign_pid_is_a_violation() {
    // `spirit_pid` in the payload must equal the ROW's pid.
    let row = tokenless(
        "capability.invocation",
        "lifecycle.start",
        1,
        r#"{"lifecycle_event":"Start","spirit_pid":9}"#,
    );
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
}

#[test]
fn lifecycle_bogus_is_a_violation_the_prefix_is_refused() {
    let row = tokenless(
        "capability.invocation",
        "lifecycle.bogus",
        1,
        r#"{"lifecycle_event":"Bogus","spirit_id":"hello-spirit"}"#,
    );
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
}

#[test]
fn tokenless_shell_turn_is_a_violation() {
    // `shell.turn` carries a token; a tokenless one is a forged row.
    let row = tokenless(
        "capability.invocation",
        "shell.turn",
        1,
        r#"{"user":"hi","response":"hi"}"#,
    );
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
}

#[test]
fn a_call_shaped_writer_stays_a_violation_at_a_spirit_pid() {
    // maos-iac's `log.recall` sites carry disposition `Call` — never exempt.
    // The measured payload shape does not matter; the disposition rules.
    let row = tokenless(
        "capability.invocation",
        "log.recall",
        1,
        r#"{"scope":"cross-wall","entries":3}"#,
    );
    assert_eq!(classify_fr4_row(&row), Fr4RowDisposition::Call);
    assert!(maos_audit::project_to_fr4(&row).is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// Non-calls — set (a) kinds, then every NonCall writer shape
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn every_never_call_kind_is_a_non_call() {
    for kind in NON_CALL_KINDS {
        let row = tokenless(kind, "anything", 1, "{}");
        assert_eq!(
            classify_fr4_row(&row),
            Fr4RowDisposition::NonCallKernelEvent,
            "kind {kind} must be a non-call kernel event"
        );
    }
}

/// One row per `NonCall` writer-shape entry, built from the MEASURED payload
/// (T0) — not read back from the table's own shape list.
#[test]
fn every_noncall_writer_shape_classifies_from_its_measured_payload() {
    // (kind, token, intent, payload, row pid) per entry, in WRITER_SHAPES
    // order. Kind 7 / tokenless is the historical default; the kind-1 FR50
    // dispositions (D-16-3-J (4)) pin their own kind and token column.
    let measured: &[(&str, Option<&str>, &str, &str, u32, &str)] = &[
        (
            "capability.invocation",
            None,
            "lifecycle.admit",
            r#"{"spirit_id":"hello-spirit"}"#,
            1,
            "scheduler load ord0",
        ),
        (
            "capability.invocation",
            None,
            "lifecycle.load",
            r#"{"lifecycle_event":"Load","spirit_id":"hello-spirit","spirit_pid":1}"#,
            1,
            "scheduler load ord1",
        ),
        (
            "capability.invocation",
            None,
            "lifecycle.start",
            r#"{"lifecycle_event":"Start","spirit_pid":1}"#,
            1,
            "start",
        ),
        (
            "capability.invocation",
            None,
            "lifecycle.pause",
            r#"{"lifecycle_event":"Pause","spirit_pid":1}"#,
            1,
            "pause",
        ),
        (
            "capability.invocation",
            None,
            "lifecycle.resume",
            r#"{"lifecycle_event":"Resume","spirit_pid":1}"#,
            1,
            "resume",
        ),
        (
            "capability.invocation",
            None,
            "lifecycle.unload",
            r#"{"lifecycle_event":"Unload","spirit_pid":1}"#,
            1,
            "unload",
        ),
        (
            "capability.invocation",
            None,
            "lifecycle.journal",
            r#"{"lifecycle_event":"Load","spirit_id":"hello-spirit"}"#,
            0,
            "journal_lifecycle",
        ),
        (
            "decision.dispatch",
            None,
            "lifecycle.Load",
            r#"{"director":"operator","spirit_id":"hello-spirit","verb":"Load"}"#,
            1,
            "verb resolver decision dispatch",
        ),
        (
            // Story 16-5: the task's 16-byte token remains in the payload;
            // the 32-byte capability-token column stays NULL.
            "task.complete",
            None,
            "task.orphaned",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","exit_signal":9,"exit_code":null,"stderr_tail":null,"cause":"fault.signaled","in_flight_tokens":[[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]],"disposition":"nack"}"#,
            4242,
            "crash task.orphaned (kind 1)",
        ),
        (
            // Story 16-6 (AC2(g)(ii)) — `unload`'s inline FR50 orphan row.
            // `exit_signal`/`exit_code` are null and `cause` names the hook,
            // not an OS fault: a failed `on_unload` is not a process death
            // and must not be logged as one.
            "task.complete",
            None,
            "task.orphaned",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","exit_signal":null,"exit_code":null,"stderr_tail":"hook on_unload panicked","cause":"on_unload_hook_failure","in_flight_tokens":[[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]],"disposition":"nack"}"#,
            4242,
            "unload task.orphaned (kind 1)",
        ),
        (
            "capability.invocation",
            None,
            "lifecycle.crash",
            r#"{"lifecycle_event":"Crash","spirit_id":"hello-spirit","spirit_pid":1,"cause":"hang"}"#,
            1,
            "crash",
        ),
        (
            "capability.invocation",
            None,
            "telemetry.self",
            "self_telemetry: pid=1 window=[0,1]",
            1,
            "self telemetry (non-JSON)",
        ),
        (
            "capability.invocation",
            None,
            "schedule.fire:daily-9",
            r#"{"spirit_id":"butler","schedule_id":"daily-9","fired_at_ns":42,"compliance_claim_ref":null,"side_effect_token_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"principal_revocability":false}"#,
            1,
            "schedule fire",
        ),
        (
            "capability.invocation",
            None,
            "hook.budget.on_idle",
            r#"{"spirit_pid":1,"hook_name":"on_idle","wall_ns":5,"cap_seconds":10,"ratio_breached":0.5}"#,
            1,
            "hook budget",
        ),
        (
            "capability.invocation",
            None,
            "spirit.quarantine_requested",
            r#"{"spirit_id":"hello-spirit","spirit_pid":1,"quarantine_requested":true}"#,
            1,
            "quarantine",
        ),
        (
            "capability.invocation",
            None,
            "spirit.upgrade",
            r#"{"spirit_id":"butler","predecessor_version":"1","successor_version":"2","policy":"forward-only","outcome":"completed","latency_ns":5,"halt_receipts_produced":0}"#,
            1,
            "upgrade",
        ),
        (
            "capability.invocation",
            None,
            "cli.subprocess.exit",
            r#"{"event":"cli_subprocess_exit","cli_child_pid":4242,"exit_cause":"exit","is_crash":false}"#,
            1,
            "cli runtime",
        ),
        (
            // AC2(b) — the resolver's structured orphan row at pid 0.
            "task.complete",
            None,
            "task.orphaned",
            r#"{"halt_id":"01HALT","disposition":"accepted_halt"}"#,
            0,
            "resolver orphan (kind 1)",
        ),
        (
            "capability.invocation",
            None,
            "cli.subprocess.exit",
            r#"{"cli":"echo","exit_code":0,"bytes":12,"duration_ms":4}"#,
            0,
            "cli smoke arm",
        ),
        (
            // D-16-3-J (4) — the tokenless kind-1 FR50 dispositions at pid 0.
            "task.complete",
            None,
            "task.nacked",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","capability_token":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2]}"#,
            0,
            "task.nacked (kind 1)",
        ),
        (
            "task.complete",
            None,
            "task.escalated",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","capability_token":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3]}"#,
            0,
            "task.escalated (kind 1)",
        ),
        (
            "task.complete",
            None,
            "task.reassigned",
            r#"{"task_id":"task-worker-1","originator_spirit_id":"butler","capability_token":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4],"replica_spirit_id":"butler-replica"}"#,
            0,
            "task.reassigned (kind 1)",
        ),
        (
            "task.complete",
            None,
            "distillate.redacted",
            r#"{"principal_id":"operator","redacted_distillate_frame_id":"01ab01ab01ab01ab01ab01ab01ab01ab"}"#,
            0,
            "distillate.redacted (kind 1)",
        ),
    ];
    let noncall_entries: Vec<&maos_audit::fr4_classifier::WriterShapeEntry> =
        WRITER_SHAPES.iter().filter(|w| w.shape.is_some()).collect();
    // Story 16-6 review 2026-09-20 — these same-shaped rows have distinct
    // teardown writers. Pin both doorbell sites so removing unload's writer
    // cannot still pass by classifying crash-detector's row.
    let task_orphan_sites: Vec<_> = noncall_entries
        .iter()
        .filter(|writer| writer.intent == WriterIntent::Exact("task.orphaned"))
        .map(|writer| writer.site)
        .collect();
    assert_eq!(
        task_orphan_sites.len(),
        3,
        "the crash, unload, and resolver task.orphaned writers are all required"
    );
    assert!(
        task_orphan_sites.contains(&("crash_detector.rs", "handle_crash", 0))
            && task_orphan_sites.contains(&("scheduler_loop.rs", "orphan_in_flight_tasks", 0,)),
        "the crash and unload task.orphaned writer sites must both remain classified: \
         {task_orphan_sites:?}"
    );
    assert_eq!(
        noncall_entries.len(),
        measured.len(),
        "the table and the measured fixtures must stay 1:1 — update both together"
    );
    let mut optional_rows: Vec<((&str, &str, u32), bool)> = Vec::new();
    for (writer, (kind, token, intent, payload, pid, label)) in noncall_entries.iter().zip(measured)
    {
        // The intent grammar must accept the measured intent.
        assert!(
            writer.intent.matches(intent),
            "{label}: table intent {:?} must match measured {intent:?}",
            writer.intent
        );
        // The pinned token column must accept the measured token presence.
        // `Optional` means production MAY emit either shape — but the measured
        // fixture is still pinned exactly, so a production change that flips
        // the measured presence reds here instead of passing silently.
        assert!(
            match writer.token {
                TokenColumn::Absent => token.is_none(),
                TokenColumn::Present => token.is_some(),
                TokenColumn::Optional => true,
            },
            "{label}: the entry's token column must accept the measured token presence"
        );
        if writer.token == TokenColumn::Optional {
            // D-16-3-J (4): the Optional column is not unpinned — the measured
            // fixture's presence is recorded, and the payload must pair it
            // with a matching `in_flight_tokens` state, so a production flip
            // of either reds here.
            optional_rows.push((writer.site, token.is_some()));
            assert_eq!(
                token.is_some(),
                !payload.contains("\"in_flight_tokens\":[]"),
                "{label}: an Optional-token row must pair token presence with \
                 a non-empty in_flight_tokens payload"
            );
        }
        // The row is built with the MEASURED kind, so a NonCall verdict
        // proves the entry's kind path — not just the shape.
        let row = entry(kind, intent, *pid, *token, payload);
        assert_eq!(
            classify_fr4_row(&row),
            Fr4RowDisposition::NonCallKernelEvent,
            "{label}: the measured payload must classify as a non-call"
        );
    }
    // Story 16-5 leaves no ambiguous optional-token writer shape.
    assert!(
        optional_rows.is_empty(),
        "all non-call token columns must be pinned present or absent"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Renderers — NDJSON emits calls only; plain renders everything with intent
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ndjson_over_a_mixed_set_emits_exactly_the_call_rows() {
    let rows = vec![
        call_with_token("inference.call", "infer:replay", 1),
        tokenless(
            "epistemic.halt",
            "task.acceptance_criterion.ambiguous",
            1,
            r#"{"halt_id":"01M2GGRA6C7KF13T46SKYC87EA","tag":"task.acceptance_criterion.ambiguous","value":1.0,"threshold":null,"policy_id":"hello-spirit.ambiguity","derived_from":"shell.directive"}"#,
        ),
        tokenless(
            "cost.attribution",
            "cost:inference-attribution",
            1,
            r#"{"usd":0.0}"#,
        ),
        tokenless(
            "capability.invocation",
            "lifecycle.load",
            1,
            r#"{"lifecycle_event":"Load","spirit_id":"hello-spirit","spirit_pid":1}"#,
        ),
        call_with_token("capability.invocation", "shell.turn", 1),
    ];
    let mut out = Vec::new();
    to_fr4_ndjson(rows, &mut out).expect("mixed set must not violate");
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "exactly the two call rows; got:\n{text}");
    for line in lines {
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        // Byte-equal to today's schema: the six keys, and a real token.
        assert!(parsed
            .get("capability_token")
            .is_some_and(|t| t.as_str().is_some()));
        assert!(parsed.get("call_id").is_some());
        assert!(parsed.get("spirit_pid").is_some());
        assert!(parsed.get("boot_nonce").is_some());
        assert!(parsed.get("call_type").is_some());
        assert!(parsed.get("timestamp_ns").is_some());
        assert_eq!(parsed.as_object().unwrap().len(), 6);
    }
    // The stderr count names each omitted kind (captured by the caller in
    // production; here the same message goes to the test's stderr — the
    // count content is asserted by the omission arithmetic above).
}

#[test]
fn plain_fr4_renders_every_row_with_its_intent() {
    let rows = vec![
        call_with_token("inference.call", "infer:replay", 1),
        tokenless(
            "epistemic.halt",
            "task.acceptance_criterion.ambiguous",
            1,
            "{}",
        ),
        tokenless(
            "telemetry.event",
            "operator.halt-resolve.op-abc:completed",
            1,
            "{}",
        ),
    ];
    let mut out = Vec::new();
    to_fr4_plain(rows, &mut out).expect("non-calls must not violate the plain table");
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("task.acceptance_criterion.ambiguous"));
    assert!(text.contains("operator.halt-resolve.op-abc:completed"));
    assert!(text.contains("infer:replay"));
    // The intent column exists in the header, TRAILING.
    let header = text.lines().next().unwrap();
    assert!(
        header.trim_end().ends_with("intent"),
        "the intent column must be last; header: {header}"
    );
}

#[test]
fn plain_table_gains_the_trailing_intent_column() {
    let mut out = Vec::new();
    to_plain(
        vec![call_with_token("inference.call", "some.intent:here", 1)],
        &mut out,
    )
    .unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("capability_token  intent"));
    assert!(text.contains("some.intent:here"));
}

// THE INVENTORY DOORBELL — every tokenless kind-7 / variable-kind / kind-1
// writer site, plus the lifecycle DecisionDispatch writer, in the tree has a
// disposition
// ─────────────────────────────────────────────────────────────────────────────

/// A writer call site the scanner derived from source. `file` is the file
/// BASENAME — the DOORBELL key component matched against `WRITER_SHAPES`;
/// `rel_path` is the workspace-relative path (set by `scan_tree`; direct
/// `scan_source` callers default it to the basename) carried only for
/// diagnostics and the basename-uniqueness guard below.
#[derive(Debug, PartialEq, Eq)]
struct ScannedSite {
    file: String,
    rel_path: String,
    f: String,
    ordinal: u32,
}

/// Token-argument position per writer method (0-based argument index).
fn token_arg_position(method: &str) -> Option<usize> {
    match method {
        "insert_frame_event" => Some(2),
        "insert_frame_event_with_sender" => Some(4),
        "insert_frame_event_with_correlation" => Some(2),
        "insert_frame_event_with_id" => Some(5),
        _ => None,
    }
}

/// Scan one file's source for `insert_frame_event*` call sites: paren-
/// balanced argument extraction, enclosing-fn resolution, `#[cfg(test)]`
/// spans skipped. Returns every site whose kind is literal
/// `CapabilityInvocation` with a literal `None` token, whose KIND is a
/// variable, whose kind is literal `TaskComplete` (kind 1 — D-16-3-J (5),
/// token-bearing included), or the lifecycle `DecisionDispatch` writer — the
/// sites that demand a disposition.
fn scan_source(src: &str, file: &str) -> Vec<ScannedSite> {
    // Compute #[cfg(test)] line spans (item start to its closing brace, approximated
    // by the next top-level closing brace at column 0).
    let lines: Vec<&str> = src.lines().collect();
    let mut test_spans: Vec<(usize, usize)> = Vec::new(); // 0-based inclusive
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "#[cfg(test)]" {
            // find the item's opening brace line, then match braces
            let mut j = i + 1;
            while j < lines.len() && !lines[j].contains('{') {
                if lines[j].trim().starts_with("fn ") || lines[j].contains("fn ") {
                    break; // a bare test fn — treat one line
                }
                j += 1;
            }
            let mut depth = 0usize;
            let mut k = j;
            let mut opened = false;
            while k < lines.len() {
                depth += lines[k].matches('{').count();
                depth -= lines[k].matches('}').count();
                if lines[k].contains('{') {
                    opened = true;
                }
                if opened && depth == 0 {
                    break;
                }
                k += 1;
            }
            test_spans.push((i, k.min(lines.len() - 1)));
        }
    }
    let in_test = |idx: usize| test_spans.iter().any(|(a, b)| idx >= *a && idx <= *b);

    let mut sites: Vec<ScannedSite> = Vec::new();
    // (file, fn) → count of ALL insert_frame_event* call sites in that fn so
    // far, ANY kind/token — the ordinal is stable under kind changes.
    let mut call_counts: std::collections::BTreeMap<(String, String), u32> =
        std::collections::BTreeMap::new();
    let bytes = src.as_bytes();
    let mut search = 0usize;
    while let Some(rel) = src[search..].find("insert_frame_event") {
        let at = search + rel;
        // Method name must be one of the four writers, called as `.method(`.
        let after = &src[at..];
        let method = [
            "insert_frame_event_with_sender",
            "insert_frame_event_with_correlation",
            "insert_frame_event_with_id",
            "insert_frame_event",
        ]
        .iter()
        .find(|m| after.starts_with(**m))
        .copied();
        let Some(method) = method else {
            search = at + "insert_frame_event".len();
            continue;
        };
        let open = at + method.len();
        if !src[open..].trim_start().starts_with('(') {
            search = open;
            continue;
        }
        // Find the closing paren (balanced).
        let mut depth = 0i32;
        let mut idx = open;
        let mut in_str = false;
        let mut prev = 0u8;
        while idx < bytes.len() {
            let c = bytes[idx];
            if in_str {
                if c == b'"' && prev != b'\\' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == b'(' {
                depth += 1;
            } else if c == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            prev = c;
            idx += 1;
        }
        let args_src = &src[open + 1..idx.min(src.len())];
        let line_no = src[..at].matches('\n').count();
        search = idx + 1;

        if in_test(line_no) {
            continue;
        }

        // Resolve the enclosing fn BEFORE any filtering — the ordinal counts
        // every writer call in the fn.
        let mut f = String::from("<unknown>");
        for line in lines[..=line_no].iter().rev() {
            let t = line.trim_start();
            if t.starts_with("fn ")
                || (t.starts_with("pub ") && t.contains(" fn "))
                || (t.starts_with("pub(") && t.contains(" fn "))
                || t.starts_with("async fn ")
                || (t.starts_with("pub ") && t.contains("async fn"))
            {
                let name: String = t
                    .split("fn ")
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    f = name;
                    break;
                }
            }
        }
        let key = (file.to_string(), f.clone());
        let ordinal = {
            let count = call_counts.entry(key).or_insert(0);
            let ord = *count;
            *count += 1;
            ord
        };

        // Split top-level commas.
        let mut args: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut d = 0i32;
        let mut instr = false;
        let mut pr = 0u8;
        for ch in args_src.chars() {
            if instr {
                cur.push(ch);
                if ch == '"' && pr != b'\\' {
                    instr = false;
                }
            } else if ch == '"' {
                instr = true;
                cur.push(ch);
            } else if ch == '(' || ch == '<' || ch == '[' {
                d += 1;
                cur.push(ch);
            } else if ch == ')' || ch == '>' || ch == ']' {
                d -= 1;
                cur.push(ch);
            } else if ch == ',' && d == 0 {
                args.push(cur.trim().to_string());
                cur = String::new();
            } else {
                cur.push(ch);
            }
            pr = ch as u8;
        }
        if !cur.trim().is_empty() {
            args.push(cur.trim().to_string());
        }

        let Some(token_pos) = token_arg_position(method) else {
            continue;
        };
        let token_arg = args.get(token_pos).cloned().unwrap_or_default();
        let kind_arg = args.first().cloned().unwrap_or_default();

        let kind_is_capability_invocation = kind_arg
            .rsplit("::")
            .next()
            .is_some_and(|last| last.starts_with("CapabilityInvocation"));
        // D-16-3-J (5) — kind-1 `TaskComplete` sites demand a disposition
        // too, token-bearing or not: kind-1 rows carry their own entries
        // now, with the token presence pinned per entry.
        let kind_is_task_complete = kind_arg
            .rsplit("::")
            .next()
            .is_some_and(|last| last.starts_with("TaskComplete"));
        let kind_is_decision_dispatch = kind_arg
            .rsplit("::")
            .next()
            .is_some_and(|last| last.starts_with("DecisionDispatch"));
        let kind_is_variable = !kind_arg.contains("::");
        if !kind_is_capability_invocation
            && !kind_is_variable
            && !kind_is_task_complete
            && !kind_is_decision_dispatch
        {
            continue;
        }
        let tokenless = token_arg == "None";
        if kind_is_capability_invocation && !tokenless {
            continue; // token-bearing call row — not an exemption question
        }
        if kind_is_variable && !tokenless {
            continue;
        }

        sites.push(ScannedSite {
            file: file.to_string(),
            rel_path: file.to_string(),
            f,
            ordinal,
        });
    }
    sites
}

fn workspace_src(rel: &str) -> String {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(rel);
    std::fs::read_to_string(&root).unwrap_or_else(|e| panic!("read {}: {e}", root.display()))
}

fn scan_tree() -> Vec<ScannedSite> {
    let mut sites = Vec::new();
    let roots = [
        "crates/maos-kernel-core/src",
        "crates/maos-bin/src",
        "crates/maos-iac/src",
    ];
    for root in roots {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(root);
        let mut stack = vec![base.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            for ent in read.flatten() {
                let path = ent.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let src = std::fs::read_to_string(&path).unwrap();
                    let file = path.file_name().unwrap().to_string_lossy().into_owned();
                    // Carry the workspace-relative path alongside the
                    // basename key: same-named files in different directories
                    // must stay distinguishable to the uniqueness guard.
                    let rel = path.strip_prefix(&base).unwrap_or(&path);
                    let rel_path = format!("{root}/{}", rel.display());
                    for mut site in scan_source(&src, &file) {
                        site.rel_path = rel_path.clone();
                        sites.push(site);
                    }
                }
            }
        }
    }
    sites
}

/// Every writer site the doorbell demands a disposition for — tokenless
/// kind-7, variable-kind, and kind-1 `TaskComplete` (16-3 / D-16-3-J (5))
/// — has one. Proven red: adding an unlisted tokenless kind-7 site to a
/// scratch copy of a scanned file reds the scanner (below); adding a
/// kind-1 `TaskComplete` site with no entry reds it the same way.
#[test]
fn every_writer_site_demanding_a_disposition_has_one() {
    let sites = scan_tree();
    assert!(
        !sites.is_empty(),
        "the scanner found nothing — itself a red"
    );

    // Story 16-2 / D-16-2-G — basename-uniqueness guard. The DOORBELL key's
    // `file` component is a bare BASENAME (WRITER_SHAPES keys are bare file
    // names, and the table is production code at a zero-headroom ceiling, so
    // the ~45 entries are NOT rewritten to full paths). The basename key is
    // therefore sound ONLY while no two distinct files that CONTRIBUTE SITES
    // share a basename: if they did, a future tokenless kind-7 site whose
    // `(basename, fn, ordinal)` coincides with an entry measured in the other
    // directory would match it and the doorbell would stay GREEN — a silent
    // false-green in a control chartered as non-degradable (§15 R2). If this
    // fires, disambiguate the key to workspace-relative paths in BOTH the
    // table and `scan_tree` in one cutover.
    let mut basename_owner: std::collections::BTreeMap<&str, &str> =
        std::collections::BTreeMap::new();
    for site in &sites {
        if let Some(first) = basename_owner.insert(site.file.as_str(), site.rel_path.as_str()) {
            assert!(
                first == site.rel_path.as_str(),
                "basename collision in the DOORBELL key: `{}` and `{}` are distinct \
                 scanned files that both contribute writer sites yet share the \
                 basename `{}` — `(file, fn, ordinal)` cannot tell them apart. \
                 Disambiguate the key (workspace-relative paths in WRITER_SHAPES \
                 and scan_tree) before trusting a green run.",
                first,
                site.rel_path,
                site.file
            );
        }
    }
    for site in &sites {
        let listed = WRITER_SHAPES.iter().any(|w| {
            w.site.0 == site.file.as_str()
                && w.site.1 == site.f.as_str()
                && w.site.2 == site.ordinal
        });
        assert!(
            listed,
            "writer site demanding a disposition (tokenless kind-7, variable-kind, \
             or kind-1 TaskComplete) ({}, {}, ord {}) has NO disposition — classify \
             it in WRITER_SHAPES (NonCall with its measured kind, token column and \
             payload shape, or Call)",
            site.file, site.f, site.ordinal
        );
    }
    // And the reverse: no stale table entries for sites that no longer exist.
    for writer in WRITER_SHAPES {
        let exists = sites
            .iter()
            .any(|s| s.file == writer.site.0 && s.f == writer.site.1 && s.ordinal == writer.site.2);
        assert!(
            exists,
            "WRITER_SHAPES entry ({}, {}, ord {}) matches no scanned site — the \
             writer moved or was deleted; update the table",
            writer.site.0, writer.site.1, writer.site.2
        );
    }
}

/// The proven-red control for the doorbell: an UNLISTED tokenless kind-7
/// site planted in a scratch copy of a SCANNED file is found by the same
/// scanner and matches no table entry. The scratch file is named with a
/// REAL scanned basename (`scheduler_loop.rs` — live WRITER_SHAPES entries,
/// fns `load`/`start`/`pause`/…), so `assert!(!listed)` performs a genuine
/// (listed-file, unlisted-fn) lookup against the live table. A fabricated
/// filename no entry could ever name would make the control tautological:
/// it could never fail, proving nothing about the table.
#[test]
fn the_scanner_reds_on_an_unlisted_scratch_site() {
    let src = r#"
fn falsifier_unlisted_writer(pid: u32) {
    let tl = tl();
    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        pid,
        None,
        "scratch.unlisted",
        b"{}",
        FrameOrigin::Kernel,
    );
}
"#;
    let sites = scan_source(src, "scheduler_loop.rs");
    assert_eq!(sites.len(), 1, "the scanner must find the planted site");
    assert_eq!(sites[0].f, "falsifier_unlisted_writer");
    let listed = WRITER_SHAPES.iter().any(|w| {
        w.site.0 == "scheduler_loop.rs" && w.site.1 == "falsifier_unlisted_writer" && w.site.2 == 0
    });
    assert!(!listed, "the planted site is unlisted — the doorbell reds");
}

/// `#[cfg(test)]` items are skipped (fourth read: `transparency_log.rs:2172`).
#[test]
fn cfg_test_items_are_skipped() {
    let src = r#"
fn prod(pid: u32) {
    tl.insert_frame_event(FrameKind::CapabilityInvocation, pid, None, "prod.site", b"{}", FrameOrigin::Kernel);
}

#[cfg(test)]
mod tests {
    fn helper() {
        tl.insert_frame_event(FrameKind::CapabilityInvocation, 0, None, "test.site", b"{}", FrameOrigin::Kernel);
    }
}
"#;
    let sites = scan_source(src, "x.rs");
    assert_eq!(sites.len(), 1, "only the prod site counts");
    assert_eq!(sites[0].f, "prod");
}
