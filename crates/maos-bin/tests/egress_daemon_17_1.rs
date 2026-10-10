#![cfg(all(unix, feature = "network"))]

use maos_bin::egress::EgressHandle;
use maos_domain::invariants::i3::FrameOrigin;
use maos_egress::Credential;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_proxy_refusal_is_a_confirmed_kernel_sandbox_block_before_reply() {
    let home = tempfile::tempdir().unwrap();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x17_1));
    let cancel = CancellationToken::new();
    let (handle, task) = EgressHandle::start(
        home.path().join("uninitialized-home"),
        log.clone(),
        cancel.clone(),
    )
    .unwrap();
    let mut lease = handle
        .spawn(
            31,
            "worker-31",
            &[],
            &[],
            Credential::None,
            Duration::from_secs(60),
        )
        .unwrap();
    let mut socket = UnixStream::connect(lease.socket_path()).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(socket, "GET /forbidden.example:443/ HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n", lease.bearer()).unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 403"));
    let rows = log
        .query_frames(FrameFilter {
            spirit_pid: Some(31),
            kind: Some(FrameKind::SandboxBlock),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "one refused request must produce exactly one real kind-8 row"
    );
    assert_eq!(rows[0].origin, FrameOrigin::Kernel);
    assert_eq!(rows[0].intent, "sandbox.block.egress.forbidden.example:443");
    assert!(!response.contains(lease.bearer()));
    assert!(!rows[0]
        .payload_redacted
        .windows(lease.bearer().len())
        .any(|window| window == lease.bearer().as_bytes()));
    lease.finish().unwrap();
    assert!(!lease.socket_path().exists());
    cancel.cancel();
    task.await.unwrap().unwrap();
    assert!(!handle.is_available());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn root_shutdown_joins_an_active_proxy_even_while_its_binding_finishes() {
    let home = tempfile::tempdir().unwrap();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x17_2));
    let cancel = CancellationToken::new();
    let (handle, task) =
        EgressHandle::start(home.path().to_owned(), log.clone(), cancel.clone()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = listener.local_addr().unwrap().to_string();
    let allowed = authority.clone().try_into().unwrap();
    let (accepted_tx, accepted_rx) = std::sync::mpsc::channel();
    let service = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
            assert!(head.len() <= 16 * 1024);
        }
        let head = String::from_utf8(head).unwrap().to_ascii_lowercase();
        let authenticated = head
            .lines()
            .filter(|line| line.starts_with("authorization:"))
            .eq(["authorization: bearer synthetic-private-daemon-key"]);
        if !authenticated {
            socket
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            return;
        }
        accepted_tx.send(()).unwrap();
        // Hold the response until the real cancellation shuts the TCP stream.
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    let mut lease = handle
        .spawn(
            32,
            "worker-32",
            std::slice::from_ref(&allowed),
            std::slice::from_ref(&allowed),
            Credential::Bearer("synthetic-private-daemon-key".into()),
            Duration::from_secs(60),
        )
        .unwrap();
    let path = lease.socket_path().to_owned();
    let mut client = UnixStream::connect(&path).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(
        client,
        "GET /{authority}/ HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
        lease.bearer()
    )
    .unwrap();
    accepted_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("authenticated upstream request");
    let finish = tokio::task::spawn_blocking(move || lease.finish());
    cancel.cancel();
    task.await.unwrap().unwrap();
    finish.await.unwrap().unwrap();
    let mut byte = [0];
    assert_eq!(client.read(&mut byte).unwrap(), 0);
    service.join().unwrap();
    assert!(!path.exists());
    assert!(!handle.is_available());
    assert_eq!(
        log.query_frames(FrameFilter {
            spirit_pid: Some(32),
            kind: Some(FrameKind::SandboxBlock),
            ..Default::default()
        })
        .unwrap(),
        vec![],
        "planned cancellation is not an egress refusal"
    );
}

/// Scheduler pids restart at 1 in every root. Two roots on one data home (the
/// cohort topology), or a root after a crashed one, must not collide on the
/// first Worker's socket directory.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn roots_sharing_one_home_bind_distinct_worker_sockets() {
    let home = tempfile::tempdir().unwrap();
    let run_dir = home.path().join("maos-home");
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x17_3));
    let roots: Vec<_> = (0..2)
        .map(|_| {
            let cancel = CancellationToken::new();
            let (handle, task) =
                EgressHandle::start(run_dir.clone(), log.clone(), cancel.clone()).unwrap();
            (handle, task, cancel)
        })
        .collect();
    let mut leases: Vec<_> = roots
        .iter()
        .map(|(handle, _, _)| {
            handle
                .spawn(
                    1,
                    "worker",
                    &[],
                    &[],
                    Credential::None,
                    Duration::from_secs(60),
                )
                .expect("each root's first Worker binds its own socket")
        })
        .collect();
    assert_ne!(leases[0].socket_path(), leases[1].socket_path());
    for lease in &leases {
        UnixStream::connect(lease.socket_path()).expect("both Worker sockets are live");
    }
    for lease in &mut leases {
        lease.finish().unwrap();
    }
    for (_, task, cancel) in roots {
        cancel.cancel();
        task.await.unwrap().unwrap();
    }
    assert_eq!(
        std::fs::read_dir(run_dir.join("egress")).unwrap().count(),
        0,
        "a cleanly stopped root leaves no per-root egress directory behind"
    );
}
