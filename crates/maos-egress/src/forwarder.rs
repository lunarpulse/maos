//! In-container loopback transport; contains no credential or policy logic.

use std::io;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const MAX_CONNECTIONS: usize = 16;
const IO_TIMEOUT: Duration = Duration::from_secs(30);
type Pair = (TcpStream, UnixStream);

pub struct Forwarder {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<io::Result<()>>>,
}

impl Forwarder {
    pub fn bind(socket: &Path, address: SocketAddr) -> io::Result<Self> {
        if !address.ip().is_loopback() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "forwarder requires loopback",
            ));
        }
        let listener = TcpListener::bind(address)?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let socket = socket.to_owned();
        let worker_stop = Arc::clone(&stop);
        let task = thread::Builder::new()
            .name("maos-egress-forwarder".into())
            .spawn(move || serve(listener, socket, worker_stop))?;
        Ok(Self {
            address,
            stop,
            task: Some(task),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    pub fn stop_and_join(&mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        match self.task.take() {
            Some(task) => task
                .join()
                .map_err(|_| io::Error::other("forwarder task panicked"))?,
            None => Ok(()),
        }
    }
}

impl Drop for Forwarder {
    fn drop(&mut self) {
        let _ = self.stop_and_join();
    }
}

struct Connection {
    pair: Arc<Pair>,
    task: JoinHandle<io::Result<()>>,
}

fn serve(listener: TcpListener, socket: PathBuf, stop: Arc<AtomicBool>) -> io::Result<()> {
    let mut connections: Vec<Connection> = Vec::with_capacity(MAX_CONNECTIONS);
    let result = (|| {
        while !stop.load(Ordering::Acquire) {
            let mut index = 0;
            while index < connections.len() {
                if connections[index].task.is_finished() {
                    join(connections.swap_remove(index))?;
                } else {
                    index += 1;
                }
            }
            if connections.len() < MAX_CONNECTIONS {
                match listener.accept() {
                    Ok((tcp, _)) => {
                        let unix = UnixStream::connect(&socket)?;
                        tcp.set_read_timeout(Some(IO_TIMEOUT))?;
                        tcp.set_write_timeout(Some(IO_TIMEOUT))?;
                        unix.set_read_timeout(Some(IO_TIMEOUT))?;
                        unix.set_write_timeout(Some(IO_TIMEOUT))?;
                        let pair = Arc::new((tcp, unix));
                        let worker_pair = Arc::clone(&pair);
                        let task = thread::Builder::new()
                            .name("maos-egress-stream".into())
                            .spawn(move || forward(&worker_pair))?;
                        connections.push(Connection { pair, task });
                        continue;
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) => return Err(error),
                }
            }
            // std has no portable cancellable accept; no wake-up FD is needed at Drop.
            thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    })();
    for connection in &connections {
        shutdown(&connection.pair);
    }
    let mut cleanup = Ok(());
    for connection in connections {
        if let Err(error) = join(connection) {
            if cleanup.is_ok() {
                cleanup = Err(error);
            }
        }
    }
    result.and(cleanup)
}

fn join(connection: Connection) -> io::Result<()> {
    connection
        .task
        .join()
        .map_err(|_| io::Error::other("forwarder stream panicked"))?
}

fn shutdown(pair: &Pair) {
    let _ = pair.0.shutdown(Shutdown::Both);
    let _ = pair.1.shutdown(Shutdown::Both);
}

fn disconnected(result: io::Result<u64>) -> io::Result<()> {
    match result {
        Ok(_) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::NotConnected
                    | io::ErrorKind::TimedOut
                    | io::ErrorKind::WouldBlock
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn forward(pair: &Pair) -> io::Result<()> {
    thread::scope(|scope| {
        let upstream = scope.spawn(|| {
            let result = io::copy(&mut &pair.0, &mut &pair.1);
            let _ = pair.1.shutdown(Shutdown::Write);
            disconnected(result)
        });
        let downstream = io::copy(&mut &pair.1, &mut &pair.0);
        shutdown(pair);
        let upstream = upstream
            .join()
            .map_err(|_| io::Error::other("forwarder upload panicked"))?;
        upstream.and(disconnected(downstream))
    })
}
