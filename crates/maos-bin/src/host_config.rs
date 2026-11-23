#![forbid(unsafe_code)]

//! Story 22-1 — the Host configuration file and the standing cast.
//!
//! `maos serve --config <host.toml>` starts a Host from ONE file. The file declares the
//! Host's settings (one documented key per setting, each mapped to one registered
//! `MAOS_*` name by [`HOST_SETTINGS`]) and the standing cast (`[[host.spirits]]`).
//! The cast is fed through the existing topology and installed boot paths: there is no
//! restore mechanism, so a restart brings the same Spirits back because the file names
//! them (D-22-1-03 A).
//!
//! ```toml
//! [host]
//! home = "/srv/maos"                  # MAOS_HOME
//! door_bind = "127.0.0.1:7600"        # MAOS_OPERATOR_HTTP_BIND
//! region = "eu-west"                  # MAOS_REGION_HOME
//! crypto_provider = "ring"            # MAOS_CRYPTO_PROVIDER
//! inference_mode = "replay"           # MAOS_INFERENCE_MODE
//!
//! [[host.spirits]]
//! manifest = "spirits/butler/manifest.toml"   # a topology entry; `path` is an alias
//!
//! [[host.spirits]]
//! installed = "researcher"            # a registry-installed Spirit
//! ```
//!
//! A refusal is typed ([`HostConfigKind`]) and never a warning. The environment overrides a
//! key for the process and never stands in for a missing or invalid file.

use crate::topology::TopologyEntry;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const ENV_HOME: &str = "MAOS_HOME";
const ENV_DOOR_BIND: &str = "MAOS_OPERATOR_HTTP_BIND";
const ENV_REGION: &str = "MAOS_REGION_HOME";
const ENV_CRYPTO: &str = "MAOS_CRYPTO_PROVIDER";
const ENV_INFERENCE: &str = "MAOS_INFERENCE_MODE";

/// Every Host-level setting: its `[host]` key and its registered environment name.
/// The Host file accepts no other setting key (`deny_unknown_fields`).
pub const HOST_SETTINGS: &[(&str, &str)] = &[
    ("home", ENV_HOME),
    ("door_bind", ENV_DOOR_BIND),
    ("region", ENV_REGION),
    ("crypto_provider", ENV_CRYPTO),
    ("inference_mode", ENV_INFERENCE),
];

/// The stable error kind of a typed boot refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostConfigKind {
    Missing,
    Parse,
    CastEntry,
    CastHostKey,
    Setting,
    OlderFile,
    InstalledMissing,
    InstalledRefused,
}

impl HostConfigKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "host_config_missing",
            Self::Parse => "host_config_parse",
            Self::CastEntry => "host_config_cast_entry",
            Self::CastHostKey => "host_config_cast_host_key",
            Self::Setting => "host_config_setting",
            Self::OlderFile => "host_config_older_file_disagrees",
            Self::InstalledMissing => "host_config_installed_missing",
            Self::InstalledRefused => "host_config_installed_refused",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostConfigError {
    pub kind: HostConfigKind,
    pub detail: String,
}

impl std::fmt::Display for HostConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.detail)
    }
}

impl std::error::Error for HostConfigError {}

fn refuse(kind: HostConfigKind, detail: impl Into<String>) -> HostConfigError {
    HostConfigError {
        kind,
        detail: detail.into(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostFile {
    host: HostSection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostSection {
    home: Option<String>,
    door_bind: Option<String>,
    region: Option<String>,
    crypto_provider: Option<String>,
    inference_mode: Option<String>,
    #[serde(default)]
    spirits: Vec<CastEntry>,
}

/// The keys of `[[topology.spirits]]` plus `installed`. `host` is parsed only so its
/// presence is a typed refusal: a cross-host target is not a standing-cast member.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CastEntry {
    manifest: Option<String>,
    path: Option<String>,
    host: Option<toml::Value>,
    installed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastSpirit {
    Manifest(PathBuf),
    Installed(String),
}

#[derive(Debug)]
pub struct HostConfig {
    path: PathBuf,
    section: HostSection,
    cast: Vec<CastSpirit>,
}

/// `maos serve --config <path>` — the only form.
pub fn parse_serve_args(args: &[String]) -> Result<PathBuf, String> {
    match args {
        [flag, path] if flag == "--config" && !path.starts_with("--") => Ok(PathBuf::from(path)),
        _ => Err("maos serve: expected: serve --config <host.toml>".into()),
    }
}

/// The home region that stamps rows: the ONE value the composition root passes to
/// `.with_home_region`, and the value the cohort boot reconciles against the signed manifest.
/// Under `serve` the Host file's `region` key reaches the reader as `MAOS_REGION_HOME`.
pub fn stamped_home_region() -> Option<maos_domain::region::Region> {
    maos_kernel_core::security::operator_config::RegionSection::resolve_from_env_and_disk()
        .home_region
}

impl HostConfig {
    pub fn load(path: &Path) -> Result<Self, HostConfigError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| refuse(HostConfigKind::Missing, format!("{}: {e}", path.display())))?;
        let file: HostFile = toml::from_str(&text)
            .map_err(|e| refuse(HostConfigKind::Parse, format!("{}: {e}", path.display())))?;
        let section = file.host;
        for (key, _) in HOST_SETTINGS {
            if let Some(value) = setting(&section, key) {
                validate_setting(key, value)?;
            }
        }
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        let mut cast = Vec::with_capacity(section.spirits.len());
        for (index, entry) in section.spirits.iter().enumerate() {
            cast.push(admit_entry(index, entry, base)?);
        }
        Ok(Self {
            path: path.to_path_buf(),
            section,
            cast,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cast(&self) -> &[CastSpirit] {
        &self.cast
    }

    /// The manifest members of the cast, as the topology entries `maos run` loads.
    pub fn manifest_entries(&self) -> Vec<TopologyEntry> {
        self.cast
            .iter()
            .filter_map(|spirit| match spirit {
                CastSpirit::Manifest(path) => Some(TopologyEntry {
                    manifest: path.to_string_lossy().into_owned(),
                    host: None,
                }),
                CastSpirit::Installed(_) => None,
            })
            .collect()
    }

    /// A cast member that boot refused is a typed refusal of the whole Host: the cast the
    /// file declares is the cast that runs, never a subset.
    pub fn installed_refusal(
        &self,
        code_of: &dyn Fn(&str) -> Option<&'static str>,
    ) -> Result<(), HostConfigError> {
        for spirit in &self.cast {
            if let CastSpirit::Installed(name) = spirit {
                if let Some(code) = code_of(name) {
                    return Err(refuse(
                        HostConfigKind::InstalledRefused,
                        format!("installed spirit '{name}' refused at boot: {code}"),
                    ));
                }
            }
        }
        Ok(())
    }

    /// The cast's manifest members as a `[topology]` document (what `maos run` would read).
    pub fn topology_toml(&self) -> String {
        let spirits: Vec<toml::Value> = self
            .manifest_entries()
            .into_iter()
            .map(|entry| {
                let mut row = toml::Table::new();
                row.insert("manifest".into(), entry.manifest.into());
                row.into()
            })
            .collect();
        let mut topology = toml::Table::new();
        topology.insert("spirits".into(), spirits.into());
        let mut root = toml::Table::new();
        root.insert("topology".into(), topology.into());
        root.to_string()
    }

    /// The installed members of the cast, picked from the registry inventory. A name the
    /// inventory does not hold is a typed refusal; unlisted installed Spirits do not boot.
    pub fn select_installed(
        &self,
        inventory: Vec<(String, PathBuf)>,
    ) -> Result<Vec<(String, PathBuf)>, HostConfigError> {
        let mut chosen = Vec::new();
        for spirit in &self.cast {
            let CastSpirit::Installed(name) = spirit else {
                continue;
            };
            let found = inventory.iter().find(|(held, _)| held == name);
            chosen.push(found.cloned().ok_or_else(|| {
                refuse(
                    HostConfigKind::InstalledMissing,
                    format!("installed spirit '{name}' is not in the registry"),
                )
            })?);
        }
        Ok(chosen)
    }

    /// The settings to place in the process environment: a key whose environment name is
    /// not already set. A set variable overrides the key (it is never overwritten).
    pub fn env_overlay(&self, is_set: &dyn Fn(&str) -> bool) -> Vec<(&'static str, String)> {
        HOST_SETTINGS
            .iter()
            .filter_map(|(key, env)| {
                let value = setting(&self.section, key)?;
                (!is_set(env)).then(|| (*env, value.to_owned()))
            })
            .collect()
    }

    /// D-22-1-05 A: an older file that disagrees with a key this file sets refuses the boot.
    /// `operator.toml` `[region]` against `region`; the provider recorded in `<home>/config.toml`
    /// (default `ring`) against the effective crypto provider (environment over key).
    pub fn reconcile_older_files(
        &self,
        env: &dyn Fn(&str) -> Option<String>,
        operator_toml: Option<&str>,
        config_toml: Option<&str>,
    ) -> Result<(), HostConfigError> {
        let older = |what: String| refuse(HostConfigKind::OlderFile, what);
        let table = |text: Option<&str>| text.and_then(|t| toml::from_str::<toml::Value>(t).ok());
        let canonical = |tag: &str| maos_domain::region::Region::canonicalize(tag).ok();
        if let Some(region) = self.section.region.as_deref() {
            let disk = table(operator_toml).and_then(|doc| {
                doc.get("region")?
                    .get("home_region")?
                    .as_str()
                    .map(str::to_owned)
            });
            if let Some(disk) = disk.filter(|disk| canonical(disk) != canonical(region)) {
                return Err(older(format!(
                    "region = {region} disagrees with operator.toml [region] home_region = {disk}"
                )));
            }
        }
        if let Some(provider) = env(ENV_CRYPTO).or_else(|| self.section.crypto_provider.clone()) {
            validate_setting("crypto_provider", &provider)?;
            let recorded = table(config_toml)
                .and_then(|doc| {
                    doc.get("crypto")?
                        .get("provider")?
                        .as_str()
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "ring".into());
            if recorded != provider {
                return Err(older(format!(
                    "crypto_provider = {provider} disagrees with the provider {recorded} recorded \
                     in <home>/config.toml"
                )));
            }
        }
        Ok(())
    }

    /// The `maos run` shape the Host boots as: the config file stands for the manifest.
    pub fn run_args(&self) -> crate::worker_spawn::RunArgs {
        crate::worker_spawn::RunArgs {
            manifest_path: self.path.to_string_lossy().into_owned(),
            live: false,
            once: false,
            scene: None,
            epic: None,
        }
    }
}

fn setting<'a>(section: &'a HostSection, key: &str) -> Option<&'a str> {
    match key {
        "home" => section.home.as_deref(),
        "door_bind" => section.door_bind.as_deref(),
        "region" => section.region.as_deref(),
        "crypto_provider" => section.crypto_provider.as_deref(),
        "inference_mode" => section.inference_mode.as_deref(),
        _ => None,
    }
}

fn validate_setting(key: &str, value: &str) -> Result<(), HostConfigError> {
    let bad = |why: &str| refuse(HostConfigKind::Setting, format!("{key} = {value:?}: {why}"));
    match key {
        "home" if !Path::new(value).is_absolute() => Err(bad("must be an absolute path")),
        "door_bind" => match value.parse::<std::net::SocketAddr>() {
            Ok(addr) if addr.ip().is_loopback() => Ok(()),
            _ => Err(bad(
                "must be a loopback socket address, e.g. 127.0.0.1:7600",
            )),
        },
        "region" => maos_domain::region::Region::canonicalize(value)
            .map(|_| ())
            .map_err(|e| bad(&e.to_string())),
        "crypto_provider" => crate::crypto_provider::validate(value)
            .map(|_| ())
            .map_err(|e| bad(&e)),
        "inference_mode" if !matches!(value, "live" | "record" | "replay") => {
            Err(bad("must be live, record or replay"))
        }
        _ => Ok(()),
    }
}

fn admit_entry(
    index: usize,
    entry: &CastEntry,
    base: &Path,
) -> Result<CastSpirit, HostConfigError> {
    let bad = |why: String| {
        refuse(
            HostConfigKind::CastEntry,
            format!("[[host.spirits]] entry {index}: {why}"),
        )
    };
    if entry.host.is_some() {
        return Err(refuse(
            HostConfigKind::CastHostKey,
            format!(
                "[[host.spirits]] entry {index} carries `host`: a cross-host delegation target is \
                 not a member of the standing cast"
            ),
        ));
    }
    let manifest = match (&entry.manifest, &entry.path, &entry.installed) {
        (Some(m), None, None) | (None, Some(m), None) => m,
        (None, None, Some(name)) if !name.is_empty() => {
            return Ok(CastSpirit::Installed(name.clone()));
        }
        _ => {
            return Err(bad(
                "declare exactly one of `manifest`/`path` or `installed`".into(),
            ))
        }
    };
    // Absolute, so the topology loader never re-bases it onto the config file's directory.
    let path = std::fs::canonicalize(base.join(manifest))
        .map_err(|e| bad(format!("cannot resolve manifest {manifest}: {e}")))?;
    let text = std::fs::read_to_string(&path)
        .map_err(|e| bad(format!("cannot read manifest {}: {e}", path.display())))?;
    let root: toml::Value = toml::from_str(&text)
        .map_err(|e| bad(format!("manifest {} does not parse: {e}", path.display())))?;
    if root.get("topology").is_some() {
        return Err(bad(format!(
            "{} is a [topology] manifest; list its Spirits in the Host file",
            path.display()
        )));
    }
    if root.get("class").is_none() && root.get("cli_wrapper").is_none() {
        return Err(bad(format!(
            "{} declares neither [class] nor [cli_wrapper]",
            path.display()
        )));
    }
    Ok(CastSpirit::Manifest(path))
}
