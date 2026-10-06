// Story 20-6: the crate under test is `#![cfg(target_os = "linux")]`.
#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use maos_exec_deps::{parse_elf, resolve, Elf};

const ELF_HEADER: usize = 64;
const PROGRAM_HEADER: usize = 56;
const PROGRAM_HEADERS: usize = ELF_HEADER;
const INTERP: usize = 0x180;
const DYNAMIC: usize = 0x200;
const DYNSTR: usize = 0x300;
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;
const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_STRSZ: u64 = 10;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;
const BASE_INTERP: &str = "/lib64/ld-linux-x86-64.so.2";
const BASE_NEEDED: &str = "libc.so.6";
const BASE_RUNPATH: &str = "/usr/lib";

/// Small ELF64-LE fixture builder. Every hostile-input case starts with one of
/// these parser-accepted images and changes only the field relevant to its
/// refusal; it never needs an external ELF producer.
struct ElfImage {
    bytes: Vec<u8>,
    dynamic: usize,
    dynstr: usize,
    strsz: usize,
}

impl ElfImage {
    fn valid() -> Self {
        Self::dynamic(BASE_INTERP, BASE_NEEDED, None, Some(BASE_RUNPATH))
    }

    fn dynamic(interp: &str, needed: &str, rpath: Option<&str>, runpath: Option<&str>) -> Self {
        Self::with_needed(interp, &[needed], rpath, runpath)
    }

    fn with_needed(
        interp: &str,
        needed: &[&str],
        rpath: Option<&str>,
        runpath: Option<&str>,
    ) -> Self {
        let mut strings = Vec::new();
        let needed_offsets = needed
            .iter()
            .map(|needed| push_string(&mut strings, needed))
            .collect::<Vec<_>>();
        let rpath_offset = rpath.map(|path| push_string(&mut strings, path));
        let runpath_offset = runpath.map(|path| push_string(&mut strings, path));
        let entry_count = 2
            + needed_offsets.len()
            + usize::from(rpath_offset.is_some())
            + usize::from(runpath_offset.is_some())
            + 1;
        let dynstr = DYNAMIC + entry_count * 16;
        let mut entries = vec![(DT_STRTAB, dynstr as u64), (DT_STRSZ, strings.len() as u64)];
        entries.extend(
            needed_offsets
                .into_iter()
                .map(|offset| (DT_NEEDED, offset as u64)),
        );
        if let Some(offset) = rpath_offset {
            entries.push((DT_RPATH, offset as u64));
        }
        if let Some(offset) = runpath_offset {
            entries.push((DT_RUNPATH, offset as u64));
        }
        entries.push((DT_NULL, 0));

        let image_len = dynstr + strings.len();
        let mut bytes = vec![0; image_len];
        write_elf_header(&mut bytes, 3);
        write_program_header(&mut bytes, 0, PT_LOAD, 0, 0, image_len as u64);
        write_program_header(
            &mut bytes,
            1,
            PT_INTERP,
            INTERP as u64,
            INTERP as u64,
            (interp.len() + 1) as u64,
        );
        write_program_header(
            &mut bytes,
            2,
            PT_DYNAMIC,
            DYNAMIC as u64,
            DYNAMIC as u64,
            (entries.len() * 16) as u64,
        );
        bytes[INTERP..INTERP + interp.len()].copy_from_slice(interp.as_bytes());
        for (i, (tag, value)) in entries.into_iter().enumerate() {
            write_dynamic(&mut bytes, i, tag, value);
        }
        bytes[dynstr..dynstr + strings.len()].copy_from_slice(&strings);

        Self {
            bytes,
            dynamic: DYNAMIC,
            dynstr,
            strsz: strings.len(),
        }
    }

    fn static_elf() -> Self {
        let mut bytes = vec![0; 0x200];
        let image_len = bytes.len() as u64;
        write_elf_header(&mut bytes, 1);
        write_program_header(&mut bytes, 0, PT_LOAD, 0, 0, image_len);
        Self {
            bytes,
            dynamic: DYNAMIC,
            dynstr: DYNSTR,
            strsz: 0,
        }
    }

    fn dynamic_value(&mut self, entry: usize, value: u64) {
        put_u64(&mut self.bytes, self.dynamic + entry * 16 + 8, value);
    }

    fn dynamic_tag(&mut self, entry: usize, tag: u64) {
        put_u64(&mut self.bytes, self.dynamic + entry * 16, tag);
    }

    fn phentsize(&mut self, value: u16) {
        put_u16(&mut self.bytes, 54, value);
    }

    fn load_offset(&mut self, value: u64) {
        put_u64(&mut self.bytes, program_header_offset(0) + 8, value);
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn push_string(strings: &mut Vec<u8>, value: &str) -> usize {
    let offset = strings.len();
    strings.extend_from_slice(value.as_bytes());
    strings.push(0);
    offset
}

fn write_elf_header(bytes: &mut [u8], phnum: u16) {
    bytes[..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    put_u64(bytes, 32, PROGRAM_HEADERS as u64);
    put_u16(bytes, 54, PROGRAM_HEADER as u16);
    put_u16(bytes, 56, phnum);
}

fn program_header_offset(index: usize) -> usize {
    PROGRAM_HEADERS + index * PROGRAM_HEADER
}

fn write_program_header(
    bytes: &mut [u8],
    index: usize,
    kind: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
) {
    let at = program_header_offset(index);
    put_u32(bytes, at, kind);
    put_u64(bytes, at + 8, offset);
    put_u64(bytes, at + 16, vaddr);
    put_u64(bytes, at + 32, filesz);
}

fn write_dynamic(bytes: &mut [u8], index: usize, tag: u64, value: u64) {
    let at = DYNAMIC + index * 16;
    put_u64(bytes, at, tag);
    put_u64(bytes, at + 8, value);
}

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn assert_parse_error(bytes: &[u8], category: &str) {
    let error = parse_elf(Cursor::new(bytes)).expect_err("hostile ELF must be refused");
    assert!(
        error.0.contains(category),
        "error {error:?} must name the {category:?} refusal category"
    );
}

#[test]
fn valid_minimal_elf_exposes_loader_needed_and_runpath() {
    let image = ElfImage::valid();
    let elf = parse_elf(Cursor::new(image.bytes())).expect("known-good crafted ELF parses");
    assert_eq!(
        elf,
        Elf {
            interp: Some(BASE_INTERP.to_owned()),
            needed: vec![BASE_NEEDED.to_owned()],
            search_paths: vec![BASE_RUNPATH.to_owned()],
            transitive_rpath: vec![],
        }
    );
}

#[test]
fn hostile_headers_are_refused_with_a_reason() {
    assert_parse_error(b"\x7fELF", "short ELF header");

    let mut elf32 = ElfImage::valid();
    elf32.bytes[4] = 1;
    assert_parse_error(elf32.bytes(), "ELF64");

    let mut big_endian = ElfImage::valid();
    big_endian.bytes[5] = 2;
    assert_parse_error(big_endian.bytes(), "little-endian");

    let mut wrong_phentsize = ElfImage::valid();
    wrong_phentsize.phentsize(55);
    assert_parse_error(wrong_phentsize.bytes(), "ELF64");
}

#[test]
fn hostile_dynamic_offsets_and_strings_are_refused() {
    let mut strtab_outside_load = ElfImage::valid();
    strtab_outside_load.dynamic_value(0, 0x4000);
    assert_parse_error(strtab_outside_load.bytes(), "DT_STRTAB");

    let mut string_past_strsz = ElfImage::valid();
    string_past_strsz.dynamic_value(2, string_past_strsz.strsz as u64);
    assert_parse_error(string_past_strsz.bytes(), "DT_STRSZ");

    let mut unterminated = ElfImage::valid();
    let runpath_offset = BASE_NEEDED.len() + 1;
    unterminated.bytes[unterminated.dynstr + runpath_offset + BASE_RUNPATH.len()] = b'x';
    assert_parse_error(unterminated.bytes(), "unterminated");

    let overlong_needed = ElfImage::dynamic(BASE_INTERP, &"x".repeat(4097), None, None);
    assert_parse_error(overlong_needed.bytes(), "unterminated");

    let mut offset_overflow = ElfImage::valid();
    offset_overflow.load_offset(u64::MAX);
    assert_parse_error(offset_overflow.bytes(), "DT_STRTAB");
}

#[test]
fn hostile_dynamic_tables_and_repeated_dependencies_are_bounded() {
    let mut oversized_table = ElfImage::valid();
    write_program_header(
        &mut oversized_table.bytes,
        2,
        PT_DYNAMIC,
        DYNAMIC as u64,
        DYNAMIC as u64,
        4097 * 16,
    );
    assert_parse_error(oversized_table.bytes(), "dynamic table exceeds");

    let names = vec!["librepeated.so"; 1025];
    let repeated = ElfImage::with_needed(BASE_INTERP, &names, None, None);
    assert_parse_error(repeated.bytes(), "too many DT_NEEDED");
}

#[test]
fn hostile_library_search_paths_are_refused() {
    let mut origin = ElfImage::valid();
    let path_at = origin.dynstr + BASE_NEEDED.len() + 1;
    origin.bytes[path_at..path_at + 8].copy_from_slice(b"$ORIGIN\0");
    assert_parse_error(origin.bytes(), "$ORIGIN");

    let mut relative = ElfImage::valid();
    let path_at = relative.dynstr + BASE_NEEDED.len() + 1;
    relative.bytes[path_at..path_at + 9].copy_from_slice(b"relative\0");
    assert_parse_error(relative.bytes(), "relative");
    let mut relative_rpath = ElfImage::valid();
    relative_rpath.dynamic_tag(3, DT_RPATH);
    let path_at = relative_rpath.dynstr + BASE_NEEDED.len() + 1;
    relative_rpath.bytes[path_at..path_at + 9].copy_from_slice(b"relative\0");
    assert_parse_error(relative_rpath.bytes(), "relative");

    let mut empty = ElfImage::valid();
    let path_at = empty.dynstr + BASE_NEEDED.len() + 1;
    empty.bytes[path_at] = 0;
    assert_parse_error(empty.bytes(), "relative");
}

#[test]
fn rpath_is_used_only_when_runpath_is_absent() {
    let mut rpath_only = ElfImage::valid();
    rpath_only.dynamic_tag(3, DT_RPATH);
    let elf = parse_elf(Cursor::new(rpath_only.bytes())).expect("DT_RPATH is supported");
    assert_eq!(elf.search_paths, [BASE_RUNPATH]);
    assert_eq!(elf.transitive_rpath, [BASE_RUNPATH]);

    let both = ElfImage::dynamic(BASE_INTERP, BASE_NEEDED, Some("/rpath"), Some("/runpath"));
    let elf = parse_elf(Cursor::new(both.bytes())).expect("both path tags parse");
    assert_eq!(elf.search_paths, ["/runpath"]);
    assert!(elf.transitive_rpath.is_empty());
}

static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);
impl TestDir {
    fn new() -> Self {
        Self::in_dir(&std::env::temp_dir())
    }

    fn in_dir(parent: &Path) -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            "maos-exec-deps-17-6-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique temporary test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path).expect("fixture metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("mark fixture executable");
}

fn write_static_elf(path: &Path) {
    fs::write(path, ElfImage::static_elf().bytes()).expect("write static ELF fixture");
    make_executable(path);
}

fn host_loader() -> PathBuf {
    let self_exe = std::env::current_exe().expect("test executable path");
    let elf = parse_elf(File::open(&self_exe).expect("open test executable"))
        .expect("test executable is a readable ELF");
    PathBuf::from(elf.interp.expect("test executable is dynamically linked"))
        .canonicalize()
        .expect("host ELF loader exists")
}

fn assert_canonical_existing(path: &Path) {
    assert!(path.exists(), "{} must exist", path.display());
    assert_eq!(
        path,
        path.canonicalize().expect("existing path canonicalizes"),
        "{} must already be canonical",
        path.display()
    );
}

#[test]
fn fifo_is_refused_without_waiting_for_a_writer() {
    let temp = TestDir::new();
    let fifo = temp.path().join("program.fifo");
    let status = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo is available on Linux");
    assert!(status.success(), "mkfifo fixture creation failed");

    let started = Instant::now();
    let error = resolve(fifo.as_os_str(), None).expect_err("FIFO is not a program");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "FIFO resolution must not block waiting for a writer"
    );
    assert!(
        error.0.contains("regular file"),
        "FIFO error must name the regular-file refusal: {error:?}"
    );
}

#[test]
fn directory_and_shebang_script_are_refused_as_non_elf_programs() {
    let temp = TestDir::new();
    let error = resolve(temp.path().as_os_str(), None).expect_err("directory is not a program");
    assert!(
        error.0.contains("regular file"),
        "directory error must name the regular-file refusal: {error:?}"
    );

    let script = temp.path().join("script");
    let mut file = File::create(&script).expect("create script fixture");
    writeln!(file, "#!/bin/sh").expect("write shebang");
    writeln!(file, "# {}", "not an ELF ".repeat(16)).expect("pad script header");
    make_executable(&script);
    let error =
        resolve(script.as_os_str(), None).expect_err("shebang script is not an ELF program");
    assert!(
        error.0.contains("ELF64"),
        "script error must name the ELF refusal: {error:?}"
    );
}

#[test]
fn directory_root_and_device_interpreters_are_refused_before_grant() {
    let temp = TestDir::new();
    let cases = [
        (temp.path().to_path_buf(), "regular file"),
        (PathBuf::from("/"), "non-root"),
        (PathBuf::from("/dev/null"), "regular file"),
        (PathBuf::from("relative-loader"), "absolute"),
    ];
    for (index, (interp, category)) in cases.into_iter().enumerate() {
        let binary = temp.path().join(format!("bad-interpreter-{index}"));
        let image = ElfImage::dynamic(
            interp
                .to_str()
                .expect("temporary and system paths are UTF-8"),
            BASE_NEEDED,
            None,
            None,
        );
        fs::write(&binary, image.bytes()).expect("write interpreter fixture");
        make_executable(&binary);

        let error = resolve(binary.as_os_str(), None).expect_err("invalid interpreter is refused");
        assert!(
            error.0.contains(category),
            "{interp:?} must be refused before granting it: {error:?}"
        );
    }
}

#[test]
fn regular_non_elf_interpreter_is_refused_before_grant() {
    let temp = TestDir::new();
    let invalid_loader = temp.path().join("not-an-elf-loader");
    fs::write(&invalid_loader, b"not an ELF loader").expect("write invalid loader fixture");
    let binary = temp.path().join("bad-loader-elf");
    let image = ElfImage::dynamic(
        invalid_loader.to_str().expect("temporary path is UTF-8"),
        BASE_NEEDED,
        None,
        None,
    );
    fs::write(&binary, image.bytes()).expect("write interpreter fixture");
    make_executable(&binary);

    resolve(binary.as_os_str(), None).expect_err("invalid loader is refused");
}

#[test]
fn symlinked_program_and_absolute_path_entry_resolve_to_static_target() {
    let temp = TestDir::new();
    let target = temp.path().join("static-target");
    write_static_elf(&target);
    let link = temp.path().join("static-link");
    symlink(&target, &link).expect("create program symlink");

    let deps = resolve(link.as_os_str(), None).expect("symlinked static ELF resolves");
    assert_eq!(
        deps.program,
        target.canonicalize().expect("canonical target")
    );
    assert_eq!(deps.loader, None);
    assert!(deps.libraries.is_empty());
}

#[test]
fn relative_path_entries_are_skipped_for_bare_program_resolution() {
    let temp = TestDir::new();
    let cwd = std::env::current_dir().expect("test current directory");
    let relative = TestDir::in_dir(&cwd);
    let target = temp.path().join("static-target");
    write_static_elf(&target);
    let bare_name = OsString::from("sandbox-static");
    write_static_elf(&relative.path().join(&bare_name));
    symlink(&target, temp.path().join(&bare_name)).expect("place absolute PATH fixture");
    let relative_entry = relative
        .path()
        .file_name()
        .expect("relative fixture directory has a name");
    let path =
        std::env::join_paths([Path::new(relative_entry), temp.path()]).expect("Unix PATH fixture");

    let deps = resolve(&bare_name, Some(&path)).expect("absolute PATH entry resolves bare program");
    assert_eq!(
        deps.program,
        target
            .canonicalize()
            .expect("canonical absolute-PATH target")
    );
    assert_eq!(deps.loader, None);
    assert!(deps.libraries.is_empty());
}

#[test]
fn static_elf_needs_no_loader_or_libraries() {
    let temp = TestDir::new();
    let binary = temp.path().join("static");
    write_static_elf(&binary);

    let deps = resolve(binary.as_os_str(), None).expect("crafted static ELF resolves");
    assert_canonical_existing(&deps.program);
    assert_eq!(deps.loader, None);
    assert!(deps.libraries.is_empty());
}

#[test]
fn missing_soname_is_refused_as_not_found() {
    let temp = TestDir::new();
    let binary = temp.path().join("missing-soname");
    let loader = host_loader();
    let image = ElfImage::dynamic(
        loader.to_str().expect("host loader UTF-8 path"),
        "libdoesnotexist.so.9",
        None,
        None,
    );
    fs::write(&binary, image.bytes()).expect("write missing-soname fixture");
    make_executable(&binary);

    let error = resolve(binary.as_os_str(), None).expect_err("missing soname must fail closed");
    assert!(
        error.0.contains("not found"),
        "missing soname error must name lookup failure: {error:?}"
    );
}

#[test]
fn device_library_candidate_is_refused_without_reading_it() {
    let temp = TestDir::new();
    let loader = host_loader();
    let binary = temp.path().join("device-library");
    let image = ElfImage::dynamic(
        loader.to_str().expect("host loader UTF-8 path"),
        "null",
        None,
        Some("/dev"),
    );
    fs::write(&binary, image.bytes()).expect("write device library fixture");
    make_executable(&binary);

    let error = resolve(binary.as_os_str(), None).expect_err("device is not a library");
    assert!(
        error.0.contains("regular file"),
        "device candidate must be rejected before reading it: {error:?}"
    );
}

#[test]
fn transitive_dependency_closure_is_bounded() {
    let temp = TestDir::new();
    let loader = host_loader();
    const LIMIT_PLUS_ONE: usize = 1025;
    for index in (0..LIMIT_PLUS_ONE - 1).rev() {
        let next = format!("libclosure-{}.so", index + 1);
        let image = ElfImage::dynamic(
            "/unused-interpreter",
            &next,
            None,
            Some(temp.path().to_str().expect("temporary path UTF-8")),
        );
        fs::write(
            temp.path().join(format!("libclosure-{index}.so")),
            image.bytes(),
        )
        .expect("write transitive library fixture");
    }
    write_static_elf(
        &temp
            .path()
            .join(format!("libclosure-{}.so", LIMIT_PLUS_ONE - 1)),
    );
    let binary = temp.path().join("too-many-transitive-libraries");
    let image = ElfImage::dynamic(
        loader.to_str().expect("host loader UTF-8 path"),
        "libclosure-0.so",
        None,
        Some(temp.path().to_str().expect("temporary path UTF-8")),
    );
    fs::write(&binary, image.bytes()).expect("write closure fixture");
    make_executable(&binary);

    let error = resolve(binary.as_os_str(), None).expect_err("closure limit must fail closed");
    assert!(
        error.0.contains("dependency closure exceeds"),
        "transitive dependency limit must be reported: {error:?}"
    );
}

fn dynamic_probe() -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../maos-wasm-host/test-fixtures/forbidden-syscall-probe");
    let probe = fixture.join("target/release/forbidden-syscall-probe");
    if !probe.exists() {
        let status = Command::new("cargo")
            .args(["build", "--release", "--locked", "--target-dir", "target"])
            .current_dir(fixture)
            .env_remove("CARGO_TARGET_DIR")
            .status()
            .expect("start forbidden-syscall-probe release build");
        assert!(
            status.success(),
            "forbidden-syscall-probe release build failed"
        );
    }
    probe
}

#[test]
fn dynamic_probe_resolves_canonical_loader_and_libc_closure() {
    let probe = dynamic_probe();
    let deps = resolve(probe.as_os_str(), None).expect("dynamic probe dependency closure resolves");
    assert_canonical_existing(&deps.program);
    let loader = deps.loader.as_deref().expect("dynamic probe has a loader");
    assert_canonical_existing(loader);
    assert!(
        deps.libraries
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "libc.so.6")),
        "dynamic probe closure must include libc.so.6: {:?}",
        deps.libraries
    );
    for path in &deps.libraries {
        assert_canonical_existing(path);
    }
}

#[test]
fn runpath_is_searched_before_the_loader_directory() {
    let loader = host_loader();
    let host_libc = loader
        .parent()
        .expect("loader has a directory")
        .join("libc.so.6")
        .canonicalize()
        .expect("host libc sits beside the loader");
    let temp = TestDir::new();
    let runpath_libc = temp.path().join("libc.so.6");
    fs::copy(&host_libc, &runpath_libc).expect("copy libc into RUNPATH fixture");
    let binary = temp.path().join("runpath-first");
    let image = ElfImage::dynamic(
        loader.to_str().expect("host loader UTF-8 path"),
        "libc.so.6",
        None,
        Some(temp.path().to_str().expect("temporary path UTF-8")),
    );
    fs::write(&binary, image.bytes()).expect("write RUNPATH fixture");
    make_executable(&binary);

    let deps = resolve(binary.as_os_str(), None).expect("RUNPATH fixture resolves");
    let runpath_libc = runpath_libc
        .canonicalize()
        .expect("copied libc canonicalizes");
    assert!(
        deps.libraries.contains(&runpath_libc),
        "RUNPATH libc must be selected before loader directory: {:?}",
        deps.libraries
    );
    assert!(
        !deps.libraries.contains(&host_libc),
        "loader-directory libc must not win over RUNPATH: {:?}",
        deps.libraries
    );
}

#[test]
fn rpath_is_transitive_while_runpath_is_not() {
    let loader = host_loader();
    let temp = TestDir::new();
    let parent_name = "libparent-17-6.so";
    let child_name = "libchild-17-6.so";
    let parent = temp.path().join(parent_name);
    let child = temp.path().join(child_name);
    let parent_image = ElfImage::dynamic(
        loader.to_str().expect("host loader UTF-8 path"),
        child_name,
        None,
        None,
    );
    fs::write(&parent, parent_image.bytes()).expect("write parent library fixture");
    write_static_elf(&child);

    let rpath_binary = temp.path().join("rpath-transitive");
    let rpath_image = ElfImage::dynamic(
        loader.to_str().expect("host loader UTF-8 path"),
        parent_name,
        Some(temp.path().to_str().expect("temporary path UTF-8")),
        None,
    );
    fs::write(&rpath_binary, rpath_image.bytes()).expect("write RPATH fixture");
    make_executable(&rpath_binary);
    let deps = resolve(rpath_binary.as_os_str(), None).expect("RPATH reaches a grandchild");
    assert!(
        deps.libraries.contains(
            &child
                .canonicalize()
                .expect("RPATH child fixture canonicalizes")
        ),
        "transitive RPATH must locate the grandchild: {:?}",
        deps.libraries
    );

    let runpath_binary = temp.path().join("runpath-not-transitive");
    let runpath_image = ElfImage::dynamic(
        loader.to_str().expect("host loader UTF-8 path"),
        parent_name,
        None,
        Some(temp.path().to_str().expect("temporary path UTF-8")),
    );
    fs::write(&runpath_binary, runpath_image.bytes()).expect("write RUNPATH fixture");
    make_executable(&runpath_binary);
    let error = resolve(runpath_binary.as_os_str(), None)
        .expect_err("RUNPATH must not locate a grandchild dependency");
    assert!(
        error.0.contains("not found"),
        "non-transitive RUNPATH must fail the grandchild lookup: {error:?}"
    );
}
