use std::{collections::BTreeSet, fmt};

use serde::{
    de::{Error, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Map;

use crate::{
    name, Result, RunReport, Value, MAX_ARTIFACT, MAX_DOCUMENT, MAX_FIELDS, MAX_STEPS, MAX_STRING,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    #[serde(deserialize_with = "input_object")]
    pub input: Value,
    pub expect: CaseExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseExpectation {
    pub outcome: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_path"
    )]
    pub path: Option<Vec<String>>,
    /// When present, each label grades the agent result at the corresponding path step.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_path"
    )]
    pub step_outcomes: Option<Vec<String>>,
}

fn present_path<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<Vec<String>>, D::Error> {
    Vec::deserialize(d).map(Some)
}

// Reject ambiguous duplicate fields before validating an evidence object.
pub(crate) fn input_object<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Value, D::Error> {
    struct Object;
    impl<'de> Visitor<'de> for Object {
        type Value = Value;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a bounded object of string, bool or i64 fields")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> std::result::Result<Value, M::Error> {
            let mut result = Map::new();
            let mut bytes = 0;
            while let Some(key) = map.next_key::<String>()? {
                name("case input field", &key).map_err(M::Error::custom)?;
                if result.len() >= MAX_FIELDS || result.contains_key(&key) {
                    return Err(M::Error::custom("duplicate or excess case input field"));
                }
                let value: Value = map.next_value()?;
                bytes += key.len()
                    + match &value {
                        Value::String(s) if s.len() <= MAX_STRING => s.len(),
                        Value::Bool(_) => 8,
                        Value::Number(n) if n.is_i64() => 8,
                        _ => return Err(M::Error::custom("expected bounded string, bool or i64")),
                    };
                if bytes > MAX_ARTIFACT {
                    return Err(M::Error::custom("case input exceeds size limit"));
                }
                result.insert(key, value);
            }
            Ok(Value::Object(result))
        }
    }
    d.deserialize_map(Object)
}

/// Strictly parse an ingress artifact, rejecting repeated fields before validation.
pub fn parse_input(text: &str) -> Result<Value> {
    if text.len() > MAX_DOCUMENT {
        return Err("input document exceeds 1 MiB".into());
    }
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let input = input_object(&mut deserializer).map_err(|error| error.to_string())?;
    deserializer.end().map_err(|error| error.to_string())?;
    Ok(input)
}

impl Case {
    pub fn parse(text: &str) -> Result<Vec<Self>> {
        if text.len() > MAX_DOCUMENT {
            return Err("case document exceeds 1 MiB".into());
        }
        let cases: Vec<Self> = serde_json::from_str(text).map_err(|error| error.to_string())?;
        if cases.is_empty() || cases.len() > 1024 {
            return Err("expected 1..1024 cases".into());
        }
        let mut ids = BTreeSet::new();
        for case in &cases {
            name("case id", &case.id)?;
            if !ids.insert(&case.id) {
                return Err(format!("duplicate case {}", case.id));
            }
            name("expected outcome", &case.expect.outcome)?;
            if let Some(path) = &case.expect.path {
                if path.is_empty() || path.len() > MAX_STEPS {
                    return Err(format!("{}: invalid expected path length", case.id));
                }
                for id in path {
                    name("expected step", id)?;
                }
            }
            if let Some(outcomes) = &case.expect.step_outcomes {
                if !case
                    .expect
                    .path
                    .as_ref()
                    .is_some_and(|path| path.len() == outcomes.len())
                {
                    return Err(format!("{}: step_outcomes must align with path", case.id));
                }
                for outcome in outcomes {
                    name("expected step outcome", outcome)?;
                }
            }
        }
        Ok(cases)
    }
}

impl CaseExpectation {
    pub fn matches(&self, report: &RunReport) -> bool {
        self.outcome == report.outcome
            && self.path.as_ref().is_none_or(|expected| {
                expected
                    .iter()
                    .eq(report.path.iter().map(|receipt| &receipt.step))
            })
            && self.step_outcomes.as_ref().is_none_or(|expected| {
                expected
                    .iter()
                    .eq(report.path.iter().map(|receipt| &receipt.outcome))
            })
    }
}
