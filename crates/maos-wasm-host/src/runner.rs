//! `maos-wasm-runner` — wasmtime component runner subprocess.
//!
//! This binary is `BridgeSpawnSpec.program` for WASM Spirits. It speaks ADR-032
//! (Content-Length + CBOR) over stdio and is launched by the kernel's existing
//! `spawn_and_bridge` path.
//!
//! # Usage
//!
//! ```text
//! maos-wasm-runner --component <path.wasm> --fuel <n>
//! ```
//!
//! # Architecture
//!
//! 1. Parse CLI arguments and create a fuel-metered wasmtime engine.
//! 2. Compile the component under a watchdog, then reject an incompatible
//!    `maos:spirit/frames` import before instantiation.
//! 3. Instantiate it against the `maos:spirit@2.0.0` bindings, call
//!    `on-start`, and exchange typed frames with `handle-frame`.
//! 4. On EOF or a guest-returned `Halt`, call `on-shutdown` and use the
//!    cause-distinguishing exit codes below.
//!
//! # Process boundary
//!
//! The runner runs inside ADR-031's T2 process boundary once launched by
//! `spawn_and_bridge`; the production launch is
//! `17-3c-wasm-spirit-on-the-bus-under-t2`. Wasmtime fuel metering is
//! defense-in-depth, not a substitute for that process boundary.

// `runner-fault-inject` is dev/CI-only: it adds a forbidden-syscall probe arm.
// A release build (`not(debug_assertions)`) with the feature MUST NOT compile.
#[cfg(all(feature = "runner-fault-inject", not(debug_assertions)))]
compile_error!(
    "runner-fault-inject is a dev/CI-only fault-injection feature and MUST NOT \
     appear in release builds (Story 17.3c ship-blocker)."
);

use std::io::{self, BufReader, BufWriter, Read};
use std::process::ExitCode;

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

use maos_wasm_host::wit_guest::Spirit;

/// Distinguishable exit codes so the parent (kernel) can attribute the kill
/// cause from the exit status alone, without parsing stderr text. AC4/D6:
/// fuel exhaustion must be attributable via a DERIVED cause, not a bare
/// `exit_code != 0`.
#[repr(u8)]
enum RunnerExit {
    Ok = 0,
    /// Generic I/O / arg-parsing failure.
    GenericError = 1,
    /// Component imports an unsupported `maos:spirit/frames` major version.
    IncompatibleWorld = 2,
    /// The supplied artifact is not a conformant `maos:spirit@2.0.0` component.
    InvalidComponent = 3,
    /// Wasmtime fuel was exhausted during a guest call (`Trap::OutOfFuel`).
    OutOfFuel = 4,
    /// An inbound domain frame has a future variant absent from the WIT world.
    UnrepresentableFrame = 5,
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("maos-wasm-runner: {e}");
            return ExitCode::from(RunnerExit::GenericError as u8);
        }
    };

    match run(args) {
        Ok(()) => ExitCode::from(RunnerExit::Ok as u8),
        Err(RunError::IncompatibleWorld(import)) => {
            eprintln!(
                "maos-wasm-runner: IncompatibleWorld: component imports {import}; \
                 this runner implements maos:spirit@2.0.0"
            );
            ExitCode::from(RunnerExit::IncompatibleWorld as u8)
        }
        Err(RunError::InvalidComponent(reason)) => {
            eprintln!("maos-wasm-runner: InvalidComponent: {reason}");
            ExitCode::from(RunnerExit::InvalidComponent as u8)
        }
        Err(RunError::OutOfFuel) => {
            eprintln!("maos-wasm-runner: OutOfFuel");
            ExitCode::from(RunnerExit::OutOfFuel as u8)
        }
        Err(RunError::UnrepresentableFrame(ty)) => {
            eprintln!("maos-wasm-runner: UnrepresentableFrame: {ty}");
            ExitCode::from(RunnerExit::UnrepresentableFrame as u8)
        }
        Err(RunError::Other(e)) => {
            eprintln!("maos-wasm-runner: {e}");
            ExitCode::from(RunnerExit::GenericError as u8)
        }
    }
}

struct RunnerArgs {
    component_path: String,
    fuel: u64,
    #[cfg(all(feature = "runner-fault-inject", debug_assertions, target_os = "linux"))]
    forbidden_syscall_after_ready: bool,
}

enum RunError {
    IncompatibleWorld(String),
    InvalidComponent(String),
    OutOfFuel,
    UnrepresentableFrame(&'static str),
    Other(String),
}

impl From<String> for RunError {
    fn from(s: String) -> Self {
        RunError::Other(s)
    }
}

fn parse_args() -> Result<RunnerArgs, String> {
    let mut component_path = None;
    let mut fuel = 10_000_000u64;
    #[cfg(all(feature = "runner-fault-inject", debug_assertions, target_os = "linux"))]
    let mut forbidden_syscall_after_ready = false;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--component" => {
                component_path = Some(
                    iter.next()
                        .ok_or_else(|| "--component requires a value".to_string())?,
                );
            }
            "--fuel" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "--fuel requires a value".to_string())?;
                fuel = v
                    .parse()
                    .map_err(|e| format!("invalid --fuel value '{v}': {e}"))?;
            }
            #[cfg(all(feature = "runner-fault-inject", debug_assertions, target_os = "linux"))]
            "--test-forbidden-syscall-after-ready" => forbidden_syscall_after_ready = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(RunnerArgs {
        component_path: component_path.ok_or_else(|| "--component is required".to_string())?,
        fuel,
        #[cfg(all(feature = "runner-fault-inject", debug_assertions, target_os = "linux"))]
        forbidden_syscall_after_ready,
    })
}

/// Maximum compile time allowed for `Component::new`/`Module::new` before
/// the runner gives up — a compile-bomb `.wasm` has no fuel backstop (fuel
/// only meters guest *execution*, not host-side validation/compilation), so
/// this is the dedicated guard for that window.
const COMPILE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(360);

fn run(args: RunnerArgs) -> Result<(), RunError> {
    // Read the opened file through a cap: metadata alone cannot bound a file
    // changed between resolution and the contained runner's open.
    const MAX_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;
    let mut file = std::fs::File::open(&args.component_path)
        .map_err(|e| RunError::InvalidComponent(format!("cannot open component: {e}")))?;
    let metadata = file
        .metadata()
        .map_err(|e| RunError::InvalidComponent(e.to_string()))?;
    if !metadata.is_file() {
        return Err(RunError::InvalidComponent(
            "component is not a regular file".into(),
        ));
    }
    if metadata.len() > MAX_COMPONENT_BYTES {
        return Err(RunError::InvalidComponent(
            "component exceeds 64 MiB cap".into(),
        ));
    }
    let mut wasm_bytes = Vec::new();
    wasm_bytes
        .try_reserve_exact(metadata.len() as usize)
        .map_err(|e| RunError::InvalidComponent(format!("component buffer: {e}")))?;
    (&mut file)
        .take(metadata.len())
        .read_to_end(&mut wasm_bytes)
        .map_err(|e| RunError::InvalidComponent(format!("component read: {e}")))?;
    let mut extra = [0u8; 1];
    if wasm_bytes.len() as u64 != metadata.len()
        || file
            .read(&mut extra)
            .map_err(|e| RunError::InvalidComponent(e.to_string()))?
            != 0
    {
        return Err(RunError::InvalidComponent(
            "component length changed during read".into(),
        ));
    }

    let mut engine_config = Config::new();
    engine_config.consume_fuel(true);
    engine_config.wasm_component_model(true);
    engine_config.wasm_threads(false);
    engine_config.wasm_memory64(false);
    engine_config.parallel_compilation(false);
    engine_config.memory_reservation(0);
    engine_config.memory_guard_size(0);
    engine_config.memory_reservation_for_growth(0);

    let engine = Engine::new(&engine_config)
        .map_err(|e| RunError::Other(format!("wasmtime engine init: {e}")))?;

    // Compile under a watchdog thread: a pathological .wasm cannot hang the
    // runner indefinitely at validation/compile time (fuel does not meter
    // this phase).
    let component = compile_with_timeout(&engine, wasm_bytes)?;

    if let Some(import) = incompatible_spirit_frames_import(&component, &engine) {
        return Err(RunError::IncompatibleWorld(import));
    }

    let mut store = Store::new(&engine, maos_wasm_host::host_state::HostState::new());
    store.limiter(|state| &mut state.limits);
    // Wasmtime charges aggregate list/string lifting BEFORE host allocations,
    // including repeated aliases into the same guest linear memory.
    store.set_hostcall_fuel(maos_frame_codec::MAX_FRAME_BYTES);
    store
        .set_fuel(args.fuel)
        .map_err(|e| RunError::Other(format!("set fuel: {e}")))?;

    let mut linker = Linker::<maos_wasm_host::host_state::HostState>::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .map_err(|e| RunError::Other(format!("wasi linker setup: {e}")))?;
    let spirit = match Spirit::instantiate(&mut store, &component, &linker) {
        Ok(spirit) => spirit,
        Err(error) => {
            let error_message = error.to_string();
            if matches!(classify_trap(error), RunError::OutOfFuel) {
                return Err(RunError::OutOfFuel);
            }
            return Err(RunError::InvalidComponent(format!(
                "component does not conform to maos:spirit@2.0.0 (instantiate failed): {error_message}"
            )));
        }
    };

    // Lifecycle: on-start.
    store
        .set_fuel(args.fuel)
        .map_err(|e| RunError::Other(e.to_string()))?;
    match spirit.call_on_start(&mut store) {
        Ok(Ok(())) => {}
        Ok(Err(halt)) => {
            return Err(RunError::Other(format!("guest on-start halted: {halt:?}")));
        }
        Err(trap) => return Err(classify_trap(trap)),
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    // Empty bodies are transport controls, never CBOR: initial Ready, then one
    // TurnComplete after each successful export, including exports emitting none.
    maos_frame_codec::write_frame(&mut writer, &[])
        .map_err(|e| RunError::Other(format!("Ready write: {e}")))?;
    #[cfg(all(feature = "runner-fault-inject", debug_assertions, target_os = "linux"))]
    if args.forbidden_syscall_after_ready {
        // Exercise the actual runner's seccomp boundary, never a synthetic
        // SandboxBlock row or a signal sent by the test parent.
        unsafe extern "C" {
            fn ptrace(
                request: std::ffi::c_uint,
                pid: std::ffi::c_int,
                address: *mut std::ffi::c_void,
                data: *mut std::ffi::c_void,
            ) -> std::ffi::c_long;
        }
        // SAFETY: PTRACE_TRACEME uses no pointed-to memory. Under admitted T2
        // the existing hostile-syscall rule terminates this process with SIGSYS.
        unsafe {
            ptrace(0, 0, std::ptr::null_mut(), std::ptr::null_mut());
        }
        return Err(RunError::Other(
            "forbidden syscall unexpectedly returned".into(),
        ));
    }
    let result = pump_frames(&mut store, &spirit, &mut reader, &mut writer, args.fuel);

    // Lifecycle: on-shutdown — best-effort, runs even if the pump errored,
    // mirroring the native form's halt-then-shutdown ordering. A shutdown
    // trap does not override the pump's own error/cause.
    let _ = store.set_fuel(args.fuel);
    let _ = spirit.call_on_shutdown(&mut store);

    result
}

/// Compile a component on a dedicated thread with a hard wall-clock cap.
fn compile_with_timeout(engine: &Engine, bytes: Vec<u8>) -> Result<Component, RunError> {
    let engine = engine.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = Component::new(&engine, &bytes);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(COMPILE_TIMEOUT) {
        Ok(Ok(component)) => Ok(component),
        Ok(Err(e)) => Err(RunError::InvalidComponent(format!(
            "component compile failed (and this runner requires a real maos:spirit@2.0.0 \
             component — core-module fallback was removed once a real component fixture \
             landed): {e}"
        ))),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(RunError::InvalidComponent(
            "component compiler thread terminated".into(),
        )),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(RunError::InvalidComponent(
            format!("component compile exceeded {COMPILE_TIMEOUT:?} — treating as a compile-bomb"),
        )),
    }
}

/// Return the first `frames` import whose semver major is not the host's
/// `maos:spirit@2.0.0` major. Other imports are left to normal instantiation.
fn incompatible_spirit_frames_import(component: &Component, engine: &Engine) -> Option<String> {
    component
        .component_type()
        .imports(engine)
        .find_map(|(name, _)| {
            let version = name.strip_prefix("maos:spirit/frames@")?;
            let major = version.split('.').next()?.parse::<u64>().ok()?;
            (major != 2).then(|| name.to_owned())
        })
}

fn classify_trap(err: wasmtime::Error) -> RunError {
    if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
        if *trap == wasmtime::Trap::OutOfFuel {
            return RunError::OutOfFuel;
        }
    }
    RunError::Other(format!("guest call failed: {err:#}"))
}

fn pump_frames<T>(
    store: &mut Store<T>,
    spirit: &Spirit,
    reader: &mut impl io::BufRead,
    writer: &mut impl io::Write,
    fuel: u64,
) -> Result<(), RunError> {
    loop {
        let frame_bytes = match maos_frame_codec::read_frame(reader) {
            Ok(Some(b)) => b,
            Ok(None) => return Ok(()), // Clean EOF — Halt::Voluntary.
            Err(e) => return Err(RunError::Other(format!("stdin read error: {e}"))),
        };

        let domain_frame: maos_domain::frame::IacFrame =
            maos_wasm_host::codec::decode_cbor(&frame_bytes)
                .map_err(|e| RunError::Other(format!("inbound frame decode error: {e}")))?;
        let wit_frame =
            maos_wasm_host::frame_bridge::lower(&domain_frame).map_err(|error| match error {
                maos_wasm_host::frame_bridge::BridgeError::Unrepresentable { ty } => {
                    RunError::UnrepresentableFrame(ty)
                }
                other => RunError::Other(format!("inbound frame lower error: {other}")),
            })?;

        store
            .set_fuel(fuel)
            .map_err(|e| RunError::Other(e.to_string()))?;
        let emitted = match spirit.call_handle_frame(&mut *store, &wit_frame) {
            Ok(Ok(frames)) => frames,
            Ok(Err(maos_wasm_host::wit_guest::maos::spirit::frames::Halt::Voluntary)) => {
                maos_frame_codec::write_frame(writer, &[])
                    .map_err(|e| RunError::Other(format!("TurnComplete write: {e}")))?;
                return Ok(());
            }
            Ok(Err(halt)) => {
                return Err(RunError::Other(format!(
                    "guest handle-frame halted: {halt:?}"
                )));
            }
            Err(trap) => {
                // Machine-readable stage: the guest/canonical call failed before
                // domain conversion, CBOR encoding, or stdout data emission.
                eprintln!("{{\"event\":\"guest_export_failed\",\"export\":\"handle-frame\",\"stage\":\"canonical-call\"}}");
                return Err(classify_trap(trap));
            }
        };

        // Turn atomicity: lift and encode every emitted frame before writing
        // any, so a failure on frame N leaves no partial turn on stdout.
        let encoded = emitted
            .into_iter()
            .map(|wit_out| {
                let domain_out = maos_wasm_host::frame_bridge::lift(wit_out)
                    .map_err(|e| RunError::Other(format!("outbound frame lift error: {e}")))?;
                maos_wasm_host::codec::encode_cbor(&domain_out)
                    .map_err(|e| RunError::Other(format!("outbound frame encode error: {e}")))
            })
            .collect::<Result<Vec<_>, RunError>>()?;
        for cbor in &encoded {
            maos_frame_codec::write_frame(writer, cbor)
                .map_err(|e| RunError::Other(format!("stdout write error: {e}")))?;
        }
        maos_frame_codec::write_frame(writer, &[])
            .map_err(|e| RunError::Other(format!("TurnComplete write: {e}")))?;
    }
}
