#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 17-2 T3 proof: real `maos run`, signed rootless image, live OCI
//! inspection, and evidence from the actual `/maos/spirit` payload process.

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
const NONCE: u64 = 0x17_02;
const RUN_BOUND: Duration = Duration::from_secs(35);

struct Root {
    dir: PathBuf,
    manifest: PathBuf,
    tag: String,
}

impl Root {
    fn new(label: &str, fd_max: u32) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "maos-worker-resource-17-2-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("home")).expect("create isolated Worker home");
        let manifest = dir.join("worker.toml");
        let tag = format!(
            "--maos-fixture-tag={}",
            dir.file_name().unwrap().to_string_lossy()
        );
        std::fs::write(
            &manifest,
            format!(
                "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=resource-probe\", \"{tag}\"]\noutput_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\nrecovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n[resources]\ncpu_max_pct = 10\nmemory_max_mb = 64\npids_max = 12\nfd_max = {fd_max}\n\n[author]\nname = \"MAOS Project\"\n"
            ),
        )
        .expect("write real T3 Worker manifest");
        Self { dir, manifest, tag }
    }

    fn command(&self) -> Command {
        let workspace = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let runtime_home = self.dir.join("home");
        let home = t3_substrate::isolated_runtime_home(&runtime_home);
        let mut paths = vec![
            workspace.join("target/debug"),
            workspace.join("target/debug/deps"),
        ];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
        command
            .env_clear()
            .current_dir(workspace)
            .arg("run")
            .arg(&self.manifest)
            .arg("--once")
            .env("HOME", home)
            .env("MAOS_HOME", self.dir.join("home"))
            .env("XDG_DATA_HOME", self.dir.join("home"))
            .env("PATH", std::env::join_paths(paths).expect("valid PATH"))
            .env("MAOS_OLLAMA_URL", "skip")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for key in [
            "MAOS_T3_RUNTIME",
            "MAOS_T3_IMAGE_LOCK_PATH",
            "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
    }

    fn owns_container(&self, details: &Value) -> bool {
        details["Config"]["Cmd"].as_array().is_some_and(|args| {
            args.iter()
                .any(|arg| arg.as_str() == Some(self.tag.as_str()))
        })
    }

    fn runtime(&self) -> String {
        std::env::var("MAOS_T3_RUNTIME").expect("declared T3 substrate runtime")
    }

    /// Every runtime-client call is bounded, including panic-time teardown.
    fn runtime_command(&self) -> Command {
        let mut command = Command::new("timeout");
        command.arg("8s").arg(self.runtime());
        command.env(
            "HOME",
            t3_substrate::isolated_runtime_home(&self.dir.join("home")),
        );
        for key in ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
    }

    fn output_rows(&self) -> Vec<(String, Value)> {
        let db = self.dir.join("home/audit/transparency.sqlite");
        if !db.exists() {
            return Vec::new();
        }
        TransparencyLogAdapter::open(&db, NONCE)
            .expect("reopen real run Transparency Log")
            .query_frames(FrameFilter {
                kind: Some(FrameKind::CliSubprocessOutput),
                ..Default::default()
            })
            .expect("read persisted real Worker output")
            .into_iter()
            .filter(|row| row.spirit_pid != 0 && row.intent == "cli.subprocess.output")
            .filter_map(|row| {
                Some((
                    row.intent,
                    serde_json::from_slice(&row.payload_redacted).ok()?,
                ))
            })
            .collect()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        // A failed assertion still must not strand a test-owned container.
        // Normal completion first asserts that production `--rm` removed it.
        let ids = self
            .runtime_command()
            .args(["ps", "-a", "--format", "{{.ID}}"])
            .output();
        if let Ok(ids) = ids {
            if ids.status.success() {
                for id in String::from_utf8_lossy(&ids.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                {
                    let Some(details) = inspect(self, id) else {
                        continue;
                    };
                    if !self.owns_container(&details) {
                        continue;
                    }
                    let _ = self
                        .runtime_command()
                        .args(["stop", "--time", "2", id])
                        .output();
                    let _ = self.runtime_command().args(["rm", "--force", id]).output();
                }
            }
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn build_fixture() {
    fixtures::profile_binary("worker-cli-fixture");
}

struct OwnedChild(Option<Child>);

impl OwnedChild {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn id(&self) -> u32 {
        self.0.as_ref().expect("owned subprocess").id()
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.0.as_mut().expect("owned subprocess").try_wait()
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn wait_bounded(mut child: OwnedChild, bound: Duration) -> Output {
    let deadline = Instant::now() + bound;
    loop {
        if child.try_wait().expect("poll maos run").is_some() {
            return child
                .0
                .take()
                .expect("owned subprocess")
                .wait_with_output()
                .expect("collect maos run output");
        }
        assert!(
            Instant::now() < deadline,
            "maos run exceeded its bounded deadline"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn runtime_ids(root: &Root) -> BTreeSet<String> {
    let output = root
        .runtime_command()
        .args(["ps", "-a", "--format", "{{.ID}}"])
        .output()
        .expect("list actual OCI containers");
    assert!(
        output.status.success(),
        "runtime ps failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn inspect(root: &Root, id: &str) -> Option<Value> {
    let output = root.runtime_command().args(["inspect", id]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value: Value = serde_json::from_slice(&output.stdout).ok()?;
    value.as_array()?.first().cloned()
}

fn find_resource_container(root: &Root, before: &BTreeSet<String>) -> Option<(String, Value)> {
    for id in runtime_ids(root).difference(before) {
        if let Some(value) = inspect(root, id) {
            // Retained create/init makes an owned container visible before
            // its payload runs. Keep polling under the caller's deadline.
            if root.owns_container(&value) && value["State"]["Running"].as_bool() == Some(true) {
                return Some((id.clone(), value));
            }
        }
    }
    None
}

fn number(value: &Value, key: &str) -> u64 {
    value[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} missing/not numeric in OCI inspect: {value}"))
}

fn payload_pid(inspect: &Value) -> u32 {
    inspect["State"]["Pid"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid > 0)
        .expect("OCI runtime inspect reports a live host PID for the container init/payload")
}

fn actual_payload(pid: u32) -> PathBuf {
    let path = PathBuf::from(format!("/proc/{pid}"));
    assert!(
        path.join("status").exists(),
        "inspected payload PID {pid} vanished before live inspection"
    );
    // Story 17-7: `init` publishes State.Pid before `start` releases the
    // runtime's exec fifo, so the PID is briefly still `crun create`.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let cmdline = loop {
        let cmdline = std::fs::read(path.join("cmdline")).expect("read actual payload cmdline");
        let cmdline = String::from_utf8_lossy(&cmdline).into_owned();
        if cmdline.contains("/maos/spirit") || std::time::Instant::now() >= deadline {
            break cmdline;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert!(
        cmdline.contains("/maos/spirit"),
        "State.Pid is not the actual Worker payload: {cmdline:?}"
    );
    path
}

fn cgroup_counter(proc: &PathBuf, pid: u32, file: &str, key: &str) -> Option<u64> {
    let mut candidates = vec![proc.join("root/sys/fs/cgroup").join(file)];
    if let Ok(membership) = std::fs::read_to_string(proc.join("cgroup")) {
        if let Some(relative) = membership.lines().find_map(|line| line.strip_prefix("0::")) {
            candidates.push(
                PathBuf::from("/sys/fs/cgroup")
                    .join(relative.trim_start_matches('/'))
                    .join(file),
            );
            candidates.push(
                PathBuf::from(format!("/proc/{pid}/root/sys/fs/cgroup"))
                    .join(relative.trim_start_matches('/'))
                    .join(file),
            );
        }
    }
    candidates.into_iter().find_map(|path| {
        std::fs::read_to_string(path)
            .ok()?
            .lines()
            .find_map(|line| {
                let mut fields = line.split_whitespace();
                (fields.next()? == key)
                    .then(|| fields.next()?.parse().ok())
                    .flatten()
            })
    })
}

fn cgroup_limit(proc: &PathBuf, pid: u32, file: &str) -> Option<String> {
    let membership = std::fs::read_to_string(proc.join("cgroup")).ok()?;
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?
        .trim_start_matches('/');
    [
        proc.join("root/sys/fs/cgroup").join(file),
        PathBuf::from("/sys/fs/cgroup").join(relative).join(file),
        PathBuf::from(format!("/proc/{pid}/root/sys/fs/cgroup"))
            .join(relative)
            .join(file),
    ]
    .into_iter()
    .find_map(|path| std::fs::read_to_string(path).ok())
    .map(|value| value.trim().to_owned())
}

#[test]
fn real_maos_worker_applies_independent_t3_limits_and_observes_exhaustion() {
    if !t3_substrate::require_t3("17-2 live Worker resource enforcement") {
        return;
    }
    build_fixture();
    let root = Root::new("applied", 64);
    let before = runtime_ids(&root);
    let mut child = OwnedChild::new(root.command().spawn().expect("spawn real maos run Worker"));
    let deadline = Instant::now() + RUN_BOUND;
    let (container_id, details) = loop {
        if let Some(found) = find_resource_container(&root, &before) {
            break found;
        }
        if let Some(status) = child
            .try_wait()
            .expect("poll maos run while locating live Worker")
        {
            panic!("maos run exited before a resource-probe OCI container appeared ({status})");
        }
        assert!(
            Instant::now() < deadline,
            "no live resource-probe OCI container appeared"
        );
        std::thread::sleep(Duration::from_millis(75));
    };

    // Read the actual running container's configuration before production
    // explicitly tears down and removes its owned CID.
    let host = &details["HostConfig"];
    assert_eq!(
        number(host, "Memory"),
        64 * 1024 * 1024,
        "actual OCI memory limit"
    );
    assert_eq!(
        number(host, "PidsLimit"),
        12,
        "PID cap must not alias FD cap"
    );
    let nano_cpus = host["NanoCpus"].as_u64();
    let quota = host["CpuQuota"].as_i64();
    let period = host["CpuPeriod"].as_i64();
    assert!(
        nano_cpus == Some(100_000_000) || (quota == Some(10_000) && period == Some(100_000)),
        "actual OCI CPU ceiling is not exactly 0.10 cores: {host}"
    );
    let ulimits = host["Ulimits"]
        .as_array()
        .expect("OCI inspect includes ulimits");
    assert!(
        ulimits.iter().any(|limit| {
            (limit["Name"] == "RLIMIT_NOFILE" || limit["Name"] == "nofile")
                && limit["Soft"].as_u64() == Some(64)
                && limit["Hard"].as_u64() == Some(64)
        }),
        "FD limit must independently be nofile 64: {host}"
    );

    let image = details["Config"]["Image"]
        .as_str()
        .expect("inspected signed image ref");
    assert!(
        image.contains("@sha256:"),
        "runtime did not run the verified digest image: {image}"
    );
    assert_eq!(
        host["NetworkMode"], "none",
        "the Worker keeps its denied-egress boundary"
    );
    assert!(
        details["Mounts"].as_array().is_some_and(|mounts| {
            mounts
                .iter()
                .any(|mount| mount["Destination"] == "/proxy/egress.sock")
        }),
        "the scoped egress proxy socket remains the Worker's only network path"
    );
    let pid = payload_pid(&details);
    let proc = actual_payload(pid);
    let limits = std::fs::read_to_string(proc.join("limits")).expect("read actual payload rlimits");
    let nofile = limits
        .lines()
        .find(|line| line.starts_with("Max open files"))
        .expect("actual payload Max open files");
    let nofile_fields: Vec<_> = nofile.split_whitespace().collect();
    assert_eq!(
        nofile_fields.get(3),
        Some(&"64"),
        "actual payload soft nofile is 64: {nofile}"
    );
    assert_eq!(
        nofile_fields.get(4),
        Some(&"64"),
        "actual payload hard nofile is 64: {nofile}"
    );
    let cgroup = std::fs::read_to_string(proc.join("cgroup"))
        .expect("read actual payload cgroup membership");
    assert!(
        cgroup.lines().any(|line| line.starts_with("0::")),
        "payload is not in cgroup v2: {cgroup}"
    );
    assert_eq!(
        cgroup_limit(&proc, pid, "memory.max").as_deref(),
        Some("67108864"),
        "actual payload cgroup memory.max"
    );
    assert_eq!(
        cgroup_limit(&proc, pid, "pids.max").as_deref(),
        Some("12"),
        "actual payload cgroup pids.max"
    );
    assert_eq!(
        cgroup_limit(&proc, pid, "cpu.max").as_deref(),
        Some("10000 100000"),
        "actual payload cgroup cpu.max"
    );

    // The live task cap must actually reject creation, and the CPU-bound
    // fixture must accrue throttling under the inspected 10% quota.
    let stats_deadline = Instant::now() + Duration::from_secs(5);
    let (mut pids_max, mut throttled_usec) = (None, None);
    while Instant::now() < stats_deadline {
        pids_max = cgroup_counter(&proc, pid, "pids.events", "max");
        throttled_usec = cgroup_counter(&proc, pid, "cpu.stat", "throttled_usec");
        if pids_max.unwrap_or(0) > 0 && throttled_usec.unwrap_or(0) > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        pids_max.unwrap_or(0) > 0,
        "actual pids.events did not record an EAGAIN task-cap event"
    );
    assert!(
        throttled_usec.unwrap_or(0) > 0,
        "actual cpu.stat did not record throttling at 10% CPU"
    );

    let output = wait_bounded(child, RUN_BOUND);
    assert!(
        output.status.success(),
        "real maos run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Read the fixture's structured report from the production Worker output
    // protocol after its completion marker has been persisted.
    let rows = root.output_rows();
    // Like the existing T3 namespace proof, the actual payload reads its own
    // namespace links; host ptrace permissions need not be widened for tests.
    for namespace in ["pid", "net", "user"] {
        let prefix = format!("worker: namespace {namespace}=");
        let payload_ns = rows
            .iter()
            .find_map(|(_, value)| value["line"].as_str()?.strip_prefix(&prefix))
            .expect("persisted actual payload namespace");
        let host_ns = std::fs::read_link(format!("/proc/self/ns/{namespace}")).unwrap();
        assert_ne!(
            payload_ns,
            host_ns.to_string_lossy(),
            "Worker must isolate its {namespace} namespace"
        );
    }
    let report = rows
        .iter()
        .find_map(|(_, value)| {
            let text = value.to_string();
            if text.contains("resource probe") {
                Some(value.clone())
            } else {
                None
            }
        })
        .expect("persisted actual Worker output includes its bounded exhaustion report");
    let report_text = report.to_string();
    assert!(
        report_text.contains("TooManyOpenFiles") || report_text.contains(":24"),
        "FD exhaustion did not report EMFILE: {report_text}"
    );
    assert!(
        report_text.contains("WouldBlock") || report_text.contains(":11"),
        "bounded thread creation did not hit EAGAIN: {report_text}"
    );
    assert!(
        rows.iter()
            .any(|(_, value)| value.to_string().contains("worker: task complete")),
        "existing Worker completion marker was not persisted"
    );

    // Existing Worker ownership removes its own OCI container; no test-side
    // runtime stop/remove is allowed to mask a lifecycle leak.
    let after = runtime_ids(&root);
    assert!(
        !after.contains(&container_id),
        "production lifecycle removal left completed Worker container {container_id}"
    );
}

fn verify_actual_runtime_fd_boundary(root: &Root) {
    use maos_domain::invariants::i9::SandboxTier;
    use maos_kernel_core::security::{
        crypto::RingCryptoProvider,
        sandbox::{
            t3::{
                argv::build_runtime_argv, image_lock::load_and_verify_lock,
                image_verify::read_trust_anchor_pub, runtime_detect::detect_container_runtime,
            },
            SandboxSpec,
        },
    };
    let anchor = read_trust_anchor_pub().unwrap();
    let image = load_and_verify_lock(&anchor, &RingCryptoProvider)
        .unwrap()
        .default_entry()
        .unwrap();
    let runtime = detect_container_runtime().unwrap();
    let fixture = std::path::Path::new(env!("CARGO_BIN_EXE_maos"))
        .parent()
        .unwrap()
        .join("worker-cli-fixture");
    let home_path = root.dir.join("home");
    let home = t3_substrate::isolated_runtime_home(&home_path);
    let guest = vec![
        "--maos-worker".into(),
        "--maos-fixture-mode=resource-probe".into(),
        root.tag.clone(),
    ];
    // Pre-PID startup failure does not expose runtime stderr through the
    // bridge. Matched real-runtime controls establish the actual FD cause.
    for limit in [64, 65] {
        let mut spec = SandboxSpec::new_for_test(SandboxTier::T3);
        spec.resolved_caps = maos_manifest::ResolvedCaps {
            cpu_max_pct: Some(10),
            memory_max_mb: Some(64),
            pids_max: Some(12),
            fd_max: Some(limit),
        };
        let name = format!(
            "fd-canary-{}-{limit}",
            root.dir.file_name().unwrap().to_string_lossy()
        );
        let state_dir = root.dir.join(format!("fd-canary-{limit}-state"));
        std::fs::create_dir(&state_dir).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&state_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let argv = build_runtime_argv(
            &runtime,
            &image,
            &spec,
            &fixture,
            &guest,
            "fd-boundary",
            NONCE,
            &name,
            &state_dir,
            None,
            None,
            &[],
        )
        .unwrap();
        // Story 17-7: the argv is `create`; the same hard-limit context then
        // starts the owned CID (conmon/OCI inherit this client's limits) and
        // removes it, so the matched control still exercises the real runtime.
        let lifecycle = r#"cid=$("$@") || exit $?
"$1" start --attach "$cid"; rc=$?
timeout 8s "$1" rm --force --volumes --time=0 "$cid" >/dev/null 2>&1
exit $rc"#;
        let mut command = Command::new("prlimit");
        command
            .args(["--nofile=64:64", "--", "sh", "-c", lifecycle, "sh"])
            .args(&argv)
            .env("HOME", home)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = wait_bounded(OwnedChild::new(command.spawn().unwrap()), RUN_BOUND);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if limit == 64 {
            assert!(
                output.status.success(),
                "same hard-limit context must admit nofile64: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                stdout.contains("worker: task complete"),
                "matched finite guest must complete"
            );
            assert!(
                stdout.contains("TooManyOpenFiles:24"),
                "matched guest must observe its FD cap"
            );
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
            assert!(
                !output.status.success(),
                "actual runtime must refuse nofile65 above hard64"
            );
            assert!(
                error.contains("nofile") || error.contains("rlimit"),
                "actual FD denial must identify the limit: {error}"
            );
            assert!(
                !stdout.contains("worker: namespace") && !stdout.contains("resource probe"),
                "impossible FD cap executed guest code"
            );
        }
    }
}

#[test]
fn fd_request_above_inherited_hard_limit_refuses_before_worker_payload() {
    if !t3_substrate::require_t3("17-2 Worker FD hard-limit refusal") {
        return;
    }
    build_fixture();
    let root = Root::new("hard-limit", 65);
    verify_actual_runtime_fd_boundary(&root);
    let before = runtime_ids(&root);
    let workspace = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let mut paths = vec![
        workspace.join("target/debug"),
        workspace.join("target/debug/deps"),
    ];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    let mut command = Command::new("prlimit");
    command
        .arg("--nofile=64:64")
        .arg("--")
        .arg(env!("CARGO_BIN_EXE_maos"))
        .arg("run")
        .arg(&root.manifest)
        .arg("--once")
        .env_clear()
        .current_dir(&workspace)
        .env(
            "HOME",
            t3_substrate::isolated_runtime_home(&root.dir.join("home")),
        )
        .env("MAOS_HOME", root.dir.join("home"))
        .env("XDG_DATA_HOME", root.dir.join("home"))
        .env("MAOS_OLLAMA_URL", "skip")
        .env("PATH", std::env::join_paths(paths).expect("valid PATH"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for key in [
        "MAOS_T3_RUNTIME",
        "MAOS_T3_IMAGE_LOCK_PATH",
        "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let child = OwnedChild::new(
        command
            .spawn()
            .expect("launch maos in actual RLIMIT_NOFILE=64 subprocess context"),
    );
    let limit_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let limits = std::fs::read_to_string(format!("/proc/{}/limits", child.id()))
            .expect("read inherited prlimit child context");
        let line = limits
            .lines()
            .find(|line| line.starts_with("Max open files"))
            .expect("inherited hard-limit line");
        let values: Vec<_> = line.split_whitespace().collect();
        if values.get(3) == Some(&"64") && values.get(4) == Some(&"64") {
            break;
        }
        assert!(
            Instant::now() < limit_deadline,
            "prlimit did not establish actual soft/hard nofile 64: {line}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let output = wait_bounded(child, RUN_BOUND);
    assert!(
        !output.status.success(),
        "fd_max=65 above inherited hard=64 must refuse"
    );
    let rows = root.output_rows();
    // The matched real-runtime control above identifies the FD denial. The
    // production Worker must also refuse before any payload runs; its existing
    // pre-PID startup error intentionally is not a runtime-stderr API.
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("resource probe"),
        "guest resource fixture executed despite impossible fd limit"
    );
    let after = runtime_ids(&root);
    assert!(
        after
            .difference(&before)
            .all(|id| { inspect(&root, id).is_none_or(|details| !root.owns_container(&details)) }),
        "hard-limit refusal must not create its own guest container"
    );
    assert!(
        rows.iter()
            .all(|(_, value)| !value.to_string().contains("resource probe")),
        "resource guest output was persisted despite refusal"
    );
}
