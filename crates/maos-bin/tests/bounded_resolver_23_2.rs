//! Story 23-2 — the production `ResolveHost` (D-23-0b-06 a, 23-0b "Open for
//! 23-2"): bounded answers, a deadline, and no thread per call.
//!
//! No sleeps. The deadline tests hold a gate inside the lookup and assert the
//! verdict while the gate is held; a gate that is never released fails CLOSED
//! (the lookup errors after a safety limit).

use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use maos_bin::bounded_resolver::{BoundedResolver, MAX_ANSWERS, QUEUE_DEPTH};
use maos_domain::outbound::{ResolveFailed, ResolveHost};

const DEADLINE: Duration = Duration::from_millis(50);
const GATE_SAFETY: Duration = Duration::from_secs(30);

fn ip(index: usize) -> IpAddr {
    IpAddr::from([93, 184, 216, u8::try_from(index).unwrap()])
}

#[derive(Default)]
struct Gate {
    state: Mutex<(usize, bool)>, // (entered, released)
    changed: Condvar,
}

impl Gate {
    fn pass(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        state.0 += 1;
        self.changed.notify_all();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, GATE_SAFETY, |state| !state.1)
            .unwrap();
        !timeout.timed_out() && state.1
    }
    fn wait_entered(&self, count: usize) {
        let state = self.state.lock().unwrap();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, GATE_SAFETY, |state| state.0 < count)
            .unwrap();
        assert!(
            !timeout.timed_out() && state.0 >= count,
            "lookup never entered"
        );
    }
    fn release(&self) {
        self.state.lock().unwrap().1 = true;
        self.changed.notify_all();
    }
}

fn answering(addresses: Vec<IpAddr>) -> BoundedResolver {
    BoundedResolver::with_lookup(Arc::new(move |_, _| Ok(addresses.clone())), DEADLINE)
}

/// The answer count is capped: the cap itself is admitted, one more is not
/// (a refusal, never a silent truncation).
#[test]
fn the_answer_count_is_capped_and_never_truncated() {
    let at_cap = answering((0..MAX_ANSWERS).map(ip).collect());
    assert_eq!(at_cap.resolve("h.example", 443).unwrap().len(), MAX_ANSWERS);
    let over = answering((0..=MAX_ANSWERS).map(ip).collect());
    assert_eq!(over.resolve("h.example", 443), Err(ResolveFailed));
}

/// Duplicates collapse (and count once toward the cap); order is kept; an
/// empty answer and a lookup error are failures.
#[test]
fn duplicates_collapse_and_empty_or_failing_answers_fail() {
    let repeated = answering(vec![ip(1), ip(2), ip(1), ip(2)]);
    assert_eq!(repeated.resolve("h.example", 443), Ok(vec![ip(1), ip(2)]));
    let many_repeats = answering(vec![ip(1); MAX_ANSWERS * 3]);
    assert_eq!(many_repeats.resolve("h.example", 443), Ok(vec![ip(1)]));
    assert_eq!(
        answering(vec![]).resolve("h.example", 443),
        Err(ResolveFailed)
    );
    let failing = BoundedResolver::with_lookup(
        Arc::new(|_, _| Err(std::io::Error::other("nxdomain"))),
        DEADLINE,
    );
    assert_eq!(failing.resolve("h.example", 443), Err(ResolveFailed));
}

/// The lookup receives the host and port the caller asked for.
#[test]
fn the_lookup_sees_the_requested_host_and_port() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let resolver = BoundedResolver::with_lookup(
        Arc::new(move |host, port| {
            sink.lock().unwrap().push((host.to_owned(), port));
            Ok(vec![ip(7)])
        }),
        DEADLINE,
    );
    resolver.resolve("idp.example", 8443).unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        vec![("idp.example".to_owned(), 8443)]
    );
}

/// A hung lookup fails the caller at the deadline, while the gate is held.
/// Many more callers do not get more threads: lookups never overlap, calls
/// beyond the queue fail at once, and jobs that expired while waiting are
/// skipped (never run late) once the lookup unblocks.
#[test]
fn a_hung_lookup_is_bounded_and_never_multiplies_threads() {
    let gate = Arc::new(Gate::default());
    let running = Arc::new(AtomicUsize::new(0));
    let overlapped = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let resolver = {
        let (gate, running, overlapped, calls) = (
            Arc::clone(&gate),
            Arc::clone(&running),
            Arc::clone(&overlapped),
            Arc::clone(&calls),
        );
        BoundedResolver::with_lookup(
            Arc::new(move |_, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                if running.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlapped.fetch_add(1, Ordering::SeqCst);
                }
                let released = gate.pass();
                running.fetch_sub(1, Ordering::SeqCst);
                if released {
                    Ok(vec![ip(9)])
                } else {
                    Err(std::io::Error::other("gate never released"))
                }
            }),
            DEADLINE,
        )
    };

    // The first call enters the lookup and hangs; its caller gives up.
    let first = {
        let resolver = resolver.clone();
        std::thread::spawn(move || resolver.resolve("h.example", 443))
    };
    gate.wait_entered(1);
    assert_eq!(first.join().unwrap(), Err(ResolveFailed));

    // Many more calls while the lookup is still hung: every one fails, none
    // starts a lookup of its own.
    let callers: Vec<_> = (0..(QUEUE_DEPTH * 4))
        .map(|_| {
            let resolver = resolver.clone();
            std::thread::spawn(move || resolver.resolve("h.example", 443))
        })
        .collect();
    for caller in callers {
        assert_eq!(caller.join().unwrap(), Err(ResolveFailed));
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the hung lookup is the only one"
    );

    // Unblocked: no two lookups ever overlapped.
    gate.release();
    assert_eq!(overlapped.load(Ordering::SeqCst), 0);
}

/// A job whose deadline passed while it waited behind a hung lookup is
/// skipped when the lookup unblocks, never run late. (Fewer waiters than the
/// queue holds, so the final call has a slot.)
#[test]
fn jobs_that_expired_in_the_queue_never_run_late() {
    let gate = Arc::new(Gate::default());
    let calls = Arc::new(AtomicUsize::new(0));
    let resolver = {
        let (gate, calls) = (Arc::clone(&gate), Arc::clone(&calls));
        BoundedResolver::with_lookup(
            Arc::new(move |_, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                if gate.pass() {
                    Ok(vec![ip(9)])
                } else {
                    Err(std::io::Error::other("gate never released"))
                }
            }),
            DEADLINE,
        )
    };
    let first = {
        let resolver = resolver.clone();
        std::thread::spawn(move || resolver.resolve("h.example", 443))
    };
    gate.wait_entered(1);
    assert_eq!(first.join().unwrap(), Err(ResolveFailed));
    let waiters: Vec<_> = (0..(QUEUE_DEPTH - 1))
        .map(|_| {
            let resolver = resolver.clone();
            std::thread::spawn(move || resolver.resolve("h.example", 443))
        })
        .collect();
    for waiter in waiters {
        assert_eq!(waiter.join().unwrap(), Err(ResolveFailed));
    }
    gate.release();
    assert_eq!(resolver.resolve("h.example", 443), Ok(vec![ip(9)]));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "only the hung call and the fresh one ran"
    );
}
