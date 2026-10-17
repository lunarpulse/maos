//! Test-only direct-syscall vehicle for the 24 executable t3-escape-corpus-v1 scenarios.
//! Only an observed blocked effect emits the D-10 confirmed-write marker.
//! No shell-tool absence is mistaken for a blocked syscall. Not shipped.

use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::os::unix::net::UnixStream;
use std::time::Duration;

const IO_TIMEOUT: Duration = Duration::from_secs(2);
const EGRESS_SOCKET: &str = "/proxy/egress.sock";
/// A routable public resolver: unreachable from `--network=none`.
const PUBLIC_RESOLVER: IpAddr = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));

fn main() {
    let args: Vec<String> = std::env::args().skip(1).filter(|arg| arg != "--").collect();
    let outcome = match args.as_slice() {
        [mode, vector, sentinel, host_pid] if mode == "--maos-escape" => {
            escape(vector, sentinel, host_pid)
        }
        [mode] if mode == "--maos-socket-reach" => socket_reach(),
        _ => Err(
            "usage: --maos-escape <vector> <host-sentinel> <host-pid> | --maos-socket-reach".into(),
        ),
    };
    if let Err(error) = outcome {
        eprintln!("t3-network-escape-probe: {error}");
        std::process::exit(1);
    }
}

fn escape(vector: &str, sentinel: &str, host_pid: &str) -> Result<(), String> {
    let host_pid: i32 = host_pid.parse().map_err(|_| "invalid host PID")?;
    if host_pid <= 1 {
        return Err("host PID must not be container init".into());
    }
    let errno = match vector {
        "container.escape.network.tcp_outbound" => tcp("10.0.0.1:80", &[libc::ENETUNREACH])?,
        "container.escape.network.udp_outbound" => udp("10.0.0.1:53")?,
        "container.escape.network.dns_resolution" => dns()?,
        "container.escape.network.ipv6_outbound" => tcp(
            "[2606:4700:4700::1111]:443",
            &[libc::ENETUNREACH, libc::EADDRNOTAVAIL, libc::EAFNOSUPPORT],
        )?,
        "container.escape.capability.cap_net_raw" => raw_icmp()?,
        "container.escape.network.raw_socket" => raw_icmp()?,
        "container.escape.filesystem.etc_passwd" => {
            denied_read(&format!("/proc/{host_pid}/root/etc/passwd"), &[libc::ENOENT, libc::EACCES, libc::EPERM])?
        }
        "container.escape.filesystem.proc_1_root" => {
            denied_read(&format!("/proc/1/root{sentinel}"), &[libc::ENOENT, libc::EACCES, libc::EPERM])?
        }
        "container.escape.filesystem.home" | "container.escape.filesystem.var_log_maos" => {
            denied_read(sentinel, &[libc::ENOENT])?
        }
        "container.escape.filesystem.host_root" => {
            denied_read(&format!("/host_root{sentinel}"), &[libc::ENOENT])?
        }
        "container.escape.process.proc_1_exe" => denied_result(
            std::fs::read_link(format!("/proc/{host_pid}/exe")),
            &[libc::ENOENT, libc::EACCES],
        )?,
        "container.escape.process.kill_pid_1" => {
            // SAFETY: the executor supplies only its owned host witness child's PID.
            denied_syscall(
                unsafe { libc::kill(host_pid, libc::SIGKILL) } as i64,
                &[libc::ESRCH, libc::EPERM],
            )?
        }
        "container.escape.process.ptrace_host_pid" => ptrace(host_pid)?,
        "container.escape.process.mount" => mount()?,
        "container.escape.process.fork_bomb" => fork_limit()?,
        "container.escape.capability.cap_sys_admin" => {
            capability_absent(21)?;
            mount()?
        }
        "container.escape.capability.cap_sys_ptrace" => {
            capability_absent(19)?;
            ptrace_nondumpable_child()?
        }
        "container.escape.capability.cap_setuid" => {
            capability_absent(7)?;
            // SAFETY: setuid has no pointer arguments; success is a test failure.
            denied_syscall(unsafe { libc::setuid(0) } as i64, &[libc::EPERM])?
        }
        "container.escape.capability.cap_dac_override" => {
            capability_absent(1)?;
            let path = std::ffi::CString::new("/etc/passwd").unwrap();
            // SAFETY: NUL-terminated static pathname.
            denied_syscall(
                unsafe { libc::chmod(path.as_ptr(), 0o777) } as i64,
                &[libc::EROFS, libc::EPERM, libc::EACCES],
            )?
        }
        "container.escape.runtime.docker_sock" | "container.escape.runtime.image_pull_via_sock" => {
            denied_result(UnixStream::connect(sentinel), &[libc::ENOENT])?;
            denied_result(
                UnixStream::connect("/var/run/docker.sock"),
                &[libc::ENOENT, libc::EACCES],
            )?
        }
        "container.escape.runtime.podman_sock" => {
            denied_result(UnixStream::connect(sentinel), &[libc::ENOENT])?;
            denied_result(
                UnixStream::connect("/run/podman/podman.sock"),
                &[libc::ENOENT, libc::EACCES],
            )?
        }
        "container.escape.runtime.runtime_config_rewrite" => denied_result(
            std::fs::OpenOptions::new().write(true).open(sentinel),
            &[libc::ENOENT],
        )?,
        other => return Err(format!("unknown scenario vector {other}")),
    };
    println!("T3-ESCAPE-BLOCKED {vector} errno={errno}");
    eprintln!("maos-sandbox-block: {vector}");
    Ok(())
}

fn errno(error: &std::io::Error) -> i32 {
    error.raw_os_error().unwrap_or(-1)
}

fn tcp(target: &str, blocked: &[i32]) -> Result<i32, String> {
    let address: SocketAddr = target
        .parse()
        .map_err(|_| format!("invalid fixed target {target}"))?;
    match TcpStream::connect_timeout(&address, IO_TIMEOUT) {
        Ok(_) => Err(format!("escape: TCP connected to {target}")),
        Err(error) if blocked.contains(&errno(&error)) => Ok(errno(&error)),
        Err(error) => Err(format!(
            "TCP {target} failed with unexpected errno {}",
            errno(&error)
        )),
    }
}

fn udp(target: &str) -> Result<i32, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|error| format!("UDP bind: {error}"))?;
    match socket.send_to(b"maos-17-1-escape", target) {
        Ok(_) => Err(format!("escape: UDP datagram left for {target}")),
        Err(error) if errno(&error) == libc::ENETUNREACH => Ok(libc::ENETUNREACH),
        Err(error) => Err(format!(
            "UDP {target} failed with unexpected errno {}",
            errno(&error)
        )),
    }
}

/// A real `example.com` A query to every configured resolver and to a public
/// one. Blocked means no resolver answered; the public one must be unroutable.
fn dns() -> Result<i32, String> {
    let mut resolvers: Vec<IpAddr> = std::fs::read_to_string("/etc/resolv.conf")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("nameserver"))
        .filter_map(|address| address.trim().parse().ok())
        .collect();
    resolvers.push(PUBLIC_RESOLVER);
    let query = dns_query();
    let mut public_errno = None;
    for resolver in resolvers {
        let local = if resolver.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = UdpSocket::bind(local).map_err(|error| format!("DNS bind: {error}"))?;
        socket
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|error| format!("DNS timeout: {error}"))?;
        let observed = match socket.send_to(&query, SocketAddr::new(resolver, 53)) {
            Err(error) => errno(&error),
            Ok(_) => {
                let mut answer = [0u8; 512];
                match socket.recv_from(&mut answer) {
                    Ok((length, _)) if length >= 2 && answer[..2] == query[..2] => {
                        return Err(format!("escape: resolver {resolver} answered"));
                    }
                    Ok(_) => {
                        return Err(format!("resolver {resolver} returned a foreign datagram"))
                    }
                    Err(error) => errno(&error),
                }
            }
        };
        println!("T3-DNS resolver={resolver} errno={observed}");
        if resolver == PUBLIC_RESOLVER {
            public_errno = Some(observed);
        }
    }
    match public_errno {
        Some(libc::ENETUNREACH) => Ok(libc::ENETUNREACH),
        other => Err(format!(
            "public resolver expected ENETUNREACH, observed {other:?}"
        )),
    }
}

fn dns_query() -> Vec<u8> {
    let mut query = vec![0x17, 0x01, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in ["example", "com"] {
        query.push(label.len() as u8);
        query.extend_from_slice(label.as_bytes());
    }
    query.extend_from_slice(&[0, 0, 1, 0, 1]);
    query
}

fn denied_read(path: &str, allowed: &[i32]) -> Result<i32, String> {
    denied_result(std::fs::read(path), allowed)
}

fn denied_result<T>(result: std::io::Result<T>, allowed: &[i32]) -> Result<i32, String> {
    match result {
        Ok(_) => Err("escape: operation unexpectedly succeeded".into()),
        Err(error) if allowed.contains(&errno(&error)) => Ok(errno(&error)),
        Err(error) => Err(format!("unexpected syscall failure: {error}")),
    }
}

fn denied_syscall(result: i64, allowed: &[i32]) -> Result<i32, String> {
    if result >= 0 {
        return Err("escape: syscall unexpectedly succeeded".into());
    }
    denied_result::<()>(Err(std::io::Error::last_os_error()), allowed)
}

fn capability_absent(bit: u32) -> Result<(), String> {
    let status = std::fs::read_to_string("/proc/self/status").map_err(|e| e.to_string())?;
    let value = status
        .lines()
        .find_map(|line| line.strip_prefix("CapBnd:\t"))
        .ok_or("CapBnd unavailable")?;
    let mask = u64::from_str_radix(value.trim(), 16).map_err(|e| e.to_string())?;
    if mask & (1 << bit) != 0 {
        return Err(format!(
            "escape: capability bit {bit} remains in bounding set {mask:x}"
        ));
    }
    Ok(())
}

fn ptrace(pid: i32) -> Result<i32, String> {
    // SAFETY: PTRACE_ATTACH ignores both null pointers.
    denied_syscall(
        unsafe {
            libc::ptrace(
                libc::PTRACE_ATTACH,
                pid,
                std::ptr::null_mut::<libc::c_void>(),
                std::ptr::null_mut::<libc::c_void>(),
            )
        } as i64,
        &[libc::EPERM, libc::ESRCH, libc::EACCES],
    )
}

fn ptrace_nondumpable_child() -> Result<i32, String> {
    let mut fds = [-1; 2];
    // SAFETY: pipe writes exactly two valid descriptors into this array.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: single-threaded test probe; child uses only libc then pauses.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // SAFETY: valid pipe descriptors, one byte, and pointer-free prctl/pause.
        unsafe {
            libc::close(fds[0]);
            let ready: u8 = if libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) == 0 {
                1
            } else {
                0
            };
            libc::write(fds[1], (&ready as *const u8).cast(), 1);
            libc::close(fds[1]);
            loop {
                libc::pause();
            }
        }
    }
    // SAFETY: parent owns these descriptors; read writes one stack byte.
    let ready = unsafe {
        libc::close(fds[1]);
        let mut ready = 0u8;
        let n = if pid > 0 {
            libc::read(fds[0], (&mut ready as *mut u8).cast(), 1)
        } else {
            -1
        };
        libc::close(fds[0]);
        n == 1 && ready == 1
    };
    let result = if ready {
        ptrace(pid)
    } else {
        Err("could not prepare live nondumpable ptrace target".into())
    };
    if pid > 0 {
        // SAFETY: only this probe's owned child, always killed and reaped.
        unsafe {
            libc::kill(pid, libc::SIGKILL);
            libc::waitpid(pid, std::ptr::null_mut(), 0);
        }
    }
    result
}

fn mount() -> Result<i32, String> {
    let path = "/tmp/maos-mount-attempt";
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    let target = std::ffi::CString::new(path).unwrap();
    let source = std::ffi::CString::new("proc").unwrap();
    // SAFETY: live NUL-terminated paths and a null data argument.
    denied_syscall(
        unsafe {
            libc::mount(
                source.as_ptr(),
                target.as_ptr(),
                source.as_ptr(),
                0,
                std::ptr::null(),
            )
        } as i64,
        &[libc::EPERM, libc::EACCES],
    )
}

fn fork_limit() -> Result<i32, String> {
    struct Children(Vec<libc::pid_t>);
    impl Drop for Children {
        fn drop(&mut self) {
            for &pid in &self.0 {
                // SAFETY: only owned, unreaped children; cannot target host PIDs.
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                }
            }
            for &pid in &self.0 {
                // SAFETY: waitpid writes no status through the null pointer.
                unsafe {
                    libc::waitpid(pid, std::ptr::null_mut(), 0);
                }
            }
        }
    }
    let limit = std::fs::read_to_string("/sys/fs/cgroup/pids.max").map_err(|e| e.to_string())?;
    if limit.trim() != "64" {
        return Err(format!(
            "production PID cgroup limit must be 64, observed {limit:?}"
        ));
    }
    let mut children = Children(Vec::with_capacity(128));
    for _ in 0..128 {
        // SAFETY: this probe is single-threaded; child calls only async-signal-safe pause.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            let blocked = denied_syscall(-1, &[libc::EAGAIN])?;
            let current = std::fs::read_to_string("/sys/fs/cgroup/pids.current")
                .map_err(|e| e.to_string())?;
            if current.trim() != "64" {
                return Err(format!(
                    "fork refused before cgroup saturation: pids.current={current:?}"
                ));
            }
            return Ok(blocked);
        }
        if pid == 0 {
            loop {
                // SAFETY: no pointers; child is killed/reaped by its owning parent.
                unsafe {
                    libc::pause();
                }
            }
        }
        children.0.push(pid);
    }
    Err("escape: pids limit allowed 128 live children (expected production limit 64)".into())
}

fn raw_icmp() -> Result<i32, String> {
    capability_absent(13)?;
    // SAFETY: plain socket(2) with constant arguments; no pointer is passed.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
    if fd >= 0 {
        // SAFETY: `fd` is the descriptor this function just obtained.
        unsafe { libc::close(fd) };
        return Err("escape: raw ICMP socket created".into());
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::EPERM) => Ok(libc::EPERM),
        other => Err(format!(
            "raw ICMP socket failed with unexpected errno {other:?}"
        )),
    }
}

fn socket_reach() -> Result<(), String> {
    let mut stream = match UnixStream::connect(EGRESS_SOCKET) {
        Ok(stream) => stream,
        Err(error) => {
            println!("T3-SOCKET-REFUSED errno={}", errno(&error));
            return Ok(());
        }
    };
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|error| format!("socket timeout: {error}"))?;
    stream
        .write_all(b"PING\n")
        .map_err(|error| format!("socket write: {error}"))?;
    let mut reply = String::new();
    BufReader::new(stream)
        .read_line(&mut reply)
        .map_err(|error| format!("socket read: {error}"))?;
    if reply.trim_end() != "PONG" {
        return Err(format!("host socket replied {reply:?}"));
    }
    println!("T3-SOCKET-REACHABLE");
    Ok(())
}
