//! Canonical team identity for physical tenant isolation (ADR-055).
//!
//! Team identifiers are signed manifest material and database-routing operands.
//! They are accepted only in frozen lowercase ASCII form; inputs are never
//! normalized because rewriting signed text would make the authority ambiguous.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct TeamId(String);

impl TeamId {
    pub const MIN_LEN: usize = 2;
    pub const MAX_LEN: usize = 32;

    pub fn new(raw: &str) -> Result<Self, TeamIdError> {
        let valid_len = (Self::MIN_LEN..=Self::MAX_LEN).contains(&raw.len());
        let valid_chars = raw
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !valid_len || !valid_chars {
            return Err(TeamIdError::ETeamIdInvalid {
                team_id: raw.to_string(),
            });
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TeamId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for TeamId {
    type Error = TeamIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl From<TeamId> for String {
    fn from(value: TeamId) -> Self {
        value.0
    }
}

impl<'de> serde::Deserialize<'de> for TeamId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(&raw).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TeamIdError {
    #[error("invalid team id '{team_id}': expected canonical lowercase ASCII [a-z0-9-]{{2,32}}")]
    ETeamIdInvalid { team_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_team_ids() {
        assert_eq!(TeamId::new("security-2").unwrap().as_str(), "security-2");
    }

    #[test]
    fn rejects_noncanonical_instead_of_normalizing() {
        for raw in ["A-team", " team-a", "a", "team_a", "tëam"] {
            assert!(TeamId::new(raw).is_err(), "{raw:?} must be rejected");
        }
    }

    #[test]
    fn serde_reuses_constructor_validation() {
        assert!(serde_json::from_str::<TeamId>("\"TEAM-A\"").is_err());
        let id: TeamId = serde_json::from_str("\"team-a\"").unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"team-a\"");
    }
}
