//! Story 18-4 (CR-1, D-18-4-06 A) — the injectable clock a Spirit reads
//! instead of `SystemTime`: the wall clock at a fixed UTC offset in
//! production, a driver-set scenario clock in a compressed scene.

use std::sync::atomic::{AtomicU64, Ordering};

/// Nanoseconds in a day.
pub const NANOS_PER_DAY: u64 = 86_400 * 1_000_000_000;

/// The env var [`SystemClock::from_env`] reads once at boot (`±HH:MM`).
pub const UTC_OFFSET_ENV: &str = "MAOS_UTC_OFFSET";

/// A Spirit's time source: an instant and the fixed offset of the
/// director's local day (DST is not modelled).
pub trait Clock: Send + Sync {
    /// Nanoseconds since the Unix epoch.
    fn now_ns(&self) -> u64;
    /// The local offset from UTC, in seconds.
    fn utc_offset_secs(&self) -> i32;
}

/// The local day (days since the epoch) of `now_ns` at `utc_offset_secs`.
pub fn local_day(now_ns: u64, utc_offset_secs: i32) -> u64 {
    let local = i128::from(now_ns) + i128::from(utc_offset_secs) * 1_000_000_000;
    (local.max(0) / i128::from(NANOS_PER_DAY)) as u64
}

/// The UTC instant local day `day` starts at.
pub fn local_day_start_ns(day: u64, utc_offset_secs: i32) -> u64 {
    let start =
        i128::from(day) * i128::from(NANOS_PER_DAY) - i128::from(utc_offset_secs) * 1_000_000_000;
    start.clamp(0, i128::from(u64::MAX)) as u64
}

/// A clock argument or setting that does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockError {
    /// `MAOS_UTC_OFFSET` (or a fixture offset) is not `±HH:MM` within ±18:00.
    UtcOffset(String),
    /// `--clock` is not `compressed:<n>(m|h|d)` with `n ≥ 1`.
    Spec(String),
}

impl std::fmt::Display for ClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UtcOffset(raw) => write!(
                f,
                "malformed UTC offset `{raw}` — expected ±HH:MM within ±18:00 (e.g. +09:00)"
            ),
            Self::Spec(raw) => write!(
                f,
                "malformed clock `{raw}` — expected compressed:<n>(m|h|d), n ≥ 1"
            ),
        }
    }
}

impl std::error::Error for ClockError {}

/// Parse `±HH:MM` into seconds east of UTC.
pub fn parse_utc_offset(raw: &str) -> Result<i32, ClockError> {
    let error = || ClockError::UtcOffset(raw.to_owned());
    let (sign, rest) = match raw.as_bytes().first() {
        Some(b'+') => (1, &raw[1..]),
        Some(b'-') => (-1, &raw[1..]),
        _ => return Err(error()),
    };
    let (hours, minutes) = rest.split_once(':').ok_or_else(error)?;
    let digits = |s: &str| s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(hours) || !digits(minutes) {
        return Err(error());
    }
    let (hours, minutes): (i32, i32) = (
        hours.parse().map_err(|_| error())?,
        minutes.parse().map_err(|_| error())?,
    );
    let secs = hours * 3600 + minutes * 60;
    if minutes > 59 || secs > 18 * 3600 {
        return Err(error());
    }
    Ok(sign * secs)
}

/// The wall clock at a fixed offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemClock {
    utc_offset_secs: i32,
}

impl SystemClock {
    /// The wall clock in UTC.
    pub const UTC: Self = Self { utc_offset_secs: 0 };

    pub fn new(utc_offset_secs: i32) -> Self {
        Self { utc_offset_secs }
    }

    /// Read [`UTC_OFFSET_ENV`] once: unset is UTC; malformed is refused.
    pub fn from_env() -> Result<Self, ClockError> {
        match std::env::var(UTC_OFFSET_ENV) {
            Err(std::env::VarError::NotPresent) => Ok(Self::UTC),
            Err(std::env::VarError::NotUnicode(raw)) => {
                Err(ClockError::UtcOffset(raw.to_string_lossy().into_owned()))
            }
            Ok(raw) => parse_utc_offset(&raw).map(Self::new),
        }
    }
}

impl Clock for SystemClock {
    fn now_ns(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos() as u64)
    }

    fn utc_offset_secs(&self) -> i32 {
        self.utc_offset_secs
    }
}

/// A clock the scene driver sets; shared by every reader of one scene.
#[derive(Debug)]
pub struct ScenarioClock {
    now_ns: AtomicU64,
    utc_offset_secs: i32,
}

impl ScenarioClock {
    pub fn new(now_ns: u64, utc_offset_secs: i32) -> Self {
        Self {
            now_ns: AtomicU64::new(now_ns),
            utc_offset_secs,
        }
    }

    /// Move the scene to `now_ns` (forward or back).
    pub fn set_ns(&self, now_ns: u64) {
        self.now_ns.store(now_ns, Ordering::SeqCst);
    }
}

impl Clock for ScenarioClock {
    fn now_ns(&self) -> u64 {
        self.now_ns.load(Ordering::SeqCst)
    }

    fn utc_offset_secs(&self) -> i32 {
        self.utc_offset_secs
    }
}

/// The unit of a compressed scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockUnit {
    Minutes,
    Hours,
    Days,
}

/// A `--clock` argument: `compressed:<n>(m|h|d)` — `n` simulated units
/// driven as fast as the scene can run them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSpec {
    pub count: u32,
    pub unit: ClockUnit,
}

impl ClockSpec {
    pub fn parse(raw: &str) -> Result<Self, ClockError> {
        let error = || ClockError::Spec(raw.to_owned());
        let body = raw.strip_prefix("compressed:").ok_or_else(error)?;
        let (count, unit) = body.split_at(body.len().saturating_sub(1));
        let unit = match unit {
            "m" => ClockUnit::Minutes,
            "h" => ClockUnit::Hours,
            "d" => ClockUnit::Days,
            _ => return Err(error()),
        };
        if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
            return Err(error());
        }
        match count.parse::<u32>() {
            Ok(count) if count >= 1 => Ok(Self { count, unit }),
            _ => Err(error()),
        }
    }

    /// The scene's length in whole days, when its unit is days.
    pub fn whole_days(&self) -> Option<u32> {
        (self.unit == ClockUnit::Days).then_some(self.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_parse_and_refuse() {
        assert_eq!(parse_utc_offset("+00:00"), Ok(0));
        assert_eq!(parse_utc_offset("+09:30"), Ok(9 * 3600 + 1800));
        assert_eq!(parse_utc_offset("-05:00"), Ok(-5 * 3600));
        for bad in [
            "", "09:00", "+9:00", "+09:60", "+19:00", "+09-00", "+0a:00", "UTC",
        ] {
            assert!(parse_utc_offset(bad).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn local_day_follows_the_offset() {
        let midnight_utc = 20_000 * NANOS_PER_DAY;
        assert_eq!(local_day(midnight_utc, 0), 20_000);
        assert_eq!(local_day(midnight_utc - 1, 0), 19_999);
        assert_eq!(local_day(midnight_utc - 3_600_000_000_000, 3600), 20_000);
        assert_eq!(local_day(midnight_utc, -3600), 19_999);
        assert_eq!(
            local_day_start_ns(20_000, 3600),
            midnight_utc - 3_600_000_000_000
        );
        assert_eq!(
            local_day(local_day_start_ns(20_000, -18_000), -18_000),
            20_000
        );
    }

    #[test]
    fn scenario_clock_is_driver_set() {
        let clock = ScenarioClock::new(5, 60);
        assert_eq!((clock.now_ns(), clock.utc_offset_secs()), (5, 60));
        clock.set_ns(7);
        assert_eq!(clock.now_ns(), 7);
    }

    #[test]
    fn clock_specs_parse_and_refuse() {
        let spec = ClockSpec::parse("compressed:21d").unwrap();
        assert_eq!(
            (spec.count, spec.unit, spec.whole_days()),
            (21, ClockUnit::Days, Some(21))
        );
        assert_eq!(
            ClockSpec::parse("compressed:90m").unwrap().whole_days(),
            None
        );
        assert_eq!(
            ClockSpec::parse("compressed:2h").unwrap().unit,
            ClockUnit::Hours
        );
        for bad in [
            "21d",
            "compressed:",
            "compressed:d",
            "compressed:0d",
            "compressed:-1d",
            "compressed:3w",
            "compressed:1.5d",
        ] {
            assert!(ClockSpec::parse(bad).is_err(), "{bad:?} must be refused");
        }
    }
}
