use std::collections::{btree_map::Entry, BTreeMap};

use serde::{Deserialize, Serialize};

use crate::{
    digest, format_one, name, parse_toml, Agent, Catalog, Result, Schema, MAX_PACKAGES, MAX_STEPS,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Route {
    Step { step: String },
    Finish { finish: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub agent: String,
    pub routes: BTreeMap<String, Route>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topology {
    pub format: u32,
    pub id: String,
    pub entry: String,
    pub steps: BTreeMap<String, Step>,
}

impl Topology {
    pub fn parse(text: &str) -> Result<Self> {
        let topology: Self = parse_toml(text)?;
        topology.check()?;
        Ok(topology)
    }

    fn check(&self) -> Result<()> {
        format_one(self.format)?;
        name("topology", &self.id)?;
        name("entry", &self.entry)?;
        if self.steps.is_empty() || self.steps.len() > MAX_STEPS {
            return Err("invalid step count".into());
        }
        if !self.steps.contains_key(&self.entry) {
            return Err(format!("unknown entry {}", self.entry));
        }
        let mut incoming: BTreeMap<&str, usize> =
            self.steps.keys().map(|k| (k.as_str(), 0)).collect();
        for (id, step) in &self.steps {
            name("step", id)?;
            name("agent", &step.agent)?;
            if !(1..=2).contains(&step.routes.len()) {
                return Err(format!("{id}: expected one or two explicit routes"));
            }
            for (outcome, route) in &step.routes {
                name("outcome", outcome)?;
                match route {
                    Route::Finish { finish } => name("terminal outcome", finish)?,
                    Route::Step { step } => {
                        let degree = incoming
                            .get_mut(step.as_str())
                            .ok_or_else(|| format!("{id}: unknown step {step}"))?;
                        *degree += 1;
                    }
                }
            }
        }
        // Kahn from the entry only: components with a different root, and
        // components blocked on a cycle, both fail the processed-count check.
        if incoming[self.entry.as_str()] != 0 {
            return Err("entry must not have incoming routes".into());
        }
        let mut ready = vec![self.entry.as_str()];
        let mut processed = 0;
        while let Some(id) = ready.pop() {
            processed += 1;
            for route in self.steps[id].routes.values() {
                if let Route::Step { step } = route {
                    let degree = incoming
                        .get_mut(step.as_str())
                        .expect("target checked above");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push(step.as_str());
                    }
                }
            }
        }
        if processed != self.steps.len() {
            return Err("topology contains a cycle or unreachable step".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub topology: Topology,
    pub environment: String,
    pub previous: Option<String>,
    pub agents: BTreeMap<String, Agent>,
    pub schemas: BTreeMap<String, Schema>,
}

fn check_previous(previous: Option<&str>) -> Result<()> {
    if previous.is_some_and(|s| {
        s.len() != 64 || !s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    }) {
        return Err("previous revision must be a lowercase SHA-256 digest".into());
    }
    Ok(())
}

impl Plan {
    pub fn resolve(
        topology: Topology,
        catalog: &Catalog,
        previous: Option<String>,
    ) -> Result<Self> {
        topology.check()?;
        check_previous(previous.as_deref())?;
        let mut agents = BTreeMap::new();
        let mut schemas = BTreeMap::new();
        for step in topology.steps.values() {
            if agents.contains_key(&step.agent) {
                continue;
            }
            let agent = catalog
                .agents
                .get(&step.agent)
                .ok_or_else(|| format!("unauthorized agent {}", step.agent))?;
            for id in [&agent.input, &agent.output] {
                if !schemas.contains_key(id) {
                    schemas.insert(id.clone(), catalog.schemas[id].clone());
                }
            }
            agents.insert(step.agent.clone(), agent.clone());
        }
        let plan = Self {
            topology,
            previous,
            agents,
            schemas,
            environment: catalog.environment.clone(),
        };
        plan.check_routes()?;
        Ok(plan)
    }

    fn check_routes(&self) -> Result<()> {
        for (id, step) in &self.topology.steps {
            let agent = self
                .agents
                .get(&step.agent)
                .ok_or_else(|| format!("{id}: missing package {}", step.agent))?;
            let (a, b) = agent.outcomes();
            if step.routes.len() != 1 + usize::from(b.is_some())
                || !step.routes.contains_key(a)
                || b.is_some_and(|b| !step.routes.contains_key(b))
            {
                return Err(format!(
                    "{id}: routes must exactly cover the package outcomes"
                ));
            }
            for route in step.routes.values() {
                if let Route::Step { step: next } = route {
                    let target = &self.topology.steps[next];
                    let receiver = self
                        .agents
                        .get(&target.agent)
                        .ok_or_else(|| format!("{next}: missing package {}", target.agent))?;
                    if agent.output != receiver.input {
                        return Err(format!(
                            "{id} -> {next}: {} does not match {}",
                            agent.output, receiver.input
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Compact JSON in declared field order, with lexically ordered map keys.
    pub fn digest(&self) -> Result<String> {
        digest(self)
    }

    pub fn validate_live(&self, catalog: &Catalog) -> Result<()> {
        let live = Self::resolve(self.topology.clone(), catalog, self.previous.clone())?;
        if *self != live {
            return Err("pinned plan differs from live catalog resolution".into());
        }
        Ok(())
    }

    // Public fields and serde permit non-compiler-created plans. Check internal
    // invariants before borrowing one for a run; this is NOT live authorization.
    pub(crate) fn check_run(&self) -> Result<()> {
        self.topology.check()?;
        name("environment", &self.environment)?;
        check_previous(self.previous.as_deref())?;
        if self.schemas.len() > MAX_PACKAGES || self.agents.len() > self.topology.steps.len() {
            return Err("invalid pin count".into());
        }
        for (id, schema) in &self.schemas {
            schema.check(id)?;
        }
        let mut used_agents = BTreeMap::new();
        let mut used_schemas = std::collections::BTreeSet::new();
        for step in self.topology.steps.values() {
            if let Entry::Vacant(entry) = used_agents.entry(&step.agent) {
                let agent = self
                    .agents
                    .get(&step.agent)
                    .ok_or_else(|| format!("missing agent {}", step.agent))?;
                for (id, binding) in [
                    (&agent.input, &agent.input_schema),
                    (&agent.output, &agent.output_schema),
                ] {
                    if self.schemas.get(id) != Some(binding) {
                        return Err(format!("{}: incorrect schema pin {id}", step.agent));
                    }
                    used_schemas.insert(id);
                }
                agent.check()?;
                entry.insert(());
            }
        }
        if used_agents.len() != self.agents.len() || used_schemas.len() != self.schemas.len() {
            return Err("plan contains unused pins".into());
        }
        self.check_routes()
    }
}
