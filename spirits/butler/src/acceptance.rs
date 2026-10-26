//! Story 18-4 — the director's acceptance record and what Butler learns
//! from it (D-18-4-01 A), the closed option vocabulary (D-18-4-03 A) and
//! the compressed scene's acceptance fixture (CR-3).
//!
//! The record is personal (FR31): it lives in the principal namespace
//! through the host's pid-fused [`PrincipalMemoryPort`], never in a TL frame;
//! the TL carries a non-personal attestation per decision (decision id and
//! scenario time; review ruling D-18-4-R1).
//!
//! [`PrincipalMemoryPort`]: maos_spirit_sdk::principal::PrincipalMemoryPort

use std::collections::BTreeMap;
use std::sync::Arc;

use maos_spirit_sdk::clock::{parse_utc_offset, ClockSpec, ScenarioClock, NANOS_PER_DAY};
use serde::{Deserialize, Serialize};

/// The principal-namespace schema the record is written under.
pub const ACCEPTANCE_SCHEMA: &str = "butler.acceptance/v1";
/// The key of the record inside [`ACCEPTANCE_SCHEMA`].
pub const ACCEPTANCE_KEY: &str = "ledger";
/// The TL intent of a decision's non-personal attestation frame.
pub const ACCEPTANCE_INTENT: &str = "butler:acceptance";
/// The rolling window, in local days before today.
pub const WINDOW_DAYS: u64 = 7;
/// Decided offers the window needs before it informs `belief_variance`
/// (review ruling D-18-4-R2: 5 of the 7 days, so an undecided day — a halt
/// day, a conflict-free day — does not hand the next week to the proxy).
pub const MIN_WINDOW_OFFERS: usize = 5;

/// Story 18-4 (D-18-4-03 A) — the closed action vocabulary an option names
/// as its `"<action>: <text>"` prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionAction {
    LinearNote,
    Reminder,
    Snooze,
}

impl OptionAction {
    pub const ALL: [OptionAction; 3] = [Self::LinearNote, Self::Reminder, Self::Snooze];

    pub fn name(self) -> &'static str {
        match self {
            Self::LinearNote => "linear_note",
            Self::Reminder => "reminder",
            Self::Snooze => "snooze",
        }
    }
}

/// Split an option into its declared action and its text. An unknown or
/// missing action prefix, or a blank text, is refused.
pub fn parse_option(option: &str) -> Result<(OptionAction, &str), String> {
    let (prefix, text) = option.split_once(':').ok_or_else(|| {
        format!("option `{option}` names no action (expected `<action>: <text>`)")
    })?;
    let action = OptionAction::ALL
        .into_iter()
        .find(|a| a.name() == prefix.trim())
        .ok_or_else(|| {
            format!(
                "option `{option}` names an unknown action `{}` (expected linear_note, reminder or snooze)",
                prefix.trim()
            )
        })?;
    let text = text.trim();
    if text.is_empty() {
        return Err(format!("option `{option}` has no text"));
    }
    Ok((action, text))
}

/// Refuse an option list with an unknown, missing or duplicate action.
pub fn check_options(options: &[String]) -> Result<(), String> {
    let mut seen = Vec::with_capacity(options.len());
    for option in options {
        let (action, _) = parse_option(option)?;
        if seen.contains(&action) {
            return Err(format!("action `{}` is offered twice", action.name()));
        }
        seen.push(action);
    }
    Ok(())
}

/// One decided offer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceRecord {
    /// The local day (days since the epoch at the clock's offset).
    pub local_day: u64,
    /// The scenario (or wall) time the decision was taken at.
    pub sim_ns: u64,
    pub work_mode: String,
    /// `pick:<n>` or `ignored`.
    pub response: String,
    /// The action the picked option declared, or `ignored`.
    pub action: String,
    /// A pick whose action is not `snooze`.
    pub accepted: bool,
}

/// Acceptance in one work mode of the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModeRate {
    pub offers: usize,
    pub accepted: usize,
    pub rate: f32,
}

/// The rolling window's pooled rate and its per-work-mode diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AcceptanceWindow {
    pub offers: usize,
    pub accepted: usize,
    pub rate: f32,
    pub by_mode: BTreeMap<String, ModeRate>,
}

impl AcceptanceWindow {
    /// D-18-4-01 A — once the window holds [`MIN_WINDOW_OFFERS`], the binary
    /// entropy of the pooled rate (≡ a rate below 0.811 when it is above
    /// 0.5; review AA-4): the unconditioned policy's uncertainty about the
    /// director, high exactly when acceptance is mixed.
    pub fn belief_variance(&self) -> Option<f32> {
        (self.offers >= MIN_WINDOW_OFFERS).then(|| binary_entropy(self.rate))
    }
}

/// H₂(p) in bits; 0 at p ∈ {0, 1}.
pub fn binary_entropy(p: f32) -> f32 {
    let term = |q: f32| if q <= 0.0 { 0.0 } else { -q * q.log2() };
    term(p.clamp(0.0, 1.0)) + term(1.0 - p.clamp(0.0, 1.0))
}

fn rate(accepted: usize, offers: usize) -> f32 {
    if offers == 0 {
        0.0
    } else {
        accepted as f32 / offers as f32
    }
}

/// The director's decisions, one per local day.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AcceptanceLedger {
    records: BTreeMap<u64, AcceptanceRecord>,
}

impl AcceptanceLedger {
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Record a decision; a later decision on the same local day replaces it.
    pub fn insert(&mut self, record: AcceptanceRecord) {
        self.records.insert(record.local_day, record);
    }

    /// The latest decided local day.
    pub fn latest_day(&self) -> Option<u64> {
        self.records.keys().next_back().copied()
    }

    /// The window `[today - 7, today - 1]`.
    pub fn window(&self, today: u64) -> AcceptanceWindow {
        let mut window = AcceptanceWindow {
            offers: 0,
            accepted: 0,
            rate: 0.0,
            by_mode: BTreeMap::new(),
        };
        for record in self
            .records
            .range(today.saturating_sub(WINDOW_DAYS)..today)
            .map(|(_, r)| r)
        {
            window.offers += 1;
            window.accepted += usize::from(record.accepted);
            let mode = window
                .by_mode
                .entry(record.work_mode.clone())
                .or_insert(ModeRate {
                    offers: 0,
                    accepted: 0,
                    rate: 0.0,
                });
            mode.offers += 1;
            mode.accepted += usize::from(record.accepted);
        }
        window.rate = rate(window.accepted, window.offers);
        for mode in window.by_mode.values_mut() {
            mode.rate = rate(mode.accepted, mode.offers);
        }
        window
    }

    /// The Markdown the principal namespace holds (one JSON record a line).
    pub fn to_markdown(&self) -> Result<String, String> {
        let mut out = String::from("# Butler acceptance record (butler.acceptance/v1)\n\n");
        for record in self.records.values() {
            let line = serde_json::to_string(record)
                .map_err(|e| format!("acceptance record encode: {e}"))?;
            out.push_str(&line);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parse [`Self::to_markdown`]; a malformed record line is refused.
    pub fn from_markdown(markdown: &str) -> Result<Self, String> {
        let mut ledger = Self::default();
        for (n, line) in markdown.lines().enumerate() {
            if !line.starts_with('{') {
                continue;
            }
            let record: AcceptanceRecord = serde_json::from_str(line)
                .map_err(|e| format!("acceptance record line {}: {e}", n + 1))?;
            ledger.insert(record);
        }
        Ok(ledger)
    }
}

/// A director's response in the fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response {
    /// The 1-based option picked.
    Pick(usize),
    Ignored,
}

impl Response {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw == "ignored" {
            return Ok(Self::Ignored);
        }
        raw.strip_prefix("pick:")
            .and_then(|n| n.parse::<usize>().ok())
            .filter(|n| *n >= 1)
            .map(Self::Pick)
            .ok_or_else(|| format!("response `{raw}` is neither `pick:<n>` (n ≥ 1) nor `ignored`"))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureHeader {
    principal: String,
    start: String,
    utc_offset: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureLine {
    day: i64,
    work_mode: String,
    response: String,
}

/// One fixture day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureDay {
    /// Scene day: `≤ 0` is history, `1..` the scene.
    pub day: i64,
    pub work_mode: String,
    pub response: Response,
    /// The raw response string (`pick:<n>` / `ignored`).
    pub response_raw: String,
}

/// Story 18-4 (CR-3) — `tests/fixtures/j-butler/acceptance-21d.jsonl`: a
/// header `{principal, start, utc_offset}` then one `{day, work_mode,
/// response}` per day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceFixture {
    pub principal: String,
    pub utc_offset_secs: i32,
    /// The local day of scene day 1.
    pub start_local_day: u64,
    pub days: BTreeMap<i64, FixtureDay>,
}

/// Story 18-4 (CR-2) — a loaded compressed scene: the acceptance fixture,
/// the scenario clock every reader shares (set by the driver, never from
/// the wall clock) and the last simulated day.
pub struct Scene {
    pub fixture: AcceptanceFixture,
    pub clock: Arc<ScenarioClock>,
    pub last_day: u32,
}

impl Scene {
    /// Read and validate a scene at boot: whole days, a well-formed fixture,
    /// a line for every reached day — refused typed otherwise.
    pub fn load(clock: &ClockSpec, acceptance: &str) -> Result<Self, String> {
        let last_day = clock
            .whole_days()
            .ok_or("--clock: a Butler scene runs whole simulated days (compressed:<n>d)")?;
        let text = std::fs::read_to_string(acceptance)
            .map_err(|e| format!("--acceptance {acceptance}: {e}"))?;
        let fixture = AcceptanceFixture::parse(&text)?;
        let first = fixture.scene_days(last_day)?[0].day;
        let clock = Arc::new(ScenarioClock::new(
            fixture.day_ns(first),
            fixture.utc_offset_secs,
        ));
        Ok(Self {
            fixture,
            clock,
            last_day,
        })
    }

    /// The days this scene runs.
    pub fn days(&self) -> Result<Vec<&FixtureDay>, String> {
        self.fixture.scene_days(self.last_day)
    }
}

/// The local hour each simulated day's pass runs at.
const SCENE_HOUR: u64 = 8;

impl AcceptanceFixture {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty());
        let (_, header) = lines.next().ok_or("acceptance fixture is empty")?;
        let header: FixtureHeader = serde_json::from_str(header)
            .map_err(|e| format!("acceptance fixture line 1 (header): {e}"))?;
        if header.principal.is_empty()
            || header
                .principal
                .chars()
                .any(|c| c == ':' || c.is_ascii_control())
        {
            return Err(format!(
                "acceptance fixture header: principal `{}` must be non-empty with no `:` or control character",
                header.principal
            ));
        }
        let utc_offset_secs = parse_utc_offset(&header.utc_offset)
            .map_err(|e| format!("acceptance fixture header: {e}"))?;
        let start_local_day = parse_date(&header.start).ok_or_else(|| {
            format!(
                "acceptance fixture header: start `{}` is not YYYY-MM-DD",
                header.start
            )
        })?;
        if day_ns_of(start_local_day, utc_offset_secs).is_none() {
            return Err(format!(
                "acceptance fixture header: start `{}` is past the clock's range",
                header.start
            ));
        }
        let mut days = BTreeMap::new();
        for (n, line) in lines {
            let line: FixtureLine = serde_json::from_str(line)
                .map_err(|e| format!("acceptance fixture line {}: {e}", n + 1))?;
            if line.work_mode.trim().is_empty() {
                return Err(format!(
                    "acceptance fixture line {}: blank work_mode",
                    n + 1
                ));
            }
            let response = Response::parse(&line.response)
                .map_err(|e| format!("acceptance fixture line {}: {e}", n + 1))?;
            // Review BH-5/EC-6 — a day before the epoch or past the clock's
            // range is refused here, never wrapped, clamped or panicked on.
            if local_day_of(start_local_day, line.day)
                .and_then(|local| day_ns_of(local, utc_offset_secs))
                .is_none()
            {
                return Err(format!(
                    "acceptance fixture line {}: day {} is outside the clock's range",
                    n + 1,
                    line.day
                ));
            }
            let day = FixtureDay {
                day: line.day,
                work_mode: line.work_mode,
                response,
                response_raw: line.response,
            };
            if days.insert(day.day, day).is_some() {
                return Err(format!(
                    "acceptance fixture line {}: day {} repeats",
                    n + 1,
                    line.day
                ));
            }
        }
        Ok(Self {
            principal: header.principal,
            utc_offset_secs,
            start_local_day,
            days,
        })
    }

    /// The local day of scene day `day` (bounded at parse, review BH-5/EC-6).
    pub fn local_day(&self, day: i64) -> u64 {
        local_day_of(self.start_local_day, day).unwrap_or(0)
    }

    /// The instant scene day `day`'s pass runs at (08:00 local).
    pub fn day_ns(&self, day: i64) -> u64 {
        day_ns_of(self.local_day(day), self.utc_offset_secs).unwrap_or(0)
    }

    /// History days (`≤ 0`), oldest first.
    pub fn history(&self) -> impl Iterator<Item = &FixtureDay> {
        self.days.range(..=0).map(|(_, d)| d)
    }

    /// The scene days `first..=last`, where `first` is the fixture's first
    /// day ≥ 1 (a resumed scene's fixture starts later); a reached day with
    /// no line is refused.
    pub fn scene_days(&self, last: u32) -> Result<Vec<&FixtureDay>, String> {
        let first = self
            .days
            .range(1..)
            .next()
            .map(|(day, _)| *day)
            .ok_or("acceptance fixture has no scene day (day ≥ 1)")?;
        if first > i64::from(last) {
            return Err(format!(
                "acceptance fixture starts at day {first}, after the clock's last day {last}"
            ));
        }
        (first..=i64::from(last))
            .map(|day| {
                self.days
                    .get(&day)
                    .ok_or_else(|| format!("acceptance fixture has no line for day {day}"))
            })
            .collect()
    }
}

/// Review BH-5/EC-6 — the local day of scene day `day` (`None` before the
/// epoch or on overflow).
fn local_day_of(start_local_day: u64, day: i64) -> Option<u64> {
    let local = i64::try_from(start_local_day)
        .ok()?
        .checked_add(day)?
        .checked_sub(1)?;
    u64::try_from(local).ok()
}

/// Review BH-5/EC-6 — the UTC instant of `local_day`'s 08:00 pass (`None`
/// past the clock's range).
fn day_ns_of(local_day: u64, utc_offset_secs: i32) -> Option<u64> {
    let ns = i128::from(local_day) * i128::from(NANOS_PER_DAY)
        - i128::from(utc_offset_secs) * 1_000_000_000
        + i128::from(SCENE_HOUR * (NANOS_PER_DAY / 24));
    u64::try_from(ns).ok()
}

/// `YYYY-MM-DD` → days since the epoch (proleptic Gregorian).
fn parse_date(raw: &str) -> Option<u64> {
    let mut parts = raw.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&m) || d < 1 || d > month_days[(m - 1) as usize] || y < 1970 {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    u64::try_from(era * 146_097 + doe - 719_468).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_days_since_the_epoch() {
        assert_eq!(parse_date("1970-01-01"), Some(0));
        assert_eq!(parse_date("2000-03-01"), Some(11_017));
        assert_eq!(parse_date("2026-03-02"), Some(20_514));
        for bad in [
            "2026-02-30",
            "2026-13-01",
            "26-03-02",
            "2026-3-2",
            "1969-12-31",
        ] {
            assert_eq!(parse_date(bad), None, "{bad}");
        }
    }

    #[test]
    fn entropy_peaks_at_even_odds() {
        assert_eq!(binary_entropy(1.0), 0.0);
        assert_eq!(binary_entropy(0.0), 0.0);
        assert!((binary_entropy(0.5) - 1.0).abs() < 1e-6);
        assert!(binary_entropy(5.0 / 7.0) > 0.7, "2 of 7 declined halts");
        assert!(binary_entropy(6.0 / 7.0) < 0.7, "1 of 7 declined does not");
    }

    #[test]
    fn options_name_one_declared_action_each() {
        assert_eq!(
            parse_option("snooze: later"),
            Ok((OptionAction::Snooze, "later"))
        );
        for bad in ["later", "email: x", "snooze:", ": x"] {
            assert!(parse_option(bad).is_err(), "{bad}");
        }
        let twice = vec!["snooze: a".to_owned(), "snooze: b".to_owned()];
        assert!(check_options(&twice).unwrap_err().contains("twice"));
    }
}
