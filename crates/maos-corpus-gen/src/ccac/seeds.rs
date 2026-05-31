//! Seed loading for the CCAC generator.

use super::CcacSeed;

/// Load CCAC seeds from TOML bytes.
pub fn load_seeds(data: &[u8]) -> Result<Vec<CcacSeed>, String> {
    let text = std::str::from_utf8(data).map_err(|e| format!("seed TOML not valid UTF-8: {e}"))?;

    #[derive(serde::Deserialize)]
    struct SeedFile {
        seeds: Vec<CcacSeed>,
    }

    let file: SeedFile =
        toml::from_str(text).map_err(|e| format!("failed to parse CCAC seed TOML: {e}"))?;
    Ok(file.seeds)
}
