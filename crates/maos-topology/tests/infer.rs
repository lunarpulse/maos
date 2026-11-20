use maos_topology::{AgentOutput, Catalog, Plan, Topology, Value};
use serde_json::json;
use sha2::{Digest, Sha256};

fn plan() -> Plan {
    let catalog = Catalog::parse(
        r#"
format = 1
environment = "local"
[schemas."finding@1".fields]
excerpt = "string"
quote = "string"
[agents.find]
version = "1.0.0"
input = "finding@1"
output = "finding@1"
[agents.find.rule]
kind = "infer"
provider = "anthropic"
model = "claude-haiku-4-5-20251001"
prompt = "Quote the evidence."
source_field = "excerpt"
citation_field = "quote"
outcome = "finding_ready"
[agents.review]
version = "1.0.0"
input = "finding@1"
output = "finding@1"
[agents.review.rule]
kind = "equals"
field = "quote"
value = "cold"
matched = "found"
otherwise = "missing"
"#,
    )
    .unwrap();
    let topology = Topology::parse(
        r#"
format = 1
id = "evidence"
entry = "find"
[steps.find]
agent = "find"
routes.finding_ready = { step = "review" }
[steps.review]
agent = "review"
routes.found = { finish = "accepted" }
routes.missing = { finish = "rejected" }
"#,
    )
    .unwrap();
    Plan::resolve(topology, &catalog, None).unwrap()
}

fn input() -> Value {
    json!({"quote": "", "excerpt": "A \"cold\" day"})
}

#[test]
fn same_schema_inference_hashes_and_dispatches_the_produced_artifact() {
    let plan = plan();
    let find = &plan.agents["find"];
    let mut run = plan.start(input(), "run".into()).unwrap();
    assert_eq!(
        find.inference(),
        Some(("anthropic", "claude-haiku-4-5-20251001"))
    );
    assert_eq!(
        find.inference_prompt(run.input()).unwrap(),
        "Quote the evidence.\n\nInput JSON:\n{\"excerpt\":\"A \\\"cold\\\" day\",\"quote\":\"\"}"
    );
    assert!(find.execute(input()).is_err());

    let output = AgentOutput {
        outcome: "finding_ready".into(),
        artifact: json!({"quote": "cold", "excerpt": "summary"}),
    };
    assert!(run.accept(output, 1).unwrap().is_none());
    let digest = format!(
        "{:x}",
        Sha256::digest(br#"["finding@1",{"excerpt":"summary","quote":"cold"}]"#)
    );
    let handoff = run.last_handoff().unwrap();
    assert_ne!(handoff.input_digest, handoff.output_digest);
    assert_eq!(handoff.output_digest, digest);
    let review = &plan.agents[&plan.topology.steps[run.step()].agent];
    let result = review.execute(run.input().clone()).unwrap();
    let report = run.accept(result, 1).unwrap().unwrap();
    assert_eq!(report.outcome, "accepted");
    assert_eq!(report.path[1].input_digest, digest);
}

#[test]
fn invalid_inference_never_advances_the_cursor() {
    let plan = plan();
    let mut run = plan.start(input(), "run".into()).unwrap();
    let digest = run.input_digest().to_owned();
    for (outcome, artifact) in [
        ("found", json!({"excerpt": "summary", "quote": "cold"})),
        (
            "finding_ready",
            json!({"excerpt": "fabricated", "quote": "fabricated"}),
        ),
        ("finding_ready", json!({"excerpt": "summary", "quote": " "})),
        (
            "finding_ready",
            json!({"excerpt": "summary", "quote": "cold", "extra": true}),
        ),
    ] {
        let output = AgentOutput {
            outcome: outcome.into(),
            artifact,
        };
        assert!(run.accept(output, 1).is_err());
        assert_eq!(run.step(), "find");
        assert_eq!(run.input(), &input());
        assert_eq!(run.input_digest(), digest);
        assert!(run.last_handoff().is_none());
    }
}
