// SPIKE — Story 11.0 (NO-MERGE). Illustrative skeleton, not compiled.
// At 11.1a this would live at `crates/maos-domain/src/ports/spirit_host.rs`.
// Mirrors the Story-10.4a `CollectiveMemoryPort` contract
// (`crates/maos-domain/src/ports/collective_memory.rs`): a SYNC port trait in
// `maos-domain` (zero kernel-core lines, zero async deps), implemented in a
// user-space adapter crate, injected at the daemon composition root.

//! Spirit Host Port — sync trait that resolves a Spirit's declared *form* into
//! a concrete, form-agnostic subprocess launch plan (program + argv + env).
//!
//! # Why a resolver, not a launcher
//!
//! The kernel is already form-agnostic: every launch primitive runs a bare
//! executable path (`Command::new(&spec.program)`,
//! `maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:461`). `SandboxSpec`
//! carries the sandbox *tier* but no `program` field and no form discriminator.
//! So the form-specific knowledge (where the WASM component runner is, how to
//! invoke it for a given `.wasm`, fuel/epoch config) belongs OUT of the kernel.
//! This port resolves a `SpiritLaunchRequest` (the manifest's form + program)
//! into a `SpiritLaunchPlan` that the kernel's EXISTING `spawn_and_bridge`
//! launches unchanged. Native subprocess form → identity resolution. WASM
//! form → program = the component runner, argv = [`--component`, module, fuel].
//!
//! ADR-031 (WASM-in-subprocess; host-as-adapter), ADR-006/ADR-041 (out of
//! kernel), ADR-032 (wire unchanged — both forms speak Content-Length + CBOR).

/// The Spirit authoring forms hosted at v2.0. Native subprocess is the v0.1
/// form (ADR-002); `WasmComponent` is the v2.0 addition (ADR-031). Both are
/// hosted as subprocesses sandboxed by the existing T2 path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiritForm {
    /// ADR-002 native form: `program` is a resolved binary run directly.
    NativeSubprocess,
    /// ADR-031 form: `program` is a wasmtime component runner; the manifest's
    /// resolved artifact is a `.wasm` module passed as argv.
    WasmComponent,
}

/// The on-wire framing both forms speak. Always ADR-032 at v2.0 — a WASM Spirit
/// does NOT get a second wire (ADR-031 §3: the WIT is a typed projection of the
/// ADR-032 frame set, the bytes stay Content-Length + CBOR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireShape {
    /// LSP-style `Content-Length: <n>\r\n\r\n` + N bytes CBOR (ADR-032).
    ContentLengthCbor,
}

/// What the composition root asks the port to resolve: the Spirit's declared
/// form plus the manifest-resolved artifact path (binary for native, `.wasm`
/// for WASM) and any form-config the manifest carried.
#[derive(Debug, Clone)]
pub struct SpiritLaunchRequest {
    pub form: SpiritForm,
    /// The manifest-resolved artifact: an executable path (native) or a
    /// `.wasm`/`.wat` component path (WASM).
    pub artifact: String,
    /// Manifest-declared form config (e.g. fuel/epoch budget for WASM). Opaque
    /// to the kernel; interpreted only by the adapter.
    pub form_config: Vec<(String, String)>,
}

/// The concrete launch plan the kernel's existing subprocess bridge consumes.
/// `program` + `argv` + `env` are exactly the inputs `BridgeSpawnSpec` already
/// takes (`runtime.rs:242`) — the kernel needs no new field and no form logic.
#[derive(Debug, Clone)]
pub struct SpiritLaunchPlan {
    pub program: String,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub wire: WireShape,
}

/// Typed, halt-safe error — mirrors `CollectivePortError` (no panic, no hang).
#[derive(Debug, thiserror::Error)]
pub enum SpiritHostError {
    /// The host runtime (e.g. the wasmtime component runner) is unavailable.
    #[error("spirit host unreachable: {reason}")]
    Unreachable { reason: String },

    /// Component validation/instantiation failed (bad `.wasm`, WIT mismatch).
    #[error("spirit component invalid: {reason}")]
    InvalidComponent { reason: String },

    /// Resolution timed out (e.g. a slow AOT compile).
    #[error("spirit host timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

/// Sync port trait for resolving a Spirit form into a launchable plan.
///
/// Injected at the daemon composition root as `Option<Arc<dyn SpiritHostPort>>`
/// next to the Loom-lite `CollectiveMemoryPort` (`maos-bin/src/main.rs:1683`).
/// When `None`, only the native form is launchable (the kernel default).
///
/// - ADR-006/ADR-041: user-space, replaceable; the kernel mediates the launch,
///   the port resolves the form.
/// - ADR-031: in-kernel/in-process wasmtime embedding is FORBIDDEN — the runner
///   is always a subprocess.
pub trait SpiritHostPort: Send + Sync {
    /// Resolve a launch request into a concrete subprocess launch plan.
    ///
    /// For `NativeSubprocess`, this is identity (`program = artifact`). For
    /// `WasmComponent`, the adapter validates the component against the
    /// `maos:spirit@1.0` WIT world, then returns `program = <runner>`,
    /// `argv = [--component, <artifact>, --fuel, <n>, ...]`. Any blocking work
    /// (component compile) is offloaded inside the adapter via the held
    /// `tokio::runtime::Handle` + `block_on_or_typed` guard.
    fn resolve_launch(
        &self,
        request: &SpiritLaunchRequest,
    ) -> Result<SpiritLaunchPlan, SpiritHostError>;

    /// The forms this host can resolve (for capability/manifest validation at
    /// admission time). Always includes `NativeSubprocess`.
    fn supported_forms(&self) -> &[SpiritForm];
}
