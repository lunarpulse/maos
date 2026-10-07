//! Story 17-6 — the files an `execve` of a program opens: the program, its ELF
//! interpreter (the dynamic loader) and the shared libraries the loader maps.
//!
//! The kernel's T2 sandbox (`maos-kernel-core` `security/sandbox/linux.rs`)
//! grants Landlock rights on exactly this set. The program is PARSED, never
//! run: no `ldd`, no subprocess. Input is hostile (any manifest can name a
//! program), so every read is bounded, non-regular files are refused before
//! they can block, and every failure is an error — never a broader answer.
//!
//! Not supported, refused typed: `#!` scripts (not ELF — a T2 program is an
//! ELF64 little-endian executable), `$ORIGIN`/`$LIB` substitution and relative
//! (or empty) `DT_RUNPATH`/`DT_RPATH` entries — ld.so resolves those against
//! the child's working directory, which the parent cannot pin.
//!
//! Linux-only by construction (ELF, `O_PATH`, procfs reopen): on any other OS
//! the crate compiles empty, so `cargo check --workspace` works there (Story
//! 20-6); its one dependent takes it only under `cfg(target_os = "linux")`.
#![forbid(unsafe_code)]

use std::collections::{BTreeSet, VecDeque};
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The resolved exec set. Paths are canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecDeps {
    pub program: PathBuf,
    /// `PT_INTERP`, canonical; `None` for a static binary.
    pub loader: Option<PathBuf>,
    /// Transitive `DT_NEEDED` closure, deduplicated, in discovery order.
    pub libraries: Vec<PathBuf>,
    /// File identities in program, loader (if any), libraries order. The
    /// sandbox compares these with its opened Landlock rule fds, so changing
    /// a path after parsing can only refuse the spawn, never grant a new inode.
    file_ids: Vec<FileIdentity>,
}

/// Device/inode identity of an ELF object while its parsed file was held open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileIdentity {
    pub dev: u64,
    pub ino: u64,
}

impl ExecDeps {
    pub fn file_ids(&self) -> &[FileIdentity] {
        &self.file_ids
    }
}

/// What one ELF object asks of the loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elf {
    /// `PT_INTERP`, as written in the file.
    pub interp: Option<String>,
    /// `DT_NEEDED` sonames, in file order.
    pub needed: Vec<String>,
    /// Search paths effective for this object's direct `DT_NEEDED` entries:
    /// `DT_RUNPATH` when present, otherwise `DT_RPATH`.
    pub search_paths: Vec<String>,
    /// Effective `DT_RPATH` entries inherited by transitive dependencies.
    ///
    /// This is empty when the object has `DT_RUNPATH`, which disables its
    /// `DT_RPATH` under the dynamic loader's search rules.
    pub transitive_rpath: Vec<String>,
}

/// Why the set could not be resolved. The caller refuses the spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err<T>(msg: impl Into<String>) -> Result<T, Error> {
    Err(Error(msg.into()))
}

/// per object: inherited `DT_RPATH`, its `DT_RUNPATH` (else `DT_RPATH`), then
/// the loader's directory. `DT_RUNPATH` does not apply to transitive lookups.
pub fn resolve(program: &OsStr, path_env: Option<&OsStr>) -> Result<ExecDeps, Error> {
    let program = resolve_program(program, path_env)?;
    let (elf, program_id) = read_elf(&program)?;
    let Some(interp) = &elf.interp else {
        return Ok(ExecDeps {
            program,
            loader: None,
            libraries: vec![],
            file_ids: vec![program_id],
        });
    };
    let interp = Path::new(interp);
    if !interp.is_absolute() || interp == Path::new("/") {
        return err(format!(
            "{}: PT_INTERP must name an absolute non-root path",
            program.display()
        ));
    }
    let loader = canonical(interp)?;
    if loader == Path::new("/") {
        return err(format!("{}: PT_INTERP resolves to root", program.display()));
    }
    // Validate the interpreter itself before returning a path the sandbox may
    // grant. In particular, never grant a directory or special file.
    let (_, loader_id) = read_elf(&loader)?;
    let loader_dir = loader.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut libraries = Vec::new();
    let mut file_ids = vec![program_id, loader_id];
    let mut seen = BTreeSet::new();
    let mut library_lookups = 0;
    let mut queue = VecDeque::from([(program.clone(), elf, Vec::<PathBuf>::new())]);
    while let Some((owner, elf, inherited_rpath)) = queue.pop_front() {
        let search: Vec<PathBuf> = inherited_rpath
            .iter()
            .cloned()
            .chain(elf.search_paths.iter().map(PathBuf::from))
            .chain(std::iter::once(loader_dir.clone()))
            .collect();
        let next_rpath = inherited_rpath
            .into_iter()
            .chain(elf.transitive_rpath.iter().map(PathBuf::from))
            .collect::<Vec<_>>();
        if next_rpath.len() > MAX_SEARCH_PATHS {
            return err(format!(
                "{}: inherited DT_RPATH exceeds {MAX_SEARCH_PATHS} entries",
                owner.display()
            ));
        }
        for soname in &elf.needed {
            if library_lookups == MAX_LIBRARY_LOOKUPS {
                return err(format!(
                    "{}: dependency lookup limit ({MAX_LIBRARY_LOOKUPS}) exceeded",
                    program.display()
                ));
            }
            library_lookups += 1;
            if soname.contains('/') {
                return err(format!("{}: DT_NEEDED {soname} is a path", owner.display()));
            }
            let Some(lib) = search
                .iter()
                .find_map(|dir| canonical(&dir.join(soname)).ok())
            else {
                return err(format!("{}: library {soname} not found", owner.display()));
            };
            if seen.insert(lib.clone()) {
                if libraries.len() == MAX_TRANSITIVE_LIBRARIES {
                    return err(format!(
                        "{}: dependency closure exceeds {MAX_TRANSITIVE_LIBRARIES} libraries",
                        program.display()
                    ));
                }
                let (parsed, file_id) = read_elf(&lib)?;
                queue.push_back((lib.clone(), parsed, next_rpath.clone()));
                file_ids.push(file_id);
                libraries.push(lib);
            }
        }
    }
    Ok(ExecDeps {
        program,
        loader: Some(loader),
        libraries,
        file_ids,
    })
}

fn canonical(path: &Path) -> Result<PathBuf, Error> {
    path.canonicalize()
        .or_else(|e| err(format!("{}: {e}", path.display())))
}

fn resolve_program(program: &OsStr, path_env: Option<&OsStr>) -> Result<PathBuf, Error> {
    // execvp: a name containing '/' is a path; anything else is searched on PATH.
    if program.as_bytes().contains(&b'/') {
        return canonical(Path::new(program));
    }
    let path = path_env
        .map(OsStr::to_os_string)
        .or_else(|| std::env::var_os("PATH"));
    let Some(path) = path else {
        return err("PATH is unset while resolving the program");
    };
    // A relative (or empty) PATH entry names the child's working directory;
    // it is never granted, so an exec that would take it fails closed.
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(program))
        .find(|c| {
            c.metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
        .map_or_else(
            || {
                err(format!(
                    "{} is not an executable on PATH",
                    Path::new(program).display()
                ))
            },
            |c| canonical(&c),
        )
}

/// Open `path` without ever reading a special file, then parse its held inode.
fn read_elf(path: &Path) -> Result<(Elf, FileIdentity), Error> {
    let fail = |what: &dyn std::fmt::Display| Error(format!("{}: {what}", path.display()));
    // O_PATH lets fstat inspect the final path component without opening a
    // device/FIFO for reading. O_NOFOLLOW makes a concurrent symlink swap fail
    // rather than changing the object inspected below.
    let held = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| fail(&e))?;
    let metadata = held.metadata().map_err(|e| fail(&e))?;
    if !metadata.is_file() {
        return Err(fail(&"not a regular file"));
    }
    // Reopen the held regular inode through procfs instead of the attacker-
    // controlled path. The identity check makes the handoff fail closed if the
    // platform cannot preserve that inode across the reopen.
    let proc_fd = format!("/proc/self/fd/{}", held.as_raw_fd());
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(proc_fd)
        .map_err(|e| fail(&e))?;
    let reopened = file.metadata().map_err(|e| fail(&e))?;
    if !reopened.is_file() || reopened.dev() != metadata.dev() || reopened.ino() != metadata.ino() {
        return Err(fail(&"regular file changed while opening"));
    }
    let parsed = parse_elf(BufReader::new(file)).map_err(|e| fail(&e))?;
    Ok((
        parsed,
        FileIdentity {
            dev: metadata.dev(),
            ino: metadata.ino(),
        },
    ))
}

const MAX_STRING: usize = 4096;
const MAX_PROGRAM_HEADERS: usize = 1024;
const MAX_DYNAMIC_ENTRIES: u64 = 4096;
const MAX_NEEDED: usize = 1024;
const MAX_SEARCH_PATH_TAGS: usize = 64;
const MAX_SEARCH_PATHS: usize = 64;
const MAX_TRANSITIVE_LIBRARIES: usize = 1024;
const MAX_LIBRARY_LOOKUPS: usize = 4096;

/// Parse the loader-facing part of an ELF64 little-endian object. Every offset
/// is checked and every string bounded; any malformation is an [`Error`].
pub fn parse_elf<R: Read + Seek>(mut f: R) -> Result<Elf, Error> {
    let fail = |what: &str| Error(what.to_owned());
    let mut h = [0u8; 64];
    f.read_exact(&mut h).map_err(|_| fail("short ELF header"))?;
    if &h[..4] != b"\x7fELF" || h[4] != 2 || h[5] != 1 || u16_at(&h, 54) != 56 {
        return err("not an ELF64 little-endian executable");
    }
    let (phoff, phnum) = (u64_at(&h, 32), u16_at(&h, 56));
    let phnum = usize::from(phnum);
    if phnum > MAX_PROGRAM_HEADERS {
        return err(format!(
            "program header table exceeds {MAX_PROGRAM_HEADERS} entries"
        ));
    }
    let mut phdrs = Vec::with_capacity(phnum);
    seek(&mut f, phoff)?;
    for _ in 0..phnum {
        let mut p = [0u8; 56];
        f.read_exact(&mut p)
            .map_err(|_| fail("short program header"))?;
        // (type, offset, vaddr, filesz)
        phdrs.push((u32_at(&p, 0), u64_at(&p, 8), u64_at(&p, 16), u64_at(&p, 32)));
    }
    let interp = match phdrs.iter().find(|p| p.0 == 3) {
        Some(p) => Some(read_str(&mut f, p.1, p.3)?),
        None => None,
    };
    let mut elf = Elf {
        interp,
        needed: vec![],
        search_paths: vec![],
        transitive_rpath: vec![],
    };
    let Some(dynamic) = phdrs.iter().find(|p| p.0 == 2) else {
        return Ok(elf);
    };
    let (mut needed, mut rpath, mut runpath, mut strtab, mut strsz) =
        (vec![], vec![], vec![], None, 0);
    let dynamic_entries = dynamic.3 / 16;
    if dynamic_entries > MAX_DYNAMIC_ENTRIES {
        return err(format!(
            "dynamic table exceeds {MAX_DYNAMIC_ENTRIES} entries"
        ));
    }
    seek(&mut f, dynamic.1)?;
    for _ in 0..dynamic_entries {
        let mut d = [0u8; 16];
        f.read_exact(&mut d)
            .map_err(|_| fail("short dynamic entry"))?;
        let value = u64_at(&d, 8);
        match u64_at(&d, 0) {
            0 => break,
            1 => {
                if needed.len() == MAX_NEEDED {
                    return err(format!("too many DT_NEEDED entries (limit {MAX_NEEDED})"));
                }
                needed.push(value);
            }
            5 => strtab = Some(value),
            10 => strsz = value,
            15 => {
                if rpath.len() == MAX_SEARCH_PATH_TAGS {
                    return err(format!(
                        "too many DT_RPATH entries (limit {MAX_SEARCH_PATH_TAGS})"
                    ));
                }
                rpath.push(value);
            }
            29 => {
                if runpath.len() == MAX_SEARCH_PATH_TAGS {
                    return err(format!(
                        "too many DT_RUNPATH entries (limit {MAX_SEARCH_PATH_TAGS})"
                    ));
                }
                runpath.push(value);
            }
            _ => {}
        }
    }
    if needed.is_empty() {
        return Ok(elf);
    }
    let Some(strtab) = strtab else {
        return err("DT_NEEDED without DT_STRTAB");
    };
    // The string table's file offset, through the PT_LOAD whose FILE bytes hold it.
    let base = phdrs
        .iter()
        .find(|p| p.0 == 1 && strtab >= p.2 && strtab - p.2 < p.3)
        .and_then(|p| p.1.checked_add(strtab - p.2))
        .ok_or_else(|| fail("DT_STRTAB outside every PT_LOAD"))?;
    let mut string = |off: u64| {
        if off >= strsz {
            return err("dynamic string outside DT_STRSZ");
        }
        let at = base
            .checked_add(off)
            .ok_or_else(|| fail("dynamic string offset overflows"))?;
        read_str(&mut f, at, strsz - off)
    };
    elf.needed = needed
        .into_iter()
        .map(&mut string)
        .collect::<Result<_, _>>()?;
    let uses_runpath = !runpath.is_empty();
    let paths = if uses_runpath { runpath } else { rpath.clone() };
    for off in paths {
        let joined = string(off)?;
        if joined.contains('$') {
            return err("$ORIGIN/$LIB substitution in a library search path is not supported");
        }
        for entry in joined.split(':') {
            if !entry.starts_with('/') {
                return err(format!(
                    "relative library search path {entry:?} (resolved against the child's CWD) is not supported"
                ));
            }
            if elf.search_paths.len() == MAX_SEARCH_PATHS {
                return err(format!(
                    "too many library search paths (limit {MAX_SEARCH_PATHS})"
                ));
            }
            elf.search_paths.push(entry.to_owned());
        }
    }
    if !uses_runpath {
        for off in rpath {
            let joined = string(off)?;
            for entry in joined.split(':') {
                elf.transitive_rpath.push(entry.to_owned());
            }
        }
    }
    Ok(elf)
}

fn seek<R: Seek>(f: &mut R, at: u64) -> Result<(), Error> {
    f.seek(SeekFrom::Start(at))
        .map(drop)
        .or_else(|_| err("seek past end"))
}

fn read_str<R: Read + Seek>(f: &mut R, at: u64, max: u64) -> Result<String, Error> {
    seek(f, at)?;
    let mut bytes = Vec::new();
    f.by_ref()
        .take(max.min(MAX_STRING as u64))
        .read_to_end(&mut bytes)
        .or_else(|_| err("short string"))?;
    let Some(end) = bytes.iter().position(|&b| b == 0) else {
        return err("unterminated string");
    };
    bytes.truncate(end);
    String::from_utf8(bytes).or_else(|_| err("non-UTF-8 string"))
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}
