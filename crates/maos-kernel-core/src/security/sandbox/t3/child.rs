//! `SandboxedContainerChild` — RAII guard for a T3 container child process.
//!
//! On drop, runs best-effort cleanup: SIGTERM the runtime parent,
//! then `stop --time=2` + `rm -f` the container.

use std::process::Child;

use super::runtime_detect::ContainerRuntime;

/// RAII guard for a T3 container-launched child.
///
/// `child` is the runtime parent process (the `podman run` / `docker run`
/// command). `host_pid` is the host-namespace PID of the in-container
/// process, captured via `<runtime> inspect`.
///
/// On drop:
/// 1. Kills the runtime parent (`Child::kill`)
/// 2. Reaps with `Child::wait`
/// 3. Stops the container (`<runtime> stop --time=2 <container_name>`)
/// 4. Removes the container (`<runtime> rm -f <container_name>`)
pub struct SandboxedContainerChild {
    pub child: Option<Child>,
    pub host_pid: u32,
    pub container_name: String,
    pub runtime: ContainerRuntime,
}

impl SandboxedContainerChild {
    /// Wait for the child to exit and collect its output.
    /// This consumes the child handle — only call once.
    pub fn wait_with_output(&mut self) -> Result<std::process::Output, std::io::Error> {
        self.child
            .take()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "child already consumed")
            })?
            .wait_with_output()
    }

    /// Try to wait for the child without blocking.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        match &mut self.child {
            Some(child) => child.try_wait(),
            None => Ok(None),
        }
    }

    /// Project the data observed at successful runtime spawn into the live
    /// operator report shape.  The caller commits it to the owning SCB.
    pub fn inspect_report(
        &self,
        spirit_id: String,
        image: &super::image_lock::VerifiedImageAttestation,
    ) -> maos_domain::sandbox::SandboxInspectReport {
        maos_domain::sandbox::SandboxInspectReport {
            spirit_id,
            pid: self.host_pid,
            runtime: self.runtime.kind.as_str().to_owned(),
            image_sha: hex::encode(image.entry().image_sha256),
            applied_t2_protections: maos_domain::sandbox::T2ProtectionSummary {
                landlock_rules: 0,
                seccomp_allow_count: 0,
                seccomp_kill_count: 0,
            },
            strictest_of_reasoning: maos_domain::sandbox::StrictestOfReasoning {
                manifest_tier: "T3".into(),
                trust_tier_floor: "T3".into(),
                operator_policy_floor: "T0".into(),
                effective_tier: "T3".into(),
                dominant_axis: "manifest".into(),
            },
        }
    }
}

impl Drop for SandboxedContainerChild {
    fn drop(&mut self) {
        // Step 1: kill the runtime parent
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }

        // Step 2: stop the container (best-effort, 2s timeout mirrors
        // the existing Child::kill + wait pattern at sandbox/mod.rs:108-118)
        let _ = std::process::Command::new(&self.runtime.path)
            .args(["stop", "--time=2", &self.container_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        // Step 3: remove the container (best-effort; --rm would also handle this)
        let _ = std::process::Command::new(&self.runtime.path)
            .args(["rm", "-f", &self.container_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}
