#![forbid(unsafe_code)]

//! Story 22-5 — CONTRACT tests for `maosctl agents` and `maosctl agents show` (AC1, AC2).
//!
//! The wire is what the client put on the loopback fixture door; the rendering and exit codes are
//! the production client's reading of scripted replies. The fixture is hand-rolled because
//! `maos-cli` must not gain a `maos-control` edge (`dep_kernel_core_free_test.rs`).

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{FixtureDoor, Reply};

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn page(boot: &str, pid: u32, rows: &str) -> Reply {
    Reply::new(
        200,
        &format!(r#"{{"spirit_id":"butler","pid":{pid},"boot_nonce":"{boot}","rows":[{rows}]}}"#),
    )
}

fn tl(rowid: u64, ts: u64, kind: &str, summary: &str) -> String {
    format!(
        r#"{{"source":"tl","rowid":{rowid},"timestamp_ns":{ts},"kind":"{kind}","intent":"i.{rowid}","frame_id":"abcd1234","from":"src","to":"dst","origin":0,"summary":"{summary}"}}"#
    )
}

fn approval(rowid: u64, ts: u64) -> String {
    format!(
        r#"{{"source":"approval","rowid":{rowid},"timestamp_ns":{ts},"kind":"approval","intent":"halt.resolve","frame_id":"-","from":"operator","to":"butler","origin":0,"summary":"decision=granted resolution=accepted_halt"}}"#
    )
}

#[test]
fn agents_prints_eight_tab_columns_per_spirit_with_no_header() {
    let body = r#"[
        {"id":"butler","pid":3,"state":"Running","form":"in-process","originator":"operator",
         "task":"task-1","more_tasks":2,"age_secs":41,"spend":{"in":120,"out":30}},
        {"id":"idle","pid":4,"state":"Loaded","form":"unknown","originator":null,
         "task":null,"more_tasks":0,"age_secs":null,"spend":null},
        {"id":"evil","pid":5,"state":"Running","form":"wasm","originator":"a\tb\nc",
         "task":"t","more_tasks":0,"age_secs":0,"spend":{"in":0,"out":0}}
    ]"#;
    let door = FixtureDoor::spawn_replying(200, body);
    let out = door.run(&["agents"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "butler\t3\tRunning\tin-process\toperator\ttask-1+2\t41s\t120/30\n\
         idle\t4\tLoaded\tunknown\t-\t-\t-\t-\n\
         evil\t5\tRunning\twasm\ta?b?c\tt\t0s\t0/0\n"
    );
    for line in stdout(&out).lines() {
        assert_eq!(line.split('\t').count(), 8, "{line:?}");
    }
    let requests = door.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/v1/agents");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str())
    );
}

#[test]
fn agents_exit_codes_are_the_doors() {
    for (status, code) in [(401, 77), (404, 1), (503, 75)] {
        let door = FixtureDoor::spawn_replying(status, r#"{"error":"x","detail":"y"}"#);
        let out = door.run(&["agents"]);
        assert_eq!(
            out.status.code(),
            Some(code),
            "HTTP {status}: {}",
            stderr(&out)
        );
        assert!(out.stdout.is_empty(), "HTTP {status} prints no rows");
    }
    let dead = FixtureDoor::spawn_dead();
    assert_eq!(dead.run(&["agents"]).status.code(), Some(69));
    assert_eq!(
        dead.run(&["agents", "show", "butler"]).status.code(),
        Some(69)
    );
    assert_eq!(
        dead.run(&["agents", "show", "butler", "--follow"])
            .status
            .code(),
        Some(69),
        "an unreachable daemon ends a follow with 69"
    );
}

#[test]
fn show_sends_the_bounded_query_and_renders_one_line_per_row() {
    let rows = format!(
        "{},{}",
        tl(5, 1_700_000_000_123_000_000, "task.assign", "x=1"),
        approval(2, 1_700_000_001_000_000_000)
    );
    let door = FixtureDoor::spawn_scripted(vec![page("77", 3, &rows)]);
    let out = door.run(&["agents", "show", "butler", "--limit", "7"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "5\t1700000000.123\ttask.assign\ti.5\tabcd1234\tsrc->dst\t0\tx=1\n\
         a2\t1700000001.000\tapproval\thalt.resolve\t-\toperator->butler\t0\tdecision=granted resolution=accepted_halt\n"
    );
    let requests = door.requests();
    assert_eq!(requests.len(), 1, "no polling without --follow");
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/v1/spirits/butler/session");
    assert_eq!(requests[0].json_body(), serde_json::json!({"limit": 7}));

    let door = FixtureDoor::spawn_scripted(vec![page("77", 3, "")]);
    let out = door.run(&["agents", "show", "butler"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert_eq!(
        door.requests()[0].json_body(),
        serde_json::json!({"limit": 50})
    );
}

#[test]
fn show_refuses_bad_ids_and_limits_before_any_round_trip() {
    let door = FixtureDoor::spawn();
    assert_eq!(
        door.run(&["agents", "show", "bad id"]).status.code(),
        Some(2)
    );
    assert_eq!(
        door.run(&["agents", "show", "x", "--limit", "501"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        door.run(&["agents", "show", "x", "--limit", "0"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(door.request_count(), 0);
}

#[test]
fn show_maps_an_unknown_id_and_a_missing_bearer_like_every_read() {
    let door = FixtureDoor::spawn_replying(404, r#"{"error":"not_found"}"#);
    assert_eq!(
        door.run(&["agents", "show", "ghost"]).status.code(),
        Some(1)
    );
    let door = FixtureDoor::spawn_replying(401, r#"{"error":"unauthorized"}"#);
    assert_eq!(
        door.run(&["agents", "show", "butler"]).status.code(),
        Some(77)
    );
    let door = FixtureDoor::spawn_replying(503, r#"{"error":"audit_unavailable","detail":"x"}"#);
    assert_eq!(
        door.run(&["agents", "show", "butler"]).status.code(),
        Some(75)
    );
}

#[test]
fn follow_resumes_from_the_printed_cursors_and_ends_0_when_the_id_stops_resolving() {
    let door = FixtureDoor::spawn_scripted(vec![
        page(
            "77",
            3,
            &format!(
                "{},{}",
                tl(5, 10_000_000_000, "task.assign", ""),
                approval(2, 11_000_000_000)
            ),
        ),
        page("77", 3, &tl(6, 12_000_000_000, "task.complete", "")),
        Reply::new(404, r#"{"error":"not_found"}"#),
    ]);
    let started = std::time::Instant::now();
    let out = door.run(&["agents", "show", "butler", "--follow", "--interval", "1"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let printed: Vec<_> = stdout(&out)
        .lines()
        .map(|l| l.split('\t').next().unwrap().to_string())
        .collect();
    assert_eq!(printed, ["5", "a2", "6"]);
    assert!(
        stderr(&out).contains("no longer resolves"),
        "{}",
        stderr(&out)
    );
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(200),
        "the interval floor is 100 ms: two sleeps before the third poll"
    );
    let bodies: Vec<_> = door.requests().iter().map(|r| r.json_body()).collect();
    assert_eq!(bodies.len(), 3);
    assert_eq!(bodies[0], serde_json::json!({"limit": 50}));
    assert_eq!(
        bodies[1],
        serde_json::json!({"limit": 50, "after_rowid": 5, "after_decision": 2})
    );
    assert_eq!(
        bodies[2],
        serde_json::json!({"limit": 50, "after_rowid": 6, "after_decision": 2})
    );
}

#[test]
fn follow_ends_0_when_the_boot_nonce_or_the_pid_changes() {
    for (second, said) in [
        (page("78", 3, &tl(1, 1, "k", "")), "boot or pid changed"),
        (page("77", 9, &tl(1, 1, "k", "")), "boot or pid changed"),
    ] {
        let door = FixtureDoor::spawn_scripted(vec![page("77", 3, &tl(5, 1, "k", "")), second]);
        let out = door.run(&["agents", "show", "butler", "--follow", "--interval", "100"]);
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        assert!(stderr(&out).contains(said), "{}", stderr(&out));
        assert_eq!(
            stdout(&out).lines().count(),
            1,
            "the new identity's rows are not printed"
        );
    }
}

#[test]
fn a_first_404_in_follow_is_an_error_not_a_clean_end() {
    let door = FixtureDoor::spawn_replying(404, r#"{"error":"not_found"}"#);
    let out = door.run(&["agents", "show", "ghost", "--follow"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
}
