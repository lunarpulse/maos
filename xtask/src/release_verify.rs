//! Story 9.4 AC-1 — `release-verify` xtask subcommand.
//!
//! Two modes:
//! - `--sign`: reads a `SHA256SUMS` file, signs it with an Ed25519 key
//!   (loaded from `--key-env` or `--key-file`), writes the 64-byte raw
//!   signature to `--output`.
//! - `--verify`: reads `SHA256SUMS` + `.sig`, verifies the Ed25519
//!   signature against the bundled `RELEASE_PUBKEY`, then verifies each
//!   file in `--artifacts-dir` against the signed manifest.
//!
//! All crypto delegated to `maos_audit::release_verify`.

use maos_audit::release_verify::{
    parse_sha256sums, sign_sha256sums, verify_release, RELEASE_PUBKEY,
};
use maos_domain::audit_key::parse_seed_bytes;
use std::path::Path;

pub fn run(
    sign: bool,
    verify: bool,
    sha256sums: Option<&str>,
    sig: Option<&str>,
    output: Option<&str>,
    key_env: Option<&str>,
    key_file: Option<&str>,
    artifacts_dir: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if sign == verify {
        return Err("exactly one of --sign or --verify must be specified".into());
    }

    if sign {
        run_sign(sha256sums, output, key_env, key_file, json)
    } else {
        run_verify(sha256sums, sig, artifacts_dir, json)
    }
}

fn run_sign(
    sha256sums: Option<&str>,
    output: Option<&str>,
    key_env: Option<&str>,
    key_file: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let sums_path = sha256sums.ok_or("--sha256sums is required for --sign")?;
    let output_path = output.ok_or("--output is required for --sign")?;

    // Load the signing key seed
    let seed = load_signing_key(key_env, key_file)?;

    // Read SHA256SUMS content
    let sums_bytes =
        std::fs::read(sums_path).map_err(|e| format!("failed to read {sums_path}: {e}"))?;

    // Validate that the SHA256SUMS content parses correctly
    let sums_str = std::str::from_utf8(&sums_bytes)
        .map_err(|e| format!("SHA256SUMS is not valid UTF-8: {e}"))?;
    let entries =
        parse_sha256sums(sums_str).map_err(|e| format!("failed to parse SHA256SUMS: {e}"))?;

    // Sign
    let sig = sign_sha256sums(&sums_bytes, &seed);

    // Write raw signature
    std::fs::write(output_path, sig)
        .map_err(|e| format!("failed to write signature to {output_path}: {e}"))?;

    if json {
        let payload = serde_json::json!({
            "mode": "sign",
            "sha256sums": sums_path,
            "output": output_path,
            "entries_signed": entries.len(),
            "passed": true,
        });
        println!("{payload}");
    } else {
        eprintln!(
            "release-verify: signed {} entries from {} → {}",
            entries.len(),
            sums_path,
            output_path,
        );
    }

    Ok(())
}

fn run_verify(
    sha256sums: Option<&str>,
    sig: Option<&str>,
    artifacts_dir: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let sums_path = sha256sums.ok_or("--sha256sums is required for --verify")?;
    let sig_path = sig.ok_or("--sig is required for --verify")?;
    let dir = artifacts_dir.ok_or("--artifacts-dir is required for --verify")?;

    // Read inputs
    let sums_bytes =
        std::fs::read(sums_path).map_err(|e| format!("failed to read {sums_path}: {e}"))?;
    let sig_bytes_vec =
        std::fs::read(sig_path).map_err(|e| format!("failed to read {sig_path}: {e}"))?;

    let sig_array: [u8; 64] = sig_bytes_vec.as_slice().try_into().map_err(|_| {
        format!(
            "signature file must be exactly 64 bytes, got {}",
            sig_bytes_vec.len()
        )
    })?;

    // Parse entries to know which files to load
    let sums_content = std::str::from_utf8(&sums_bytes)
        .map_err(|e| format!("SHA256SUMS is not valid UTF-8: {e}"))?;
    let entries =
        parse_sha256sums(sums_content).map_err(|e| format!("failed to parse SHA256SUMS: {e}"))?;

    // Load artifact files from directory. Fail if any manifest entry is missing
    // so the CI gate is honest and cannot pass with a partial download.
    let dir_path = Path::new(dir);
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for entry in &entries {
        let file_path = dir_path.join(&entry.filename);
        if !file_path.exists() {
            return Err(format!(
                "manifest entry '{}' not found in artifacts dir {}",
                entry.filename,
                dir_path.display()
            ));
        }
        let data = std::fs::read(&file_path)
            .map_err(|e| format!("failed to read {}: {e}", file_path.display()))?;
        files.push((entry.filename.clone(), data));
    }
    // Build references for verify_release
    let file_refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();

    // Run the full verification pipeline in strict mode: every manifest entry
    // must be present in the artifacts directory.
    let verified = verify_release(&sums_bytes, &sig_array, &RELEASE_PUBKEY, &file_refs, false)
        .map_err(|e| format!("release verification failed: {e}"))?;

    if json {
        let payload = serde_json::json!({
            "mode": "verify",
            "sha256sums": sums_path,
            "sig": sig_path,
            "artifacts_dir": dir,
            "entries_verified": verified.len(),
            "passed": true,
        });
        println!("{payload}");
    } else {
        eprintln!(
            "release-verify: PASS — signature valid, {} artifact(s) verified",
            verified.len(),
        );
    }

    Ok(())
}

/// Load the Ed25519 signing key from either an env var name or a file path.
fn load_signing_key(key_env: Option<&str>, key_file: Option<&str>) -> Result<[u8; 32], String> {
    let raw = if let Some(env_name) = key_env {
        let hex_str = std::env::var(env_name)
            .map_err(|_| format!("env var {env_name} not set or not valid UTF-8"))?;
        hex_str.into_bytes()
    } else if let Some(path) = key_file {
        std::fs::read(path).map_err(|e| format!("failed to read key file {path}: {e}"))?
    } else {
        return Err("one of --key-env or --key-file is required for --sign".into());
    };

    parse_seed_bytes(&raw).map_err(|e| format!("failed to parse signing key: {e}"))
}
