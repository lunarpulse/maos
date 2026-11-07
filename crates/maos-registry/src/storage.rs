//! Registry storage — content-addressed filesystem tree.
//!
//! The `RegistryStorage` trait provides the persistence layer for the
//! Spirit Registry server.  `LocalFsRegistryStorage` implements it using
//! a directory tree at `~/.local/share/maos/registry/`.
//!
//! # Directory layout
//!
//! ```text
//! ~/.local/share/maos/registry/
//!   spirits/
//!     hello-spirit/
//!       0.1.0/
//!         manifest.toml
//!         artifact.bin
//!         signed_package.json
//!         compliance_claim.envelope
//!   index.json
//!   yanks.json
//!   installed-pins.json  # publisher + version digests; survives uninstall
//! ```

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledPin {
    publisher: [u8; 32],
    versions: BTreeMap<String, [u8; 32]>,
}

fn package_digest(pkg: &SignedPackage) -> Result<[u8; 32], StorageError> {
    struct Writer(Sha256);
    impl io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer(Sha256::new());
    serde_json::to_writer(&mut writer, pkg).map_err(|e| StorageError::Serde(e.to_string()))?;
    Ok(writer.0.finalize().into())
}

use maos_domain::ports::registry::{
    SearchQuery, SearchResultItem, SearchResults, SignedArtifact, SignedManifest, SignedPackage,
    SpiritId, YankEntry, YankList, YankReason, YankReceipt,
};

/// Persistent storage for the Spirit Registry.
pub trait RegistryStorage: Send + Sync {
    /// Store a signed package.
    fn put(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
    ) -> Result<(), StorageError>;

    /// Retrieve a signed manifest.
    fn get_manifest(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedManifest, StorageError>;

    /// Retrieve a signed artifact.
    fn get_artifact(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedArtifact, StorageError>;

    /// Search the registry index.
    fn search(&self, q: &SearchQuery) -> Result<SearchResults, StorageError>;

    /// Yank (deprecate) a version.
    fn yank(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        reason: &YankReason,
    ) -> Result<YankReceipt, StorageError>;

    /// List yanks after a durable server sequence (`since_ns` is the legacy wire name).
    fn yanks_since(&self, since_ns: u64) -> Result<YankList, StorageError>;

    /// Story 7.2 — store a signed package with origin metadata.
    fn publish_with_origin(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
        origin: &crate::origin::RegistryOrigin,
    ) -> Result<(), StorageError>;

    /// Story 18-2 (D-18-2-04 A) — store a version's signed halt benchmark
    /// sidecar ([`BENCHMARK_FILE`]). The producer signs; `maosctl spirit
    /// benchmark` verifies. Storage never inspects the bytes.
    fn put_benchmark(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        sidecar: &[u8],
    ) -> Result<(), StorageError>;

    /// Story 18-2 — a version's sidecar, or (`None`) the highest version
    /// that has one. Returns the version read and the bytes.
    fn get_benchmark(
        &self,
        spirit_id: &SpiritId,
        version: Option<&str>,
    ) -> Result<(String, Vec<u8>), StorageError>;
}

/// Filesystem-backed registry storage using `~/.local/share/maos/registry/`.
pub struct LocalFsRegistryStorage {
    root: PathBuf,
    index: Mutex<BTreeMap<String, Vec<SearchResultItem>>>,
    yanks: Mutex<Vec<YankEntry>>,
}

static INSTALLED_UNINSTALL: Mutex<()> = Mutex::new(());

/// Roll back boot inventory and generation ownership unless the erasure proof
/// has been persisted. The caller holds the canonical offline/live store locks.
pub struct PendingPackageUninstall {
    index_path: PathBuf,
    directory: PathBuf,
    staged: Option<PathBuf>,
    installs: BTreeMap<String, String>,
    removed: Option<(String, String)>,
    _serializer: std::sync::MutexGuard<'static, ()>,
    _mutation_lock: fs::File,
    backup: Option<fs::File>,
    deleted: bool,
    restored: bool,
}

impl PendingPackageUninstall {
    pub fn generation(&self) -> Option<&str> {
        self.removed
            .as_ref()
            .map(|(_, generation)| generation.as_str())
    }

    /// Keep the complete archive through proof and lifecycle finalization.
    pub fn delete_generation(&mut self) -> Result<(), StorageError> {
        if !self.deleted {
            if let Some(path) = &self.staged {
                if let Err(error) = fs::remove_dir_all(path) {
                    self.rollback().map_err(|recovery| {
                        StorageError::Io(format!(
                            "delete failed: {error}; recovery failed: {recovery}"
                        ))
                    })?;
                    return Err(StorageError::Io(error.to_string()));
                }
            }
            self.deleted = true;
        }
        Ok(())
    }
    pub fn accept(mut self) {
        self.removed = None;
    }

    pub fn rollback(&mut self) -> Result<(), StorageError> {
        let Some((name, generation)) = &self.removed else {
            return Ok(());
        };
        if !self.restored {
            if let Some(backup) = &mut self.backup {
                use std::io::{Seek, SeekFrom};
                backup
                    .seek(SeekFrom::Start(0))
                    .map_err(|error| StorageError::Io(error.to_string()))?;
                let restored = tempfile::tempdir_in(
                    self.directory
                        .parent()
                        .ok_or_else(|| StorageError::Io("package parent missing".into()))?,
                )
                .map_err(|error| StorageError::Io(error.to_string()))?;
                tar::Archive::new(backup)
                    .unpack(restored.path())
                    .map_err(|error| StorageError::Io(error.to_string()))?;
                fs::rename(restored.path(), &self.directory)
                    .map_err(|error| StorageError::Io(error.to_string()))?;
            }
            self.restored = true;
        }
        self.installs.insert(name.clone(), generation.clone());
        if LocalFsRegistryStorage::load_installs(
            self.index_path
                .parent()
                .ok_or_else(|| StorageError::Io("inventory parent missing".into()))?,
        )? != self.installs
        {
            write_json_atomic(&self.index_path, &self.installs)?;
        }
        self.removed = None;
        if let Some(path) = &self.staged {
            remove_staging(path).map_err(|error| StorageError::Io(error.to_string()))?;
        }
        Ok(())
    }
}

impl Drop for PendingPackageUninstall {
    fn drop(&mut self) {
        if let Err(error) = self.rollback() {
            eprintln!("maos-registry: explicit uninstall recovery required: {error}");
        }
    }
}

fn remove_staging(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)?.permissions().mode();
        fs::set_permissions(path, fs::Permissions::from_mode(mode | 0o700))?;
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            remove_staging(&entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    fs::remove_dir(path)
}

impl LocalFsRegistryStorage {
    /// Construct storage rooted at `~/.local/share/maos/registry/`.
    pub fn new() -> Result<Self, io::Error> {
        let root = Self::default_root();
        fs::create_dir_all(root.join("spirits"))?;
        let index = Self::load_index(&root);
        let yanks = Self::load_yanks(&root)?;
        Ok(Self {
            root,
            index: Mutex::new(index),
            yanks: Mutex::new(yanks),
        })
    }

    /// Construct storage at a custom root path (for testing).
    pub fn at_path(root: PathBuf) -> Result<Self, io::Error> {
        fs::create_dir_all(root.join("spirits"))?;
        let index = Self::load_index(&root);
        let yanks = Self::load_yanks(&root)?;
        Ok(Self {
            root,
            index: Mutex::new(index),
            yanks: Mutex::new(yanks),
        })
    }
    fn default_root() -> PathBuf {
        dirs_fallback().join(".local/share/maos/registry")
    }

    /// Enumerate boot inventory without creating registry or home directories.
    pub fn default_installed_manifests() -> Result<Vec<(String, PathBuf)>, StorageError> {
        Self::installed_manifests_at(&Self::default_root())
    }

    /// Only the operator install transaction publishes this inventory.
    /// Registry inbox publications never become boot-visible.
    pub fn installed_manifests(&self) -> Result<Vec<(String, PathBuf)>, StorageError> {
        Self::installed_manifests_at(&self.root)
    }

    fn installed_manifests_at(root: &Path) -> Result<Vec<(String, PathBuf)>, StorageError> {
        Self::load_installs(root)?
            .into_iter()
            .map(|(name, relative)| {
                path_segment(&name)?;
                let path = Path::new(&relative);
                let mut parts = path.components();
                if parts.next() != Some(std::path::Component::Normal(name.as_ref()))
                    || !matches!(parts.next(), Some(std::path::Component::Normal(_)))
                    || parts.next().is_some()
                {
                    return Err(StorageError::InvalidSegment(relative));
                }
                Ok((
                    name,
                    root.join("installed").join(path).join("manifest.toml"),
                ))
            })
            .collect()
    }

    /// Remove active inventory while retaining recoverable bytes until the
    /// caller persists its proof. A failed proof restores the package on drop.
    pub fn uninstall_package(
        spirit_id: &str,
    ) -> Result<Option<PendingPackageUninstall>, StorageError> {
        path_segment(spirit_id)?;
        let serializer = INSTALLED_UNINSTALL
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let root = Self::default_root();
        let storage = Self::at_path(root.clone()).map_err(|e| StorageError::Io(e.to_string()))?;
        let mutation_lock = storage.mutation_lock()?;
        let mut installs = Self::load_installs(&root)?;
        let Some(generation) = installs.remove(spirit_id) else {
            return Ok(None);
        };
        let mut pins = Self::load_pins_at(&root)?;
        if storage.import_legacy_pin(spirit_id, &mut pins)? {
            write_json_atomic(&root.join("installed-pins.json"), &pins)?;
        }
        Self::installed_manifests_at(&root)?;
        let directory = root.join("installed").join(spirit_id);
        let staged = root.join(format!(".uninstall-{spirit_id}-{}", unique_suffix()));
        let backup = if directory.exists() {
            let mut file = tempfile::tempfile_in(&root)
                .map_err(|error| StorageError::Io(error.to_string()))?;
            {
                let mut archive = tar::Builder::new(&mut file);
                archive.follow_symlinks(false);
                archive
                    .append_dir_all(".", &directory)
                    .map_err(|error| StorageError::Io(error.to_string()))?;
                archive
                    .finish()
                    .map_err(|error| StorageError::Io(error.to_string()))?;
            }
            file.sync_all()
                .map_err(|error| StorageError::Io(error.to_string()))?;
            Some(file)
        } else {
            None
        };
        let staged = match fs::rename(&directory, &staged) {
            Ok(()) => Some(staged),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(StorageError::Io(error.to_string())),
        };
        let mut pending = PendingPackageUninstall {
            index_path: root.join("installed-index.json"),
            directory,
            staged,
            installs,
            removed: Some((spirit_id.to_owned(), generation)),
            _serializer: serializer,
            _mutation_lock: mutation_lock,
            backup,
            deleted: false,
            restored: false,
        };
        if let Err(error) = write_json_atomic(&pending.index_path, &pending.installs) {
            pending.rollback().map_err(|recovery| {
                StorageError::Io(format!(
                    "stage failed: {error}; recovery failed: {recovery}"
                ))
            })?;
            return Err(error);
        }
        Ok(Some(pending))
    }

    fn load_installs(root: &Path) -> Result<BTreeMap<String, String>, StorageError> {
        match fs::read(root.join("installed-index.json")) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| StorageError::Serde(e.to_string()))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(StorageError::Io(e.to_string())),
        }
    }

    fn mutation_lock(&self) -> Result<fs::File, StorageError> {
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join(".mutation.lock"))
            .map_err(|e| StorageError::Io(e.to_string()))?;
        #[cfg(unix)]
        rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        #[cfg(not(unix))]
        {
            Err(StorageError::Io(
                "registry mutation locking requires Unix".into(),
            ))
        }
        #[cfg(unix)]
        {
            Ok(file)
        }
    }

    fn load_pins_at(root: &Path) -> Result<BTreeMap<String, InstalledPin>, StorageError> {
        match fs::File::open(root.join("installed-pins.json")) {
            Ok(file) => serde_json::from_reader(io::BufReader::new(file))
                .map_err(|e| StorageError::Serde(e.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(StorageError::Io(e.to_string())),
        }
    }

    /// Durable trust state is independent of installed generation lifetime.
    pub fn default_installed_publisher(name: &str) -> Result<Option<[u8; 32]>, StorageError> {
        path_segment(name)?;
        Ok(Self::load_pins_at(&Self::default_root())?
            .get(name)
            .map(|pin| pin.publisher))
    }

    /// Explicit local operator authority; uninstall never invokes this verb.
    pub fn release_installed_pin(&self, name: &str) -> Result<bool, StorageError> {
        path_segment(name)?;
        let _serializer = INSTALLED_UNINSTALL
            .lock()
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let _lock = self.mutation_lock()?;
        let mut pins = Self::load_pins_at(&self.root)?;
        let removed = pins.remove(name).is_some();
        if removed {
            write_json_atomic(&self.root.join("installed-pins.json"), &pins)?;
        }
        Ok(removed)
    }

    /// First use after cutover imports ALL retained signed versions. Ordinary
    /// installs then use the ledger; historical artifact decoding is not repeated.
    fn import_legacy_pin(
        &self,
        name: &str,
        pins: &mut BTreeMap<String, InstalledPin>,
    ) -> Result<bool, StorageError> {
        if pins.contains_key(name) {
            return Ok(false);
        }
        let generations = match fs::read_dir(self.root.join("installed").join(name)) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(StorageError::Io(e.to_string())),
        };
        let mut imported: Option<InstalledPin> = None;
        for generation in generations {
            let path = generation
                .map_err(|e| StorageError::Io(e.to_string()))?
                .path()
                .join("signed_package.json");
            let file = match fs::File::open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(StorageError::Io(e.to_string())),
            };
            let pkg: SignedPackage = serde_json::from_reader(io::BufReader::new(file))
                .map_err(|e| StorageError::Serde(e.to_string()))?;
            if pkg.spirit_id.as_str() != name || !crate::admission::verify_publisher_sig(&pkg) {
                return Err(StorageError::Serde(
                    "invalid legacy installed signed package".into(),
                ));
            }
            let pin = imported.get_or_insert_with(|| InstalledPin {
                publisher: pkg.publisher_pubkey,
                versions: BTreeMap::new(),
            });
            if pin.publisher != pkg.publisher_pubkey {
                return Err(StorageError::PublisherMismatch(name.into()));
            }
            let digest = package_digest(&pkg)?;
            if pin
                .versions
                .get(&pkg.version)
                .is_some_and(|stored| stored != &digest)
            {
                return Err(StorageError::ImmutableVersion {
                    spirit_id: name.into(),
                    version: pkg.version,
                });
            }
            pin.versions.insert(pkg.version, digest);
        }
        if let Some(pin) = imported {
            pins.insert(name.into(), pin);
            return Ok(true);
        }
        Ok(false)
    }

    /// Pin across every version, including inactive installed generations.
    fn check_publisher(
        directory: &Path,
        pkg: &SignedPackage,
    ) -> Result<Option<PathBuf>, StorageError> {
        #[derive(serde::Deserialize)]
        struct Identity {
            version: String,
            publisher_pubkey: String,
        }
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StorageError::Io(e.to_string())),
        };
        let mut existing = None;
        for entry in entries {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            let directory = entry.path();
            let path = directory.join("signed_package.json");
            if !path.exists() {
                continue;
            }
            // Stream past artifact/compliance bytes; ownership checks must not
            // allocate every historical artifact just to read its publisher.
            let stored: Identity = serde_json::from_reader(std::io::BufReader::new(
                fs::File::open(path).map_err(|e| StorageError::Io(e.to_string()))?,
            ))
            .map_err(|e| StorageError::Serde(e.to_string()))?;
            let mut publisher = [0; 32];
            hex::decode_to_slice(&stored.publisher_pubkey, &mut publisher)
                .map_err(|e| StorageError::Serde(e.to_string()))?;
            if publisher != pkg.publisher_pubkey {
                return Err(StorageError::PublisherMismatch(
                    pkg.spirit_id.as_str().into(),
                ));
            }
            if stored.version == pkg.version {
                existing = Some(directory);
            }
        }
        Ok(existing)
    }

    /// Prepare every file before the atomic inventory switch. A failed install
    /// cannot replace an active generation or activate partially written bytes.
    pub(crate) fn install_package(
        &self,
        pkg: &SignedPackage,
        origin: &crate::origin::RegistryOrigin,
        vetting: Option<&crate::install::InstalledVetting>,
    ) -> Result<(), StorageError> {
        path_segment(pkg.spirit_id.as_str())?;
        path_segment(&pkg.version)?;
        let _serializer = INSTALLED_UNINSTALL
            .lock()
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let _lock = self.mutation_lock()?;
        let mut pins = Self::load_pins_at(&self.root)?;
        let imported = self.import_legacy_pin(pkg.spirit_id.as_str(), &mut pins)?;
        let digest = package_digest(pkg)?;
        if let Some(pin) = pins.get(pkg.spirit_id.as_str()) {
            if pin.publisher != pkg.publisher_pubkey {
                return Err(StorageError::PublisherMismatch(
                    pkg.spirit_id.as_str().into(),
                ));
            }
            if pin
                .versions
                .get(&pkg.version)
                .is_some_and(|stored| stored != &digest)
            {
                return Err(StorageError::ImmutableVersion {
                    spirit_id: pkg.spirit_id.as_str().into(),
                    version: pkg.version.clone(),
                });
            }
        }
        let pin = pins
            .entry(pkg.spirit_id.as_str().into())
            .or_insert_with(|| InstalledPin {
                publisher: pkg.publisher_pubkey,
                versions: BTreeMap::new(),
            });
        let pins_changed = pin.versions.insert(pkg.version.clone(), digest).is_none() || imported;
        let parent = self.root.join("installed").join(pkg.spirit_id.as_str());
        if let Some(directory) = Self::check_publisher(&parent, pkg)? {
            let stored: SignedPackage = serde_json::from_slice(
                &fs::read(directory.join("signed_package.json"))
                    .map_err(|e| StorageError::Io(e.to_string()))?,
            )
            .map_err(|e| StorageError::Serde(e.to_string()))?;
            let stored_origin: crate::origin::RegistryOrigin = serde_json::from_slice(
                &fs::read(directory.join("origin.json"))
                    .map_err(|e| StorageError::Io(e.to_string()))?,
            )
            .map_err(|e| StorageError::Serde(e.to_string()))?;
            if stored != *pkg || stored_origin != *origin {
                return Err(StorageError::ImmutableVersion {
                    spirit_id: pkg.spirit_id.as_str().into(),
                    version: pkg.version.clone(),
                });
            }
            // Evidence can be renewed, but the immutable package and origin
            // above cannot change. The install entry point verifies new evidence.
            if let Some(vetting) = vetting {
                write_json_atomic(&directory.join("vetting.json"), vetting)?;
            }
            if pins_changed {
                write_json_atomic(&self.root.join("installed-pins.json"), &pins)?;
            }
            let relative = directory
                .strip_prefix(self.root.join("installed"))
                .map_err(|e| StorageError::Io(e.to_string()))?
                .to_str()
                .ok_or_else(|| StorageError::Io("installed generation is not UTF-8".into()))?;
            let mut installs = Self::load_installs(&self.root)?;
            if installs.get(pkg.spirit_id.as_str()).map(String::as_str) != Some(relative) {
                installs.insert(pkg.spirit_id.as_str().to_owned(), relative.to_owned());
                write_json_atomic(&self.root.join("installed-index.json"), &installs)?;
            }
            return Ok(());
        }
        fs::create_dir_all(&parent).map_err(|e| StorageError::Io(e.to_string()))?;
        let generation = format!("{}-{}", pkg.version, unique_suffix());
        let directory = parent.join(&generation);
        fs::create_dir(&directory).map_err(|e| StorageError::Io(e.to_string()))?;
        let result = (|| {
            Self::write_package(&directory, pkg)?;
            let document = maos_manifest::ManifestDocument::parse(&pkg.manifest_toml)
                .map_err(|e| StorageError::Serde(e.to_string()))?;
            if let Some(class) = document
                .class_section()
                .map_err(|e| StorageError::Serde(e.to_string()))?
            {
                if let Some(artifact) = class.artifact {
                    let path = directory.join(artifact);
                    fs::create_dir_all(path.parent().expect("artifact has version parent"))
                        .map_err(|e| StorageError::Io(e.to_string()))?;
                    fs::write(path, &pkg.artifact_bytes)
                        .map_err(|e| StorageError::Io(e.to_string()))?;
                }
            }
            write_json_atomic(&directory.join("origin.json"), origin)?;
            if let Some(vetting) = vetting {
                write_json_atomic(&directory.join("vetting.json"), vetting)?;
            }
            if pins_changed {
                write_json_atomic(&self.root.join("installed-pins.json"), &pins)?;
            }
            let mut installs = Self::load_installs(&self.root)?;
            installs.insert(
                pkg.spirit_id.as_str().to_owned(),
                format!("{}/{generation}", pkg.spirit_id.as_str()),
            );
            write_json_atomic(&self.root.join("installed-index.json"), &installs)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&directory);
            let _ = fs::remove_dir(&parent);
        }
        result
    }

    fn write_package(directory: &Path, pkg: &SignedPackage) -> Result<(), StorageError> {
        fs::write(directory.join("manifest.toml"), &pkg.manifest_toml)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        fs::write(directory.join("artifact.bin"), &pkg.artifact_bytes)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        write_json_atomic(&directory.join("signed_package.json"), pkg)
    }

    fn load_index(root: &Path) -> BTreeMap<String, Vec<SearchResultItem>> {
        let path = root.join("index.json");
        if path.exists() {
            match fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
            {
                Some(idx) => idx,
                None => {
                    eprintln!("maos-registry: warning: failed to load index.json, starting with empty index");
                    BTreeMap::new()
                }
            }
        } else {
            BTreeMap::new()
        }
    }

    fn load_yanks(root: &Path) -> io::Result<Vec<YankEntry>> {
        match fs::read(root.join("yanks.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    fn save_index_data(
        root: &Path,
        index: &BTreeMap<String, Vec<SearchResultItem>>,
    ) -> Result<(), StorageError> {
        write_json_atomic(&root.join("index.json"), index)
    }

    fn save_yanks_data(root: &Path, yanks: &[YankEntry]) -> Result<(), StorageError> {
        write_json_atomic(&root.join("yanks.json"), &yanks)
    }

    fn spirit_dir(&self, spirit_id: &SpiritId) -> Result<PathBuf, StorageError> {
        Ok(self
            .root
            .join("spirits")
            .join(path_segment(spirit_id.as_str())?))
    }

    fn version_dir(&self, spirit_id: &SpiritId, version: &str) -> Result<PathBuf, StorageError> {
        Ok(self.spirit_dir(spirit_id)?.join(path_segment(version)?))
    }
}

impl LocalFsRegistryStorage {
    fn write_origin(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        origin: &crate::origin::RegistryOrigin,
    ) -> Result<(), StorageError> {
        let vdir = self.version_dir(spirit_id, version)?;
        let origin_json =
            serde_json::to_vec_pretty(origin).map_err(|e| StorageError::Serde(e.to_string()))?;
        fs::write(vdir.join("origin.json"), &origin_json)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(())
    }
}

impl RegistryStorage for LocalFsRegistryStorage {
    fn put(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
    ) -> Result<(), StorageError> {
        let vdir = self.version_dir(spirit_id, version)?;
        if pkg.spirit_id != *spirit_id || pkg.version != version {
            return Err(StorageError::InvalidSegment(
                "package identity mismatch".into(),
            ));
        }
        let _lock = self.mutation_lock()?;
        if let Some(directory) = Self::check_publisher(&self.spirit_dir(spirit_id)?, pkg)? {
            let stored: SignedPackage = serde_json::from_reader(io::BufReader::new(
                fs::File::open(directory.join("signed_package.json"))
                    .map_err(|e| StorageError::Io(e.to_string()))?,
            ))
            .map_err(|e| StorageError::Serde(e.to_string()))?;
            if stored != *pkg {
                return Err(StorageError::ImmutableVersion {
                    spirit_id: spirit_id.as_str().into(),
                    version: version.into(),
                });
            }
        } else {
            let parent = self.spirit_dir(spirit_id)?;
            fs::create_dir_all(&parent).map_err(|e| StorageError::Io(e.to_string()))?;
            let staged = tempfile::Builder::new()
                .prefix(".publish-")
                .tempdir_in(&self.root)
                .map_err(|e| StorageError::Io(e.to_string()))?;
            Self::write_package(staged.path(), pkg)?;
            fs::rename(staged.path(), &vdir).map_err(|e| StorageError::Io(e.to_string()))?;
        }

        let mut idx = self
            .index
            .lock()
            .map_err(|e| StorageError::Io(format!("index lock poisoned: {e}")))?;
        let name = spirit_id.as_str();
        let previous = idx.get(name).cloned();
        let items = idx.entry(name.to_owned()).or_default();
        if items.iter().any(|item| item.version == version) {
            return Ok(());
        }
        items.push(SearchResultItem::new(
            spirit_id.clone(),
            version.to_owned(),
            extract_summary(&pkg.manifest_toml),
        ));
        if let Err(error) = Self::save_index_data(&self.root, &idx) {
            match previous {
                Some(items) => {
                    idx.insert(name.to_owned(), items);
                }
                None => {
                    idx.remove(name);
                }
            }
            return Err(error);
        }

        Ok(())
    }

    fn publish_with_origin(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
        origin: &crate::origin::RegistryOrigin,
    ) -> Result<(), StorageError> {
        self.put(spirit_id, version, pkg)?;
        self.write_origin(spirit_id, version, origin)
    }

    fn get_manifest(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedManifest, StorageError> {
        let vdir = self.version_dir(spirit_id, version)?;
        let manifest_toml =
            fs::read(vdir.join("manifest.toml")).map_err(|_| StorageError::VersionNotFound {
                spirit_id: spirit_id.as_str().to_string(),
                version: version.to_string(),
            })?;
        // Reconstruct from stored files.  Signature data is in signed_package.json.
        let pkg_json = fs::read_to_string(vdir.join("signed_package.json"))
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let pkg: SignedPackage =
            serde_json::from_str(&pkg_json).map_err(|e| StorageError::Serde(e.to_string()))?;
        Ok(SignedManifest::new(
            spirit_id.clone(),
            version.to_string(),
            manifest_toml,
            pkg.signature,
            pkg.publisher_pubkey,
        ))
    }

    fn get_artifact(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedArtifact, StorageError> {
        let vdir = self.version_dir(spirit_id, version)?;
        let artifact_bytes =
            fs::read(vdir.join("artifact.bin")).map_err(|_| StorageError::VersionNotFound {
                spirit_id: spirit_id.as_str().to_string(),
                version: version.to_string(),
            })?;
        let pkg_json = fs::read_to_string(vdir.join("signed_package.json"))
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let pkg: SignedPackage =
            serde_json::from_str(&pkg_json).map_err(|e| StorageError::Serde(e.to_string()))?;
        let mut artifact = SignedArtifact::new(
            spirit_id.clone(),
            version.to_string(),
            artifact_bytes,
            pkg.signature,
            pkg.publisher_pubkey,
        );
        artifact.compliance_envelope = Some(pkg.compliance_envelope);
        Ok(artifact)
    }

    fn search(&self, q: &SearchQuery) -> Result<SearchResults, StorageError> {
        let query_lower = q.text.to_lowercase();
        if query_lower.is_empty() {
            return Ok(SearchResults::new(Vec::new()));
        }

        // Story 7.2 (closes 5.5d Low #28) — snapshot yanks ONCE outside the index
        // lock so we don't re-acquire the yanks Mutex inside the index walk.
        // Pre-7.2: holding `idx` while repeatedly locking `yanks` produced O(N×M)
        // contention; the snapshot reduces it to O(N) + O(1) yanks lock acquisitions.
        let yanks_snapshot: Vec<YankEntry> = if !q.include_yanked {
            self.yanks
                .lock()
                .map_err(|e| StorageError::Io(format!("yanks lock poisoned: {e}")))?
                .clone()
        } else {
            Vec::new()
        };

        let idx = self
            .index
            .lock()
            .map_err(|e| StorageError::Io(format!("index lock poisoned: {e}")))?
            .clone();
        let mut all_items: Vec<SearchResultItem> = Vec::new();
        for items in idx.values() {
            for item in items {
                let matches = item
                    .spirit_id
                    .as_str()
                    .to_lowercase()
                    .contains(&query_lower);
                if !matches {
                    continue;
                }
                if !q.include_yanked {
                    let is_yanked = yanks_snapshot
                        .iter()
                        .any(|y| y.spirit_id == item.spirit_id && y.version == item.version);
                    if is_yanked {
                        continue;
                    }
                }
                all_items.push(item.clone());
            }
        }

        let limit = q.limit.min(200).max(1) as usize;
        all_items.truncate(limit);

        Ok(SearchResults::new(all_items))
    }

    fn yank(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        reason: &YankReason,
    ) -> Result<YankReceipt, StorageError> {
        path_segment(spirit_id.as_str())?;
        path_segment(version)?;
        let _lock = self.mutation_lock()?;
        let mut yanks = self
            .yanks
            .lock()
            .map_err(|e| StorageError::Io(e.to_string()))?;
        *yanks = Self::load_yanks(&self.root).map_err(|e| StorageError::Io(e.to_string()))?;
        let authorization = reason
            .authorization
            .as_deref()
            .map(crate::yank_auth::decode)
            .transpose()
            .map_err(StorageError::Serde)?;
        let signer = authorization.as_ref().map(|auth| auth.public_key);
        if let Some(auth) = &authorization {
            for entry in &*yanks {
                if let Ok(stored) = crate::yank_auth::decode(&entry.authorization) {
                    if stored.public_key == auth.public_key
                        && stored.nonce == auth.nonce
                        && (entry.spirit_id != *spirit_id
                            || entry.version != version
                            || entry.reason != reason.summary
                            || stored.origin != auth.origin
                            || stored.registry_uri != auth.registry_uri)
                    {
                        return Err(StorageError::Serde(
                            "yank nonce reused for another withdrawal".into(),
                        ));
                    }
                }
            }
        }
        if let Some(existing) = yanks.iter().find(|e| {
            e.spirit_id == *spirit_id
                && e.version == version
                && e.origin == authorization.as_ref().map(|auth| auth.origin)
                && crate::yank_auth::decode(&e.authorization)
                    .ok()
                    .map(|auth| auth.public_key)
                    == signer
        }) {
            return Ok(YankReceipt::new(
                format!("yank-{}", existing.sequence),
                spirit_id.clone(),
                version.into(),
            ));
        }
        let sequence = yanks
            .last()
            .map_or(0, |e| e.sequence)
            .checked_add(1)
            .ok_or_else(|| StorageError::Io("yank sequence exhausted".into()))?;
        let wall = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| StorageError::Io(e.to_string()))?
            .as_nanos() as u64;
        let mut entry = YankEntry::new(
            spirit_id.clone(),
            version.into(),
            wall,
            reason.summary.clone(),
        );
        entry.sequence = sequence;
        if let Some(bytes) = &reason.authorization {
            entry.origin = Some(
                crate::yank_auth::decode(bytes)
                    .map_err(StorageError::Serde)?
                    .origin,
            );
            entry.authorization = bytes.clone();
        }
        yanks.push(entry);
        if let Err(error) = Self::save_yanks_data(&self.root, &yanks) {
            yanks.pop();
            return Err(error);
        }
        Ok(YankReceipt::new(
            format!("yank-{sequence}"),
            spirit_id.clone(),
            version.to_string(),
        ))
    }

    fn yanks_since(&self, since_ns: u64) -> Result<YankList, StorageError> {
        let yanks = Self::load_yanks(&self.root).map_err(|e| StorageError::Io(e.to_string()))?;
        let entries: Vec<YankEntry> = yanks
            .iter()
            .filter(|e| since_ns == 0 || e.sequence > since_ns)
            .cloned()
            .collect();
        Ok(YankList::new(entries))
    }

    fn put_benchmark(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        sidecar: &[u8],
    ) -> Result<(), StorageError> {
        let vdir = self.version_dir(spirit_id, version)?;
        fs::create_dir_all(&vdir).map_err(|e| StorageError::Io(e.to_string()))?;
        fs::write(vdir.join(BENCHMARK_FILE), sidecar).map_err(|e| StorageError::Io(e.to_string()))
    }

    fn get_benchmark(
        &self,
        spirit_id: &SpiritId,
        version: Option<&str>,
    ) -> Result<(String, Vec<u8>), StorageError> {
        let not_found = |version: &str| StorageError::BenchmarkNotFound {
            spirit_id: spirit_id.as_str().to_string(),
            version: version.to_string(),
        };
        let version = match version {
            Some(version) => version.to_owned(),
            None => fs::read_dir(self.spirit_dir(spirit_id)?)
                .map_err(|_| not_found("any"))?
                .flatten()
                .filter(|entry| entry.path().join(BENCHMARK_FILE).is_file())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .max_by_key(|v| {
                    v.split('.')
                        .map(|p| p.parse().unwrap_or(0))
                        .collect::<Vec<u64>>()
                })
                .ok_or_else(|| not_found("any"))?,
        };
        let bytes = fs::read(self.version_dir(spirit_id, &version)?.join(BENCHMARK_FILE))
            .map_err(|_| not_found(&version))?;
        Ok((version, bytes))
    }
}

/// Story 18-2 — a version directory's signed halt benchmark sidecar.
pub const BENCHMARK_FILE: &str = "benchmark.signed.json";

/// One directory name under the registry root: never empty, `.`/`..`, or a
/// path separator.
pub(crate) fn path_segment(segment: &str) -> Result<&str, StorageError> {
    if segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.len() > 128
        || !segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
    {
        return Err(StorageError::InvalidSegment(segment.to_owned()));
    }
    Ok(segment)
}

fn unique_suffix() -> String {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

fn write_json_atomic(path: &Path, value: &impl serde::Serialize) -> Result<(), StorageError> {
    let temporary = path.with_extension(format!("tmp-{}", unique_suffix()));
    let result = (|| {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        // Serializing byte arrays directly to File emits a syscall per token.
        // Buffer writes, but flush before sync/rename to retain atomic durability.
        let mut writer = std::io::BufWriter::new(&file);
        serde_json::to_writer(&mut writer, value)
            .map_err(|e| StorageError::Serde(e.to_string()))?;
        std::io::Write::flush(&mut writer).map_err(|e| StorageError::Io(e.to_string()))?;
        file.sync_all()
            .map_err(|e| StorageError::Io(e.to_string()))?;
        fs::rename(&temporary, path).map_err(|e| StorageError::Io(e.to_string()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Truncate a string at a character boundary, keeping at most `max_chars` characters.
fn truncate_str(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Extract a short summary from a manifest TOML.
fn extract_summary(manifest_toml: &[u8]) -> String {
    let text = String::from_utf8_lossy(manifest_toml);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("description") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let s = val.trim().trim_matches('"').to_string();
                if !s.is_empty() {
                    return s;
                }
            }
        }
    }
    for line in text.lines() {
        if !line.trim().is_empty() && !line.trim().starts_with('[') && !line.trim().starts_with('#')
        {
            let s = line.trim().to_string();
            if s.len() <= 120 {
                return s;
            }
            return format!("{}...", truncate_str(&s, 117));
        }
    }
    String::from("(no description)")
}

/// Monotonic nanosecond counter (per Story 5.5c §1366).
pub(crate) fn monotonic_now_ns() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let start = START.get_or_init(|| std::time::Instant::now());
    start.elapsed().as_nanos() as u64
}

/// Resolve the user's home directory.
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Storage errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(String),
    #[error("serde error: {0}")]
    Serde(String),
    #[error("version '{version}' not found for spirit '{spirit_id}'")]
    VersionNotFound { spirit_id: String, version: String },
    #[error("no benchmark sidecar for spirit '{spirit_id}' version '{version}'")]
    BenchmarkNotFound { spirit_id: String, version: String },
    #[error("invalid registry path segment '{0}'")]
    InvalidSegment(String),
    #[error("publisher key differs from pinned owner for '{0}'")]
    PublisherMismatch(String),
    #[error("version '{spirit_id}@{version}' is immutable")]
    ImmutableVersion { spirit_id: String, version: String },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};

    fn empty_envelope() -> ComplianceClaimEnvelope {
        ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [1u8; 32],
            claim_bytes: vec![0xA1, 0x01],
            signing_alg: SigningAlg::Ed25519,
        }
    }

    fn test_pkg(id: &str, ver: &str, desc: &str) -> SignedPackage {
        let manifest = format!(
            "[spirit]\nname = \"{}\"\nversion = \"{}\"\ndescription = \"{}\"\n",
            id, ver, desc
        );
        SignedPackage::new(
            SpiritId::from(id),
            ver.to_string(),
            manifest.into_bytes(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        )
    }

    fn temp_storage() -> LocalFsRegistryStorage {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("maos-registry-test-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&dir);
        LocalFsRegistryStorage::at_path(dir).unwrap()
    }

    #[test]
    fn put_and_get_manifest() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "A friendly Spirit");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        let manifest = s.get_manifest(&sid, "0.1.0").unwrap();
        assert_eq!(manifest.manifest_toml, pkg.manifest_toml);
    }

    #[test]
    fn put_and_get_artifact() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "A friendly Spirit");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        let artifact = s.get_artifact(&sid, "0.1.0").unwrap();
        assert_eq!(artifact.artifact_bytes, pkg.artifact_bytes);
    }

    #[test]
    fn search_finds_spirit_by_name() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "A friendly Spirit");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        let q = SearchQuery::new("hello".into(), false, 50);
        let results = s.search(&q).unwrap();
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].spirit_id.as_str(), "hello-spirit");
    }

    #[test]
    fn search_excludes_yanked_by_default() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "Test");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        s.yank(&sid, "0.1.0", &YankReason::new("buggy".into()))
            .unwrap();
        let q = SearchQuery::new("hello".into(), false, 50);
        let results = s.search(&q).unwrap();
        assert!(results.items.is_empty());
    }

    #[test]
    fn search_includes_yanked_when_requested() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "Test");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        s.yank(&sid, "0.1.0", &YankReason::new("buggy".into()))
            .unwrap();
        let q = SearchQuery::new("hello".into(), true, 50);
        let results = s.search(&q).unwrap();
        assert_eq!(results.items.len(), 1);
    }

    #[test]
    fn yanks_since_returns_filtered_entries() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "Test");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        s.yank(&sid, "0.1.0", &YankReason::new("buggy".into()))
            .unwrap();

        let list = s.yanks_since(0).unwrap();
        assert_eq!(list.entries.len(), 1);
    }

    #[test]
    fn version_not_found_error() {
        let s = temp_storage();
        let sid = SpiritId::from("nonexistent");
        let err = s.get_manifest(&sid, "1.0.0").unwrap_err();
        assert!(matches!(err, StorageError::VersionNotFound { .. }));
    }
    #[test]
    fn absent_install_inventory_does_not_create_a_store() {
        let root = std::env::temp_dir().join(format!("read-only-inventory-{}", unique_suffix()));
        assert!(!root.exists());
        assert!(LocalFsRegistryStorage::installed_manifests_at(&root)
            .unwrap()
            .is_empty());
        assert!(!root.exists(), "inventory discovery is read-only");
    }
}
