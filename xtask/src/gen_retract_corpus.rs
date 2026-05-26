#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! serde_json = "1.0"
//! ```

use std::fs;
use std::path::Path;

fn main() {
    let out_dir = Path::new("crates/maos-eval/fixtures/retract-corpus-v0");
    fs::create_dir_all(out_dir).unwrap();

    let mut counter = 1;

    // Category 1: before-delivery (10 scenarios)
    for i in 1..=10 {
        let scenario = serde_json::json!({
            "scenario_id": format!("retract-before-delivery-{i:03}"),
            "category": "before_delivery",
            "description": "Sender retracts their own frame before recipient processes it",
            "original_frame": {
                "frame_id_hex": format!("{:032x}", counter),
                "from_spirit": format!("spirit-{}", i % 5 + 1),
                "to_spirit": format!("spirit-{}", (i + 1) % 5 + 1),
                "kind": if i % 3 == 0 { "TaskComplete" } else if i % 3 == 1 { "DecisionDispatch" } else { "TaskAssign" },
                "payload_size_bytes": 256 + i * 100
            },
            "retract_request": {
                "retracting_spirit": format!("spirit-{}", i % 5 + 1),
                "reason": format!("test retract before delivery scenario {i}")
            },
            "expected_outcome": {
                "success": true,
                "outcome_variant": "Retracted",
                "error_variant": null
            }
        });
        fs::write(
            out_dir.join(format!("scenario-{counter:03}.json")),
            serde_json::to_string_pretty(&scenario).unwrap(),
        ).unwrap();
        counter += 1;
    }

    // Category 2: after-delivery (10 scenarios)
    for i in 1..=10 {
        let scenario = serde_json::json!({
            "scenario_id": format!("retract-after-delivery-{i:03}"),
            "category": "after_delivery",
            "description": "Sender retracts their own frame after recipient has processed it",
            "original_frame": {
                "frame_id_hex": format!("{:032x}", counter),
                "from_spirit": format!("spirit-{}", i % 5 + 1),
                "to_spirit": format!("spirit-{}", (i + 2) % 5 + 1),
                "kind": if i % 4 == 0 { "TelemetryEvent" } else if i % 4 == 1 { "ConsentRequest" } else if i % 4 == 2 { "TaskComplete" } else { "TaskAssign" },
                "payload_size_bytes": 512 + i * 50
            },
            "retract_request": {
                "retracting_spirit": format!("spirit-{}", i % 5 + 1),
                "reason": format!("test retract after delivery scenario {i}")
            },
            "expected_outcome": {
                "success": true,
                "outcome_variant": "Retracted",
                "error_variant": null
            }
        });
        fs::write(
            out_dir.join(format!("scenario-{counter:03}.json")),
            serde_json::to_string_pretty(&scenario).unwrap(),
        ).unwrap();
        counter += 1;
    }

    // Category 3: authority-violation (5 scenarios)
    for i in 1..=5 {
        let scenario = serde_json::json!({
            "scenario_id": format!("retract-authority-violation-{i:03}"),
            "category": "authority_violation",
            "description": "Non-sender attempts to retract a frame they did not send",
            "original_frame": {
                "frame_id_hex": format!("{:032x}", counter),
                "from_spirit": format!("spirit-{}", i % 3 + 1),
                "to_spirit": format!("spirit-{}", (i + 1) % 3 + 1),
                "kind": "TaskAssign",
                "payload_size_bytes": 128 + i * 64
            },
            "retract_request": {
                "retracting_spirit": format!("spirit-{}", (i + 1) % 3 + 1),
                "reason": format!("malicious retract attempt {i}")
            },
            "expected_outcome": {
                "success": false,
                "outcome_variant": "Error",
                "error_variant": Some("RetractAuthorityViolation")
            }
        });
        fs::write(
            out_dir.join(format!("scenario-{counter:03}.json")),
            serde_json::to_string_pretty(&scenario).unwrap(),
        ).unwrap();
        counter += 1;
    }

    // Category 4: idempotent (5 scenarios)
    for i in 1..=5 {
        let scenario = serde_json::json!({
            "scenario_id": format!("retract-idempotent-{i:03}"),
            "category": "idempotent",
            "description": "Sender retracts same frame twice; second retract returns Already",
            "original_frame": {
                "frame_id_hex": format!("{:032x}", counter),
                "from_spirit": format!("spirit-{}", i % 4 + 1),
                "to_spirit": format!("spirit-{}", (i + 1) % 4 + 1),
                "kind": if i % 2 == 0 { "TaskComplete" } else { "TaskAssign" },
                "payload_size_bytes": 256 + i * 128
            },
            "retract_request": {
                "retracting_spirit": format!("spirit-{}", i % 4 + 1),
                "reason": format!("idempotent retract test {i}")
            },
            "expected_outcome": {
                "success": true,
                "outcome_variant": "Already",
                "error_variant": null
            }
        });
        fs::write(
            out_dir.join(format!("scenario-{counter:03}.json")),
            serde_json::to_string_pretty(&scenario).unwrap(),
        ).unwrap();
        counter += 1;
    }

    println!("Generated {} scenarios in {}", counter - 1, out_dir.display());
}
