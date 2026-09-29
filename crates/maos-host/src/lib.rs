#![forbid(unsafe_code)]

//! `maos-host` — Spirit Host Port for daemon-side form→launch-plan resolution.
//!
//! # Architecture (Story 11.1a, ADR-031)
//!
//! The kernel is form-agnostic: every launch primitive runs an executable path
//! (`Command::new(&spec.program)`), and `spawn_and_bridge` confines it under the
//! admitted `BridgeSpawnSpec::sandbox` (T2 on Linux — Story 17-6 AC5). Neither
//! spec has a form discriminator. The form-specific knowledge (where the WASM
//! component runner lives, how to invoke it, fuel/epoch config) stays OUT of it.
//!
//! This crate defines the `SpiritHostPort` trait and its plan types. The
//! trait resolves a `SpiritLaunchRequest` (the manifest's form + artifact)
//! into a `SpiritLaunchPlan` that the kernel's **existing** `spawn_and_bridge`
//! launches unchanged.
//!
//! - **Native subprocess** form → identity resolution (`program = artifact`).
//! - **WASM component** form → `program = <component-runner>`,
//!   `argv = [--component, <wasm>, --fuel, <n>]`.
//!
//! # Decision D1 (11.1a preflight)
//!
//! This trait lives here in `maos-host` (daemon-side), NOT in `maos-domain`.
//! The program-bearing spawn input (`BridgeSpawnSpec`) is consumed only at
//! the daemon composition root (`maos-bin/src/main.rs`). A trait the kernel
//! never calls must NOT be pinned into the frozen ABI surface.
//!
//! # Decision D2 (11.1a preflight)
//!
//! The wasmtime adapter implementing this trait lives in a SEPARATE crate
//! `maos-wasm-host` (ADR-041 isolation, cargo-deny dependency-closure
//! containment). `maos-bin` depends on both.
//!
//! # Zero-kernel-delta guarantee
//!
//! This crate has no dependency on `maos-kernel-core` or `maos-domain`.
//! Adding or modifying it cannot change `check-kernel-baseline` (22964).

/// The Spirit authoring forms hosted at v2.0.
///
/// Native subprocess is the v0.1 form (ADR-002); `WasmComponent` is the v2.0
/// addition (ADR-031), to run as a subprocess under T2 through `spawn_and_bridge`
/// (the route exists since Story 17-6; its first production caller is 17-3c) —
/// the WASM component sandbox (WIT capability gating, fuel/epoch limits) composes
/// ON TOP of that OS process boundary (defense in depth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpiritForm {
    /// ADR-002 native form: `program` is a resolved binary run directly.
    NativeSubprocess,
    /// ADR-031 form: `program` is a wasmtime component runner; the manifest's
    /// resolved artifact is a `.wasm` module passed as argv.
    WasmComponent,
}

/// The on-wire framing both forms speak.
///
/// Always ADR-032 at v2.0 — a WASM Spirit does NOT get a second wire
/// (ADR-031 §3: the WIT is a typed projection of the ADR-032 frame set,
/// the bytes stay Content-Length + CBOR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WireShape {
    /// LSP-style `Content-Length: <n>\r\n\r\n` + N bytes CBOR (ADR-032).
    ContentLengthCbor,
}

/// What the composition root asks the port to resolve.
///
/// Carries the Spirit's declared form plus the manifest-resolved artifact
/// path (binary for native, `.wasm` for WASM) and any form-config the
/// manifest carried.
#[derive(Debug, Clone)]
pub struct SpiritLaunchRequest {
    /// The declared Spirit form.
    pub form: SpiritForm,
    /// The manifest-resolved artifact: an executable path (native) or a
    /// `.wasm`/`.wat` component path (WASM).
    pub artifact: String,
    /// Manifest-declared form config (e.g. `fuel`/`epoch` budget for WASM).
    /// Opaque to the kernel; interpreted only by the adapter.
    pub form_config: Vec<(String, String)>,
}

/// The concrete launch plan the kernel's existing subprocess bridge consumes.
///
/// `program` + `argv` + `env` are the inputs `BridgeSpawnSpec` takes; the tier
/// travels in its `sandbox` field as admission's grant (Story 17-6), not here.
#[derive(Debug, Clone)]
pub struct SpiritLaunchPlan {
    /// The executable to run (binary for native, component-runner for WASM).
    pub program: String,
    /// Additional arguments prepended before the task args.
    pub argv: Vec<String>,
    /// Additional environment variables for the subprocess.
    pub env: Vec<(String, String)>,
    /// The wire shape both sides speak (always `ContentLengthCbor` at v2.0).
    pub wire: WireShape,
}

/// Typed, halt-safe error — mirrors `CollectivePortError` (no panic, no hang).
#[derive(Debug, thiserror::Error)]
pub enum SpiritHostError {
    /// The host runtime (e.g. the wasmtime component runner binary) is
    /// unavailable or unreachable.
    #[error("spirit host unreachable: {reason}")]
    Unreachable { reason: String },

    /// Component validation/instantiation failed (bad `.wasm`, WIT mismatch,
    /// unsupported imports). The composition root maps this to a typed
    /// admission rejection.
    #[error("spirit component invalid: {reason}")]
    InvalidComponent { reason: String },

    /// Resolution timed out (e.g. a slow AOT compile step).
    #[error("spirit host timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

/// Sync port trait for resolving a Spirit form into a launchable plan.
///
/// Injected at the daemon composition root as `Option<Arc<dyn SpiritHostPort>>`
/// next to the Loom-lite `CollectiveMemoryPort` (`maos-bin/src/main.rs:1683`).
/// When `None`, only the native form is launchable (the kernel default).
///
/// - ADR-006/ADR-041: user-space, replaceable; the kernel mediates the
///   launch, the port resolves the form.
/// - ADR-031: in-kernel/in-process wasmtime embedding is FORBIDDEN — the
///   runner is always a subprocess.
pub trait SpiritHostPort: Send + Sync {
    /// Resolve a launch request into a concrete subprocess launch plan.
    ///
    /// For `NativeSubprocess`, this is identity (`program = artifact`).
    /// For `WasmComponent`, the adapter validates the component against the
    /// `maos:spirit@1.0` WIT world (real wasmtime parse + instantiate probe,
    /// bounded by a timeout), then returns `program = <runner>`,
    /// `argv = [--component, <artifact>, --fuel, <n>, ...]`.
    fn resolve_launch(
        &self,
        request: &SpiritLaunchRequest,
    ) -> Result<SpiritLaunchPlan, SpiritHostError>;

    /// The forms this host can resolve (for capability/manifest validation
    /// at admission time). Always includes `NativeSubprocess`.
    fn supported_forms(&self) -> &[SpiritForm];
}
