#![forbid(unsafe_code)]

//! Atomic orchestration helper for scalar-write → policy-eval → halt-invocation.
//!
//! Sequences the full kernel-side flow in one call:
//! `set_scalar` → `scalar.tap` publish → policy evaluation → `invoke_halt`.
//!
//! ## Classification target
//! universal-arithmetic (§4.0.7)

use std::sync::Arc;

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::HaltReceipt;
use maos_domain::ports::CapabilityRegistryPort;

use crate::capability::working_memory::policy_runtime::{
    evaluate_after_set_scalar, PolicyEvaluationOutcome,
};
use crate::capability::CapabilityRegistryAdapter;
use crate::halt::{invoke_halt, HaltRegistry};
use crate::iac::transparency_log::TransparencyLogAdapter;
use crate::journal::JournalAdapter;
use crate::security::manifest::EpistemicPolicySection;

/// Orchestrates the atomic scalar-write pipeline.
///
/// Holds the capability registry and halt registry. The transparency log
/// and journal are passed by reference because `JournalAdapter` owns a
/// file handle and does not implement `Clone`.
#[maos_attrs::i9_exempt(
    reason = "working-memory orchestrator holding Arc handles to the already-exempt capability + halt registries; supervised composite per I9, no parameter drift (Story 7.1.7 baseline-reset)"
)]
pub struct WorkingMemoryOrchestrator {
    capability: Arc<CapabilityRegistryAdapter>,
    halt_registry: Arc<HaltRegistry>,
}

impl WorkingMemoryOrchestrator {
    pub fn new(
        capability: Arc<CapabilityRegistryAdapter>,
        halt_registry: Arc<HaltRegistry>,
    ) -> Self {
        Self {
            capability,
            halt_registry,
        }
    }

    /// Atomic entry point: validate → persist → telemetry → evaluate → halt.
    ///
    /// Returns the `HaltReceipt` when a halt is triggered, `None` otherwise.
    pub fn process_scalar_write(
        &self,
        tl: &TransparencyLogAdapter,
        journal: &JournalAdapter,
        spirit_pid: u32,
        spirit_id: &str,
        boot_nonce: u64,
        tag: &str,
        value: f64,
        derived_from: &str,
        policy: &EpistemicPolicySection,
    ) -> Result<Option<HaltReceipt>, Box<dyn std::error::Error>> {
        // Step 1: persist + publish tap
        let event = self
            .capability
            .set_scalar(spirit_pid, spirit_id, tag, value, derived_from)?;

        // Step 2: evaluate policy
        let outcome = evaluate_after_set_scalar(
            spirit_id,
            spirit_pid,
            boot_nonce,
            tag,
            value,
            derived_from,
            policy,
            &*self.capability as &dyn CapabilityRegistryPort,
        )?;

        // Step 3: invoke halt if predicate fired
        match outcome {
            Some(PolicyEvaluationOutcome::Halt(payload)) => {
                let receipt = invoke_halt(
                    tl,
                    journal,
                    &self.halt_registry,
                    payload,
                    spirit_pid,
                    spirit_id,
                    boot_nonce,
                )?;
                Ok(Some(receipt))
            }
            _ => Ok(None),
        }
    }

    /// Story 4.3 — Publish a marker scalar without policy evaluation.
    ///
    /// Used by the `ProvidedContext` halt-resolution arm to emit the
    /// `halt.context_provided` marker so the Spirit's epistemic_policy
    /// can detect that a halt was resolved with context.  The marker is
    /// informational — no halt trigger — so policy evaluation is skipped.
    pub fn publish_scalar_marker(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.capability
            .set_scalar(spirit_pid, spirit_id, tag, value, derived_from)?;
        Ok(())
    }
}
