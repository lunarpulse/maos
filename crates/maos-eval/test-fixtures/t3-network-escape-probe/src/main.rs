//! Story 17-1 D-12 — the five `--network=none` scenarios of
//! `crates/maos-eval/fixtures/t3-escape-corpus-v0/`, attempted for real from
//! inside a verified T3 container, where this binary is the mounted
//! `/maos/spirit`.
//!
//! Each `--maos-escape <attempted_syscall_substring>` mode makes the attempt
//! with a syscall, not a shell tool (the digest-pinned image carries no `nc`,
//! `nslookup` or `ping`). A blocked attempt prints
//! `T3-ESCAPE-BLOCKED <substring> errno=<n>` on stdout and the D-10 marker
//! `maos-sandbox-block: <substring>` on stderr, which the kernel bridge
//! journals as a kind-8 row by confirmed write. An attempt that succeeds, or
//! fails with an errno the boundary cannot produce, exits 1.
//!
//! `--maos-socket-reach` is the M3 custody leg: it dials the bind-mounted
//! `/proxy/egress.sock` and reports `T3-SOCKET-REACHABLE` or
//! `T3-SOCKET-REFUSED errno=<n>`; the executor decides which one is correct.
//!
//! This binary is NOT shipped: it lives outside the main workspace (own
//! `[workspace]` table) and is built statically by the T3 CI jobs.

use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::os::unix::net::UnixStream;
use std::time::Duration;

const IO_TIMEOUT: Duration = Duration::from_secs(2);
const EGRESS_SOCKET: &str = "/proxy/egress.sock";
/// A routable public resolver: unreachable from `--network=none`.
const PUBLIC_RESOLVER: IpAddr = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--")
        .collect();
    let outcome = match args.as_slice() {
        [mode, vector] if mode == "--maos-escape" => escape(vector),
        [mode] if mode == "--maos-socket-reach" => socket_reach(),
        _ => Err("usage: --maos-escape <attempted_syscall_substring> | --maos-socket-reach".into()),
    };
    if let Err(error) = outcome {
        eprintln!("t3-network-escape-probe: {error}");
        std::process::exit(1);
    }
}

fn escape(vector: &str) -> Result<(), String> {
    let errno = match vector {
        "container.escape.network.tcp_outbound" => tcp("10.0.0.1:80", &[libc::ENETUNREACH])?,
        "container.escape.network.udp_outbound" => udp("10.0.0.1:53")?,
        "container.escape.network.dns_resolution" => dns()?,
        "container.escape.network.ipv6_outbound" => tcp(
            "[2606:4700:4700::1111]:443",
            &[libc::ENETUNREACH, libc::EADDRNOTAVAIL, libc::EAFNOSUPPORT],
        )?,
        "container.escape.capability.cap_net_raw" => raw_icmp()?,
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
        Err(error) => Err(format!("TCP {target} failed with unexpected errno {}", errno(&error))),
    }
}

fn udp(target: &str) -> Result<i32, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|error| format!("UDP bind: {error}"))?;
    match socket.send_to(b"maos-17-1-escape", target) {
        Ok(_) => Err(format!("escape: UDP datagram left for {target}")),
        Err(error) if errno(&error) == libc::ENETUNREACH => Ok(libc::ENETUNREACH),
        Err(error) => Err(format!("UDP {target} failed with unexpected errno {}", errno(&error))),
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
        let local = if resolver.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
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
                    Ok(_) => return Err(format!("resolver {resolver} returned a foreign datagram")),
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
        other => Err(format!("public resolver expected ENETUNREACH, observed {other:?}")),
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

fn raw_icmp() -> Result<i32, String> {
    // SAFETY: plain socket(2) with constant arguments; no pointer is passed.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
    if fd >= 0 {
        // SAFETY: `fd` is the descriptor this function just obtained.
        unsafe { libc::close(fd) };
        return Err("escape: raw ICMP socket created".into());
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::EPERM) => Ok(libc::EPERM),
        other => Err(format!("raw ICMP socket failed with unexpected errno {other:?}")),
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
