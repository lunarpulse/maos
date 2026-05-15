#![forbid(unsafe_code)]

//! IAC round-trip telemetry metrics (AC4).
//!
//! Hand-rolled Prometheus-compatible metrics using `AtomicU64` counters.
//! Zero new dependencies — dep-free, deterministic test output, exact
//! control over bucket boundaries and label rendering.
//!
//! The `metrics` + `metrics-exporter-prometheus` alternative was
//! considered (Decision Register item (c)) but hand-rolling was chosen
//! for minimal dependency blast and deterministic test output.

use std::fmt::Write;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// Histogram buckets for `iac_rt_duration_us` (microseconds).
/// Exponential, base √2, anchored on the 1500µs SLO from §13.1.
/// 17 explicit boundaries + implicit +Inf = 18 buckets total.
pub const IAC_RT_BUCKETS_US: &[f64] = &[
    50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 450.0, 700.0,
    1000.0, 1500.0, 2200.0, 3300.0, 5000.0, 7500.0, 11000.0,
    16000.0, 25000.0,
];

/// Service label — five variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    Security,
    Memory,
    Iac,
    Capability,
    SpiritScheduler,
}

impl Service {
    fn as_str(&self) -> &'static str {
        match self {
            Service::Security => "security",
            Service::Memory => "memory",
            Service::Iac => "iac",
            Service::Capability => "capability",
            Service::SpiritScheduler => "spirit_scheduler",
        }
    }
}

/// Outcome label for histogram observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Ok,
    Err,
    Timeout,
}

impl Outcome {
    fn as_str(&self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::Err => "err",
            Outcome::Timeout => "timeout",
        }
    }
}

/// Error kind label for `iac_rt_errors_total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Transport,
    Decode,
    Timeout,
    App,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::Transport => "transport",
            ErrorKind::Decode => "decode",
            ErrorKind::Timeout => "timeout",
            ErrorKind::App => "app",
        }
    }
}

/// One histogram series keyed by (service, outcome).
#[maos_attrs::i9_exempt(reason = "IAC telemetry accumulator; AtomicU64 buckets are the sanctioned metric state (IAC round-trip telemetry binding per Epic 1b Owns)")]
struct HistogramSeries {
    buckets: Vec<AtomicU64>,
    sum: AtomicU64,
    count: AtomicU64,
}

impl HistogramSeries {
    fn new() -> Self {
        Self {
            buckets: (0..IAC_RT_BUCKETS_US.len() + 1)
                .map(|_| AtomicU64::new(0))
                .collect(),
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    fn observe(&self, duration_us: u64) {
        let mut bucket_idx = IAC_RT_BUCKETS_US.len();
        for (i, boundary) in IAC_RT_BUCKETS_US.iter().enumerate() {
            if (duration_us as f64) <= *boundary {
                bucket_idx = i;
                break;
            }
        }
        self.buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(duration_us, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }
}

/// One counter series keyed by (service, kind).
#[maos_attrs::i9_exempt(reason = "IAC telemetry accumulator; AtomicU64 counter is the sanctioned metric state (IAC round-trip telemetry binding per Epic 1b Owns)")]
struct CounterSeries {
    value: AtomicU64,
}

impl CounterSeries {
    fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
}

/// In-memory metrics registry for IAC round-trip telemetry.
#[maos_attrs::i9_exempt(reason = "IAC round-trip telemetry registry (sanctioned persistent location per Epic 1b Owns); Vec<Atomic> are metric accumulators, not mutable kernel state")]
pub struct IacRtMetrics {
    histograms: Vec<(Service, Outcome, HistogramSeries)>,
    inflight: Vec<(Service, AtomicI64)>,
    errors: Vec<(Service, ErrorKind, CounterSeries)>,
}

impl IacRtMetrics {
    /// Create a new metrics registry.
    pub fn new() -> Self {
        let mut histograms = Vec::new();
        let mut inflight = Vec::new();
        let mut errors = Vec::new();

        for svc in [
            Service::Security,
            Service::Memory,
            Service::Iac,
            Service::Capability,
            Service::SpiritScheduler,
        ] {
            inflight.push((svc, AtomicI64::new(0)));
            for out in [Outcome::Ok, Outcome::Err, Outcome::Timeout] {
                histograms.push((svc, out, HistogramSeries::new()));
            }
            for kind in [ErrorKind::Transport, ErrorKind::Decode, ErrorKind::Timeout, ErrorKind::App] {
                errors.push((svc, kind, CounterSeries::new()));
            }
        }

        Self {
            histograms,
            inflight,
            errors,
        }
    }

    /// Record an IAC round-trip duration observation.
    pub fn record_iac_rt(&self, service: Service, outcome: Outcome, duration_us: u64) {
        for (s, o, h) in &self.histograms {
            if *s == service && *o == outcome {
                h.observe(duration_us);
                return;
            }
        }
    }

    /// Record an IAC error.
    pub fn record_iac_error(&self, service: Service, kind: ErrorKind) {
        for (s, k, c) in &self.errors {
            if *s == service && *k == kind {
                c.increment();
                return;
            }
        }
    }

    /// Return an RAII inflight guard.
    pub fn inflight(&self, service: Service) -> InflightGuard {
        for (s, g) in &self.inflight {
            if *s == service {
                g.fetch_add(1, Ordering::Relaxed);
                return InflightGuard {
                    gauge: Some(g),
                };
            }
        }
        // Fallback (should never happen if enum is exhaustive)
        InflightGuard { gauge: None }
    }

    /// Render metrics in Prometheus text format.
    pub fn render_prometheus(&self) -> String {
        let mut out = String::new();

        // Histogram
        for (svc, outc, h) in &self.histograms {
            let label = format!("service=\"{}\",outcome=\"{}\"", svc.as_str(), outc.as_str());
            for (i, boundary) in IAC_RT_BUCKETS_US.iter().enumerate() {
                let le = if *boundary == boundary.trunc() {
                    format!("{:.0}", boundary)
                } else {
                    format!("{}", boundary)
                };
                let _ = writeln!(
                    &mut out,
                    "iac_rt_duration_us_bucket{{{label},le=\"{le}\"}} {}",
                    h.buckets[i].load(Ordering::Relaxed)
                );
            }
            let _ = writeln!(
                &mut out,
                "iac_rt_duration_us_bucket{{{label},le=\"+Inf\"}} {}",
                h.buckets[IAC_RT_BUCKETS_US.len()].load(Ordering::Relaxed)
            );
            let _ = writeln!(
                &mut out,
                "iac_rt_duration_us_sum{{{label}}} {}",
                h.sum.load(Ordering::Relaxed)
            );
            let _ = writeln!(
                &mut out,
                "iac_rt_duration_us_count{{{label}}} {}",
                h.count.load(Ordering::Relaxed)
            );
        }

        // Inflight gauge
        for (svc, g) in &self.inflight {
            let _ = writeln!(
                &mut out,
                "iac_rt_inflight{{service=\"{}\"}} {}",
                svc.as_str(),
                g.load(Ordering::Relaxed)
            );
        }

        // Error counter
        for (svc, kind, c) in &self.errors {
            let _ = writeln!(
                &mut out,
                "iac_rt_errors_total{{service=\"{}\",kind=\"{}\"}} {}",
                svc.as_str(),
                kind.as_str(),
                c.value.load(Ordering::Relaxed)
            );
        }

        out
    }
}

impl Default for IacRtMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII inflight guard — increments on construct, decrements on Drop.
pub struct InflightGuard<'a> {
    gauge: Option<&'a AtomicI64>,
}

impl<'a> Drop for InflightGuard<'a> {
    fn drop(&mut self) {
        if let Some(g) = self.gauge {
            g.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_buckets_equality() {
        let expected: &[f64] = &[
            50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 450.0, 700.0,
            1000.0, 1500.0, 2200.0, 3300.0, 5000.0, 7500.0, 11000.0,
            16000.0, 25000.0,
        ];
        assert_eq!(IAC_RT_BUCKETS_US, expected);
        assert_eq!(IAC_RT_BUCKETS_US.len(), 17);
    }

    #[test]
    fn label_strings_are_snake_case() {
        assert_eq!(Service::Security.as_str(), "security");
        assert_eq!(Service::SpiritScheduler.as_str(), "spirit_scheduler");
        assert_eq!(Outcome::Ok.as_str(), "ok");
        assert_eq!(Outcome::Timeout.as_str(), "timeout");
        assert_eq!(ErrorKind::Transport.as_str(), "transport");
        assert_eq!(ErrorKind::App.as_str(), "app");
    }

    #[test]
    fn render_prometheus_has_required_lines() {
        let metrics = IacRtMetrics::new();
        metrics.record_iac_rt(Service::Capability, Outcome::Ok, 1200);
        metrics.record_iac_error(Service::Capability, ErrorKind::Timeout);
        let rendered = metrics.render_prometheus();
        assert!(rendered.contains("iac_rt_duration_us_bucket{service=\"capability\",outcome=\"ok\",le=\"1500\"}"));
        assert!(rendered.contains("iac_rt_duration_us_count{service=\"capability\",outcome=\"ok\"}"));
        assert!(rendered.contains("iac_rt_duration_us_sum{service=\"capability\",outcome=\"ok\"}"));
        assert!(rendered.contains("iac_rt_inflight{service=\"capability\"}"));
        assert!(rendered.contains("iac_rt_errors_total{service=\"capability\",kind=\"timeout\"}"));
    }

    #[test]
    fn inflight_guard_increments_and_decrements() {
        let metrics = IacRtMetrics::new();
        {
            let _guard = metrics.inflight(Service::Iac);
            let rendered = metrics.render_prometheus();
            assert!(rendered.contains("iac_rt_inflight{service=\"iac\"} 1"));
        }
        let rendered = metrics.render_prometheus();
        assert!(rendered.contains("iac_rt_inflight{service=\"iac\"} 0"));
    }

    #[test]
    fn inflight_guard_decrements_on_early_return() {
        let metrics = IacRtMetrics::new();
        fn inner(metrics: &IacRtMetrics) -> Option<()> {
            let _guard = metrics.inflight(Service::Memory);
            return None; // early return drops guard
        }
        inner(&metrics);
        let rendered = metrics.render_prometheus();
        assert!(rendered.contains("iac_rt_inflight{service=\"memory\"} 0"));
    }

    #[test]
    fn histogram_observation_in_correct_bucket() {
        let metrics = IacRtMetrics::new();
        metrics.record_iac_rt(Service::Security, Outcome::Ok, 45);
        metrics.record_iac_rt(Service::Security, Outcome::Ok, 55);
        metrics.record_iac_rt(Service::Security, Outcome::Ok, 1501);
        let rendered = metrics.render_prometheus();
        // 45us -> bucket le="50"
        assert!(rendered.contains("iac_rt_duration_us_bucket{service=\"security\",outcome=\"ok\",le=\"50\"} 1"));
        // 55us -> bucket le="75"
        assert!(rendered.contains("iac_rt_duration_us_bucket{service=\"security\",outcome=\"ok\",le=\"75\"} 1"));
        // 1501us -> bucket le="2200"
        assert!(rendered.contains("iac_rt_duration_us_bucket{service=\"security\",outcome=\"ok\",le=\"2200\"} 1"));
    }
}
