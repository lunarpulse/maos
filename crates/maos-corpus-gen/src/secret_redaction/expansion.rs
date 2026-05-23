//! Deterministic expansion for the secret-redaction generator.
//!
//! Every item is produced as a pure function of `(seed_index, variant_combo_index)`.
//! No RNG, no system clock, no env reads.  Byte-identical output across all hosts.
//!
//! Items are allocated **per seed** to guarantee per-class floor satisfaction:
//! each seed produces `n / seed_count` items, distributed across deterministic
//! variant combos.

use sha2::{Digest, Sha256};

use super::SecretRedactionItem;
use super::SecretRedactionSeed;

/// Expand `n` items deterministically from the seed corpus.
///
/// Items are allocated per seed: each seed produces `items_per_seed = n / seed_count`
/// variants, with any remainder distributed to the first seeds. After generation,
/// a stable-sort + dedup-by-canonical-form pass removes duplicates. If dedup drops
/// items, additional items are generated from under-represented seeds to backfill
/// toward `n`. This guarantees per-class floor satisfaction proportional to seed
/// distribution.
pub fn expand_deterministic(seeds: &[SecretRedactionSeed], n: usize) -> Vec<SecretRedactionItem> {
    if seeds.is_empty() || n == 0 {
        return vec![];
    }

    let seed_count = seeds.len();
    let base_per_seed = n / seed_count;
    let remainder = n % seed_count;

    // Phase 1: Generate n items with deterministic variant combos.
    let mut items: Vec<SecretRedactionItem> = Vec::with_capacity(n);
    let mut global_idx = 0;

    for (seed_idx, seed) in seeds.iter().enumerate() {
        let this_seed_n = if seed_idx < remainder {
            base_per_seed + 1
        } else {
            base_per_seed
        };

        for variant_idx in 0..this_seed_n {
            let variant_combo = deterministic_variant(variant_idx, seed.id.as_bytes());
            let item = build_item(global_idx, seed, &variant_combo);
            items.push(item);
            global_idx += 1;
        }
    }

    // Phase 2: Stable sort by id for cross-host stability, then dedup.
    items.sort_by(|a, b| a.id.cmp(&b.id));

    let pre_dedup_len = items.len();
    items.dedup_by(|a, b| canonical_form(a) == canonical_form(b));
    let dropped = pre_dedup_len - items.len();

    // Phase 3: Backfill if dedup dropped items.
    if dropped > 0 {
        let mut backfill_idx = global_idx;
        let seed_ids_present: std::collections::BTreeSet<String> =
            items.iter().map(|i| i.seed_id.clone()).collect();

        for seed in seeds.iter() {
            while items.len() < n {
                if !seed_ids_present.contains(&seed.id) && items.len() >= n {
                    break;
                }
                let variant_combo = deterministic_variant(backfill_idx, seed.id.as_bytes());
                let item = build_item(backfill_idx, seed, &variant_combo);
                let cf = canonical_form(&item);
                let is_dup = items.iter().any(|existing| canonical_form(existing) == cf);
                if !is_dup {
                    items.push(item);
                }
                backfill_idx += 1;
                if backfill_idx > global_idx + dropped * 3 {
                    break;
                }
            }
        }

        // Sort by id again and re-number.
        items.sort_by(|a, b| a.id.cmp(&b.id));
    }

    // Re-number ids sequentially so they are consecutive 1..=items.len()
    for (i, item) in items.iter_mut().enumerate() {
        item.id = format!("secret-red-{:05}", i + 1);
    }

    // Truncate to exactly n if oversampling produced more.
    items.truncate(n);

    items
}

/// Canonical form for deduplication: class + raw content hash.
fn canonical_form(item: &SecretRedactionItem) -> String {
    use sha2::Digest;
    let mut h = Sha256::new();
    h.update(item.class.as_bytes());
    h.update(item.raw.as_bytes());
    format!("{:x}", h.finalize())
}

/// Build a single corpus item from a seed and variant combo.
fn build_item(idx: usize, seed: &SecretRedactionSeed, variant_combo: &str) -> SecretRedactionItem {
    let id = format!("secret-red-{:05}", idx + 1);
    let raw = synthetic_raw(seed, idx, variant_combo);
    let expected_redacted = synthetic_redacted(seed, &raw);

    SecretRedactionItem {
        id,
        class: seed.class.clone(),
        raw,
        expected_redacted,
        seed_id: seed.id.clone(),
        variant_combo: variant_combo.to_string(),
    }
}

/// Deterministic variant string derived from a (variant_idx, seed_id) pair.
fn deterministic_variant(variant_idx: usize, seed_id_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed_id_bytes);
    hasher.update(b":variant:");
    hasher.update(variant_idx.to_le_bytes());
    let hash = hasher.finalize();
    let hex = hash
        .iter()
        .take(4)
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    format!("v-{}", hex)
}

/// Build a synthetic secret-containing raw string.
fn synthetic_raw(seed: &SecretRedactionSeed, idx: usize, variant_combo: &str) -> String {
    let dummy_secret = synthetic_secret_value(&seed.class, seed.id.as_bytes(), idx);
    let form = (idx + variant_combo.len()) % 6;

    match form {
        0 => format!(
            r#"{{"api_key": "{}", "model": "claude-test-{}"}}"#,
            dummy_secret,
            idx % 10
        ),
        1 => format!("export TEST_MAOS_SECRET_{}=\"{}\"", idx % 100, dummy_secret),
        2 => format!(
            "connection test established with key: {} [timestamp: test-{}]",
            dummy_secret, idx
        ),
        3 => format!(
            "[database-test]\nurl={}\npool_size={}\n",
            dummy_secret,
            idx % 50 + 1
        ),
        4 => format!(
            "Authorization: Bearer {} {{ \"scope\": \"test:{}\" }}",
            dummy_secret,
            idx % 5
        ),
        _ => format!("https://api.test.example.com/v1?key={}", dummy_secret),
    }
}

/// Generate a clearly synthetic secret value per seed class.
fn synthetic_secret_value(class: &str, seed_id: &[u8], idx: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed_id);
    hasher.update(&idx.to_le_bytes());
    let h = hasher.finalize();
    let hex: String = h.iter().take(12).map(|b| format!("{:02x}", b)).collect();

    match class {
        "api_key_anthropic" => format!("sk-ant-TEST-{}", hex),
        "api_key_openai" => format!("sk-test-{}", hex),
        "oauth_token" => match idx % 5 {
            0 => format!("xoxb-TEST-{}", hex),
            1 => format!("ghp_TEST_{}", hex),
            2 => format!("ghs_TEST_{}", hex),
            3 => format!("gho_TEST_{}", hex),
            _ => format!("pat-TEST-{}", hex),
        },
        "private_key_pem" => format!(
            "-----BEGIN TEST PRIVATE KEY-----\n{}\n-----END TEST PRIVATE KEY-----",
            (0..4)
                .map(|j| {
                    let mut h2 = Sha256::new();
                    h2.update(hex.as_bytes());
                    h2.update(&[j as u8]);
                    hex::encode(&h2.finalize()[..8])
                })
                .collect::<Vec<_>>()
                .join("\n")
        ),
        "database_url" => format!("postgres://test-user:{}@localhost:5432/testdb", hex),
        "jwt" => {
            let header = base64url_synth(&format!("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCIsInRlc3QiOnRydWV9"));
            let payload = base64url_synth(&format!(
                r#"{{"sub":"test-{}","iat":{},"exp":9999999999}}"#,
                idx, 1700000000 + idx as u64
            ));
            let sig = base64url_synth(&hex);
            format!("{}.{}.{}", header, payload, sig)
        }
        "aws_credentials" => {
            let access = format!("AKIA-TEST-{}", &hex[..16].to_uppercase());
            let secret = format!("wJalrXUtnFEMI/K7MDENG/bPxRfiCY-TEST-{}", &hex);
            match idx % 3 {
                0 => format!("AWS_ACCESS_KEY_ID={}\nAWS_SECRET_ACCESS_KEY={}", access, secret),
                1 => format!("export AWS_SESSION_TOKEN=TEST-{}", &hex[..8]),
                _ => format!(
                    "[default]\naws_access_key_id = {}\naws_secret_access_key = {}",
                    access, secret
                ),
            }
        }
        "gcp_service_account" => format!(
            r#"{{"type":"service_account","project_id":"test-project-{}","private_key_id":"{}","private_key":"-----BEGIN TEST PRIVATE KEY-----\\nSynthetic test key for seed {} idx {}\\n-----END TEST PRIVATE KEY-----","client_email":"test-{}@test-project-{}.iam.gserviceaccount.com"}}"#,
            idx % 100, hex, seed_id.first().copied().unwrap_or(b'?'), idx, idx % 100, idx % 100
        ),
        "azure_credentials" => match idx % 3 {
            0 => format!(
                "DefaultEndpointsProtocol=https;AccountName=testaccount{};AccountKey=TEST-{}==;EndpointSuffix=core.windows.net",
                idx % 50, hex
            ),
            1 => format!("sv=2024-01-01&sig=TEST-{}&srt=co", &hex[..16]),
            _ => format!(
                "AZURE_SUBSCRIPTION_ID=test-sub-{}\nAZURE_CLIENT_SECRET=TEST-{}",
                idx % 10, hex
            ),
        },
        "ssh_key_block" => format!(
            "-----BEGIN OPENSSH TEST PRIVATE KEY-----\n{}\n-----END OPENSSH TEST PRIVATE KEY-----",
            (0..5)
                .map(|j| format!("b3BlbnNzaC1rZXkvdGVzdC12MS0{}TEST{}", j, idx))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        "gpg_armored_block" => format!(
            "-----BEGIN PGP TEST PRIVATE KEY BLOCK-----\n\n{}\n={}\n-----END PGP TEST PRIVATE KEY BLOCK-----",
            (0..4)
                .map(|j| format!("mQFNTEST{}", j))
                .collect::<Vec<_>>()
                .join("\n"),
            &base64url_synth(&hex)[..6]
        ),
        "canary_marker" => format!("<CANARY-TEST-{:04}-{}>", idx, hex),
        _ => format!("SECRET-TEST-{}-{}", class, hex),
    }
}

/// Deterministic "base64url-ish" encoding (synthetic-only, no base64 crate).
fn base64url_synth(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let digest = h.finalize();
    let map = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(16);
    for chunk in digest[..6].chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(map[(triple >> 18) as usize & 0x3f] as char);
        out.push(map[(triple >> 12) as usize & 0x3f] as char);
        if chunk.len() > 1 {
            out.push(map[(triple >> 6) as usize & 0x3f] as char);
        }
        if chunk.len() > 2 {
            out.push(map[(triple) as usize & 0x3f] as char);
        }
    }
    out
}

/// Build the expected redacted form for a raw string containing a synthetic secret.
fn synthetic_redacted(seed: &SecretRedactionSeed, raw: &str) -> String {
    let short_hash = {
        let mut h = Sha256::new();
        h.update(raw.as_bytes());
        let d = h.finalize();
        d.iter()
            .take(4)
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    };
    format!(
        "<REDACTED:type={},len={},hash={}>",
        seed.class,
        raw.len(),
        short_hash
    )
}

/// Convert bytes to hex string.
pub(crate) mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
