#![forbid(unsafe_code)]
//! Story 18-6 AC4: 50 erased-only cascades through ONE production daemon door.
#[path = "../../../tests/harness/forget_18_6.rs"]
mod h;
use serde_json::Value;
use std::path::Path;

#[test]
fn fifty_door_cascades_and_one_hundred_all_store_probes() {
    let home = tempfile::tempdir().unwrap();
    let maos = Path::new(env!("CARGO_BIN_EXE_maos"));
    let ctl = maos.with_file_name("maosctl");
    assert!(ctl.is_file(), "run cargo build -p maos-cli first");
    let mut init = h::command(maos, home.path());
    init.arg("init");
    h::checked(init);
    let scenarios: Vec<Value> = include_str!("../../../tests/corpora/gdpr-cascade-v1.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let probes: Vec<Value> = include_str!("../../../tests/corpora/gdpr-cascade-probe-v1.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(scenarios.len(), 50);
    assert_eq!(probes.len(), 100);
    let mut targets = Vec::new();
    let mut controls = Vec::new();
    for s in &scenarios {
        assert_eq!(s["expected_outcome"], "erased");
        let principal = s["principal"].as_str().unwrap();
        let canary = s["canary"].as_str().unwrap();
        let pid = s["spirit_pid"].as_u64().unwrap() as u32;
        let other = s["secondary_spirit_pid"].as_u64().unwrap() as u32;
        let stores = h::Stores::open(home.path(), s["boot_nonce"].as_u64().unwrap());
        // Both the indexed and the unindexed strata have a marker-bearing TL row.
        // The unindexed-only strata intentionally never write a principal namespace.
        if matches!(
            s["stratum"].as_str().unwrap(),
            "indexed-private" | "multi-writer"
        ) {
            stores.private(
                pid,
                principal,
                &format!("{principal} {canary} {}", "spill ".repeat(1024)),
            );
        }
        let raw = stores.frame(pid, principal, canary);
        let mut ids = vec![raw];
        match s["stratum"].as_str().unwrap() {
            "unindexed-distillate" => {
                ids.push(stores.digest(pid, vec![raw], &format!("{principal} {canary}")))
            }
            "derived-distillate" => {
                let first = stores.digest(pid, vec![raw], "non-personal first summary");
                let second = stores.digest(pid, vec![first], "non-personal digest of digest");
                ids.extend([first, second]);
            }
            "multi-writer" => {
                let second = stores.frame(other, principal, canary);
                ids.extend([
                    second,
                    stores.digest(other, vec![second], "multi-writer summary"),
                ]);
            }
            "cross-boot-pid-reuse" => {
                let old = h::Stores::open(home.path(), s["boot_nonce"].as_u64().unwrap() + 10000);
                ids.push(old.digest(pid, vec![raw], &format!("{principal} {canary}")));
                // The same numeric pid in the other boot also wrote an unrelated
                // subject. Its bytes and lineage must survive this cascade.
                let control = s["control_principal"].as_str().unwrap();
                let kept = old.frame(pid, control, "other-boot-control");
                controls.extend([
                    kept,
                    old.digest(pid, vec![kept], "other boot control digest"),
                ]);
            }
            "indexed-private" | "raw-mcp" => {}
            other => panic!("unknown stratum {other}"),
        }
        let control = s["control_principal"].as_str().unwrap();
        stores.private(other + 1000, control, &format!("untouched {control}"));
        let source = stores.frame(other + 1000, control, "control-canary");
        let digest = stores.digest(other + 1000, vec![source], "untouched control digest");
        controls.extend([source, digest]);
        targets.push(ids);
    }
    let before = h::rows(home.path());
    let mut serve = h::command(maos, home.path());
    serve.args(["run", "spirits/researcher/manifest.toml"]);
    let daemon = h::Daemon::spawn(serve);
    h::wait("operator door", &daemon, || {
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "researcher"])
            .status
            .success()
    });
    let mut clean = 0;
    let mut marked = 0;
    for (s, ids) in scenarios.iter().zip(&targets) {
        let p = s["principal"].as_str().unwrap();
        let out = h::ctl(&ctl, home.path(), &["forget", "--principal", p]);
        assert!(
            out.status.success(),
            "forget {p}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let terminal = h::json(&out);
        h::assert_pending(&terminal);
        let requests = terminal["forget"]["redistill"].as_array().unwrap();
        assert_eq!(
            requests.len(),
            terminal["pending_count"].as_u64().unwrap() as usize
        );
        h::assert_absent(home.path(), p, s["canary"].as_str().unwrap());
        clean += 1;
        let rows = h::rows(home.path());
        for id in ids {
            let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
            let target = rows.iter().find(|r| r.0 == hex).unwrap();
            assert_eq!(
                target.3["redacted"], true,
                "{}: {target:?}",
                s["scenario_id"]
            );
            assert!(
                rows.iter()
                    .any(
                        |r| ["frame.redacted", "distillate.redacted"].contains(&r.2.as_str())
                            && r.3
                                .get("redacted_frame_id")
                                .or_else(|| r.3.get("redacted_distillate_frame_id"))
                                .and_then(Value::as_str)
                                .is_some_and(|id| id.replace(':', "") == hex)
                    ),
                "marker missing for {hex}"
            );
        }
        marked += 1;
    }
    for p in &probes {
        h::assert_absent(
            home.path(),
            p["principal"].as_str().unwrap(),
            p["canary"].as_str().unwrap(),
        );
        let control = p["control_principal"].as_str().unwrap();
        assert_eq!(
            maos_audit::subject_access_query(&h::db(home.path()), control)
                .unwrap()
                .len(),
            1
        );
    }
    let after = h::rows(home.path());
    for id in controls {
        let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            before.iter().find(|r| r.0 == hex),
            after.iter().find(|r| r.0 == hex),
            "control changed"
        );
    }
    assert_eq!((clean, marked, probes.len()), (50, 50, 100));
    eprintln!("18-6 production floors: clean={clean}/50 markers={marked}/50 leakage=0/100");
}
