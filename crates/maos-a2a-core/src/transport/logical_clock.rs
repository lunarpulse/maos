//! Lamport logical clock for A2A cross-Host frame ordering.
//!
//! Architecture §7.2: "Logical-clock frame ordering. Cross-Host frame
//! ordering uses logical clocks (Lamport or hybrid logical clock — final
//! pick by v0.5); wall-clock is metadata only." Story 6.3 picks **Lamport**
//! for v0.5 — rationale documented in Dev Notes "Lamport vs HLC decision".

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct LamportClock {
    counter: AtomicU64,
}

impl LamportClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_initial(initial: u64) -> Self {
        Self {
            counter: AtomicU64::new(initial),
        }
    }

    /// Outbound send: increment counter and return the new value to stamp on
    /// the frame.
    pub fn send_tick(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Inbound receive: advance to `max(local, observed) + 1`. Returns the
    /// new local value. Per architecture §7.2 the receiver's clock must
    /// strictly advance past the observed value to preserve causal order.
    pub fn recv_advance(&self, observed: u64) -> u64 {
        let mut prev = self.counter.load(Ordering::SeqCst);
        loop {
            let new = std::cmp::max(prev, observed) + 1;
            match self
                .counter
                .compare_exchange(prev, new, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return new,
                Err(curr) => prev = curr,
            }
        }
    }

    /// Read current value (testing / diagnostics).
    pub fn current(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn send_tick_monotonic_increment() {
        let c = LamportClock::new();
        assert_eq!(c.send_tick(), 1);
        assert_eq!(c.send_tick(), 2);
        assert_eq!(c.send_tick(), 3);
    }

    #[test]
    fn recv_advance_max_plus_one() {
        let c = LamportClock::with_initial(50);
        let v = c.recv_advance(100);
        assert_eq!(v, 101);
        let v2 = c.recv_advance(50);
        assert_eq!(v2, 102);
    }

    #[test]
    fn recv_advance_when_observed_is_zero() {
        let c = LamportClock::with_initial(5);
        let v = c.recv_advance(0);
        assert_eq!(v, 6);
    }

    #[test]
    fn concurrent_send_tick_each_unique() {
        let c = Arc::new(LamportClock::new());
        let mut handles = Vec::new();
        for _ in 0..16 {
            let c = c.clone();
            handles.push(std::thread::spawn(move || c.send_tick()));
        }
        let mut values: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        values.sort_unstable();
        for (i, v) in values.iter().enumerate() {
            assert_eq!(
                *v,
                (i + 1) as u64,
                "values must be 1..=16 with no duplicates"
            );
        }
    }

    #[test]
    fn concurrent_recv_advance_preserves_invariant() {
        let c = Arc::new(LamportClock::with_initial(0));
        let mut handles = Vec::new();
        for i in 0..16 {
            let c = c.clone();
            handles.push(std::thread::spawn(move || c.recv_advance(i * 10)));
        }
        for h in handles {
            h.join().unwrap();
        }
        // Final value must be > max(observed)
        let final_val = c.current();
        assert!(final_val > 150);
    }
}
