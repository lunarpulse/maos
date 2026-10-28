#![forbid(unsafe_code)]

//! Story 19-2 (AC3, AC4, AC6; D-19-2-05/07 A) — CONTRACT tests for
//! `maosctl orchestrator recall` and `maosctl spirit pick` over the 16-1 door.
//!
//! The pick body is duplicated from `maos-control` on purpose
//! (`door_client.rs` module doc: no `maos-control` edge). This file pins the
//! client's half from both directions:
//!
//! 1. the WIRE — `POST /v1/orchestrator/<id>/recall` with NO body, and
//!    `POST /v1/spirits/<id>/pick` with exactly `{"pick":n,"work_mode":"…"}`,
//!    bearer from `control.json`; and
//! 2. the MAPPING — 200 prints the daemon's body verbatim (exit 0); every
//!    typed refusal (404 `spirit_not_loaded|butler_not_found`, 409
//!    `principal_unbound|no_pending_notification|pick_out_of_range|
//!    undeclared_action`, 500 `approval_log_failed|acceptance_ledger_failed`,
//!    502 `mcp_failed`) is exit 1 naming its code and status, 401 is 77, and
//!    the shared D-16-1-P matrix reaches both routes. A malformed pick (`0`,
//!    non-integer, blank `--work-mode`, bad id) is refused locally, exit 2.

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{assert_discovery_failures, assert_typed_matrix, FixtureDoor};
use maos_cli::door_client::OptionPickRequest;

const RECALL_ROUTE: &str = "/v1/orchestrator/orchestrator/recall";
const PICK_ROUTE: &str = "/v1/spirits/butler/pick";

fn recall() -> Vec<&'static str> {
    vec!["orchestrator", "recall", "--spirit", "orchestrator"]
}

fn pick(n: &'static str) -> Vec<&'static str> {
    vec!["spirit", "pick", "butler", n, "--work-mode", "focus"]
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn recall_posts_an_empty_body_and_prints_the_recalled_set_verbatim() {
    let body = r#"{"spirit_id":"orchestrator","recalled":[{"instruction_id":3,"text":"use the staging db"}],"pending":0}"#;
    let door = FixtureDoor::spawn_replying(200, body);
    let out = door.run(&recall());
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(
        out.stdout,
        format!("{body}\n").into_bytes(),
        "200 prints verbatim"
    );
    let requests = door.requests();
    assert_eq!(requests.len(), 1, "exactly one round trip");
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, RECALL_ROUTE);
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str()),
        "the bearer must be the token from control.json"
    );
    assert!(request.body.is_empty(), "a recall carries no body");
}

/// An empty buffer is a 200 with `recalled: []` — still exit 0, still
/// printed, never a refusal.
#[test]
fn an_empty_recall_is_a_success_not_a_refusal() {
    let body = r#"{"spirit_id":"orchestrator","recalled":[],"pending":0}"#;
    let door = FixtureDoor::spawn_replying(200, body);
    let out = door.run(&recall());
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(out.stdout, format!("{body}\n").into_bytes());
}

#[test]
fn pick_posts_exactly_the_option_pick_request() {
    let body = r#"{"spirit_id":"butler","action":"reminder","accepted":true,"effect":"stub (live send v0.4)","message":"Butler: Slack message queued"}"#;
    let door = FixtureDoor::spawn_replying(200, body);
    let out = door.run(&pick("2"));
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(
        out.stdout,
        format!("{body}\n").into_bytes(),
        "200 prints verbatim"
    );
    let requests = door.requests();
    assert_eq!(requests.len(), 1, "exactly one round trip");
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, PICK_ROUTE);
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str())
    );
    assert_eq!(request.content_type.as_deref(), Some("application/json"));
    // `deny_unknown_fields`: the deserialization is the exact-key-set check.
    let sent: OptionPickRequest =
        serde_json::from_slice(&request.body).expect("body is exactly OptionPickRequest");
    assert_eq!(
        sent,
        OptionPickRequest {
            pick: 2,
            work_mode: "focus".to_owned(),
        }
    );
    assert!(
        request.json_body()["pick"].is_u64(),
        "`pick` travels as a JSON integer — the route refuses anything else"
    );
}

struct Outcome {
    label: &'static str,
    status: u16,
    body: &'static str,
    exit: i32,
    stderr: &'static [&'static str],
}

const RECALL_OUTCOMES: &[Outcome] = &[
    Outcome {
        label: "404 spirit_not_loaded",
        status: 404,
        body: r#"{"error":"spirit_not_loaded","detail":"no spirit 'orchestrator'"}"#,
        exit: 1,
        stderr: &["orchestrator recall", "spirit_not_loaded", "HTTP 404"],
    },
    Outcome {
        label: "500 approval_log_failed",
        status: 500,
        body: r#"{"error":"approval_log_failed","detail":"recalled (not re-enqueued), a recall row failed"}"#,
        exit: 1,
        stderr: &[
            "orchestrator recall",
            "approval_log_failed",
            "not re-enqueued",
            "HTTP 500",
        ],
    },
    Outcome {
        label: "401 bad bearer",
        status: 401,
        body: r#"{"error":"unauthorized"}"#,
        exit: 77,
        stderr: &["HTTP 401"],
    },
];

const PICK_OUTCOMES: &[Outcome] = &[
    Outcome {
        label: "404 spirit_not_loaded",
        status: 404,
        body: r#"{"error":"spirit_not_loaded","detail":"no spirit 'butler'"}"#,
        exit: 1,
        stderr: &["spirit pick", "spirit_not_loaded", "HTTP 404"],
    },
    Outcome {
        label: "404 butler_not_found",
        status: 404,
        body: r#"{"error":"butler_not_found","detail":"no running Butler 'butler' at pid 4"}"#,
        exit: 1,
        stderr: &["spirit pick", "butler_not_found", "HTTP 404"],
    },
    Outcome {
        label: "409 principal_unbound",
        status: 409,
        body: r#"{"error":"principal_unbound","detail":"this daemon was booted with no Butler principal"}"#,
        exit: 1,
        stderr: &["spirit pick", "principal_unbound", "HTTP 409"],
    },
    Outcome {
        label: "409 no_pending_notification",
        status: 409,
        body: r#"{"error":"no_pending_notification","detail":"nothing pending"}"#,
        exit: 1,
        stderr: &["spirit pick", "no_pending_notification", "HTTP 409"],
    },
    Outcome {
        label: "409 pick_out_of_range",
        status: 409,
        body: r#"{"error":"pick_out_of_range","detail":"pick 9 is out of range"}"#,
        exit: 1,
        stderr: &["spirit pick", "pick_out_of_range", "HTTP 409"],
    },
    Outcome {
        label: "409 undeclared_action",
        status: 409,
        body: r#"{"error":"undeclared_action","detail":"no action declared"}"#,
        exit: 1,
        stderr: &["spirit pick", "undeclared_action", "HTTP 409"],
    },
    Outcome {
        label: "500 acceptance_ledger_failed",
        status: 500,
        body: r#"{"error":"acceptance_ledger_failed","detail":"ledger write failed"}"#,
        exit: 1,
        stderr: &["spirit pick", "acceptance_ledger_failed", "HTTP 500"],
    },
    Outcome {
        label: "502 mcp_failed",
        status: 502,
        body: r#"{"error":"mcp_failed","detail":"pick 1 was recorded; its effect failed: timeout"}"#,
        exit: 1,
        stderr: &["spirit pick", "mcp_failed", "was recorded", "HTTP 502"],
    },
    Outcome {
        label: "401 bad bearer",
        status: 401,
        body: r#"{"error":"unauthorized"}"#,
        exit: 77,
        stderr: &["HTTP 401"],
    },
];

fn assert_outcomes(args: &[&str], route: &str, outcomes: &[Outcome]) {
    for outcome in outcomes {
        let door = FixtureDoor::spawn_replying(outcome.status, outcome.body);
        let out = door.run(args);
        let err = stderr(&out);
        assert_eq!(
            out.status.code(),
            Some(outcome.exit),
            "[{}]: stderr: {err}",
            outcome.label
        );
        assert!(
            out.stdout.is_empty(),
            "[{}]: a refusal prints no body",
            outcome.label
        );
        for needle in outcome.stderr {
            assert!(
                err.contains(needle),
                "[{}]: missing {needle:?} in {err}",
                outcome.label
            );
        }
        let requests = door.requests();
        assert_eq!(requests.len(), 1, "[{}]", outcome.label);
        assert_eq!(requests[0].path, route, "[{}]", outcome.label);
    }
}

#[test]
fn recall_outcomes_map_to_typed_exits() {
    assert_outcomes(&recall(), RECALL_ROUTE, RECALL_OUTCOMES);
}

#[test]
fn pick_outcomes_map_to_typed_exits() {
    assert_outcomes(&pick("1"), PICK_ROUTE, PICK_OUTCOMES);
}

#[test]
fn recall_and_pick_reach_the_d16_1_p_matrix() {
    assert_typed_matrix(&recall(), "POST", RECALL_ROUTE);
    assert_typed_matrix(&pick("1"), "POST", PICK_ROUTE);
}

#[test]
fn recall_and_pick_discovery_failures_map_to_typed_exits() {
    assert_discovery_failures(&recall());
    assert_discovery_failures(&pick("1"));
}

/// A pick of 0, a non-integer or negative pick, a blank or missing
/// `--work-mode`, and an id outside the route charset are refused locally
/// (exit 2), never sent; so is a recall with a bad or missing `--spirit`.
#[test]
fn malformed_picks_and_recalls_are_refused_before_the_round_trip() {
    let door = FixtureDoor::spawn();
    for args in [
        vec!["spirit", "pick", "butler", "0", "--work-mode", "focus"],
        vec!["spirit", "pick", "butler", "-1", "--work-mode", "focus"],
        vec!["spirit", "pick", "butler", "two", "--work-mode", "focus"],
        vec!["spirit", "pick", "butler", "1.5", "--work-mode", "focus"],
        vec!["spirit", "pick", "butler", "1", "--work-mode", "   "],
        vec!["spirit", "pick", "butler", "1", "--work-mode", ""],
        vec!["spirit", "pick", "butler", "1"],
        vec!["spirit", "pick", "bad/id", "1", "--work-mode", "focus"],
        vec!["orchestrator", "recall", "--spirit", "bad/id"],
        vec!["orchestrator", "recall"],
    ] {
        let out = door.run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
        assert!(out.stdout.is_empty(), "{args:?}: nothing printed");
    }
    let blank = door.run(&["spirit", "pick", "butler", "1", "--work-mode", "  "]);
    assert!(
        stderr(&blank).contains("--work-mode must be non-blank"),
        "a blank work mode names the flag: {}",
        stderr(&blank)
    );
    assert_eq!(
        door.request_count(),
        0,
        "a locally refused pick or recall never reaches the door"
    );
}

/// The duplicated pick shape, pinned as literal JSON — the bytes the
/// `maos-control` `"pick"` arm must accept.
#[test]
fn the_pick_wire_shape_is_pinned_as_literal_json() {
    let request: OptionPickRequest =
        serde_json::from_str(r#"{"pick":3,"work_mode":"deep work"}"#).expect("pick shape");
    assert_eq!(
        request,
        OptionPickRequest {
            pick: 3,
            work_mode: "deep work".to_owned()
        }
    );
    assert_eq!(
        serde_json::to_string(&request).expect("re-serialize"),
        r#"{"pick":3,"work_mode":"deep work"}"#,
        "field order and spelling"
    );
    for refused in [
        r#"{"pick":"3","work_mode":"focus"}"#,
        r#"{"pick":-1,"work_mode":"focus"}"#,
        r#"{"pick":3}"#,
        r#"{"work_mode":"focus"}"#,
        r#"{"pick":3,"work_mode":"focus","principal":"ada"}"#,
    ] {
        assert!(
            serde_json::from_str::<OptionPickRequest>(refused).is_err(),
            "{refused}"
        );
    }
}
