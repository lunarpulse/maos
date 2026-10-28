//! Stub-runtime ownership and refusal scenes for Story 17-7.
use super::*;
use maos_domain::{
    invariants::i9::SandboxTier,
    ports::crypto::CryptoProvider,
    sandbox::{ImageAttestationId, T3ImageAttestation, T3ImageEntry},
};
use maos_kernel_core::security::{
    crypto::RingCryptoProvider,
    sandbox::{
        t3::{
            image_lock::{load_and_verify_lock_at, VerifiedImageAttestation},
            runtime_detect::ContainerRuntime,
            spawn::{launch_t3, T3SpawnContext},
        },
        SandboxSpec,
    },
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::os::unix::fs::PermissionsExt;

const BEARER: &str = "distinctive-test-bearer-not-for-logs";
const FOREIGN: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

pub(super) fn podman(dir: &Path, mode: &str) -> ContainerRuntime {
    let path = dir.join("stub-podman");
    let log = dir.join("calls");
    let pid = dir.join("state").join("pid");
    let script = format!(
        r#"#!/bin/sh
printf 'argv=' >> '{log}'; for arg do printf '<%s>' "$arg" >> '{log}'; done
printf ' env=' >> '{log}'; env | while IFS='=' read -r k v; do printf '<%s>' "$k"; done >> '{log}'; printf '\n' >> '{log}'
mode='{mode}'; case "$1" in
create) cid=; for arg do case "$arg" in --cidfile=*) cid=${{arg#--cidfile=}};; esac; done; case "$mode" in
 receipt-hang) printf '%s\n' '{CID}' > "$cid"; sleep 30;; no-receipt) printf '%s\n' '{FOREIGN}';; malformed) printf 'bad\n' > "$cid"; printf '%s\n' '{FOREIGN}';; disagree) printf '%s\n' '{CID}' > "$cid"; printf '%s\n' '{FOREIGN}';; create-fails-bearer) printf '%s\n' "$WORKER_SECRET" >&1; printf '%s\n' "$WORKER_SECRET" >&2; printf '%s\n' '{CID}' > "$cid"; exit 1;; bearer-create) printf '%s\n' "$WORKER_SECRET" >&2; printf '%s\n' '{CID}' > "$cid"; printf '%s\n' '{CID}';; *) printf '%s\n' '{CID}' > "$cid"; printf '%s\n' '{CID}';; esac;;
init) case "$mode" in pid-missing) :;; pid-symlink) ln -s x '{pid}';; pid-fifo) mkfifo '{pid}';; pid-nonnumeric) printf 'abc\n' > '{pid}';; pid-zero) printf '0\n' > '{pid}';; pid-oversized) printf '12345678901234567890\n' > '{pid}';; inspect-fail|marker-inspect-fail|marker-nonterminal|marker-clean|kill-hang-rm-fail-inspect-fail|fallback-137|normal|bearer-create|create-fails-bearer|rm-fail|kill-hang-rm-fail) printf '%s\n' "$$" > '{pid}';; *) sleep 30 & printf '%s\n' "$!" > '{pid}';; esac;;
start) case "$mode" in marker-inspect-fail|marker-nonterminal|marker-clean) printf 'worker: task complete\n';; esac; exit 0;; kill) case "$mode" in kill-hang|kill-hang-rm-fail|kill-hang-rm-fail-inspect-fail) sleep 30;; kill-fail) exit 1;; esac;;
inspect) case "$mode" in marker-nonterminal) printf '{{"Status":"running","ExitCode":0,"OOMKilled":false,"Error":"","StartedAt":"2026-10-15T00:00:00Z","FinishedAt":"0001-01-01T00:00:00Z"}}\n';; inspect-fail|marker-inspect-fail|kill-hang-rm-fail-inspect-fail) exit 1;; bearer-create|create-fails-bearer) printf '{{"Status":"exited","ExitCode":0,"OOMKilled":false,"Error":"{BEARER}","StartedAt":"0001-01-01T00:00:00Z","FinishedAt":"0001-01-01T00:00:00Z"}}\n';; fallback-137) printf '{{"Status":"exited","ExitCode":137,"OOMKilled":false,"Error":"","StartedAt":"0001-01-01T00:00:00Z","FinishedAt":"0001-01-01T00:00:00Z"}}\n';; *) printf '{{"Status":"exited","ExitCode":0,"OOMKilled":false,"Error":"%s","StartedAt":"0001-01-01T00:00:00Z","FinishedAt":"0001-01-01T00:00:00Z"}}\n' "${{WORKER_SECRET:-}}";; esac;;
rm) case "$mode" in rm-hang) sleep 30;; rm-fail|kill-hang-rm-fail|kill-hang-rm-fail-inspect-fail) exit 1;; rm-bearer|bearer-create|create-fails-bearer) printf '%s\n' '{BEARER}' >&2;; esac;;
container) case "$mode" in exists-hang) sleep 30;; exists-fail) exit 2;; exists-present) exit 0;; *) exit 1;; esac;; *) exit 64;; esac
"#,
        log = log.display(),
        mode = mode,
        pid = pid.display(),
        CID = CID,
        FOREIGN = FOREIGN
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    ContainerRuntime {
        kind: ContainerRuntimeKind::Podman,
        path,
        version: "stub".into(),
    }
}
pub(super) fn image() -> VerifiedImageAttestation {
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
    std::fs::write(&lock, serde_json::to_vec(&vec![attestation]).unwrap()).unwrap();
    load_and_verify_lock_at(&lock, &public, &RingCryptoProvider)
        .unwrap()
        .default_entry()
        .unwrap()
}
pub(super) fn state(dir: &Path) -> PathBuf {
    let p = dir.join("state");
    std::fs::create_dir(&p).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
    p
}
pub(super) fn launch(
    dir: &Path,
    mode: &str,
    st: &Path,
    env: &[(String, String)],
) -> Result<SandboxedContainerChild, maos_kernel_core::security::sandbox::SpawnError> {
    let binary = dir.join("spirit");
    std::fs::write(&binary, b"stub").unwrap();
    let spec = SandboxSpec::new_for_test(SandboxTier::T3);
    let verified = image();
    launch_t3(
        podman(dir, mode),
        &spec,
        &verified,
        &["true".into()],
        T3SpawnContext {
            spirit_binary_path: &binary,
            boot_nonce: 1,
            container_name: "stub-worker".into(),
            egress_socket: None,
            state_dir: st,
            payload_pidfd: false,
            env,
            launch_custody: None,
            workspace: None,
        },
    )
}
pub(super) fn calls(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("calls")).unwrap_or_default()
}

#[test]
fn ownership_receipt_authority_refuses_foreign_and_disagreement() {
    for (mode, text) in [
        ("no-receipt", "creation ownership unknown"),
        ("malformed", "creation ownership unknown"),
        ("disagree", "launch refused at create"),
    ] {
        let dir = scratch("ownership");
        let st = state(&dir);
        let error = launch(&dir, mode, &st, &[])
            .map_err(|e| e.to_string())
            .err()
            .unwrap();
        assert!(error.contains(text), "{error}");
        if mode == "disagree" {
            assert!(
                !st.exists(),
                "successfully removed owned container should release state"
            );
        } else {
            assert!(
                st.exists(),
                "unknown ownership must retain its state directory"
            );
        }
        let log = calls(&dir);
        assert!(!log.contains(FOREIGN), "foreign CID touched: {log}");
        assert!(!log.contains("<init>"));
        if mode == "disagree" {
            assert!(log.contains("<rm>"));
            assert!(log.contains(CID));
        } else {
            assert!(!log.contains("<rm>"));
        }
    }
}
#[test]
fn launch_refuses_nonempty_and_wrong_mode_state_before_runtime() {
    for wrong_mode in [false, true] {
        let dir = scratch("state-refusal");
        let st = state(&dir);
        if wrong_mode {
            std::fs::set_permissions(&st, std::fs::Permissions::from_mode(0o755)).unwrap();
        } else {
            std::fs::write(st.join("occupied"), b"x").unwrap();
        }
        let result = launch(&dir, "normal", &st, &[]);
        assert!(result.is_err());
        assert!(calls(&dir).is_empty());
    }
}
#[test]
fn init_pidfile_invalid_forms_refuse_and_remove_only_receipt_cid() {
    for mode in [
        "pid-missing",
        "pid-symlink",
        "pid-fifo",
        "pid-nonnumeric",
        "pid-zero",
        "pid-oversized",
    ] {
        let dir = scratch("pidfile-refusal");
        let st = state(&dir);
        let error = launch(&dir, mode, &st, &[])
            .map_err(|e| e.to_string())
            .err()
            .unwrap();
        assert!(
            error.contains("launch refused at init") && error.contains(CID),
            "{error}"
        );
        let log = calls(&dir);
        assert!(log.contains("<init>") && log.contains("<rm>") && !log.contains(FOREIGN));
    }
}
#[test]
fn ownership_create_timeout_cleans_the_receipt_cid() {
    let dir = scratch("create-timeout");
    let st = state(&dir);
    let started = Instant::now();
    let error = launch(&dir, "receipt-hang", &st, &[])
        .map_err(|e| e.to_string())
        .err()
        .unwrap();
    assert!(error.contains("launch refused at create"), "{error}");
    assert!(started.elapsed() <= Duration::from_secs(12));
    let log = calls(&dir);
    assert!(log.contains("<rm>") && log.contains(CID));
    assert!(!log.contains(FOREIGN));
}

#[test]
fn bearer_create_diagnostic_and_runtime_arguments_do_not_echo_secret() {
    const CHILD_FLAG: &str = "MAOS_17_7_BEARER_CHILD";
    if std::env::var_os(CHILD_FLAG).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "stub_scenes::bearer_create_diagnostic_and_runtime_arguments_do_not_echo_secret",
                "--nocapture",
            ])
            .env(CHILD_FLAG, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains(BEARER));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(BEARER));
        return;
    }
    let dir = scratch("bearer-redaction");
    let refusal_dir = dir.join("refusal");
    std::fs::create_dir(&refusal_dir).unwrap();
    let refusal_state = state(&refusal_dir);
    let env = vec![("WORKER_SECRET".to_owned(), BEARER.to_owned())];
    let refusal = match launch(&refusal_dir, "create-fails-bearer", &refusal_state, &env) {
        Ok(_) => panic!("failed create must refuse launch"),
        Err(error) => error.to_string(),
    };
    assert!(!refusal.contains(BEARER), "create refusal is redacted");
    let refusal_log = calls(&refusal_dir);
    assert!(refusal_log.contains("<rm>") && refusal_log.contains(CID));
    assert!(!refusal_log.contains(FOREIGN));

    let st = state(&dir);
    let child = launch(&dir, "bearer-create", &st, &env).unwrap();
    let facts = child.terminal().facts();
    assert!(format!("{facts:?}").contains("error_reported: true"));
    assert!(!format!("{facts:?}").contains(BEARER));
    let verdict = child.terminal().teardown(Instant::now() + TEARDOWN_BUDGET);
    assert_eq!(verdict, Ok(()));
    let log = calls(&dir);
    assert!(log.contains("<rm>") && log.contains(CID));
    assert!(!log.contains(BEARER));
    assert!(!log.contains("WORKER_SECRET="));
}
#[test]
fn deadlines_kill_is_bounded_and_verdict_is_shared() {
    let dir = scratch("kill-deadline");
    let st = state(&dir);
    let child = SandboxedContainerChild::adopt(podman(&dir, "kill-hang"), CID.to_owned(), &st);
    let terminal = child.terminal().clone();
    let start = Instant::now();
    let one = terminal.teardown(start + TEARDOWN_BUDGET);
    let elapsed = start.elapsed();
    let two = terminal.teardown(Instant::now() + TEARDOWN_BUDGET);
    assert_eq!(one, two);
    assert!(
        elapsed <= KILL_BUDGET + Duration::from_millis(1500),
        "{elapsed:?}"
    );
    assert!(calls(&dir).contains("<kill>"));
}
#[test]
fn deadlines_remove_failure_is_typed() {
    let dir = scratch("rm-failure");
    let st = state(&dir);
    let child = SandboxedContainerChild::adopt(podman(&dir, "rm-fail"), CID.to_owned(), &st);
    assert_eq!(
        child.terminal().teardown(Instant::now() + TEARDOWN_BUDGET),
        Err(maos_domain::sandbox::ContainerResidue::RemoveFailed)
    );
}
#[test]
fn deadlines_hung_rm_is_bounded() {
    let dir = scratch("rm-timeout");
    let st = state(&dir);
    let child = SandboxedContainerChild::adopt(podman(&dir, "rm-hang"), CID.to_owned(), &st);
    let terminal = child.terminal().clone();
    let started = Instant::now();
    let verdict = terminal.teardown(started + TEARDOWN_BUDGET);
    assert_eq!(
        verdict,
        Err(maos_domain::sandbox::ContainerResidue::RemoveTimeout)
    );
    assert!(started.elapsed() <= REMOVE_BUDGET + Duration::from_millis(1500));
}

#[test]
fn deadlines_hung_exists_is_typed_and_bounded() {
    let dir = scratch("exists-timeout");
    let st = state(&dir);
    let child = SandboxedContainerChild::adopt(podman(&dir, "exists-hang"), CID.to_owned(), &st);
    let terminal = child.terminal().clone();
    let started = Instant::now();
    let verdict = terminal.teardown(started + TEARDOWN_BUDGET);
    assert_eq!(
        verdict,
        Err(maos_domain::sandbox::ContainerResidue::ExistsFailed)
    );
    assert!(started.elapsed() <= TEARDOWN_BUDGET + Duration::from_millis(1500));
}

#[test]
fn concurrent_facts_and_teardown_share_snapshot_and_verdict() {
    let dir = scratch("facts-race");
    let st = state(&dir);
    let child = SandboxedContainerChild::adopt(podman(&dir, "normal"), CID.to_owned(), &st);
    let terminal = child.terminal().clone();
    let mut threads = Vec::new();
    for _ in 0..3 {
        let terminal = terminal.clone();
        threads.push(std::thread::spawn(move || {
            let facts = terminal.facts();
            let verdict = terminal.teardown(Instant::now() + TEARDOWN_BUDGET);
            (facts, verdict)
        }));
    }
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(std::sync::Arc::ptr_eq(&results[0].0, &results[1].0));
    assert!(std::sync::Arc::ptr_eq(&results[0].0, &results[2].0));
    assert!(results.iter().all(|r| r.1 == results[0].1));
    assert_eq!(calls(&dir).matches("<rm>").count(), 1);
}
