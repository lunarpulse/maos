//! Seed loading for the secret-redaction generator.
//!
//! Parses the TOML seed file.  At compile time the seed file is included
//! via `include_bytes!`; at test time fixture paths are read from disk.

use super::SecretRedactionSeed;

/// Load seeds from TOML bytes.
pub fn load_seeds(data: &[u8]) -> Result<Vec<SecretRedactionSeed>, String> {
    let text =
        std::str::from_utf8(data).map_err(|e| format!("seed TOML not valid UTF-8: {}", e))?;

    #[derive(serde::Deserialize)]
    struct SeedFile {
        seeds: Vec<TomlSeed>,
    }

    #[derive(serde::Deserialize)]
    struct TomlSeed {
        id: String,
        class: String,
        pattern_regex: String,
        false_positive_negative_anchors: Vec<String>,
        example_redacted_form: String,
    }

    let file: SeedFile =
        toml::from_str(text).map_err(|e| format!("failed to parse seed TOML: {}", e))?;

    Ok(file
        .seeds
        .into_iter()
        .map(|s| SecretRedactionSeed {
            id: s.id,
            class: s.class,
            pattern_regex: s.pattern_regex,
            false_positive_negative_anchors: s.false_positive_negative_anchors,
            example_redacted_form: s.example_redacted_form,
        })
        .collect())
}
