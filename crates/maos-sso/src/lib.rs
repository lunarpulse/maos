#![forbid(unsafe_code)]

//! `maos-sso` - out-of-kernel enterprise identity: OIDC assertion verifier.
//!
//! Verifies OIDC JWT assertions against configured static JWKS material with an
//! explicit algorithm allowlist. The verifier is offline and deterministic for
//! CI: no live IdP/network dependency is required for the tripwire tests.

#[cfg(all(feature = "sso-fault-inject", not(debug_assertions)))]
compile_error!("sso-fault-inject is dev/CI-only and MUST NOT ship in release builds");

use std::collections::{HashMap, HashSet};

use base64::Engine;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use maos_domain::ports::{AuthenticatedPrincipal, IdentityAssertionPort, IdentityError};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OidcAlgorithm {
    Rs256,
    Es256,
}

impl OidcAlgorithm {
    fn jwt_alg(self) -> Algorithm {
        match self {
            Self::Rs256 => Algorithm::RS256,
            Self::Es256 => Algorithm::ES256,
        }
    }

    fn header_name(self) -> &'static str {
        match self {
            Self::Rs256 => "RS256",
            Self::Es256 => "ES256",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OidcVerifierError {
    #[error("static JWKS JSON is malformed: {0}")]
    MalformedJwks(String),
    #[error("no supported signing keys found in JWKS")]
    NoKeys,
    #[error("algorithm allowlist is empty")]
    EmptyAlgorithmAllowlist,
    #[error("trusted issuer allowlist is empty")]
    EmptyTrustedIssuers,
    #[error("expected audience is empty")]
    EmptyAudience,
}

#[derive(Debug, Clone)]
pub struct OidcVerifier {
    keys: HashMap<String, Jwk>,
    allowed_algorithms: HashSet<OidcAlgorithm>,
    trusted_issuers: HashSet<String>,
    expected_audience: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernedAuthorization {
    pub principal_attributes: HashMap<String, String>,
    pub provenance: IdentityProvenanceRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityProvenanceRecord {
    pub kind: String,
    pub subject: String,
    pub issuer: String,
    pub spirit_pid: u32,
    pub capability_key: String,
    pub decision_time_ns: u64,
    attested: bool,
}

impl IdentityProvenanceRecord {
    pub fn synthetic(
        kind: impl Into<String>,
        subject: impl Into<String>,
        issuer: impl Into<String>,
        spirit_pid: u32,
        capability_key: impl Into<String>,
        decision_time_ns: u64,
    ) -> Self {
        Self {
            kind: kind.into(),
            subject: subject.into(),
            issuer: issuer.into(),
            spirit_pid,
            capability_key: capability_key.into(),
            decision_time_ns,
            attested: false,
        }
    }

    fn attested(
        subject: String,
        issuer: String,
        spirit_pid: u32,
        capability_key: String,
        decision_time_ns: u64,
    ) -> Self {
        Self {
            kind: "identity.asserted".to_string(),
            subject,
            issuer,
            spirit_pid,
            capability_key,
            decision_time_ns,
            attested: true,
        }
    }
}

pub fn reconcile_provenance(records: &[IdentityProvenanceRecord]) -> usize {
    records
        .iter()
        .filter(|record| record.attested && record.kind == "identity.asserted")
        .count()
}

impl OidcVerifier {
    pub fn from_static_jwks(
        jwks_json: &str,
        allowed_algorithms: &[OidcAlgorithm],
        trusted_issuers: &[&str],
        expected_audience: &str,
    ) -> Result<Self, OidcVerifierError> {
        if allowed_algorithms.is_empty() {
            return Err(OidcVerifierError::EmptyAlgorithmAllowlist);
        }
        if trusted_issuers.is_empty() {
            return Err(OidcVerifierError::EmptyTrustedIssuers);
        }
        if expected_audience.is_empty() {
            return Err(OidcVerifierError::EmptyAudience);
        }

        let jwks: Jwks = serde_json::from_str(jwks_json)
            .map_err(|e| OidcVerifierError::MalformedJwks(e.to_string()))?;
        let keys: HashMap<String, Jwk> = jwks
            .keys
            .into_iter()
            .filter(|key| key.kty == "RSA" || key.kty == "EC")
            .map(|key| (key.kid.clone(), key))
            .collect();
        if keys.is_empty() {
            return Err(OidcVerifierError::NoKeys);
        }

        Ok(Self {
            keys,
            allowed_algorithms: allowed_algorithms.iter().copied().collect(),
            trusted_issuers: trusted_issuers.iter().map(|issuer| (*issuer).to_string()).collect(),
            expected_audience: expected_audience.to_string(),
        })
    }

    pub fn govern_authorization(
        &self,
        assertion: &str,
        spirit_pid: u32,
        capability_key: &str,
    ) -> Result<GovernedAuthorization, IdentityError> {
        let principal = self.verify(assertion)?;
        let provenance = IdentityProvenanceRecord::attested(
            principal.subject.clone(),
            principal.issuer.clone(),
            spirit_pid,
            capability_key.to_string(),
            now_ns()?,
        );
        Ok(GovernedAuthorization {
            principal_attributes: principal.attributes,
            provenance,
        })
    }

    fn verify_real(&self, assertion: &str) -> Result<AuthenticatedPrincipal, IdentityError> {
        let header = parse_header(assertion)?;
        let algorithm = parse_algorithm(&header.alg)?;
        if !self.allowed_algorithms.contains(&algorithm) {
            return Err(IdentityError::AlgorithmRejected);
        }

        let kid = header.kid.ok_or(IdentityError::JwksUnavailable)?;
        let jwk = self.keys.get(&kid).ok_or(IdentityError::JwksUnavailable)?;
        if let Some(jwk_alg) = &jwk.alg {
            if jwk_alg != algorithm.header_name() {
                return Err(IdentityError::AlgorithmRejected);
            }
        }

        let decoding_key = decoding_key_for(jwk, algorithm)?;
        let mut validation = Validation::new(algorithm.jwt_alg());
        // P2: fail-closed clock-skew — no 60s library default. An assertion
        // valid only inside the default leeway window is still rejected.
        validation.leeway = 0;
        // P2: validate_exp explicit (do not rely on the library default).
        validation.validate_exp = true;
        validation.algorithms = self
            .allowed_algorithms
            .iter()
            .map(|alg| alg.jwt_alg())
            .collect();
        validation.set_audience(&[self.expected_audience.as_str()]);
        validation.set_issuer(
            &self
                .trusted_issuers
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );
        validation.validate_nbf = true;

        let data = decode::<Claims>(assertion, &decoding_key, &validation)
            .map_err(map_jwt_error)?;
        let claims = data.claims;
        if !self.trusted_issuers.contains(&claims.iss) {
            return Err(IdentityError::IssuerUntrusted);
        }
        if !claims.aud.iter().any(|aud| aud == &self.expected_audience) {
            return Err(IdentityError::AudienceMismatch);
        }

        Ok(claims.into_principal())
    }
}

impl IdentityAssertionPort for OidcVerifier {
    fn verify(&self, assertion: &str) -> Result<AuthenticatedPrincipal, IdentityError> {
        #[cfg(feature = "sso-fault-inject")]
        {
            let claims = parse_unverified_claims(assertion)?;
            return Ok(claims.into_principal());
        }

        #[cfg(not(feature = "sso-fault-inject"))]
        {
            self.verify_real(assertion)
        }
    }

    fn is_healthy(&self) -> bool {
        !self.keys.is_empty() && !self.allowed_algorithms.is_empty() && !self.trusted_issuers.is_empty()
    }
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kty: String,
    kid: String,
    #[serde(default)]
    alg: Option<String>,
    #[serde(default)]
    n: Option<String>,
    #[serde(default)]
    e: Option<String>,
    #[serde(default)]
    x: Option<String>,
    #[serde(default)]
    y: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    alg: String,
    #[serde(default)]
    kid: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Claims {
    sub: String,
    iss: String,
    #[serde(deserialize_with = "deserialize_audience")]
    aud: Vec<String>,
    #[serde(default)]
    exp: Option<u64>,
    #[serde(default)]
    nbf: Option<u64>,
    #[serde(default)]
    iat: Option<u64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

impl Claims {
    fn into_principal(self) -> AuthenticatedPrincipal {
        let mut attributes = HashMap::new();
        attributes.insert("sub".to_string(), self.sub.clone());
        attributes.insert("iss".to_string(), self.iss.clone());
        if let Some(aud) = self.aud.first() {
            attributes.insert("aud".to_string(), aud.clone());
        }
        if let Some(exp) = self.exp {
            attributes.insert("exp".to_string(), exp.to_string());
        }
        if let Some(nbf) = self.nbf {
            attributes.insert("nbf".to_string(), nbf.to_string());
        }
        if let Some(iat) = self.iat {
            attributes.insert("iat".to_string(), iat.to_string());
        }
        if let Some(name) = self.name {
            attributes.insert("name".to_string(), name);
        }
        if let Some(email) = self.email {
            attributes.insert("email".to_string(), email);
        }
        for (key, value) in self.extra {
            if let Some(s) = value.as_str() {
                attributes.entry(key).or_insert_with(|| s.to_string());
            }
        }
        AuthenticatedPrincipal {
            subject: self.sub,
            issuer: self.iss,
            audience: self.aud.into_iter().next().unwrap_or_default(),
            attributes,
        }
    }
}

fn deserialize_audience<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(vec![s]),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(|v| match v {
                serde_json::Value::String(s) => Ok(s),
                _ => Err(serde::de::Error::custom("aud array contains non-string")),
            })
            .collect(),
        _ => Err(serde::de::Error::custom("aud must be string or string array")),
    }
}

fn now_ns() -> Result<u64, IdentityError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| IdentityError::ClockUnavailable)
        .map(|duration| duration.as_nanos().min(u128::from(u64::MAX)) as u64)
}

fn parse_algorithm(alg: &str) -> Result<OidcAlgorithm, IdentityError> {
    match alg {
        "RS256" => Ok(OidcAlgorithm::Rs256),
        "ES256" => Ok(OidcAlgorithm::Es256),
        _ => Err(IdentityError::AlgorithmRejected),
    }
}

fn parse_header(assertion: &str) -> Result<JwtHeader, IdentityError> {
    let header_segment = assertion
        .split('.')
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or(IdentityError::MalformedAssertion)?;
    decode_segment(header_segment).and_then(|bytes| {
        serde_json::from_slice(&bytes).map_err(|_| IdentityError::MalformedAssertion)
    })
}

#[cfg(feature = "sso-fault-inject")]
fn parse_unverified_claims(assertion: &str) -> Result<Claims, IdentityError> {
    let mut segments = assertion.split('.');
    let _header = segments.next().ok_or(IdentityError::MalformedAssertion)?;
    let claims = segments.next().ok_or(IdentityError::MalformedAssertion)?;
    let signature = segments.next().ok_or(IdentityError::MalformedAssertion)?;
    if segments.next().is_some() || signature.is_empty() {
        return Err(IdentityError::MalformedAssertion);
    }
    decode_segment(claims).and_then(|bytes| {
        serde_json::from_slice(&bytes).map_err(|_| IdentityError::MalformedAssertion)
    })
}

fn decode_segment(segment: &str) -> Result<Vec<u8>, IdentityError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| IdentityError::MalformedAssertion)
}

fn decoding_key_for(jwk: &Jwk, algorithm: OidcAlgorithm) -> Result<DecodingKey, IdentityError> {
    match algorithm {
        OidcAlgorithm::Rs256 => {
            let n = jwk.n.as_deref().ok_or(IdentityError::JwksUnavailable)?;
            let e = jwk.e.as_deref().ok_or(IdentityError::JwksUnavailable)?;
            DecodingKey::from_rsa_components(n, e).map_err(|_| IdentityError::JwksUnavailable)
        }
        OidcAlgorithm::Es256 => {
            let x = jwk.x.as_deref().ok_or(IdentityError::JwksUnavailable)?;
            let y = jwk.y.as_deref().ok_or(IdentityError::JwksUnavailable)?;
            DecodingKey::from_ec_components(x, y).map_err(|_| IdentityError::JwksUnavailable)
        }
    }
}

fn map_jwt_error(error: jsonwebtoken::errors::Error) -> IdentityError {
    match error.kind() {
        ErrorKind::ExpiredSignature => IdentityError::Expired,
        ErrorKind::ImmatureSignature => IdentityError::NotYetValid,
        ErrorKind::InvalidAudience => IdentityError::AudienceMismatch,
        ErrorKind::InvalidIssuer => IdentityError::IssuerUntrusted,
        ErrorKind::InvalidAlgorithm => IdentityError::AlgorithmRejected,
        ErrorKind::InvalidSignature => IdentityError::SignatureInvalid,
        ErrorKind::InvalidToken | ErrorKind::InvalidRsaKey(_) | ErrorKind::InvalidEcdsaKey => {
            IdentityError::MalformedAssertion
        }
        _ => IdentityError::SignatureInvalid,
    }
}
