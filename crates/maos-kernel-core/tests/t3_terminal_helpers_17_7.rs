#![cfg(target_os = "linux")]

#[path = "../../../tests/harness/ci.rs"]
mod ci;

use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::sandbox::{
    ContainerResidue, ContainerRuntimeKind, ImageAttestationId, T3ImageAttestation, T3ImageEntry,
};
use maos_kernel_core::security::sandbox::{
    linux,
    t3::{
        child::{ContainerLaunchCustody, SandboxedContainerChild},
        image_lock::{load_and_verify_lock_at, VerifiedImageAttestation},
        runtime_detect::ContainerRuntime,
        spawn::{
            invoke_bounded, launch_t3, read_receipt, read_receipt_after_hold, validate_state_dir,
            Invocation, T3SpawnContext,
        },
    },
    SandboxSpec,
};
use std::{
    fs,
    os::fd::AsFd,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

fn stub_runtime(dir: &Path, body: &str) -> (ContainerRuntime, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let executable = dir.join("stub-runtime");
    let log = dir.join("runtime.log");
    let script = format!(
        "#!/bin/sh\nLOG='{}'\nprintf '%s\\n' \"$1\" >> \"$LOG\"\n{}\n",
        log.display(),
        body
    );
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    (
        ContainerRuntime {
            kind: ContainerRuntimeKind::Podman,
            path: executable,
            version: "stub".into(),
        },
        log,
    )
}

fn signed_test_image() -> VerifiedImageAttestation {
    use maos_kernel_core::security::crypto::RingCryptoProvider;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    let seed = [0u8; 32];
    let keypair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let public: [u8; 32] = keypair.public_key().as_ref().try_into().unwrap();
    let entries =
        vec![
            T3ImageEntry::new("registry.example/maos", [0x11; 32], "stub test image", true)
                .unwrap(),
        ];
    let signature = RingCryptoProvider
        .sign_capability_token(&seed, &serde_json::to_vec(&entries).unwrap())
        .unwrap();
    let attestation = T3ImageAttestation::new(
        ImageAttestationId([1; 32]),
        1,
        1,
        entries,
        signature.try_into().unwrap(),
        public,
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let lock = temp.path().join("lock.json");
    fs::write(&lock, serde_json::to_vec(&vec![attestation]).unwrap()).unwrap();
    load_and_verify_lock_at(&lock, &public, &RingCryptoProvider)
        .unwrap()
        .default_entry()
        .unwrap()
}

#[test]
fn state_directory_validation_enforces_fresh_private_owned_directory() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let temp = tempfile::tempdir().unwrap();
    let fresh = temp.path().join("fresh");
    fs::create_dir(&fresh).unwrap();
    fs::set_permissions(&fresh, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(validate_state_dir(&fresh).is_ok());
    for mode in [0o755, 0o711] {
        let path = temp.path().join(format!("mode-{mode:o}"));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        assert!(validate_state_dir(&path).is_err());
    }
    let nonempty = temp.path().join("nonempty");
    fs::create_dir(&nonempty).unwrap();
    fs::set_permissions(&nonempty, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(nonempty.join("pid"), b"42").unwrap();
    assert!(validate_state_dir(&nonempty).is_err());
    let linked = temp.path().join("linked");
    symlink(&fresh, &linked).unwrap();
    assert!(validate_state_dir(&linked).is_err());
    let regular = temp.path().join("regular");
    fs::write(&regular, b"x").unwrap();
    assert!(validate_state_dir(&regular).is_err());
    assert!(validate_state_dir(&temp.path().join("missing")).is_err());
    if unsafe { libc::geteuid() } != 0 {
        assert!(validate_state_dir(Path::new("/")).is_err());
    }
}

#[test]
fn receipts_are_nonblocking_held_regular_files_with_bounded_contents() {
    use std::{
        ffi::CString,
        os::unix::{ffi::OsStrExt, fs::symlink},
    };
    let temp = tempfile::tempdir().unwrap();
    let good = temp.path().join("good");
    fs::write(&good, b"123\n").unwrap();
    assert_eq!(read_receipt(&good, 11), Some(b"123\n".to_vec()));
    assert_eq!(read_receipt(&temp.path().join("missing"), 11), None);
    let dangling = temp.path().join("dangling");
    symlink(temp.path().join("absent"), &dangling).unwrap();
    assert_eq!(read_receipt(&dangling, 11), None);
    let valid_link = temp.path().join("valid-link");
    symlink(&good, &valid_link).unwrap();
    assert_eq!(read_receipt(&valid_link, 11), None);
    let dir = temp.path().join("directory");
    fs::create_dir(&dir).unwrap();
    assert_eq!(read_receipt(&dir, 11), None);
    for bytes in [b"123456789012".as_slice(), b"12x", b"abc", b"0"] {
        let path = temp.path().join("case");
        fs::write(&path, bytes).unwrap();
        let result = read_receipt(&path, 11);
        if bytes.len() > 11 {
            assert_eq!(result, None);
        } else {
            assert_eq!(result, Some(bytes.to_vec()));
            assert_eq!(maos_domain::sandbox::parse_pidfile(&result.unwrap()), None);
        }
    }
    let fifo = temp.path().join("fifo");
    let c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = read_receipt(&fifo, 11);
        let _ = tx.send(result);
    });
    assert_eq!(
        rx.recv_timeout(Duration::from_millis(500))
            .expect("FIFO read must not block"),
        None
    );
    let replaced = temp.path().join("replaced");
    let old_inode = temp.path().join("replaced-old");
    fs::write(&replaced, b"42").unwrap();
    let bytes = read_receipt_after_hold(&replaced, 11, || {
        fs::rename(&replaced, &old_inode).unwrap();
        fs::write(&replaced, b"99").unwrap();
    });
    assert_eq!(bytes, Some(b"42".to_vec()));
    assert_eq!(fs::read(&replaced).unwrap(), b"99");
}

#[test]
fn bounded_invocation_drains_caps_and_enforces_deadlines() {
    let shell = Path::new("/bin/sh");
    let filled = invoke_bounded(
        shell,
        &[
            "-c",
            "head -c 100000 /dev/zero; head -c 100000 /dev/zero >&2",
        ],
        &[],
        Instant::now() + Duration::from_secs(3),
    );
    match filled {
        Invocation::Exited {
            status,
            stdout,
            stderr,
            stdout_complete,
            stderr_complete,
        } => {
            assert!(status.success());
            assert_eq!(stdout.len(), 64 * 1024);
            assert_eq!(stderr.len(), 64 * 1024);
            assert!(!stdout_complete, "truncated stdout is incomplete");
            assert!(!stderr_complete, "truncated stderr is incomplete");
        }
        other => panic!("unexpected invocation: {other:?}"),
    }
    let held = Instant::now();
    let descendant = invoke_bounded(
        shell,
        &["-c", "(sleep 3) & exit 0"],
        &[],
        held + Duration::from_secs(3),
    );
    match descendant {
        Invocation::Exited {
            stdout_complete,
            stderr_complete,
            ..
        } => {
            assert!(!stdout_complete, "descendant-held stdout is incomplete");
            assert!(!stderr_complete, "descendant-held stderr is incomplete");
        }
        other => panic!("unexpected invocation: {other:?}"),
    }
    assert!(held.elapsed() < Duration::from_secs(2));
    let delayed = invoke_bounded(
        shell,
        &[
            "-c",
            "(sleep 0.4; printf 'volume cleanup failed\\n' >&2) & exit 0",
        ],
        &[],
        Instant::now() + Duration::from_secs(2),
    );
    match delayed {
        Invocation::Exited {
            stderr,
            stderr_complete,
            ..
        } => {
            assert!(!stderr_complete, "200 ms drain grace cannot certify stderr");
            assert!(!String::from_utf8_lossy(&stderr).contains("volume cleanup failed"));
        }
        other => panic!("unexpected invocation: {other:?}"),
    }
    let start = Instant::now();
    assert!(matches!(
        invoke_bounded(
            shell,
            &["-c", "sleep 2"],
            &[],
            start + Duration::from_millis(150)
        ),
        Invocation::TimedOut
    ));
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(matches!(
        invoke_bounded(
            Path::new("/definitely/not/a/program"),
            &[],
            &[],
            Instant::now() + Duration::from_secs(1)
        ),
        Invocation::Failed
    ));
}

fn kernel_supports_pidfd_info_exit() -> bool {
    let output = Command::new("uname").arg("-r").output().unwrap();
    let version = String::from_utf8_lossy(&output.stdout);
    let mut parts = version
        .split('.')
        .filter_map(|part| part.split('-').next()?.parse::<u32>().ok());
    matches!((parts.next(), parts.next()), (Some(major), Some(minor)) if major > 6 || (major == 6 && minor >= 15))
}
fn require_pidfd_exit() -> bool {
    if kernel_supports_pidfd_info_exit() {
        return true;
    }
    let version = Command::new("uname").arg("-r").output().unwrap();
    let kernel = String::from_utf8_lossy(&version.stdout).trim().to_owned();
    if ci::skip_unless(
        &format!(
            "t3_terminal_helpers_17_7: Linux >= 6.15 PIDFD_INFO_EXIT (kernel {kernel} predates it)"
        ),
        false,
    ) {
        return false;
    }
    panic!("PIDFD_INFO_EXIT requires Linux >= 6.15; CI kernel is {kernel}");
}
/// An owner shell that prints its grandchild's pid first; scripts that reap
/// the grandchild then print `reaped` (read by [`await_reaped`]).
fn external_child(
    script: &str,
) -> (
    std::process::Child,
    u32,
    std::io::BufReader<std::process::ChildStdout>,
) {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
    std::io::BufRead::read_line(&mut stdout, &mut line).unwrap();
    (child, line.trim().parse().unwrap(), stdout)
}
/// 18-0 AC4(d): barrier on the owner shell's reap acknowledgement. After it,
/// the grandchild is released, so its PIDFD_INFO_EXIT data is final; the 10 s
/// timeout is a hang guard only.
fn await_reaped(mut stdout: std::io::BufReader<std::process::ChildStdout>) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let read = std::io::BufRead::read_line(&mut stdout, &mut line).map(|_| line);
        let _ = tx.send(read);
    });
    let line = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("hang guard (10 s): the owner shell never acknowledged the reap")
        .unwrap();
    assert_eq!(line.trim(), "reaped", "the owner shell reaps, then says so");
}

#[test]
fn failed_init_releases_launch_permit_before_cid_guard_drop() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state");
    fs::create_dir(&state_dir).unwrap();
    fs::set_permissions(&state_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let private_cwd = fs::canonicalize(&state_dir).unwrap();
    let cid = "c".repeat(64);
    let (runtime, calls) = stub_runtime(
        temp.path(),
        &format!(
            r#"
case "$1" in
  create) printf '%s\n' '{cid}' > '{}/cid'; printf '%s\n' '{cid}' ;;
  init) pwd -P > '{}/init.cwd'; exit 1 ;;
  kill|rm) exit 0 ;;
  container) exit 1 ;;
esac
"#,
            state_dir.display(),
            temp.path().display()
        ),
    );
    let image = signed_test_image();
    let spec = SandboxSpec::new_for_test(SandboxTier::T3);
    let published = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let published_by_callback = Arc::clone(&published);
    let custody = ContainerLaunchCustody::new(state_dir.clone(), move |_| {
        published_by_callback.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    let started = Instant::now();
    let result = launch_t3(
        runtime,
        &spec,
        &image,
        &["true".into()],
        T3SpawnContext {
            spirit_binary_path: Path::new("/maos/test-spirit"),
            boot_nonce: 17,
            container_name: "failed-init".into(),
            egress_socket: None,
            state_dir: &state_dir,
            payload_pidfd: false,
            env: &[],
            launch_custody: Some(&custody),
        },
    );

    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(published.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(!state_dir.exists(), "failed init must run CID teardown");
    // Legacy conmon writes `oom` into the init client's cwd: it must be the private state dir.
    assert_eq!(
        fs::read_to_string(temp.path().join("init.cwd"))
            .unwrap()
            .trim_end(),
        private_cwd.to_str().unwrap()
    );
    assert_eq!(
        fs::read_to_string(&calls)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        vec!["create", "init", "rm", "container"]
    );
    assert_eq!(
        custody.cancel_and_teardown(Instant::now() + Duration::from_secs(3)),
        Ok(()),
        "root joins the already completed coordinator verdict"
    );
    assert_eq!(
        fs::read_to_string(calls)
            .unwrap()
            .lines()
            .filter(|call| *call == "rm")
            .count(),
        1,
        "joining cannot issue a duplicate removal"
    );
}

#[test]
fn incomplete_rm_stderr_cannot_certify_volume_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let (runtime, calls) = stub_runtime(
        temp.path(),
        r#"
case "$1" in
  kill) exit 0 ;;
  inspect) printf '{"Status":"exited","ExitCode":0,"OOMKilled":false,"Error":"","StartedAt":"zero","FinishedAt":"zero"}\n' ;;
  rm) head -c 70000 /dev/zero >&2; (sleep 0.4; printf 'volume cleanup failed\n' >&2) & exit 0 ;;
  container) exit 1 ;;
esac
"#,
    );
    let state_dir = temp.path().join("state");
    fs::create_dir(&state_dir).unwrap();
    let child = SandboxedContainerChild::adopt(runtime, "a".repeat(64), &state_dir);
    let verdict = child
        .terminal()
        .teardown(Instant::now() + Duration::from_secs(3));
    assert_eq!(verdict, Err(ContainerResidue::VolumeCleanupUnconfirmed));
    assert!(
        state_dir.exists(),
        "incomplete stderr must retain teardown evidence"
    );
    let log = fs::read_to_string(calls).unwrap();
    assert_eq!(log.lines().filter(|call| *call == "rm").count(), 1);
    assert!(log.lines().any(|call| call == "container"));
}

#[test]
fn teardown_join_honors_its_remaining_deadline_without_removing_during_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let (runtime, calls) = stub_runtime(
        temp.path(),
        r#"
case "$1" in
  kill) exit 0 ;;
  inspect) sleep 1.2; printf '{"Status":"exited","ExitCode":0,"OOMKilled":false,"Error":"","StartedAt":"zero","FinishedAt":"zero"}\n' ;;
  rm) printf '%s\n' rm >> "$LOG"; exit 0 ;;
  container) exit 1 ;;
esac
"#,
    );
    let state_dir = temp.path().join("state");
    fs::create_dir(&state_dir).unwrap();
    let child = SandboxedContainerChild::adopt(runtime, "b".repeat(64), &state_dir);
    let terminal = Arc::clone(child.terminal());
    let observer = {
        let terminal = Arc::clone(&terminal);
        std::thread::spawn(move || terminal.facts())
    };
    let until = Instant::now() + Duration::from_secs(2);
    while !fs::read_to_string(&calls)
        .unwrap_or_default()
        .lines()
        .any(|call| call == "inspect")
    {
        assert!(
            Instant::now() < until,
            "observer did not enter runtime inspect"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    let start = Instant::now();
    let verdict = terminal.teardown(start + Duration::from_millis(300));
    assert_eq!(verdict, Err(ContainerResidue::TerminationUnconfirmed));
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(
        !fs::read_to_string(&calls)
            .unwrap_or_default()
            .lines()
            .any(|call| call == "rm"),
        "CID removal must not race the in-flight snapshot initializer"
    );

    let facts = observer.join().unwrap();
    assert_eq!(facts.state.as_ref().unwrap().status, "exited");
    let same_facts = terminal.facts();
    assert!(
        Arc::ptr_eq(&facts, &same_facts),
        "snapshot evidence stays immutable"
    );
    assert_eq!(
        terminal.teardown(Instant::now() + Duration::from_secs(3)),
        Err(ContainerResidue::TerminationUnconfirmed),
        "followers receive the one frozen teardown verdict"
    );
    let log = fs::read_to_string(calls).unwrap();
    assert_eq!(log.lines().filter(|call| *call == "kill").count(), 1);
    assert!(!log.lines().any(|call| call == "rm"));
}

#[test]
fn pidfd_reports_non_child_exit_status_signals_and_live_absence() {
    if !require_pidfd_exit() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let release = dir.path().join("release");
    // A grandchild (never our child) held until its pidfd is open.
    let (mut owner, pid, stdout) = external_child(&format!(
        "(while [ ! -e '{}' ]; do sleep 0.01; done; exit 137) & p=$!; echo $p; wait $p; echo reaped",
        release.display()
    ));
    let fd = linux::pidfd_open(pid).unwrap();
    std::fs::write(&release, b"").unwrap();
    await_reaped(stdout);
    assert!(linux::readable(fd.as_fd(), Duration::from_secs(5)));
    // ONE read after the barrier: any error, ESRCH included, is a failure.
    assert_eq!(
        linux::pidfd_exit_status(fd.as_fd()).unwrap(),
        Some(0x8900),
        "exit(137) is Exited(137)"
    );
    let _ = owner.wait();
    let (mut owner, pid, stdout) = external_child("sleep 30 & p=$!; echo $p; wait $p; echo reaped");
    let fd = linux::pidfd_open(pid).unwrap();
    Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status()
        .unwrap();
    await_reaped(stdout);
    assert!(linux::readable(fd.as_fd(), Duration::from_secs(5)));
    assert_eq!(
        linux::pidfd_exit_status(fd.as_fd()).unwrap(),
        Some(9),
        "an external SIGKILL is Signaled(9)"
    );
    let _ = owner.wait();
    let mut live = Command::new("sleep").arg("30").spawn().unwrap();
    let fd = linux::pidfd_open(live.id()).unwrap();
    assert!(!linux::readable(fd.as_fd(), Duration::from_millis(20)));
    assert_eq!(linux::pidfd_exit_status(fd.as_fd()).unwrap(), None);
    live.kill().unwrap();
    let _ = live.wait();
}
