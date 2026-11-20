use std::mem;

use serde::{Deserialize, Serialize};
use serde_json::Map;

use crate::{
    catalog::OrderedFields, digest, name, AgentOutput, Plan, Result, Route, Schema, Value,
};

fn artifact_digest(id: &str, schema: &Schema, object: &Map<String, Value>) -> Result<String> {
    digest(&(id, OrderedFields(schema, object)))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepReceipt {
    pub step: String,
    pub package: String,
    pub version: String,
    /// None denotes ingress; otherwise this step consumed the named producer's output.
    pub from_step: Option<String>,
    pub input_schema: String,
    pub input_digest: String,
    pub output_schema: String,
    pub output_digest: String,
    pub elapsed_ns: u64,
    pub outcome: String,
    pub route: Route,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReport {
    pub run_id: String,
    pub revision: String,
    pub environment: String,
    pub topology: String,
    pub outcome: String,
    pub path: Vec<StepReceipt>,
    pub artifact: Value,
}

#[derive(Debug)]
pub struct Run<'a> {
    plan: &'a Plan,
    run_id: String,
    revision: String,
    step: &'a str,
    input: Value,
    input_digest: String,
    path: Vec<StepReceipt>,
    complete: bool,
}

impl Plan {
    pub fn start(&self, input: Value, run_id: String) -> Result<Run<'_>> {
        name("run id", &run_id)?;
        self.check_run()?;
        let step = self.topology.entry.as_str();
        let agent = &self.agents[&self.topology.steps[step].agent];
        let fields = agent.input_schema.validate(&agent.input, &input)?;
        let input_digest = artifact_digest(&agent.input, &agent.input_schema, fields)?;
        Ok(Run {
            plan: self,
            run_id,
            revision: self.digest()?,
            step,
            input,
            input_digest,
            path: Vec::new(),
            complete: false,
        })
    }
}

impl Run<'_> {
    pub fn step(&self) -> &str {
        self.step
    }

    pub fn input(&self) -> &Value {
        &self.input
    }

    pub fn input_digest(&self) -> &str {
        &self.input_digest
    }

    /// Most recent accepted nonterminal handoff, before its recipient is dispatched.
    pub fn last_handoff(&self) -> Option<&StepReceipt> {
        self.path.last()
    }

    /// On error neither the cursor nor the receipt chain changes. On success
    /// the new cursor owns the artifact, or the terminal report does.
    pub fn accept(&mut self, output: AgentOutput, elapsed_ns: u64) -> Result<Option<RunReport>> {
        if self.complete {
            return Err("run has already finished".into());
        }
        let step = &self.plan.topology.steps[self.step];
        let agent = &self.plan.agents[&step.agent];
        agent.verify_output(&self.input, &output)?;
        let route = step
            .routes
            .get(&output.outcome)
            .ok_or("unrouted package outcome")?;
        let fields = output
            .artifact
            .as_object()
            .ok_or("output must be an object")?;
        // Only verified projections preserve the artifact under the same schema.
        let output_digest = if agent.inference().is_none() && agent.input == agent.output {
            self.input_digest.clone()
        } else {
            artifact_digest(&agent.output, &agent.output_schema, fields)?
        };
        let from_step = self.path.last().map(|r| r.step.clone());
        let input_digest = match route {
            Route::Step { .. } => mem::replace(&mut self.input_digest, output_digest.clone()),
            Route::Finish { .. } => self.input_digest.clone(),
        };
        self.path.push(StepReceipt {
            step: self.step.to_owned(),
            package: step.agent.clone(),
            version: agent.version.clone(),
            from_step,
            input_schema: agent.input.clone(),
            input_digest,
            output_schema: agent.output.clone(),
            output_digest,
            elapsed_ns,
            outcome: output.outcome,
            route: route.clone(),
        });
        match route {
            Route::Step { step } => {
                self.step = step;
                self.input = output.artifact;
                Ok(None)
            }
            Route::Finish { finish } => {
                self.complete = true;
                Ok(Some(RunReport {
                    run_id: mem::take(&mut self.run_id),
                    revision: mem::take(&mut self.revision),
                    environment: self.plan.environment.clone(),
                    topology: self.plan.topology.id.clone(),
                    outcome: finish.clone(),
                    path: mem::take(&mut self.path),
                    artifact: output.artifact,
                }))
            }
        }
    }
}
