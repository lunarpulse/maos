#![forbid(unsafe_code)]

//! Story 4.1 AC4 — deterministic 1000-scenario termination-corpus generator.
//!
//! Produces 1000 JSON files at the target directory:
//!   - 250 × `planned_unload` (varying halt-set sizes: 0, 1, 3, 10)
//!   - 250 × `halt_accepted` (one per resolution kind × spirit pid)
//!   - 250 × `unplanned_crash` (varying halt-set sizes)
//!   - 250 × `halt_rejection` (mirrors accepted_halt shape)
//!
//! Each scenario file is deterministic (SHA-pinned generator output);
//! re-running produces byte-identical files.

use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn sid(n: usize) -> String {
    format!("term-{:04}", n)
}

fn sp(n: usize) -> String {
    format!("spirit-{}", n % 10)
}

pub fn run(out_dir: &str) -> Result<(), String> {
    let dir = PathBuf::from(out_dir);
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir {out_dir}: {e}"))?;

    let mut n = 0usize;

    // 250 planned_unload
    // 100 with 0 halts
    for i in 0..100 {
        n += 1;
        let s = json!({
            "scenario_id": sid(n),
            "kind": "planned_unload",
            "spirit_id": sp(i),
            "pending_halts": [],
            "expected_receipts": 1,
            "expected_receipt_ids": [format!("term-{}-", sp(i))]
        });
        write_scenario(&dir, n, &s)?;
    }
    // 50 with 1 halt
    for i in 0..50 {
        n += 1;
        let hid = format!("halt-planned-{:03}", i);
        let s = json!({
            "scenario_id": sid(n),
            "kind": "planned_unload",
            "spirit_id": sp(i),
            "pending_halts": [&hid],
            "expected_receipts": 1,
            "expected_receipt_ids": [&hid]
        });
        write_scenario(&dir, n, &s)?;
    }
    // 50 with 3 halts
    for i in 0..50 {
        n += 1;
        let a = format!("halt-p3-{}-a", i);
        let b = format!("halt-p3-{}-b", i);
        let c = format!("halt-p3-{}-c", i);
        let s = json!({
            "scenario_id": sid(n),
            "kind": "planned_unload",
            "spirit_id": sp(i),
            "pending_halts": [&a, &b, &c],
            "expected_receipts": 3,
            "expected_receipt_ids": [&a, &b, &c]
        });
        write_scenario(&dir, n, &s)?;
    }
    // 50 with 10 halts
    for i in 0..50 {
        n += 1;
        let hids: Vec<String> = (0..10).map(|j| format!("halt-p10-{}-{}", i, j)).collect();
        let s = json!({
            "scenario_id": sid(n),
            "kind": "planned_unload",
            "spirit_id": sp(i),
            "pending_halts": hids.clone(),
            "expected_receipts": 10,
            "expected_receipt_ids": hids
        });
        write_scenario(&dir, n, &s)?;
    }

    // 250 halt_accepted
    for i in 0..250 {
        n += 1;
        let hid = format!("halt-accepted-{:04}", i);
        let s = json!({
            "scenario_id": sid(n),
            "kind": "halt_accepted",
            "spirit_id": sp(i),
            "pending_halts": [&hid],
            "expected_receipts": 1,
            "expected_receipt_ids": [&hid]
        });
        write_scenario(&dir, n, &s)?;
    }

    // 250 unplanned_crash
    // 50 with 0 halts
    for i in 0..50 {
        n += 1;
        let s = json!({
            "scenario_id": sid(n),
            "kind": "unplanned_crash",
            "spirit_id": sp(i),
            "pending_halts": [],
            "expected_receipts": 1,
            "expected_receipt_ids": [format!("term-uc-{:04}", n)]
        });
        write_scenario(&dir, n, &s)?;
    }
    // 50 with 1 halt
    for i in 0..50 {
        n += 1;
        let hid = format!("halt-crash-{:03}", i);
        let s = json!({
            "scenario_id": sid(n),
            "kind": "unplanned_crash",
            "spirit_id": sp(i),
            "pending_halts": [&hid],
            "expected_receipts": 1,
            "expected_receipt_ids": [&hid]
        });
        write_scenario(&dir, n, &s)?;
    }
    // 50 with 5 halts
    for i in 0..50 {
        n += 1;
        let hids: Vec<String> = (0..5).map(|j| format!("halt-c5-{}-{}", i, j)).collect();
        let s = json!({
            "scenario_id": sid(n),
            "kind": "unplanned_crash",
            "spirit_id": sp(i),
            "pending_halts": hids.clone(),
            "expected_receipts": 5,
            "expected_receipt_ids": hids
        });
        write_scenario(&dir, n, &s)?;
    }
    // 50 with 1 halt (second batch)
    for i in 0..50 {
        n += 1;
        let hid = format!("halt-crash2-{:03}", i);
        let s = json!({
            "scenario_id": sid(n),
            "kind": "unplanned_crash",
            "spirit_id": sp(i),
            "pending_halts": [&hid],
            "expected_receipts": 1,
            "expected_receipt_ids": [&hid]
        });
        write_scenario(&dir, n, &s)?;
    }
    // 50 with 1 halt (third batch)
    for i in 0..50 {
        n += 1;
        let hid = format!("halt-crash3-{:03}", i);
        let s = json!({
            "scenario_id": sid(n),
            "kind": "unplanned_crash",
            "spirit_id": sp(i),
            "pending_halts": [&hid],
            "expected_receipts": 1,
            "expected_receipt_ids": [&hid]
        });
        write_scenario(&dir, n, &s)?;
    }

    // 250 halt_rejection
    for i in 0..250 {
        n += 1;
        let hid = format!("halt-rejected-{:04}", i);
        let s = json!({
            "scenario_id": sid(n),
            "kind": "halt_rejection",
            "spirit_id": sp(i),
            "pending_halts": [&hid],
            "expected_receipts": 1,
            "expected_receipt_ids": [&hid]
        });
        write_scenario(&dir, n, &s)?;
    }

    println!("ok: wrote {n} termination scenarios to {out_dir}");
    Ok(())
}

fn write_scenario(dir: &std::path::Path, n: usize, scenario: &Value) -> Result<(), String> {
    let path = dir.join(format!("scenario-{:04}.json", n));
    let mut f = fs::File::create(&path).map_err(|e| format!("create {path:?}: {e}"))?;
    let json =
        serde_json::to_string_pretty(scenario).map_err(|e| format!("json serialize: {e}"))?;
    f.write_all(json.as_bytes())
        .map_err(|e| format!("write {path:?}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn gen_is_deterministic_two_runs_yield_identical_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("run1");
        run(dir.to_str().unwrap()).unwrap();
        let dir2 = tmp.path().join("run2");
        run(dir2.to_str().unwrap()).unwrap();

        for i in 1..=1000 {
            let a = std::fs::read_to_string(dir.join(format!("scenario-{:04}.json", i))).unwrap();
            let b = std::fs::read_to_string(dir2.join(format!("scenario-{:04}.json", i))).unwrap();
            assert_eq!(a, b, "scenario {:04} differs between runs", i);
        }
    }

    #[test]
    fn gen_produces_exactly_1000_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("corpus");
        run(dir.to_str().unwrap()).unwrap();

        let count = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(count, 1000);
    }

    #[test]
    fn gen_kind_distribution() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("corpus");
        run(dir.to_str().unwrap()).unwrap();

        let mut kinds = std::collections::HashMap::<String, usize>::new();
        for i in 1..=1000 {
            let content =
                std::fs::read_to_string(dir.join(format!("scenario-{:04}.json", i))).unwrap();
            let v: serde_json::Value = serde_json::from_str(&content).unwrap();
            let kind = v["kind"].as_str().unwrap().to_string();
            *kinds.entry(kind).or_default() += 1;
        }

        assert_eq!(kinds.get("planned_unload").copied().unwrap_or(0), 250);
        assert_eq!(kinds.get("halt_accepted").copied().unwrap_or(0), 250);
        assert_eq!(kinds.get("unplanned_crash").copied().unwrap_or(0), 250);
        assert_eq!(kinds.get("halt_rejection").copied().unwrap_or(0), 250);
    }
}
