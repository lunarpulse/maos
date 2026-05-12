//! Seed loading for the red-team generator.

use super::RedTeamSeed;

/// Load seeds from TOML bytes.
pub fn load_seeds(data: &[u8]) -> Result<Vec<RedTeamSeed>, String> {
    let text = std::str::from_utf8(data).map_err(|e| format!("seed TOML not valid UTF-8: {}", e))?;

    #[derive(serde::Deserialize)]
    struct SeedFile {
        seeds: Vec<TomlSeed>,
    }

    #[derive(serde::Deserialize)]
    struct TomlSeed {
        id: String,
        class: String,
        attack_summary: String,
        kernel_defense_mechanism: String,
        expected_detection_surface: String,
        parameter_axes: Vec<String>,
        canonical_assertion: String,
    }

    let file: SeedFile =
        toml::from_str(text).map_err(|e| format!("failed to parse seed TOML: {}", e))?;

    Ok(file
        .seeds
        .into_iter()
        .map(|s| RedTeamSeed {
            id: s.id,
            class: s.class,
            attack_summary: s.attack_summary,
            kernel_defense_mechanism: s.kernel_defense_mechanism,
            expected_detection_surface: s.expected_detection_surface,
            parameter_axes: s.parameter_axes,
            canonical_assertion: s.canonical_assertion,
        })
        .collect())
}
