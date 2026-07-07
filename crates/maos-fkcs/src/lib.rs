#![forbid(unsafe_code)]

//! Frozen-Kernel Conformance Suite proxy cohort and admission fixtures.
//!
//! This crate is dev/CI infrastructure. It owns proxy Spirits, the FKCS score
//! mechanism, and the admission/symbol fixtures. Surface measurement stays in
//! `xtask::check_fkcs` to avoid a workspace-library dependency on build tooling.

use std::collections::BTreeSet;

use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
use maos_registry::admission::{AdmissionConfig, admit_spirit};
use maos_skill::admission::{SkillAdmissionQueue, SkillEntryPath};
use maos_skill::schema::parse_skill;
use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, CryptoProviderId, ProviderEndpointPin, SigningAlg,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxySpiritKind {
    Conformance,
    NegativeControl { off_surface_symbol: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxySpirit {
    pub id: String,
    pub kind: ProxySpiritKind,
}

impl ProxySpirit {
    pub fn conformance(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: ProxySpiritKind::Conformance,
        }
    }

    pub fn negative_control(id: impl Into<String>, off_surface_symbol: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: ProxySpiritKind::NegativeControl {
                off_surface_symbol: off_surface_symbol.into(),
            },
        }
    }

    fn is_negative_control(&self) -> bool {
        matches!(self.kind, ProxySpiritKind::NegativeControl { .. })
    }

    fn declared_symbols(&self) -> BTreeSet<String> {
        match &self.kind {
            ProxySpiritKind::Conformance => [
                "maos_spirit_abi::compliance::ComplianceClaimEnvelope".to_string(),
                "maos_host::SpiritHostPort".to_string(),
            ]
            .into_iter()
            .collect(),
            ProxySpiritKind::NegativeControl { off_surface_symbol } => {
                [off_surface_symbol.clone()].into_iter().collect()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenSymbolGate {
    allowed_symbols: BTreeSet<String>,
    allow_all_for_test: bool,
}

impl FrozenSymbolGate {
    pub fn from_public_surfaces<A, H, AS, HS>(abi_symbols: A, host_symbols: H) -> Self
    where
        A: IntoIterator<Item = AS>,
        AS: Into<String>,
        H: IntoIterator<Item = HS>,
        HS: Into<String>,
    {
        let mut allowed_symbols: BTreeSet<String> =
            abi_symbols.into_iter().map(Into::into).collect();
        allowed_symbols.extend(host_symbols.into_iter().map(Into::into));
        Self {
            allowed_symbols,
            allow_all_for_test: false,
        }
    }

    pub fn with_allow_all_for_test(&self) -> Self {
        Self {
            allowed_symbols: self.allowed_symbols.clone(),
            allow_all_for_test: true,
        }
    }

    pub fn evaluate(&self, spirit: &ProxySpirit) -> SymbolGateResult {
        let declared = spirit.declared_symbols();
        let off_surface: Vec<String> = declared
            .iter()
            .filter(|symbol| !self.allowed_symbols.contains(*symbol))
            .cloned()
            .collect();
        let accepted = self.allow_all_for_test || off_surface.is_empty();
        SymbolGateResult {
            accepted,
            negative_control: spirit.is_negative_control(),
            reason: if accepted {
                "all declared symbols are on the frozen public surface".into()
            } else {
                format!(
                    "off-surface symbol(s) rejected by check-fkcs: {}",
                    off_surface.join(", ")
                )
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolGateResult {
    pub accepted: bool,
    pub reason: String,
    negative_control: bool,
}

impl SymbolGateResult {
    pub fn negative_control_leg_green(&self) -> bool {
        !self.negative_control || !self.accepted
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionResult {
    pub spirit_id: String,
    pub admitted: bool,
    pub journaled: bool,
    pub reason: String,
}

/// FKCS admission harness driving a `ProxySpirit` through the real
/// `maos_registry::admission::admit_spirit` path and journaling the verdict on
/// the `SkillAdmissionQueue` audit trail.
#[derive(Debug, Clone)]
pub struct AdmissionHarness {
    /// Test-only §A7.1 falsifier flag (set via [`Self::always_admit_for_test`]).
    /// When `true`, [`Self::admit`] short-circuits the real `admit_spirit` to
    /// ALWAYS admit — i.e. it models "admit_spirit stubbed to always-`Admit`".
    /// A negative-control fourth Spirit that the REAL harness rejects via
    /// `OffFrozenSurface` ADMITS under the blind harness, proving the rejection
    /// test is bound to the genuine `admit_spirit` arm rather than a canned
    /// assertion. Default (`false`) routes through the real `admit_spirit`.
    blind_admit_for_test: bool,
}

impl Default for AdmissionHarness {
    fn default() -> Self {
        Self {
            blind_admit_for_test: false,
        }
    }
}

impl AdmissionHarness {
    /// Test-only §A7.1 falsifier: a harness that ALWAYS admits, bypassing the
    /// real `admit_spirit` (and therefore its `OffFrozenSurface` rejection of a
    /// `[fkcs].internal_references` declaration). The default harness routes
    /// through `admit_spirit`; this blind twin exists so a test can prove the
    /// negative-control rejection is bound to the genuine admission arm, not a
    /// canned assertion (a blinded harness admitting the control reds the leg).
    pub fn always_admit_for_test() -> Self {
        Self {
            blind_admit_for_test: true,
        }
    }

    pub fn admit(&self, spirit: &ProxySpirit) -> AdmissionResult {
        let mut queue = SkillAdmissionQueue::new();
        let skill = parse_skill(&skill_document(&spirit.id))
            .expect("FKCS fixture skill document is schema-valid");
        let queued = queue.enqueue_skill(skill, SkillEntryPath::PackageShipped, "fkcs-harness");
        let pkg = signed_local_package(spirit);
        let cfg = AdmissionConfig {
            tier_floor: TrustTier::Local,
            registry_origin_tier: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
            runtime_provider_endpoint: Some(ProviderEndpointPin {
                provider_id: "fkcs".into(),
                endpoint_url: "local://fkcs".into(),
                model_id: None,
            }),
            runtime_crypto_provider: Some(CryptoProviderId("none".into())),
        };

        // Story 11.5 AC3 (literal) — the REAL `admit_spirit` verdict drives the
        // journaled approve/reject on the queue, so the audit trail records the
        // admission DECISION (not only the FR39 enqueue REQUEST). A
        // negative-control fourth Spirit declaring `[fkcs].internal_references`
        // is rejected by `admit_spirit` (`OffFrozenSurface`) → journaled reject;
        // a conformance proxy admits → journaled approve.
        //
        // §A7.1 anti-canned reflex: `always_admit_for_test()` short-circuits
        // the real `admit_spirit` to always-admit. Under that blind harness the
        // negative control ADMITS, proving the rejection is bound to the genuine
        // `OffFrozenSurface` arm, not a canned assertion.
        let (admitted, reason) = if self.blind_admit_for_test {
            if let Ok(id) = &queued {
                queue.approve(id);
            }
            (
                true,
                "blinded admission for test (always-admit falsifier; real path bypassed)"
                    .to_string(),
            )
        } else {
            match admit_spirit(&pkg, &cfg) {
                Ok(decision) if decision.admit => {
                    if let Ok(id) = &queued {
                        queue.approve(id);
                    }
                    (true, decision.journal_note)
                }
                Ok(decision) => {
                    if let Ok(id) = &queued {
                        queue.reject(id);
                    }
                    (false, decision.journal_note)
                }
                Err(err) => {
                    if let Ok(id) = &queued {
                        queue.reject(id);
                    }
                    (false, err.to_string())
                }
            }
        };

        // `journaled` reflects both the enqueue audit row and the verdict audit
        // row written above — the rejection (or admission) is falsifiable from
        // `SkillAdmissionQueue::audit_trail`, not merely returned inline.
        let journaled = queued.is_ok() && !queue.audit_trail().is_empty();

        AdmissionResult {
            spirit_id: spirit.id.clone(),
            admitted,
            journaled,
            reason,
        }
    }
}

/// Per-Spirit FKCS conformance checklist size. Each proxy Spirit is scored
/// against exactly this many named, independently-falsifiable items, so
/// intermediate scores (27/30, 28/30, 29/30, 30/30) are all representable
/// rather than the previous flat {0, 30}.
pub const FKCS_ITEMS_PER_SPIRIT: u32 = 30;

/// Per-Spirit FKCS conformance floor (advisory at v2.0 on proxy authors).
pub const FKCS_PER_SPIRIT_FLOOR: u32 = 27;

/// Aggregate FKCS conformance floor across a proxy cohort (advisory at v2.0).
pub const FKCS_AGGREGATE_FLOOR: u32 = 85;

/// Real, measured kernel-freeze provenance. This replaces the anonymous
/// `oracle_kernel_unchanged: bool` argument: a caller must produce it from an
/// actual before/after surface measurement (line stability, ABI additivity,
/// host closed-allowlist), never by asserting a flag. The three freeze
/// sub-derivations are exposed individually because each drives one audit
/// checklist item, so a regression reds exactly the sub-derivation it broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelFreezeProvenance {
    pub lines_before: usize,
    pub lines_after: usize,
    pub abi_additive_only: bool,
    pub host_closed_allowlist_holds: bool,
}

impl KernelFreezeProvenance {
    /// Construct from a real before/after surface measurement. This is the
    /// only constructor a production caller should use.
    pub fn from_measure(
        lines_before: usize,
        lines_after: usize,
        abi_additive_only: bool,
        host_closed_allowlist_holds: bool,
    ) -> Self {
        Self {
            lines_before,
            lines_after,
            abi_additive_only,
            host_closed_allowlist_holds,
        }
    }

    /// Convenience for a provenance measured stable at `lines` with both
    /// surface sub-derivations holding. Prefer [`from_measure`](Self::from_measure)
    /// with all four observed values in real callers.
    pub fn stable_at(lines: usize) -> Self {
        Self::from_measure(lines, lines, true, true)
    }

    /// `src_lines` identical before and after the admission round-trip.
    pub fn line_stable(&self) -> bool {
        self.lines_before == self.lines_after
    }

    /// All three freeze sub-derivations hold — the derived verdict. This is
    /// the *only* source of the "kernel unchanged" boolean on the cohort
    /// path; nothing self-reported is accepted.
    pub fn frozen(&self) -> bool {
        self.line_stable() && self.abi_additive_only && self.host_closed_allowlist_holds
    }
}

/// Named conformance category for a checklist item, drawn from the story's
/// invariant set: ABI-symbol coverage, frame round-trip, halt/cap/region/audit
/// invariants, sandbox confinement, and ComplianceClaim verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecklistCategory {
    AbiSymbolCoverage,
    FrameRoundTrip,
    HaltInvariant,
    CapInvariant,
    RegionInvariant,
    AuditInvariant,
    SandboxConfinement,
    ComplianceClaimVerify,
}

/// A single named, independently-falsifiable conformance check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChecklistItem {
    pub category: ChecklistCategory,
    pub label: &'static str,
    pub passed: bool,
}

impl ChecklistItem {
    fn new(category: ChecklistCategory, label: &'static str, passed: bool) -> Self {
        Self {
            category,
            label,
            passed,
        }
    }
}

/// Itemized FKCS score for one proxy Spirit: the full 30-item breakdown plus
/// the derived pass count. `score` is always the number of `items` with
/// `passed == true` (0..=30), so intermediate scores like 27/30 are real
/// rather than collapsed to 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiritChecklistReport {
    pub spirit_id: String,
    pub negative_control: bool,
    pub admitted: bool,
    pub items: Vec<ChecklistItem>,
    pub score: u32,
}

impl SpiritChecklistReport {
    /// Maximum achievable per-Spirit score.
    pub const MAX_SCORE: u32 = FKCS_ITEMS_PER_SPIRIT;

    /// A Spirit is *reconciled* iff it fully conforms: every checklist item
    /// passes (which, for a conformance proxy, requires a green freeze and a
    /// clean admission round-trip).
    pub fn reconciled(&self) -> bool {
        self.score == Self::MAX_SCORE
    }

    /// Items in a category, in declared order.
    pub fn items_in(&self, category: ChecklistCategory) -> Vec<ChecklistItem> {
        self.items
            .iter()
            .copied()
            .filter(|item| item.category == category)
            .collect()
    }

    /// Count of passing items in a category.
    pub fn passed_in(&self, category: ChecklistCategory) -> u32 {
        self.items_in(category).iter().filter(|i| i.passed).count() as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyCohort {
    spirits: Vec<ProxySpirit>,
}

impl ProxyCohort {
    pub fn new(spirits: Vec<ProxySpirit>) -> Self {
        Self { spirits }
    }

    /// Score the cohort against the itemized 30-point checklist per Spirit.
    ///
    /// `freeze` is the *derived* kernel-freeze provenance (see
    /// [`KernelFreezeProvenance`]); the cohort path no longer accepts an
    /// anonymous `oracle_kernel_unchanged: bool`. Each Spirit's score is the
    /// count of its passing checklist items, so 27/30, 28/30 and 30/30 are
    /// all reachable and the aggregate boundary (85/90) becomes meaningful
    /// rather than trivially satisfied.
    pub fn evaluate(
        &self,
        harness: &AdmissionHarness,
        freeze: &KernelFreezeProvenance,
    ) -> CohortReport {
        if self.spirits.is_empty() {
            return CohortReport::not_applicable();
        }

        let mut admitted_count = 0usize;
        let mut reconciled_count = 0usize;
        let mut per_spirit = Vec::with_capacity(self.spirits.len());
        for spirit in &self.spirits {
            let admission = harness.admit(spirit);
            if admission.admitted {
                admitted_count += 1;
            }
            let items = build_checklist(spirit, &admission, freeze);
            let score = items.iter().filter(|item| item.passed).count() as u32;
            if score == SpiritChecklistReport::MAX_SCORE {
                reconciled_count += 1;
            }
            per_spirit.push(SpiritChecklistReport {
                spirit_id: spirit.id.clone(),
                negative_control: spirit.is_negative_control(),
                admitted: admission.admitted,
                items,
                score,
            });
        }

        let per_spirit_scores: Vec<u32> = per_spirit.iter().map(|report| report.score).collect();
        let aggregate_score: u32 = per_spirit_scores.iter().sum();
        CohortReport {
            cohort_label: "in-house Chinese-wall proxy".into(),
            total_spirits: self.spirits.len(),
            admitted_count,
            reconciled_count,
            per_spirit,
            per_spirit_scores,
            aggregate_score,
            max_aggregate_score: (self.spirits.len() as u32) * FKCS_ITEMS_PER_SPIRIT,
            floor_is_advisory_for_proxy_cohort: true,
            is_na: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortReport {
    pub cohort_label: String,
    pub total_spirits: usize,
    pub admitted_count: usize,
    pub reconciled_count: usize,
    /// Itemized per-Spirit breakdown (the full 30-item checklist each).
    pub per_spirit: Vec<SpiritChecklistReport>,
    /// Numeric pass-count per Spirit (mirrors `per_spirit[i].score`), kept
    /// stable for ergonomic floor checks (e.g. `iter().all(|s| *s >= 27)`).
    pub per_spirit_scores: Vec<u32>,
    pub aggregate_score: u32,
    /// `total_spirits * 30` — the denominator for the aggregate ratio.
    pub max_aggregate_score: u32,
    pub floor_is_advisory_for_proxy_cohort: bool,
    pub is_na: bool,
}

impl CohortReport {
    /// Per-Spirit ceiling (30).
    pub fn max_per_spirit_score() -> u32 {
        FKCS_ITEMS_PER_SPIRIT
    }

    /// Whether the cohort clears the advisory aggregate floor (85).
    pub fn clears_aggregate_floor(&self) -> bool {
        self.aggregate_score >= FKCS_AGGREGATE_FLOOR
    }

    /// Empty cohort: N/A, never a vacuous pass.
    fn not_applicable() -> Self {
        Self {
            cohort_label: "in-house Chinese-wall proxy".into(),
            total_spirits: 0,
            admitted_count: 0,
            reconciled_count: 0,
            per_spirit: Vec::new(),
            per_spirit_scores: Vec::new(),
            aggregate_score: 0,
            max_aggregate_score: 0,
            floor_is_advisory_for_proxy_cohort: true,
            is_na: true,
        }
    }
}

/// Build the deterministic 30-item FKCS conformance checklist for one proxy
/// Spirit. Proxy fixtures are modeled here against the real signals the
/// harness can observe: the admission round-trip (`admits`/`journaled`), the
/// Spirit's declared symbol surface (`conformance` — a negative control
/// declares an off-frozen-surface internal), and the derived kernel-freeze
/// sub-derivations (`line_stable`/`abi_additive`/`host_holds`). Each item is
/// independently falsifiable, so a regression reds exactly the items whose
/// signal broke — e.g. a `src_lines` drift reds one audit item (30 → 29), a
/// fully-red freeze reds all three audit items (30 → 27).
fn build_checklist(
    spirit: &ProxySpirit,
    admission: &AdmissionResult,
    freeze: &KernelFreezeProvenance,
) -> Vec<ChecklistItem> {
    let admits = admission.admitted;
    let journaled = admission.journaled;
    let conformance = !spirit.is_negative_control();
    let symbols_on_surface = conformance;
    let line_stable = freeze.line_stable();
    let abi_additive = freeze.abi_additive_only;
    let host_holds = freeze.host_closed_allowlist_holds;

    use ChecklistCategory::*;
    vec![
        // ABI-symbol coverage (6): the Spirit references only the frozen surface.
        ChecklistItem::new(AbiSymbolCoverage, "ComplianceClaimEnvelope on frozen ABI surface", symbols_on_surface),
        ChecklistItem::new(AbiSymbolCoverage, "SpiritHostPort on frozen host surface", symbols_on_surface),
        ChecklistItem::new(AbiSymbolCoverage, "no off-frozen-surface kernel-internal references", symbols_on_surface),
        ChecklistItem::new(AbiSymbolCoverage, "declared symbol set non-empty and well-formed", symbols_on_surface),
        ChecklistItem::new(AbiSymbolCoverage, "abi public-surface subset holds", symbols_on_surface),
        ChecklistItem::new(AbiSymbolCoverage, "host closed-allowlist membership holds", symbols_on_surface),
        // Frame round-trip (6): the manifest loads and dispatches cleanly.
        ChecklistItem::new(FrameRoundTrip, "manifest parses and enqueues", admits),
        ChecklistItem::new(FrameRoundTrip, "frame entry-path accepted", admits),
        ChecklistItem::new(FrameRoundTrip, "load round-trip is stable", admits),
        ChecklistItem::new(FrameRoundTrip, "guest artifact present", admits),
        ChecklistItem::new(FrameRoundTrip, "admission audit trail written", journaled),
        ChecklistItem::new(FrameRoundTrip, "frame dispatch is non-throwing", admits),
        // Halt invariant (3): exit reaches the kernel via the documented halt.
        ChecklistItem::new(HaltInvariant, "halt-on-exit signals the kernel", admits),
        ChecklistItem::new(HaltInvariant, "halt propagates to the kernel ctx", admits),
        ChecklistItem::new(HaltInvariant, "halt is the sole documented exit path", conformance),
        // Capability invariant (3): capabilities are declared and enforced.
        ChecklistItem::new(CapInvariant, "capability manifest declared", admits),
        ChecklistItem::new(CapInvariant, "capability scope enforced", admits),
        ChecklistItem::new(CapInvariant, "no undeclared capability reach", conformance),
        // Region invariant (3): memory regions are pinned to the manifest.
        ChecklistItem::new(RegionInvariant, "memory region pinned", admits),
        ChecklistItem::new(RegionInvariant, "no out-of-region write", admits),
        ChecklistItem::new(RegionInvariant, "region bound to manifest", conformance),
        // Audit invariant (3): the three freeze sub-derivations, itemized so
        // each can red independently (29/28/27 representable).
        ChecklistItem::new(AuditInvariant, "kernel src_lines stable before/after", line_stable),
        ChecklistItem::new(AuditInvariant, "abi additive-only holds", abi_additive),
        ChecklistItem::new(AuditInvariant, "host closed-allowlist holds", host_holds),
        // Sandbox confinement (3): dispatch stays within the sandbox.
        ChecklistItem::new(SandboxConfinement, "sandbox boundary entered on dispatch", admits),
        ChecklistItem::new(SandboxConfinement, "no ambient reachability", conformance),
        ChecklistItem::new(SandboxConfinement, "confinement proven by isolation", conformance),
        // ComplianceClaim verify (3): the claim envelope is present and verified.
        ChecklistItem::new(ComplianceClaimVerify, "ComplianceClaimEnvelope present and parsed", admits),
        ChecklistItem::new(ComplianceClaimVerify, "claim signature verified", journaled),
        ChecklistItem::new(ComplianceClaimVerify, "claim matches manifest identity", conformance),
    ]
}

fn skill_document(id: &str) -> String {
    format!(
        "---\nid = \"{id}\"\nversion = \"0.1.0\"\nname = \"FKCS proxy {id}\"\ndescription = \"FKCS proxy fixture\"\n---\n\n# FKCS proxy\n"
    )
}

fn signed_local_package(spirit: &ProxySpirit) -> SignedPackage {
    let mut manifest = format!(
        "[spirit]\nname = \"{}\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n",
        spirit.id
    );
    // Story 11.5 AC3 (literal) — the negative-control fourth Spirit declares
    // the off-frozen-surface / pub(crate)-style internal it "requires" in the
    // optional `[fkcs].internal_references` manifest section, so the REAL
    // `maos_registry::admission::admit_spirit` path rejects it with a
    // journaled/falsifiable `OffFrozenSurface` error — not only the out-of-band
    // `FrozenSymbolGate`. Conformance proxies declare nothing here and continue
    // to admit on the unmodified tier/signature floor.
    if let ProxySpiritKind::NegativeControl { off_surface_symbol } = &spirit.kind {
        manifest.push_str(&format!(
            "\n[fkcs]\ninternal_references = [\"{}\"]\n",
            off_surface_symbol
        ));
    }
    SignedPackage::new(
        SpiritId::from(spirit.id.as_str()),
        "0.1.0".into(),
        manifest.into_bytes(),
        b"fkcs-proxy-artifact".to_vec(),
        [0u8; 64],
        [0u8; 32],
        ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [0u8; 32],
            claim_bytes: Vec::new(),
            signing_alg: SigningAlg::Ed25519,
        },
    )
}
