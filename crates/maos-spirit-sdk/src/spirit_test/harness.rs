#![forbid(unsafe_code)]

//! `SpiritTest<S>` — wraps `LocalRunner` (Story 2.3) with halt resolution
//! + manifest self-check + frame capture support.

use crate::local_runner::{LocalRunner, LocalRunnerFixture, RunReport, MockBusFrame};
use crate::{Spirit, SpiritVtable};
use crate::spirit_test::halt::{HaltResolutionKind, HaltResolutionRecord};
use crate::spirit_test::manifest::{ManifestSelfCheckReport, ManifestSelfCheckViolation, manifest_self_check};

/// Extended report carrying everything `RunReport` carries plus the
/// halt resolutions the simulator recorded.
#[derive(Debug, Clone, Default)]
pub struct ExtendedRunReport {
    pub base: RunReport,
    pub halt_resolutions: Vec<HaltResolutionRecord>,
    pub captured_frames: Vec<MockBusFrame>,
}

/// The spirit-test harness. Owns a fixture, an extended report, and
/// the surfaces an author drives the Spirit through.
pub struct SpiritTest<'a, S: Spirit + 'static> {
    pub spirit: &'a S,
    pub vtable: &'a SpiritVtable<S>,
    fixture: LocalRunnerFixture,
    report: ExtendedRunReport,
}

impl<'a, S: Spirit + 'static> SpiritTest<'a, S> {
    /// Construct a new harness around a Spirit + its vtable.
    pub fn new(spirit: &'a S, vtable: &'a SpiritVtable<S>) -> Self {
        Self {
            spirit,
            vtable,
            fixture: LocalRunnerFixture::default(),
            report: ExtendedRunReport::default(),
        }
    }

    /// Mutable access to the underlying fixture so authors can add
    /// frames / telemetry events / schedule payloads / etc.
    pub fn fixture_mut(&mut self) -> &mut LocalRunnerFixture {
        &mut self.fixture
    }

    /// Simulate a halt resolution. Records the resolution in the report.
    /// At v0.3 prerequisite this does NOT yet invoke an
    /// `on_epistemic_resolve` hook (that hook ships at Story 4.1).
    pub fn resolve_halt(&mut self, halt_id: String, kind: HaltResolutionKind) {
        self.report.halt_resolutions.push(HaltResolutionRecord { halt_id, kind });
    }

    /// Run the fixture against the Spirit through the vtable. Returns
    /// an `ExtendedRunReport` carrying the base report + halt resolutions
    /// + captured frames (the last two are populated by the harness
    /// surfaces above; the base report is whatever `LocalRunner` produces).
    pub fn run(mut self) -> ExtendedRunReport {
        let base = LocalRunner::run(self.spirit, self.vtable, &self.fixture);
        self.report.captured_frames = base.mock_bus_frames.clone();
        self.report.base = base;
        self.report
    }

    /// Manifest self-check primitive. Returns a typed report listing
    /// parsed sections + violations + edge-case warnings.
    pub fn manifest_self_check(
        &self,
        manifest_toml_bytes: &[u8],
    ) -> Result<ManifestSelfCheckReport, ManifestSelfCheckViolation> {
        manifest_self_check(manifest_toml_bytes)
    }
}
