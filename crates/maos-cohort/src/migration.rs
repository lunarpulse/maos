#![forbid(unsafe_code)]

//! Operator-declared migration-chain resolution.
//!
//! The resolver deliberately knows only the candidate set supplied by the
//! operator. It does not discover versions from a registry: candidate manifests
//! are trusted operator input, while this module verifies only the candidate
//! graph's structure and the requested route.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CohortError, MigrationChainNotLinearReason};

/// The migration-relevant projection of an operator-supplied Spirit manifest.
///
/// `class` and `version` identify the successor; `migrates_from` is copied from
/// its `[migrates_from].versions` declaration. The projection keeps the chain
/// resolver out of `maos-kernel-core`; execution still delegates each resolved
/// hop to the existing single-hop upgrade primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationCandidate {
    pub class: String,
    pub version: String,
    pub migrates_from: Vec<String>,
}

impl MigrationCandidate {
    pub fn new(
        class: impl Into<String>,
        version: impl Into<String>,
        migrates_from: Vec<String>,
    ) -> Self {
        Self {
            class: class.into(),
            version: version.into(),
            migrates_from,
        }
    }

    /// Project only the migration declarations from a Spirit manifest. Unknown
    /// top-level fields remain the kernel's concern when it loads each hop.
    pub fn from_manifest_file(path: &std::path::Path) -> Result<Self, CohortError> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            CohortError::ParseError(format!("read {}: {error}", path.display()))
        })?;
        let raw: RawCandidateManifest = toml::from_str(&source).map_err(|error| {
            CohortError::ParseError(format!("parse {}: {error}", path.display()))
        })?;
        if raw.class.name.is_empty() || raw.class.version.is_empty() {
            return Err(CohortError::ParseError(format!(
                "migration candidate {} has an empty [class] name or version",
                path.display()
            )));
        }
        if raw.migrates_from.versions.iter().any(String::is_empty) {
            return Err(CohortError::ParseError(format!(
                "migration candidate {} has an empty migrates_from version pattern",
                path.display()
            )));
        }
        Ok(Self::new(
            raw.class.name,
            raw.class.version,
            raw.migrates_from.versions,
        ))
    }
}

#[derive(Deserialize)]
struct RawCandidateManifest {
    class: RawCandidateClass,
    #[serde(default)]
    migrates_from: RawMigratesFrom,
}

#[derive(Deserialize)]
struct RawCandidateClass {
    name: String,
    version: String,
}

#[derive(Default, Deserialize)]
struct RawMigratesFrom {
    versions: Vec<String>,
}

/// One existing-kernel-compatible migration hop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationHop {
    pub predecessor_version: String,
    pub successor: MigrationCandidate,
}

/// A validated, ordered route through an operator-declared candidate set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationChain {
    hops: Vec<MigrationHop>,
}

impl MigrationChain {
    pub fn hops(&self) -> &[MigrationHop] {
        &self.hops
    }
}

/// Durable operator approval for a migration chain.
///
/// Candidate manifests remain trusted operator input. The hash commits to the
/// resolved ordered hops, providing plan-to-execution drift detection rather
/// than manifest authenticity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub schema_version: String,
    pub spirit_id: String,
    pub from_version: String,
    pub to_version: String,
    pub candidate_manifest_paths: Vec<String>,
    pub hops: Vec<MigrationHop>,
    pub plan_hash: String,
}

impl MigrationPlan {
    pub const SCHEMA_VERSION: &'static str = "maos.upgrade-plan.v1";

    pub fn new(
        spirit_id: impl Into<String>,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
        candidate_manifest_paths: Vec<String>,
        chain: MigrationChain,
    ) -> Self {
        let mut plan = Self {
            schema_version: Self::SCHEMA_VERSION.into(),
            spirit_id: spirit_id.into(),
            from_version: from_version.into(),
            to_version: to_version.into(),
            candidate_manifest_paths,
            hops: chain.hops,
            plan_hash: String::new(),
        };
        plan.plan_hash = plan.resolved_chain_hash();
        plan
    }

    pub fn resolved_chain_hash(&self) -> String {
        hex::encode(hash_hops(&self.hops))
    }

    pub fn verify_live_chain(&self, live_chain: &MigrationChain) -> Result<(), CohortError> {
        let actual_chain_hash = hex::encode(hash_hops(live_chain.hops()));
        if self.plan_hash != actual_chain_hash {
            return Err(CohortError::EMigrationPlanDrift {
                expected_plan_hash: self.plan_hash.clone(),
                actual_chain_hash,
            });
        }
        Ok(())
    }

    pub fn validate_persisted_hash(&self) -> Result<(), CohortError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(CohortError::ParseError(format!(
                "unsupported migration plan schema_version: {}",
                self.schema_version
            )));
        }
        let actual_chain_hash = self.resolved_chain_hash();
        if self.plan_hash != actual_chain_hash {
            return Err(CohortError::EMigrationPlanDrift {
                expected_plan_hash: self.plan_hash.clone(),
                actual_chain_hash,
            });
        }
        Ok(())
    }
}

fn hash_hops(hops: &[MigrationHop]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(128);
    bytes.extend_from_slice(b"maos.upgrade-plan.v1\0");
    bytes.extend_from_slice(&(hops.len() as u32).to_be_bytes());
    for hop in hops {
        write_lp_bytes(&mut bytes, hop.predecessor_version.as_bytes());
        write_lp_bytes(&mut bytes, hop.successor.class.as_bytes());
        write_lp_bytes(&mut bytes, hop.successor.version.as_bytes());
        bytes.extend_from_slice(&(hop.successor.migrates_from.len() as u32).to_be_bytes());
        for pattern in &hop.successor.migrates_from {
            write_lp_bytes(&mut bytes, pattern.as_bytes());
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn write_lp_bytes(buffer: &mut Vec<u8>, bytes: &[u8]) {
    buffer.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buffer.extend_from_slice(bytes);
}

/// Resolve a route, rejecting a malformed candidate set before walking it.
///
/// Linearity is checked against concrete versions, not the declared pattern
/// strings. Thus `1.0` and `1.x` are a fork for concrete source `1.0`.
pub fn resolve_migration_chain(
    from: &str,
    to: &str,
    candidates: &[MigrationCandidate],
) -> Result<MigrationChain, CohortError> {
    validate_linearity(from, candidates)?;

    if from == to {
        return Ok(MigrationChain { hops: Vec::new() });
    }

    let mut current = from.to_owned();
    let mut seen = BTreeSet::new();
    let mut hops = Vec::new();

    while current != to {
        if !seen.insert(current.clone()) {
            return Err(CohortError::ECohortMigrationChainNotLinear {
                reason: MigrationChainNotLinearReason::Cycle,
                source_version: current,
            });
        }

        let successor = candidates
            .iter()
            .find(|candidate| candidate_matches(candidate, &current))
            .cloned()
            .ok_or_else(|| CohortError::ECohortNoMigrationPath {
                from: from.to_owned(),
                to: to.to_owned(),
            })?;

        let predecessor_version = current;
        current = successor.version.clone();
        hops.push(MigrationHop {
            predecessor_version,
            successor,
        });
    }

    Ok(MigrationChain { hops })
}

fn validate_linearity(from: &str, candidates: &[MigrationCandidate]) -> Result<(), CohortError> {
    let mut concrete_sources = candidates
        .iter()
        .map(|candidate| candidate.version.clone())
        .collect::<BTreeSet<_>>();
    concrete_sources.insert(from.to_owned());

    for source_version in &concrete_sources {
        if candidates
            .iter()
            .filter(|candidate| candidate_matches(candidate, source_version))
            .nth(1)
            .is_some()
        {
            return Err(CohortError::ECohortMigrationChainNotLinear {
                reason: MigrationChainNotLinearReason::ForkAtSource,
                source_version: source_version.clone(),
            });
        }
    }

    for candidate in candidates {
        if candidate_matches(candidate, &candidate.version) {
            return Err(CohortError::ECohortMigrationChainNotLinear {
                reason: MigrationChainNotLinearReason::SelfLoop,
                source_version: candidate.version.clone(),
            });
        }
    }

    for source_version in concrete_sources {
        let mut seen = BTreeSet::new();
        let mut current = source_version;
        loop {
            if !seen.insert(current.clone()) {
                return Err(CohortError::ECohortMigrationChainNotLinear {
                    reason: MigrationChainNotLinearReason::Cycle,
                    source_version: current,
                });
            }

            let Some(successor) = candidates
                .iter()
                .find(|candidate| candidate_matches(candidate, &current))
            else {
                break;
            };
            current = successor.version.clone();
        }
    }

    Ok(())
}

fn candidate_matches(candidate: &MigrationCandidate, version: &str) -> bool {
    candidate
        .migrates_from
        .iter()
        .any(|pattern| matches_version_pattern(pattern, version))
}

/// Exact semantic twin of the existing kernel's single-hop matcher.
///
/// The chain layer cannot depend on `maos-kernel-core` without violating its
/// zero-kernel boundary, so it preserves the same exact-or-final-`x` grammar.
fn matches_version_pattern(pattern: &str, version: &str) -> bool {
    let parts: Vec<&str> = pattern.split('.').collect();
    let version_parts: Vec<&str> = version.split('.').collect();
    if parts.len() != version_parts.len() {
        return false;
    }

    parts.iter().enumerate().all(|(index, part)| {
        (*part == "x" && index == parts.len() - 1) || *part == version_parts[index]
    })
}
