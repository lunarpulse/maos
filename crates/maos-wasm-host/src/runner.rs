//! `maos-wasm-runner` — wasmtime component runner subprocess.
//!
//! This binary IS `BridgeSpawnSpec.program` for WASM Spirits. The kernel's
//! existing `spawn_and_bridge` launches it unchanged — it is an ordinary
//! subprocess that speaks ADR-032 (Content-Length + CBOR) over stdio.
//!
//! # Usage
//!
//! ```text
//! maos-wasm-runner --component <path.wasm> --fuel <n>
//! ```
//!
//! # Architecture (Story 11.1a AC3 — REAL component-model call path)
//!
//! 1. Parse CLI args (component path + fuel budget).
//! 2. Create a wasmtime `Engine` with `consume_fuel(true)`.
//! 3. Load + instantiate the component against `maos:spirit@1.0` via the
//!    `bindgen!`-generated `Spirit` bindings (`crate::wit_guest`).
//! 4. Call `on-start` on the guest. A non-conformant component (missing the
//!    `handle-frame`/`on-start`/`on-shutdown` exports) fails `instantiate`
//!    or the typed call itself — both map to `InvalidComponent`.
//! 5. Read ADR-032 frames from stdin, decode to a domain `IacFrame`, lower
//!    to the WIT shape (`maos_wasm_host::frame_bridge::lower`), call `handle-frame`
//!    on the guest, lift each emitted frame back to domain shape, encode,
//!    write to stdout. This is the REAL guest round-trip — not an echo.
//! 6. On EOF or a guest-returned `Halt`, call `on-shutdown` and exit with a
//!    cause-distinguishing exit code (see `RunnerExit` below) so the parent
//!    can attribute the kill cause without relying on stderr text.
//!
//! # Security (AC4)
//!
//! The runner subprocess is sandboxed by the existing T2 path (deny-by-default
//! caps). Wasmtime fuel metering provides defense-in-depth: fuel bound is
//! strictly < T2 bound so fuel always wins gracefully with a clean error frame.
//! T2 is strictly the backstop (proven by the kernel's own `spawn_sandboxed`/
//! `classify_exit`, exercised against this same binary in
//! `tests/t2_sandbox_kill.rs`).

use std::io::{self, BufReader, BufWriter};
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
    /// The supplied artifact is not a conformant `maos:spirit@1.0` component
    /// (fails to parse as a component/module, fails to instantiate, or does
    /// not export the required world functions).
    InvalidComponent = 3,
    /// Wasmtime fuel was exhausted during a guest call (`Trap::OutOfFuel`).
    OutOfFuel = 4,
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
        Err(RunError::InvalidComponent(reason)) => {
            eprintln!("maos-wasm-runner: InvalidComponent: {reason}");
            ExitCode::from(RunnerExit::InvalidComponent as u8)
        }
        Err(RunError::OutOfFuel) => {
            eprintln!("maos-wasm-runner: OutOfFuel");
            ExitCode::from(RunnerExit::OutOfFuel as u8)
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
}

enum RunError {
    InvalidComponent(String),
    OutOfFuel,
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
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(RunnerArgs {
        component_path: component_path.ok_or_else(|| "--component is required".to_string())?,
        fuel,
    })
}

/// Maximum compile time allowed for `Component::new`/`Module::new` before
/// the runner gives up — a compile-bomb `.wasm` has no fuel backstop (fuel
/// only meters guest *execution*, not host-side validation/compilation), so
/// this is the dedicated guard for that window.
const COMPILE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

fn run(args: RunnerArgs) -> Result<(), RunError> {
    let wasm_bytes = std::fs::read(&args.component_path).map_err(|e| {
        RunError::InvalidComponent(format!(
            "cannot read component '{}': {e}",
            args.component_path
        ))
    })?;

    let mut engine_config = Config::new();
    engine_config.consume_fuel(true);
    engine_config.wasm_component_model(true);

    let engine = Engine::new(&engine_config)
        .map_err(|e| RunError::Other(format!("wasmtime engine init: {e}")))?;

    // Compile under a watchdog thread: a pathological .wasm cannot hang the
    // runner indefinitely at validation/compile time (fuel does not meter
    // this phase).
    let component = compile_with_timeout(&engine, &wasm_bytes)?;

    let mut store = Store::new(&engine, maos_wasm_host::host_state::HostState::new());
    store
        .set_fuel(args.fuel)
        .map_err(|e| RunError::Other(format!("set fuel: {e}")))?;

    let mut linker = Linker::<maos_wasm_host::host_state::HostState>::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .map_err(|e| RunError::Other(format!("wasi linker setup: {e}")))?;
    let spirit = Spirit::instantiate(&mut store, &component, &linker).map_err(|e| {
        RunError::InvalidComponent(format!(
            "component does not conform to maos:spirit@1.0 (instantiate failed): {e}"
        ))
    })?;

    // Lifecycle: on-start.
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

    let result = pump_frames(&mut store, &spirit, &mut reader, &mut writer);

    // Lifecycle: on-shutdown — best-effort, runs even if the pump errored,
    // mirroring the native form's halt-then-shutdown ordering. A shutdown
    // trap does not override the pump's own error/cause.
    let _ = spirit.call_on_shutdown(&mut store);

    result
}

/// Compile the component (or fall back to a core module — needed for the
/// crypto-free echo/spin/benign/mutator fixtures, which predate real
/// component support) on a dedicated thread with a hard wall-clock cap.
fn compile_with_timeout(engine: &Engine, wasm_bytes: &[u8]) -> Result<Component, RunError> {
    let engine = engine.clone();
    let bytes = wasm_bytes.to_vec();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = Component::new(&engine, &bytes);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(COMPILE_TIMEOUT) {
        Ok(Ok(component)) => Ok(component),
        Ok(Err(e)) => Err(RunError::InvalidComponent(format!(
            "component compile failed (and this runner requires a real maos:spirit@1.0 \
             component — core-module fallback was removed once a real component fixture \
             landed): {e}"
        ))),
        Err(_) => Err(RunError::InvalidComponent(format!(
            "component compile exceeded {COMPILE_TIMEOUT:?} — treating as a compile-bomb"
        ))),
    }
}

fn classify_trap(err: wasmtime::Error) -> RunError {
    if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
        if *trap == wasmtime::Trap::OutOfFuel {
            return RunError::OutOfFuel;
        }
    }
    RunError::Other(format!("guest trapped: {err}"))
}

fn pump_frames<T>(
    store: &mut Store<T>,
    spirit: &Spirit,
    reader: &mut impl io::BufRead,
    writer: &mut impl io::Write,
) -> Result<(), RunError> {
    loop {
        let frame_bytes = match maos_wasm_host::codec::read_frame(reader) {
            Ok(Some(b)) => b,
            Ok(None) => return Ok(()), // Clean EOF — Halt::Voluntary.
            Err(e) => return Err(RunError::Other(format!("stdin read error: {e}"))),
        };

        let domain_frame: maos_domain::frame::IacFrame =
            maos_wasm_host::codec::decode_cbor(&frame_bytes)
                .map_err(|e| RunError::Other(format!("inbound frame decode error: {e}")))?;

        let wit_frame = maos_wasm_host::frame_bridge::lower(&domain_frame);

        let emitted = match spirit.call_handle_frame(&mut *store, &wit_frame) {
            Ok(Ok(frames)) => frames,
            Ok(Err(halt)) => {
                return Err(RunError::Other(format!(
                    "guest handle-frame halted: {halt:?}"
                )));
            }
            Err(trap) => return Err(classify_trap(trap)),
        };

        for wit_out in emitted {
            let domain_out = maos_wasm_host::frame_bridge::lift(wit_out)
                .map_err(|e| RunError::Other(format!("outbound frame lift error: {e}")))?;
            let cbor = maos_wasm_host::codec::encode_cbor(&domain_out)
                .map_err(|e| RunError::Other(format!("outbound frame encode error: {e}")))?;
            maos_wasm_host::codec::write_frame(writer, &cbor)
                .map_err(|e| RunError::Other(format!("stdout write error: {e}")))?;
        }
    }
}
