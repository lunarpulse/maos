//! Story 16-1 — the operator door's discovery file, and the ONE home rule.
//!
//! Measured at `f72b557b`: four resolvers disagreed about where MAOS lives
//! (`maos_shell::maos_home`, `maos_audit`'s Transparency-Log resolver, the
//! memory/archive resolvers that ignore `MAOS_HOME` entirely, and a hardcoded
//! `/tmp/maos/crl`). Three processes — `maos init`, the daemon roots and
//! `maosctl` — must agree on exactly one path to reach the same
//! `control.json`, so the rule lives here once (`maos-domain` is the only crate
//! all three already depend on; `maos-cli` cannot depend on `maos-shell`,
//! because the edge runs the other way).
//!
//! ADR-062 keeps **one writer** of `control.json`: `maos init`. Rotation is
//! `rm control.json && maos init`, never an in-place rewrite by a daemon —
//! a second writer is a second trust path.
//!
//! ⚠ No lock file is declared here on purpose. Liveness (D-16-1-D) is `flock`
//! on the store *directories' own handles*: a `.lock` file inside a store
//! directory would change the file counts `erasure_uninstall_13_5b` asserts.

use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};

/// The discovery file's name inside the MAOS home.
pub const CONTROL_FILE_NAME: &str = "control.json";

/// The only schema version this build understands. A reader that meets any
/// other value REFUSES rather than guessing at a field set.
pub const CONTROL_FILE_VERSION: u64 = 1;

/// The only endpoint scheme the door speaks. Qualified on purpose: a bare
/// `127.0.0.1:9000` cannot grow a UNIX-socket or TLS form without ambiguity.
pub const ENDPOINT_SCHEME: &str = "tcp://";

/// Mode of `control.json`: owner read/write, nothing else. The file carries a
/// bearer token that is the whole authorization boundary of the door.
pub const CONTROL_FILE_MODE: u32 = 0o600;

/// Mode of a MAOS home directory this code creates. `0700` and not `0755`
/// because the store lock set flocks directory handles: any uid that can
/// `open()` a store directory can hold `LOCK_SH` on it and make a root refuse
/// to boot (D-16-1-D, residual V-23). Directories that already exist are left
/// alone — widening or narrowing someone else's tree is not this code's call.
pub const HOME_DIR_MODE: u32 = 0o700;

/// Bearer-token entropy. 32 bytes rendered as 64 lowercase hex characters.
const TOKEN_BYTES: usize = 32;

/// Invalid configuration of the ONE home rule.
#[derive(Debug, thiserror::Error)]
pub enum MaosHomeError {
    #[error("maos: {variable} must be an absolute path, got {path}")]
    Relative {
        variable: &'static str,
        path: PathBuf,
    },
}

/// The ONE home rule: `MAOS_HOME`, else `$HOME/.maos`.
///
/// `Ok(None)` when neither is set. Relative homes are refused because init,
/// the daemon, and maosctl may have different working directories.
pub fn maos_home() -> Result<Option<PathBuf>, MaosHomeError> {
    if let Some(home) = std::env::var_os("MAOS_HOME") {
        let home = PathBuf::from(home);
        if !home.as_os_str().is_empty() {
            if !home.is_absolute() {
                return Err(MaosHomeError::Relative {
                    variable: "MAOS_HOME",
                    path: home,
                });
            }
            return Ok(Some(home));
        }
    }
    let Some(home) = std::env::var_os("HOME") else {
        return Ok(None);
    };
    let home = PathBuf::from(home);
    if home.as_os_str().is_empty() {
        return Ok(None);
    }
    if !home.is_absolute() {
        return Err(MaosHomeError::Relative {
            variable: "HOME",
            path: home,
        });
    }
    Ok(Some(home.join(".maos")))
}

/// `<home>/control.json`.
pub fn control_file_path(home: &Path) -> PathBuf {
    home.join(CONTROL_FILE_NAME)
}

/// The door's discovery record, as `maos init` writes it and every other
/// process reads it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlFile {
    /// Always [`CONTROL_FILE_VERSION`] on write; refused on read when it is
    /// anything else.
    pub version: u64,
    /// `tcp://127.0.0.1:<port>`.
    pub endpoint: String,
    /// 64 lowercase hex characters.
    pub token: String,
}

/// Every way `control.json` can fail to be an authorization boundary.
///
/// Each variant names the file and the offending field, because the operator's
/// next action differs per variant: a wider mode needs `chmod`, an unknown
/// version needs a newer `maosctl`, a missing file needs `maos init`.
#[derive(Debug, thiserror::Error)]
pub enum ControlFileError {
    #[error("maos: no operator door configured — {path} does not exist; run `maos init`")]
    Missing { path: PathBuf },
    #[error("maos: cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("maos: {path} is not a valid control file: {detail}")]
    Malformed { path: PathBuf, detail: String },
    #[error(
        "maos: {path} declares version {version}, but this build understands only version {expected} — upgrade maos or remove the file and re-run `maos init`"
    )]
    UnsupportedVersion {
        path: PathBuf,
        version: u64,
        expected: u64,
    },
    #[error(
        "maos: {path} field `endpoint` is {endpoint:?}, which does not start with `{expected}`"
    )]
    UnsupportedScheme {
        path: PathBuf,
        endpoint: String,
        expected: &'static str,
    },
    #[error("maos: {path} field `endpoint` is {endpoint:?}: {detail}")]
    InvalidEndpoint {
        path: PathBuf,
        endpoint: String,
        detail: String,
    },
    #[error("maos: {path} field `endpoint` {endpoint:?} is not a loopback address")]
    NotLoopback { path: PathBuf, endpoint: String },
    #[error("maos: {path} field `token`: {detail}")]
    InvalidToken { path: PathBuf, detail: String },
    #[error(
        "maos: {path} has mode {mode:04o}, wider than {expected:04o} — it carries the operator bearer token; run `chmod {expected:o} {path}`"
    )]
    ModeTooWide {
        path: PathBuf,
        mode: u32,
        expected: u32,
    },
    #[error("maos: {path} is owned by uid {owner}, not by uid {expected}")]
    ForeignOwner {
        path: PathBuf,
        owner: u32,
        expected: u32,
    },
}

impl ControlFileError {
    /// The file every variant is about.
    pub fn path(&self) -> &Path {
        match self {
            Self::Missing { path }
            | Self::Io { path, .. }
            | Self::Malformed { path, .. }
            | Self::UnsupportedVersion { path, .. }
            | Self::UnsupportedScheme { path, .. }
            | Self::InvalidEndpoint { path, .. }
            | Self::NotLoopback { path, .. }
            | Self::InvalidToken { path, .. }
            | Self::ModeTooWide { path, .. }
            | Self::ForeignOwner { path, .. } => path,
        }
    }

    /// `true` only for [`Self::Missing`] — the one variant callers are allowed
    /// to treat as "not configured" rather than "misconfigured". Every other
    /// variant is a refusal: a `control.json` that exists and is wrong must
    /// never fall back to a default, because the fallback would be an
    /// unauthenticated or differently-authenticated door.
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing { .. })
    }
}

impl ControlFile {
    /// A fresh record for `endpoint`, with a newly minted token.
    pub fn mint(endpoint: SocketAddr) -> Self {
        Self {
            version: CONTROL_FILE_VERSION,
            endpoint: format!("{ENDPOINT_SCHEME}{endpoint}"),
            token: mint_token_hex(),
        }
    }

    /// The endpoint as a socket address, re-validated.
    ///
    /// Re-validated rather than cached: [`Self::load`] already refused a
    /// non-loopback endpoint, but this type is also constructed by tests and by
    /// `serde`, and a door bound off-loopback is the one mistake that turns a
    /// local authorization boundary into a network one.
    pub fn endpoint_addr(&self) -> Result<SocketAddr, ControlFileError> {
        self.parse_endpoint(Path::new(CONTROL_FILE_NAME))
    }

    fn parse_endpoint(&self, path: &Path) -> Result<SocketAddr, ControlFileError> {
        let Some(authority) = self.endpoint.strip_prefix(ENDPOINT_SCHEME) else {
            return Err(ControlFileError::UnsupportedScheme {
                path: path.to_path_buf(),
                endpoint: self.endpoint.clone(),
                expected: ENDPOINT_SCHEME,
            });
        };
        let addr: SocketAddr = authority
            .parse()
            .map_err(
                |error: std::net::AddrParseError| ControlFileError::InvalidEndpoint {
                    path: path.to_path_buf(),
                    endpoint: self.endpoint.clone(),
                    detail: error.to_string(),
                },
            )?;
        if !addr.ip().is_loopback() {
            return Err(ControlFileError::NotLoopback {
                path: path.to_path_buf(),
                endpoint: self.endpoint.clone(),
            });
        }
        if addr.port() == 0 {
            return Err(ControlFileError::InvalidEndpoint {
                path: path.to_path_buf(),
                endpoint: self.endpoint.clone(),
                detail: "port zero is not connectable from control.json".to_owned(),
            });
        }
        Ok(addr)
    }

    /// Read and fully validate `path`: permissions and ownership first, then
    /// schema version, then endpoint, then token shape.
    ///
    /// Ownership and mode are checked **before** the bytes are trusted for
    /// anything, because the token in a world-readable or foreign-owned file is
    /// not a secret and is not ours.
    pub fn load(path: &Path) -> Result<Self, ControlFileError> {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ControlFileError::Missing {
                    path: path.to_path_buf(),
                })
            }
            Err(source) => {
                return Err(ControlFileError::Io {
                    path: path.to_path_buf(),
                    source,
                })
            }
        };
        check_custody(path, &metadata)?;
        let bytes = std::fs::read(path).map_err(|source| ControlFileError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let parsed: Self =
            serde_json::from_slice(&bytes).map_err(|error| ControlFileError::Malformed {
                path: path.to_path_buf(),
                detail: error.to_string(),
            })?;
        if parsed.version != CONTROL_FILE_VERSION {
            return Err(ControlFileError::UnsupportedVersion {
                path: path.to_path_buf(),
                version: parsed.version,
                expected: CONTROL_FILE_VERSION,
            });
        }
        parsed.parse_endpoint(path)?;
        parsed.check_token(path)?;
        Ok(parsed)
    }

    fn check_token(&self, path: &Path) -> Result<(), ControlFileError> {
        if self.token.len() != TOKEN_BYTES * 2 {
            return Err(ControlFileError::InvalidToken {
                path: path.to_path_buf(),
                detail: format!(
                    "expected {} hex characters, found {}",
                    TOKEN_BYTES * 2,
                    self.token.len()
                ),
            });
        }
        if !self
            .token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ControlFileError::InvalidToken {
                path: path.to_path_buf(),
                detail: "expected lowercase hex characters only".to_owned(),
            });
        }
        Ok(())
    }

    /// Write this record to `path` atomically at mode `0600`.
    ///
    /// Same-directory temp file + `sync_all` + no-replace hard-link
    /// publication, so concurrent initializers cannot overwrite one another
    /// and a reader never sees a partial door config.
    pub fn write_atomic(&self, path: &Path) -> Result<(), std::io::Error> {
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} has no parent directory", path.display()),
            )
        })?;
        let body = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        let mut suffix = [0_u8; 8];
        getrandom::fill(&mut suffix).map_err(|error| std::io::Error::other(error.to_string()))?;
        let temp = parent.join(format!(".{CONTROL_FILE_NAME}.{}.tmp", hex::encode(suffix)));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(CONTROL_FILE_MODE);
        }
        let write_result = (|| -> Result<(), std::io::Error> {
            let mut file = options.open(&temp)?;
            file.write_all(&body)?;
            file.sync_all()?;
            std::fs::hard_link(&temp, path)?;
            std::fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        let _ = std::fs::remove_file(&temp);
        write_result
    }
}

/// 32 fresh bytes as 64 lowercase hex characters.
///
/// `getrandom` is already a direct dependency of this crate, so the door's
/// bearer token adds no dependency to the workspace.
pub fn mint_token_hex() -> String {
    let mut bytes = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).expect("operating system entropy for the operator bearer token");
    hex::encode(bytes)
}

/// Ask the OS for a free loopback port by binding `127.0.0.1:0` and dropping
/// the listener.
///
/// Inherently advisory — nothing stops another process taking the port between
/// the probe and the daemon's bind — which is exactly why a second root on the
/// same endpoint fails fast and typed (`EndpointInUse`) instead of silently
/// picking another port the way the retired `127.0.0.1:8787` fallback did.
pub fn probe_loopback_endpoint() -> Result<SocketAddr, std::io::Error> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    listener.local_addr()
}

/// Create `home` (and its parents) at [`HOME_DIR_MODE`] if it does not exist.
pub fn ensure_home_dir(home: &Path) -> Result<(), std::io::Error> {
    if home.is_dir() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(HOME_DIR_MODE)
            .create(home)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(home)
    }
}

/// What [`ensure_control_file`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFileState {
    /// A valid record was already on disk; not one byte was rewritten.
    Existing,
    /// The file was absent and has been minted. This is also the upgrade path
    /// for a home initialised before Story 16-1.
    Minted,
}

/// The single writer of `control.json`, called by `maos init` only.
///
/// Idempotent by construction: an existing, valid file is returned untouched
/// (AC2 — "re-running changes neither byte"), an absent one is minted, and an
/// existing but INVALID one is an error rather than an overwrite, so a mode or
/// ownership problem is reported instead of being laundered away by the next
/// `maos init`.
pub fn ensure_control_file(
    home: &Path,
) -> Result<(ControlFile, ControlFileState), ControlFileError> {
    let path = control_file_path(home);
    match ControlFile::load(&path) {
        Ok(existing) => return Ok((existing, ControlFileState::Existing)),
        Err(error) if error.is_missing() => {}
        Err(error) => return Err(error),
    }
    ensure_home_dir(home).map_err(|source| ControlFileError::Io {
        path: home.to_path_buf(),
        source,
    })?;
    let endpoint = probe_loopback_endpoint().map_err(|source| ControlFileError::Io {
        path: path.clone(),
        source,
    })?;
    let minted = ControlFile::mint(endpoint);
    match minted.write_atomic(&path) {
        Ok(()) => Ok((minted, ControlFileState::Minted)),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            ControlFile::load(&path).map(|winner| (winner, ControlFileState::Existing))
        }
        Err(source) => Err(ControlFileError::Io {
            path: path.clone(),
            source,
        }),
    }
}

#[cfg(unix)]
fn check_custody(path: &Path, metadata: &std::fs::Metadata) -> Result<(), ControlFileError> {
    use std::os::unix::fs::MetadataExt;
    let mode = metadata.mode() & 0o7777;
    if mode & !CONTROL_FILE_MODE != 0 {
        return Err(ControlFileError::ModeTooWide {
            path: path.to_path_buf(),
            mode,
            expected: CONTROL_FILE_MODE,
        });
    }
    let expected = effective_uid();
    if metadata.uid() != expected {
        return Err(ControlFileError::ForeignOwner {
            path: path.to_path_buf(),
            owner: metadata.uid(),
            expected,
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_custody(_path: &Path, _metadata: &std::fs::Metadata) -> Result<(), ControlFileError> {
    // No POSIX mode or uid to check. The door's non-unix story is
    // `OfflineUnsupported`; nothing in `release.yml` ships a non-unix target.
    Ok(())
}

/// The effective uid from the process credentials, with no filesystem probe.
#[cfg(unix)]
fn effective_uid() -> u32 {
    rustix::process::geteuid().as_raw()
}
