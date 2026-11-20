use std::{collections::BTreeMap, sync::Arc};

use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use serde_json::Map;

use crate::{
    format_one, name, parse_toml, Result, Value, MAX_ARTIFACT, MAX_FIELDS, MAX_PACKAGES, MAX_STRING,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Bool,
    I64,
}

impl FieldType {
    fn accepts(self, value: &Value) -> bool {
        match self {
            Self::String => value.as_str().is_some_and(|s| s.len() <= MAX_STRING),
            Self::Bool => value.is_boolean(),
            Self::I64 => value.is_i64(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub fields: Arc<BTreeMap<String, FieldType>>,
}

// Serialize validated scalar fields in schema order, even with serde_json/preserve_order.
pub(crate) struct OrderedFields<'a>(pub &'a Schema, pub &'a Map<String, Value>);

impl Serialize for OrderedFields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.1.len()))?;
        for field in self.0.fields.keys() {
            map.serialize_entry(field, &self.1[field])?;
        }
        map.end()
    }
}

impl Schema {
    pub(crate) fn check(&self, id: &str) -> Result<()> {
        let (base, revision) = id
            .split_once('@')
            .ok_or("schema id must be name@revision")?;
        name("schema name", base)?;
        if revision.is_empty()
            || revision.starts_with('0')
            || revision.len() > 10
            || !revision.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(format!("invalid revision for schema {base}"));
        }
        if self.fields.len() > MAX_FIELDS {
            return Err(format!("too many fields in {id}"));
        }
        for key in self.fields.keys() {
            name("field", key)?;
        }
        Ok(())
    }

    pub(crate) fn validate<'a>(
        &self,
        id: &str,
        value: &'a Value,
    ) -> Result<&'a Map<String, Value>> {
        let fields = value
            .as_object()
            .ok_or_else(|| format!("{id}: expected an object"))?;
        if fields.len() != self.fields.len() {
            return Err(format!("{id}: field set mismatch"));
        }
        let mut bytes = 0;
        for (key, ty) in self.fields.iter() {
            let field = fields
                .get(key)
                .ok_or_else(|| format!("{id}: missing {key}"))?;
            if !ty.accepts(field) {
                return Err(format!("{id}.{key}: expected bounded {ty:?}"));
            }
            bytes += key.len() + field.as_str().map_or(8, str::len);
        }
        if bytes > MAX_ARTIFACT {
            return Err(format!("{id}: artifact exceeds size limit"));
        }
        Ok(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum Scalar {
    Bool(bool),
    I64(i64),
    String(String),
}

impl Scalar {
    fn ty(&self) -> FieldType {
        match self {
            Self::Bool(_) => FieldType::Bool,
            Self::I64(_) => FieldType::I64,
            Self::String(_) => FieldType::String,
        }
    }

    fn equals(&self, other: &Value) -> bool {
        match self {
            Self::Bool(b) => Some(*b) == other.as_bool(),
            Self::I64(i) => Some(*i) == other.as_i64(),
            Self::String(s) => Some(s.as_str()) == other.as_str(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum Rule {
    Emit {
        outcome: String,
    },
    Equals {
        field: String,
        value: Scalar,
        matched: String,
        otherwise: String,
    },
    Infer {
        provider: String,
        model: String,
        prompt: String,
        source_field: String,
        citation_field: String,
        outcome: String,
    },
}

impl Rule {
    fn outcomes(&self) -> (&str, Option<&str>) {
        match self {
            Self::Emit { outcome } | Self::Infer { outcome, .. } => (outcome, None),
            Self::Equals {
                matched, otherwise, ..
            } => (matched, Some(otherwise)),
        }
    }

    fn dispatch(&self, input: &Map<String, Value>) -> Result<&str> {
        match self {
            Self::Emit { outcome } => Ok(outcome),
            Self::Infer { .. } => Err("inference requires host dispatch".into()),
            Self::Equals {
                field,
                value,
                matched,
                otherwise,
            } => {
                let actual = input
                    .get(field)
                    .ok_or_else(|| format!("missing rule field {field}"))?;
                Ok(if value.equals(actual) {
                    matched
                } else {
                    otherwise
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    version: String,
    input: String,
    output: String,
    rule: Rule,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: u32,
    environment: String,
    schemas: BTreeMap<String, Schema>,
    agents: BTreeMap<String, Package>,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub(crate) environment: String,
    pub(crate) agents: BTreeMap<String, Agent>,
    pub(crate) schemas: BTreeMap<String, Schema>,
}

impl Catalog {
    pub fn parse(text: &str) -> Result<Self> {
        let manifest: Manifest = parse_toml(text)?;
        format_one(manifest.format)?;
        name("environment", &manifest.environment)?;
        if manifest.schemas.len() > MAX_PACKAGES || manifest.agents.len() > MAX_PACKAGES {
            return Err("catalog exceeds package or schema limit".into());
        }
        for (id, schema) in &manifest.schemas {
            schema.check(id)?;
        }
        let mut agents = BTreeMap::new();
        for (id, package) in manifest.agents {
            name("agent", &id)?;
            let lookup = |key: &str| {
                manifest
                    .schemas
                    .get(key)
                    .cloned()
                    .ok_or_else(|| format!("{id}: unknown schema {key}"))
            };
            let agent = Agent {
                input_schema: lookup(&package.input)?,
                output_schema: lookup(&package.output)?,
                input: package.input,
                output: package.output,
                version: package.version,
                rule: package.rule,
            };
            agent.check().map_err(|error| format!("{id}: {error}"))?;
            agents.insert(id, agent);
        }
        Ok(Self {
            environment: manifest.environment,
            schemas: manifest.schemas,
            agents,
        })
    }
}

/// Bound schemas make execution independent of ambient catalog state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Agent {
    pub version: String,
    pub input: String,
    pub output: String,
    rule: Rule,
    pub(crate) input_schema: Schema,
    pub(crate) output_schema: Schema,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentOutput {
    pub outcome: String,
    #[serde(deserialize_with = "crate::case::input_object")]
    pub artifact: Value,
}

impl Agent {
    pub(crate) fn check(&self) -> Result<()> {
        if self.version.len() > 96 || semver::Version::parse(&self.version).is_err() {
            return Err("invalid package version".into());
        }
        let (first, second) = self.outcomes();
        name("outcome", first)?;
        if let Some(second) = second {
            name("outcome", second)?;
            if first == second {
                return Err("equals outcomes must be distinct".into());
            }
        }
        if let Rule::Infer {
            provider,
            model,
            prompt,
            source_field,
            citation_field,
            ..
        } = &self.rule
        {
            if !(1..=96).contains(&provider.len())
                || provider.starts_with('-')
                || !provider
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err("invalid inference provider".into());
            }
            if model.len() > 96
                || !model
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                || !model
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-._:+".contains(&b))
            {
                return Err("invalid inference model".into());
            }
            if prompt.len() > MAX_STRING || prompt.trim().is_empty() {
                return Err("expected bounded nonblank inference prompt".into());
            }
            if self.input_schema.fields.get(source_field) != Some(&FieldType::String)
                || self.output_schema.fields.get(citation_field) != Some(&FieldType::String)
            {
                return Err("inference requires string source and citation fields".into());
            }
            return Ok(());
        }
        for (key, ty) in self.output_schema.fields.iter() {
            if self.input_schema.fields.get(key) != Some(ty) {
                return Err(format!(
                    "output field {key} cannot be projected from {}",
                    self.input
                ));
            }
        }
        if let Rule::Equals { field, value, .. } = &self.rule {
            if self.input_schema.fields.get(field) != Some(&value.ty()) {
                return Err(format!("rule literal incompatible with field {field}"));
            }
            if matches!(value, Scalar::String(s) if s.len() > MAX_STRING) {
                return Err("rule literal exceeds string limit".into());
            }
        }
        Ok(())
    }

    pub(crate) fn outcomes(&self) -> (&str, Option<&str>) {
        self.rule.outcomes()
    }

    pub fn inference(&self) -> Option<(&str, &str)> {
        match &self.rule {
            Rule::Infer {
                provider, model, ..
            } => Some((provider, model)),
            _ => None,
        }
    }

    pub fn inference_prompt(&self, input: &Value) -> Result<String> {
        let Rule::Infer { prompt, .. } = &self.rule else {
            return Err("not an inference package".into());
        };
        let fields = self.input_schema.validate(&self.input, input)?;
        const SEPARATOR: &[u8] = b"\n\nInput JSON:\n";
        let mut bytes = Vec::with_capacity(prompt.len() + SEPARATOR.len() + 2);
        bytes.extend_from_slice(prompt.as_bytes());
        bytes.extend_from_slice(SEPARATOR);
        serde_json::to_writer(&mut bytes, &OrderedFields(&self.input_schema, fields))
            .map_err(|_| "cannot serialize inference input")?;
        String::from_utf8(bytes).map_err(|_| "invalid inference prompt encoding".into())
    }

    pub fn execute(&self, mut input: Value) -> Result<AgentOutput> {
        let fields = self.input_schema.validate(&self.input, &input)?;
        let outcome = self.rule.dispatch(fields)?.to_owned();
        if self.input_schema.fields.len() != self.output_schema.fields.len() {
            if let Value::Object(fields) = &mut input {
                fields.retain(|key, _| self.output_schema.fields.contains_key(key));
            }
        }
        self.output_schema.validate(&self.output, &input)?;
        Ok(AgentOutput {
            outcome,
            artifact: input,
        })
    }

    pub(crate) fn verify_output(&self, input: &Value, output: &AgentOutput) -> Result<()> {
        // Run admits only schema-checked input, including on every transition.
        let fields = input.as_object().ok_or("invalid run input")?;
        let expected = match &self.rule {
            Rule::Infer { outcome, .. } => outcome.as_str(),
            rule => rule.dispatch(fields)?,
        };
        if expected != output.outcome {
            return Err("reported outcome disagrees with catalog rule".into());
        }
        let artifact = self
            .output_schema
            .validate(&self.output, &output.artifact)?;
        if let Rule::Infer {
            source_field,
            citation_field,
            ..
        } = &self.rule
        {
            let source = fields[source_field]
                .as_str()
                .ok_or("invalid source field")?;
            let quote = artifact[citation_field]
                .as_str()
                .ok_or("invalid citation field")?;
            if quote.trim().is_empty() || !source.contains(quote) {
                return Err("citation must be a nonblank substring of the input source".into());
            }
        } else if artifact.iter().any(|(key, v)| fields.get(key) != Some(v)) {
            return Err("reported artifact is not a read-only projection".into());
        }
        Ok(())
    }
}
