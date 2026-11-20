//! Pure format-1 workflow compilation and execution. Parse the operator catalog,
//! then resolve a topology against it. A deserialized plan is not authority:
//! call [`Plan::validate_live`] against the trusted catalog before dispatch.
#![forbid(unsafe_code)]

mod case;
mod catalog;
mod plan;
mod run;

pub use case::{parse_input, Case, CaseExpectation};
pub use catalog::{Agent, AgentOutput, Catalog, FieldType, Schema};
pub use plan::{Plan, Route, Step, Topology};
pub use run::{Run, RunReport, StepReceipt};
pub use serde_json::Value;

pub type Result<T, E = String> = std::result::Result<T, E>;

const MAX_DOCUMENT: usize = 1024 * 1024;
const MAX_STEPS: usize = 256;
const MAX_PACKAGES: usize = 512;
const MAX_FIELDS: usize = 128;
const MAX_STRING: usize = 16 * 1024;
const MAX_ARTIFACT: usize = 256 * 1024;

fn parse_toml<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > MAX_DOCUMENT {
        return Err("TOML document exceeds 1 MiB".into());
    }
    toml::from_str(text).map_err(|error| error.message().to_owned())
}

fn format_one(format: u32) -> Result<()> {
    if format != 1 {
        return Err(format!("unsupported format {format}; expected 1"));
    }
    Ok(())
}

fn name(kind: &str, id: &str) -> Result<()> {
    let bytes = id.as_bytes();
    if bytes.len() > 96
        || !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'_' | b'-'))
    {
        return Err(format!(
            "invalid {kind}: expected a 1..96 byte ASCII identifier"
        ));
    }
    Ok(())
}

fn digest<T: serde::Serialize>(value: &T) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    serde_json::to_writer(&mut hash, value).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", hash.finalize()))
}
