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
