#![forbid(unsafe_code)]

//! `local_runner` — fires Spirit lifecycle hooks through the SpiritVtable
//! using a mock Ctx + in-memory mock IAC bus. Zero kernel dependency.
//!
//! Per Story 2.3 (v0.3 NFR-Onb-1 prerequisite): the runner is the substrate
//! Spirit authors test their Spirits against without spinning up a real
//! kernel. Full spirit-test SDK with assertion macros + halt resolution +
//! manifest self-check + class-specific regression corpus is Story 2.4 seed
//! → Story 7.1 full.

use crate::{Ctx, Spirit, SpiritVtable};
use crate::{ConsolidatePayload, FramePayload, SchedulePayload, SwapInPayload, TelemetryEventPayload};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Fixture describing which hooks to fire and with what payloads.
#[derive(Debug, Clone, Default)]
pub struct LocalRunnerFixture {
    pub invoke_on_load: bool,
    pub invoke_on_start: bool,
    pub invoke_on_idle: bool,
    pub invoke_on_pause: bool,
    pub invoke_on_resume: bool,
    pub invoke_on_unload: bool,
    /// Each entry fires one `on_frame` invocation.
    pub frames: Vec<Vec<u8>>,
    /// Each entry fires one `on_telemetry_event` invocation.
    pub telemetry_events: Vec<Vec<u8>>,
    /// Each entry fires one `on_schedule` invocation.
    pub schedule_payloads: Vec<Vec<u8>>,
    /// Each entry fires one `on_swap_in` invocation.
    pub swap_in_payloads: Vec<Vec<u8>>,
    /// Each entry fires one `on_consolidate` invocation.
    pub consolidate_payloads: Vec<Vec<u8>>,
}

/// Forward-anchor type for Story 2.4 full spirit-test SDK. At v0.3
/// prerequisite the runner does NOT actually capture frames from Spirit
/// emits (Spirits have no real capability handles in the mock Ctx, so
/// they cannot emit). The MockBusFrame type exists so Story 2.4 can
/// extend the runner without breaking the LocalRunnerFixture / RunReport
/// public surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockBusFrame {
    pub kind: MockBusFrameKind,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockBusFrameKind {
    /// Reserved for Story 2.4 — IAC frame the Spirit attempted to send.
    Send,
    /// Reserved for Story 2.4 — capability invocation the Spirit attempted.
    CapInvoke,
}

/// Report from a `LocalRunner::run` invocation.
#[derive(Debug, Clone, Default)]
pub struct RunReport {
    /// hook-name → fire count.
    pub hooks_fired: BTreeMap<String, u32>,
    /// Empty at v0.3 prerequisite; populated by Story 2.4's full SDK.
    pub mock_bus_frames: Vec<MockBusFrame>,
    /// hook-name → elapsed wall-clock for that hook's invocations.
    pub elapsed_per_hook: BTreeMap<String, Duration>,
}

/// The local runner — instantiate with no arguments (it's stateless).
pub struct LocalRunner;

impl LocalRunner {
    /// Run the fixture against the Spirit through its vtable. Returns
    /// a report carrying per-hook fire counts, accumulated elapsed
    /// wall-clock, and (forward-anchor) mock bus frames.
    pub fn run<S: Spirit>(
        spirit: &S,
        vtable: &SpiritVtable<S>,
        fixture: &LocalRunnerFixture,
    ) -> RunReport {
        let mut report = RunReport::default();
        let mut ctx = Ctx::mock();

        macro_rules! fire {
            ($name:expr, $expr:expr) => {{
                let start = Instant::now();
                $expr;
                let elapsed = start.elapsed();
                *report.hooks_fired.entry($name.to_string()).or_insert(0) += 1;
                *report.elapsed_per_hook.entry($name.to_string()).or_insert(Duration::ZERO) += elapsed;
            }};
        }

        if fixture.invoke_on_load {
            fire!("on_load", (vtable.on_load)(spirit, &mut ctx));
        }
        if fixture.invoke_on_start {
            fire!("on_start", (vtable.on_start)(spirit, &mut ctx));
        }
        for bytes in &fixture.frames {
            let p = FramePayload { frame_data: bytes.as_slice(), frame_len: bytes.len() };
            fire!("on_frame", (vtable.on_frame)(spirit, &mut ctx, &p));
        }
        if fixture.invoke_on_idle {
            fire!("on_idle", (vtable.on_idle)(spirit, &mut ctx));
        }
        for bytes in &fixture.telemetry_events {
            let p = TelemetryEventPayload { event_data: bytes.as_slice(), event_len: bytes.len() };
            fire!("on_telemetry_event", (vtable.on_telemetry_event)(spirit, &mut ctx, &p));
        }
        for bytes in &fixture.schedule_payloads {
            let p = SchedulePayload { schedule_data: bytes.as_slice(), schedule_len: bytes.len() };
            fire!("on_schedule", (vtable.on_schedule)(spirit, &mut ctx, &p));
        }
        for bytes in &fixture.swap_in_payloads {
            let p = SwapInPayload { predecessor_state: bytes.as_slice(), state_len: bytes.len() };
            fire!("on_swap_in", (vtable.on_swap_in)(spirit, &mut ctx, &p));
        }
        if fixture.invoke_on_pause {
            fire!("on_pause", (vtable.on_pause)(spirit, &mut ctx));
        }
        if fixture.invoke_on_resume {
            fire!("on_resume", (vtable.on_resume)(spirit, &mut ctx));
        }
        for bytes in &fixture.consolidate_payloads {
            let p = ConsolidatePayload { batch_data: bytes.as_slice(), batch_len: bytes.len() };
            fire!("on_consolidate", (vtable.on_consolidate)(spirit, &mut ctx, &p));
        }
        if fixture.invoke_on_unload {
            fire!("on_unload", (vtable.on_unload)(spirit, &mut ctx));
        }

        report
    }
}
