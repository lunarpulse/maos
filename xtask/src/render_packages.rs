//! Story 20-2: render package recipes from the signed artifact denominator.
use std::{collections::BTreeMap, fs, path::Path, process::Command};

use maos_audit::release_verify::{parse_sha256sums, sha256_hex, verify_release};

const BINS: [&str; 4] = ["maos", "maosctl", "maos-spirit", "maos-registry-server"];

pub fn versions(version: &str) -> Result<(String, String), String> {
    let (core, pre) = version.split_once('-').unwrap_or((version, ""));
    if core.split('.').count() != 3
        || core
            .split('.')
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        || (!pre.is_empty()
            && (!pre.as_bytes()[0].is_ascii_alphabetic()
                || pre
                    .split('.')
                    .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_alphanumeric()))))
        || version.ends_with('-')
    {
        return Err(format!("unsupported release version: {version}"));
    }
    Ok(if pre.is_empty() {
        (core.to_owned(), core.to_owned())
    } else {
        (
            format!("{core}~{pre}"),
            format!("{core}{}", pre.replace('.', "")),
        )
    })
}

pub fn render(template: &str, values: &BTreeMap<String, String>) -> Result<String, String> {
    let mut text = template.to_owned();
    for (key, value) in values {
        text = text.replace(&format!("@{key}@"), value);
    }
    if text.contains("SCAFFOLD")
        || text.contains("PLACEHOLDER_")
        || regex::Regex::new(r"@[A-Z][A-Za-z0-9_-]*@")
            .map_err(|e| e.to_string())?
            .is_match(&text)
    {
        return Err("unrendered package token survived".into());
    }
    Ok(text)
}

/// `maosctl` must be the locally built trusted tool, not an unverified download.
pub fn run(dist: &str, output: &str, maosctl: &str, base_url: Option<&str>) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("workspace root")?;
    let metadata: toml::Value = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: toml::de::Error| e.to_string())?;
    let version = metadata["workspace"]["package"]["version"]
        .as_str()
        .ok_or("workspace version")?;
    let key = Command::new(maosctl)
        .arg("release-pubkey")
        .output()
        .map_err(|e| e.to_string())?;
    if !key.status.success() {
        return Err("trusted maosctl release-pubkey failed".into());
    }
    let key = String::from_utf8(key.stdout).map_err(|e| e.to_string())?;
    let key = key.trim();
    let pubkey: [u8; 32] = hex::decode(key)
        .map_err(|e| e.to_string())?
        .try_into()
        .map_err(|_| "public key length")?;
    let dist = Path::new(dist);
    let sums = fs::read(dist.join("SHA256SUMS")).map_err(|e| e.to_string())?;
    let signature = fs::read(dist.join("SHA256SUMS.sig")).map_err(|e| e.to_string())?;
    let sig: [u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| "signature length")?;
    let entries = parse_sha256sums(std::str::from_utf8(&sums).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut files = Vec::with_capacity(entries.len());
    let mut hashes = BTreeMap::new();
    for entry in entries {
        if !entry
            .filename
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || hashes.contains_key(&entry.filename)
        {
            return Err("unsafe or duplicate artifact filename".into());
        }
        let path = dist.join(&entry.filename);
        if !fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_file()
        {
            return Err("artifact must be a regular file".into());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        hashes.insert(entry.filename.clone(), sha256_hex(&bytes));
        files.push((entry.filename, bytes));
    }
    let refs: Vec<_> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    verify_release(&sums, &sig, &pubkey, &refs, false).map_err(|e| e.to_string())?;
    let (deb, arch) = versions(version)?;
    let verifier = fs::read(root.join("packaging/verify-release.py")).map_err(|e| e.to_string())?;
    let url = base_url.map(str::to_owned).unwrap_or_else(|| {
        format!("https://github.com/lunarpulse/maos/releases/download/v{version}")
    });
    if !url
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b":/._-".contains(&b))
    {
        return Err("unsafe package base URL".into());
    }
    let mut values = BTreeMap::from([
        ("VERSION".into(), version.to_owned()),
        ("DEB_VERSION".into(), deb),
        ("ARCH_VERSION".into(), arch),
        ("PUBKEY".into(), key.to_owned()),
        ("BASE_URL".into(), url.clone()),
        ("SHA256SUMS_HASH".into(), sha256_hex(&sums)),
        ("SIGNATURE_HASH".into(), sha256_hex(&signature)),
        ("VERIFIER_HASH".into(), sha256_hex(&verifier)),
    ]);
    let mut brew = String::new();
    let mut sources = String::new();
    let mut arches = Vec::new();
    let targets = [
        ("linux-amd64", "x86_64"),
        ("linux-arm64", "aarch64"),
        ("darwin-arm64", ""),
    ];
    let brew_width = hashes
        .keys()
        .filter_map(|name| name.strip_prefix("maos-"))
        .filter(|suffix| targets.iter().any(|(target, _)| target == suffix))
        .map(str::len)
        .max()
        .unwrap_or_default();
    for (suffix, arch) in targets {
        let present = BINS
            .iter()
            .filter(|b| hashes.contains_key(&format!("{b}-{suffix}")))
            .count();
        if present == 0 {
            continue;
        }
        if present != BINS.len() {
            return Err(format!("incomplete four-binary set: {suffix}"));
        }
        let hs: Vec<_> = BINS
            .iter()
            .map(|b| format!("\"{}\"", hashes[&format!("{b}-{suffix}")]))
            .collect();
        brew.push_str("    \"");
        brew.push_str(suffix);
        brew.push('"');
        for _ in suffix.len()..=brew_width {
            brew.push(' ');
        }
        brew.push_str("=> %w[\n");
        for hash in &hs {
            brew.push_str("      ");
            brew.push_str(hash.trim_matches('"'));
            brew.push('\n');
        }
        brew.push_str("    ],\n");
        if !arch.is_empty() {
            arches.push(format!("'{arch}'"));
            let urls: Vec<_> = BINS
                .iter()
                .map(|b| format!("'{url}/{b}-{suffix}'"))
                .collect();
            sources.push_str(&format!(
                "source_{arch}=({})\nsha256sums_{arch}=({})\n",
                urls.join(" "),
                hs.join(" ")
            ));
        }
    }
    if arches.is_empty() {
        return Err("no Linux package artifacts".into());
    }
    values.insert("BREW_RELEASES".into(), brew);
    values.insert("ARCHES".into(), arches.join(" "));
    values.insert("ARCH_SOURCES".into(), sources);
    let output = Path::new(output);
    for (source, dest) in [
        ("deb/rules", "debian/rules"),
        ("deb/control", "debian/control"),
        ("deb/changelog", "debian/changelog"),
        ("deb/source/format", "debian/source/format"),
        ("rpm/maos.spec", "maos.spec"),
        ("aur/PKGBUILD", "PKGBUILD"),
        ("homebrew/maos.rb", "maos.rb"),
    ] {
        let template =
            fs::read_to_string(root.join("packaging").join(source)).map_err(|e| e.to_string())?;
        let rendered = render(&template, &values)?;
        let dest = output.join(dest);
        fs::create_dir_all(dest.parent().ok_or("package parent")?).map_err(|e| e.to_string())?;
        fs::write(dest, rendered).map_err(|e| e.to_string())?;
    }
    fs::write(output.join("verify-release.py"), verifier).map_err(|e| e.to_string())?;
    println!(
        "render-packages: verified and rendered {version} from {}",
        dist.display()
    );
    Ok(())
}
