//! Windows T2 sandbox enforcement: restricted-token + Job Object.
//!
//! Story 10.5 AC3 — `CreateRestrictedToken` + low-integrity token +
//! `CreateProcessAsUserW` with suspended-start Job Object assignment.
//!
//! ## Security Model
//!
//! T2 restricted-token: disabled privileges plus low integrity. Per-Spirit
//! resource caps are enforced by a Job Object before the child thread resumes.
//! The Job Object is configured with kill-on-close and is closed last by the
//! RAII child guard.
#![allow(unsafe_code)]

use std::ffi::OsStr;
use std::io;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::ExitStatusExt;
use std::process::{Command, ExitStatus};

use super::{Cleanup, SandboxSpec, SandboxedChild, SpawnError};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, BOOL, HANDLE, STILL_ACTIVE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Security::{
    CreateRestrictedToken, CreateWellKnownSid, SetTokenInformation, TokenIntegrityLevel,
    WinLowLabelSid, DISABLE_MAX_PRIVILEGE, PSID, SID_AND_ATTRIBUTES, TOKEN_ADJUST_DEFAULT,
    TOKEN_ADJUST_SESSIONID, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_MANDATORY_LABEL,
    TOKEN_QUERY,
};
use windows::Win32::System::JobObjects::{
    JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation, SetInformationJobObject,
    JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
};
use windows::Win32::System::Threading::{
    CreateProcessAsUserW, GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, ResumeThread,
    TerminateProcess, WaitForSingleObject, CREATE_SUSPENDED, PROCESS_INFORMATION, STARTUPINFOW,
};

/// Windows child process handle set owned by [`SandboxedChild`].
pub struct WindowsChild {
    process: HANDLE,
    thread: HANDLE,
    job_handle: isize,
    id: u32,
}

impl WindowsChild {
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        // SAFETY: `process` is a valid process handle until `Drop` closes it.
        unsafe {
            WaitForSingleObject(self.process, u32::MAX);
        }
        self.exit_status()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        // SAFETY: `process` is a valid process handle until `Drop` closes it.
        let wait = unsafe { WaitForSingleObject(self.process, 0) };
        if wait == WAIT_TIMEOUT {
            return Ok(None);
        }
        if wait == WAIT_OBJECT_0 {
            return self.exit_status().map(Some);
        }
        Err(io::Error::last_os_error())
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn kill(&mut self) -> io::Result<()> {
        // SAFETY: `process` is a valid process handle until `Drop` closes it.
        unsafe { TerminateProcess(self.process, 1) }
            .map_err(|e| io::Error::from_raw_os_error(e.code().0))
    }

    fn exit_status(&self) -> io::Result<ExitStatus> {
        let mut code = 0u32;
        // SAFETY: `process` is valid and `code` points to initialized storage.
        unsafe { GetExitCodeProcess(self.process, &mut code) }
            .map_err(|e| io::Error::from_raw_os_error(e.code().0))?;
        if code == STILL_ACTIVE.0 as u32 {
            return Err(io::Error::from(io::ErrorKind::WouldBlock));
        }
        Ok(ExitStatus::from_raw(code))
    }
}

impl Drop for WindowsChild {
    fn drop(&mut self) {
        // SAFETY: all handles are owned by this guard. Close the job last so
        // kill-on-close remains active through process/thread cleanup.
        unsafe {
            let _ = TerminateProcess(self.process, 1);
            let _ = CloseHandle(self.thread);
            let _ = CloseHandle(self.process);
            let _ = CloseHandle(HANDLE(self.job_handle as *mut core::ffi::c_void));
        }
    }
}

/// Spawn a command under Windows T2 sandbox: restricted-token + Job Object.
pub fn spawn_sandboxed(
    spec: &SandboxSpec,
    command: &mut Command,
) -> Result<SandboxedChild, SpawnError> {
    let restricted_token = create_low_integrity_restricted_token()?;
    let job = create_limited_job(spec)?;
    let mut cmdline = command_line(command);
    let current_dir = command
        .get_current_dir()
        .map(|dir| wide_null(dir.as_os_str()));
    let current_dir_ptr = current_dir
        .as_ref()
        .map(|v| PCWSTR(v.as_ptr()))
        .unwrap_or_else(PCWSTR::null);

    let mut startup = STARTUPINFOW::default();
    startup.cb = mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi = PROCESS_INFORMATION::default();

    // SAFETY: all pointers are NUL-terminated and live for the duration of the
    // call. The child is created suspended so we can assign it to the Job Object
    // before any untrusted instruction executes.
    unsafe {
        CreateProcessAsUserW(
            restricted_token,
            PCWSTR::null(),
            PWSTR(cmdline.as_mut_ptr()),
            None,
            None,
            BOOL(0),
            CREATE_SUSPENDED,
            None,
            current_dir_ptr,
            &startup,
            &mut pi,
        )
        .map_err(|e| {
            let _ = CloseHandle(restricted_token);
            SpawnError::SandboxSetup(format!("CreateProcessAsUserW: {e}"))
        })?;
        let _ = CloseHandle(restricted_token);
    }

    if let Err(e) = job.assign_process(pi.hProcess.0 as isize) {
        // SAFETY: handles were returned by CreateProcessAsUserW and are still owned here.
        unsafe {
            let _ = TerminateProcess(pi.hProcess, 1);
            let _ = CloseHandle(pi.hThread);
            let _ = CloseHandle(pi.hProcess);
        }
        return Err(SpawnError::SandboxSetup(format!("assign to job: {e}")));
    }

    // SAFETY: the primary thread is valid and suspended from creation.
    let resumed = unsafe { ResumeThread(pi.hThread) };
    if resumed == u32::MAX {
        // SAFETY: handles were returned by CreateProcessAsUserW and are still owned here.
        unsafe {
            let _ = TerminateProcess(pi.hProcess, 1);
            let _ = CloseHandle(pi.hThread);
            let _ = CloseHandle(pi.hProcess);
        }
        return Err(SpawnError::SandboxSetup("ResumeThread failed".into()));
    }

    let job_handle = job.into_handle();
    Ok(SandboxedChild {
        child: WindowsChild {
            process: pi.hProcess,
            thread: pi.hThread,
            job_handle,
            id: pi.dwProcessId,
        },
        cleanup: Cleanup::None,
    })
}

fn create_low_integrity_restricted_token() -> Result<HANDLE, SpawnError> {
    let mut current_token = HANDLE::default();
    let desired = TOKEN_ASSIGN_PRIMARY
        | TOKEN_DUPLICATE
        | TOKEN_QUERY
        | TOKEN_ADJUST_DEFAULT
        | TOKEN_ADJUST_SESSIONID;

    // SAFETY: GetCurrentProcess returns a pseudo-handle and OpenProcessToken
    // initializes `current_token` on success.
    unsafe { OpenProcessToken(GetCurrentProcess(), desired, &mut current_token) }
        .map_err(|e| SpawnError::SandboxSetup(format!("OpenProcessToken: {e}")))?;

    let mut restricted_token = HANDLE::default();
    // SAFETY: `current_token` is valid; CreateRestrictedToken initializes
    // `restricted_token` on success. DISABLE_MAX_PRIVILEGE removes privileges
    // other than SeChangeNotifyPrivilege.
    unsafe {
        CreateRestrictedToken(
            current_token,
            DISABLE_MAX_PRIVILEGE,
            None,
            None,
            None,
            &mut restricted_token,
        )
        .map_err(|e| {
            let _ = CloseHandle(current_token);
            SpawnError::SandboxSetup(format!("CreateRestrictedToken: {e}"))
        })?;
        let _ = CloseHandle(current_token);
    }

    let mut low_sid_buf = [0u8; 68];
    let mut sid_size = low_sid_buf.len() as u32;
    let low_sid = PSID(low_sid_buf.as_mut_ptr().cast());
    // SAFETY: the buffer is SECURITY_MAX_SID_SIZE bytes and `sid_size` is
    // initialized with its length.
    unsafe { CreateWellKnownSid(WinLowLabelSid, None, low_sid, &mut sid_size) }.map_err(|e| {
        // SAFETY: `restricted_token` is valid at this point.
        unsafe {
            let _ = CloseHandle(restricted_token);
        }
        SpawnError::SandboxSetup(format!("CreateWellKnownSid(WinLowLabelSid): {e}"))
    })?;

    let label = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: low_sid,
            Attributes: 0x20 | 0x40, // SE_GROUP_INTEGRITY | SE_GROUP_INTEGRITY_ENABLED
        },
    };
    // SAFETY: `label` references `low_sid_buf`, which lives through this call.
    unsafe {
        SetTokenInformation(
            restricted_token,
            TokenIntegrityLevel,
            (&label as *const TOKEN_MANDATORY_LABEL).cast(),
            mem::size_of::<TOKEN_MANDATORY_LABEL>() as u32,
        )
        .map_err(|e| {
            let _ = CloseHandle(restricted_token);
            SpawnError::SandboxSetup(format!("SetTokenInformation(TokenIntegrityLevel): {e}"))
        })?;
    }

    Ok(restricted_token)
}

fn create_limited_job(spec: &SandboxSpec) -> Result<win32job::Job, SpawnError> {
    let mut limits = win32job::ExtendedLimitInfo::new();
    limits.limit_kill_on_job_close();
    let job = win32job::Job::create_with_limit_info(&limits)
        .map_err(|e| SpawnError::SandboxSetup(format!("Job::create_with_limit_info: {e}")))?;

    // H1 fix (review finding): the memory cap must be a hard COMMIT cap
    // (JOB_OBJECT_LIMIT_PROCESS_MEMORY), not a working-set limit. win32job only
    // exposes `limit_working_memory`, which sets JOB_OBJECT_LIMIT_WORKINGSET with
    // a zero minimum — SetInformationJobObject rejects min=0/max>0 with
    // ERROR_INVALID_PARAMETER, so the previous code failed every capped spawn.
    // Working-set is also swappable (not a real ceiling). Set the per-process
    // commit limit directly via SetInformationJobObject, re-asserting
    // KILL_ON_JOB_CLOSE in the same call so the extended-limit write does not
    // clear the flag win32job applied at creation.
    if let Some(memory_mb) = spec.resolved_caps.memory_max_mb {
        let bytes = (memory_mb as usize).saturating_mul(1024 * 1024);
        let mut ext = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        ext.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        ext.ProcessMemoryLimit = bytes;
        // SAFETY: `job.handle()` is a valid Job Object handle; `ext` has the exact
        // layout expected for JobObjectExtendedLimitInformation and lives through
        // the call.
        unsafe {
            SetInformationJobObject(
                HANDLE(job.handle() as *mut core::ffi::c_void),
                JobObjectExtendedLimitInformation,
                (&ext as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(|e| SpawnError::SandboxSetup(format!("set process memory limit: {e}")))?;
    }

    if let Some(cpu_pct) = spec.resolved_caps.cpu_max_pct {
        if cpu_pct > 0 && cpu_pct < 100 {
            let rate = (cpu_pct as u32).saturating_mul(100).clamp(1, 10_000);
            let cpu = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
                ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE
                    | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
                Anonymous:
                    windows::Win32::System::JobObjects::JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 {
                        CpuRate: rate,
                    },
            };
            // SAFETY: `job.handle()` is a valid Job Object handle; `cpu` has the
            // exact layout expected for JobObjectCpuRateControlInformation.
            unsafe {
                SetInformationJobObject(
                    HANDLE(job.handle() as *mut core::ffi::c_void),
                    JobObjectCpuRateControlInformation,
                    (&cpu as *const JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast(),
                    mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
                )
            }
            .map_err(|e| SpawnError::SandboxSetup(format!("set CPU rate control: {e}")))?;
        }
    }

    Ok(job)
}

fn command_line(command: &Command) -> Vec<u16> {
    let mut s = quote_windows_arg(command.get_program());
    for arg in command.get_args() {
        s.push(' ');
        s.push_str(&quote_windows_arg(arg));
    }
    OsStr::new(&s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn wide_null(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn quote_windows_arg(arg: &OsStr) -> String {
    let s = arg.to_string_lossy();
    if s.is_empty() {
        return "\"\"".into();
    }
    if !s.chars().any(|c| c == ' ' || c == '\t' || c == '"') {
        return s.into_owned();
    }

    let mut out = String::from("\"");
    let mut backslashes = 0;
    for ch in s.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat('\\').take(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat('\\').take(backslashes));
                backslashes = 0;
                out.push(ch);
            }
        }
    }
    out.extend(std::iter::repeat('\\').take(backslashes * 2));
    out.push('"');
    out
}
