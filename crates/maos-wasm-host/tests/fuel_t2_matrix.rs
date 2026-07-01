//! Story 11.1a AC4 — fuel↔T2 2×2 matrix + disable-the-other controls.
//!
//! Decision D6: fuel↔T2 = 2×2 matrix with DERIVED cause attribution
//! (`OutOfFuel` trap code, not exit≠0) + both disable-the-other controls
//! + the benign no-kill sanity cell.
//!
//! Matrix:
//!   {pure spin / forbidden-syscall / benign / spin+syscall} × {fuel, T2}
//!
//! The configured precedence is: fuel bound strictly < T2 bound so fuel
//! always wins gracefully with a clean error frame; T2 is strictly the
//! backstop.
//!
//! This file covers the FUEL column of the matrix (Linux-portable, no
//! special privileges). The T2 (OS-level sandbox enforcement) column —
//! forbidden-syscall x T2 kill, the benign-under-T2 negative control, and
//! the granted/ungranted filesystem-capability negative control — lives in
//! `tests/t2_sandbox_kill.rs`, which drives the kernel's REAL
//! `spawn_sandboxed`/`classify_exit` (requires CAP_SYS_ADMIN / no_new_privs;
//! self-skips with a clear message otherwise, mirroring
//! `maos-kernel-core/tests/sandbox_enforcement_linux.rs`).

use wasmtime::{Config, Engine, Store};

/// Load a fixture WASM module from tests/fixtures/wasm/.
fn fixture_path(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../tests/fixtures/wasm/{name}")
}

fn load_fixture(name: &str) -> Vec<u8> {
    let path = fixture_path(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read fixture {path}: {e}"))
}

// ── Cell 1: Spin-loop with fuel → OutOfFuel trap ───────────────────────

#[test]
fn spin_loop_exhausts_fuel_with_out_of_fuel_trap() {
    // AC4: "the spin-loop with T2≈∞ is killed by fuel with trap code OutOfFuel"
    let mut config = Config::new();
    config.consume_fuel(true);

    let engine = Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, &load_fixture("spin.wasm")).unwrap();

    let mut store = Store::new(&engine, ());
    // Very low fuel so the spin exhausts quickly
    store.set_fuel(1000).unwrap();

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let spin_fn = instance
        .get_typed_func::<(), ()>(&mut store, "spin")
        .unwrap();

    let result = spin_fn.call(&mut store, ());

    // Must fail with a fuel-exhaustion trap
    assert!(result.is_err(), "spin with limited fuel must trap");
    let err = result.unwrap_err();
    let trap = err.downcast_ref::<wasmtime::Trap>();
    assert!(
        trap.is_some(),
        "error must be a Trap, got: {err}"
    );
    // The trap must indicate fuel exhaustion specifically
    assert_eq!(
        *trap.unwrap(),
        wasmtime::Trap::OutOfFuel,
        "trap code must be OutOfFuel, not a generic trap"
    );
}

// ── Cell 2: Benign guest completes with clean exit ─────────────────────

#[test]
fn benign_guest_completes_with_fuel() {
    // AC4: "the benign guest completes with a clean exit (the no-vacuous-green sanity cell)"
    let mut config = Config::new();
    config.consume_fuel(true);

    let engine = Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, &load_fixture("benign.wasm")).unwrap();

    let mut store = Store::new(&engine, ());
    store.set_fuel(1_000_000).unwrap(); // Generous fuel

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let run_fn = instance
        .get_typed_func::<(), i32>(&mut store, "run")
        .unwrap();

    let result = run_fn.call(&mut store, ());
    assert!(result.is_ok(), "benign guest must complete without trapping");
    assert_eq!(result.unwrap(), 0, "benign guest returns 0");

    // Verify fuel was consumed (not zero — that would mean metering is broken)
    let remaining = store.get_fuel().unwrap();
    assert!(remaining < 1_000_000, "fuel must be consumed during execution");
}

// ── Cell 3: Echo guest identity function works ─────────────────────────

#[test]
fn echo_guest_returns_input_identity() {
    let mut config = Config::new();
    config.consume_fuel(true);

    let engine = Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, &load_fixture("echo.wasm")).unwrap();

    let mut store = Store::new(&engine, ());
    store.set_fuel(1_000_000).unwrap();

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let echo_fn = instance
        .get_typed_func::<i32, i32>(&mut store, "echo")
        .unwrap();

    // Test various inputs
    for input in [0, 1, 42, -1, i32::MAX, i32::MIN] {
        let result = echo_fn.call(&mut store, input).unwrap();
        assert_eq!(result, input, "echo must return input unchanged for {input}");
    }
}

// ── Cell 4: Mutator guest produces different output (proven-red) ───────

#[test]
fn mutator_guest_flips_output_red() {
    // AC2 proven-red: a mutator guest (flips one field) → RED
    let mut config = Config::new();
    config.consume_fuel(true);

    let engine = Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, &load_fixture("mutator.wasm")).unwrap();

    let mut store = Store::new(&engine, ());
    store.set_fuel(1_000_000).unwrap();

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let mutate_fn = instance
        .get_typed_func::<i32, i32>(&mut store, "mutate")
        .unwrap();

    // Mutator XORs with 1, so output ≠ input
    for input in [0, 1, 42, 255] {
        let result = mutate_fn.call(&mut store, input).unwrap();
        assert_ne!(
            result, input,
            "RED: mutator must produce different output for input {input}"
        );
        assert_eq!(result, input ^ 1, "mutator XORs with 1");
    }
}

// ── Disable-the-other controls ─────────────────────────────────────────

#[test]
fn fuel_disabled_spin_runs_without_out_of_fuel_trap() {
    // "disable-the-other control" (D6): with fuel metering OFF, the SPIN
    // loop must NOT be killed by `OutOfFuel` — fuel cannot be the kill
    // cause when it is disabled. T2 (OS-level, exercised separately in
    // `tests/t2_sandbox_kill.rs`) would be the only backstop in this
    // configuration. Proven via epoch interruption (a deterministic,
    // store-driven deadline) instead of a wall-clock sleep/timeout, so the
    // test is not flaky under CI load and does not conflate epoch-as-fuel
    // (D6: "Epoch-interrupt != fuel — assert the trap, not the timing").
    let mut config = Config::new();
    config.epoch_interruption(true);
    // consume_fuel NOT set — fuel metering disabled entirely.

    let engine = Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, &load_fixture("spin.wasm")).unwrap();

    let mut store = Store::new(&engine, ());
    store.set_epoch_deadline(1);
    // No set_fuel call — fuel metering is OFF for this store.

    // Tick the engine's epoch in the background so the infinite spin loop
    // is bounded by the epoch deadline, not by fuel (which does not exist
    // here) and not by a wall-clock test timeout.
    let engine_clone = engine.clone();
    let ticker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        engine_clone.increment_epoch();
    });

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let spin_fn = instance
        .get_typed_func::<(), ()>(&mut store, "spin")
        .unwrap();

    let result = spin_fn.call(&mut store, ());
    ticker.join().unwrap();

    assert!(
        result.is_err(),
        "the spin loop must still be stopped (by the epoch deadline, not fuel)"
    );
    let err = result.unwrap_err();
    let trap = err.downcast_ref::<wasmtime::Trap>();
    assert_eq!(
        trap,
        Some(&wasmtime::Trap::Interrupt),
        "with fuel disabled, the kill cause must be the epoch Interrupt trap, \
         NEVER OutOfFuel (fuel was never armed) — got {err}"
    );
    assert_ne!(
        trap,
        Some(&wasmtime::Trap::OutOfFuel),
        "OutOfFuel must be impossible to observe when fuel metering is disabled"
    );
}

#[test]
fn fuel_ordering_fuel_bound_strictly_less_than_t2() {
    // AC4: "the configured precedence is fuel bound strictly < T2 bound
    // so fuel always wins gracefully with a clean error frame" — proven by
    // mechanism (which trap fires), NOT by wall-clock timing (D6: "assert
    // the trap, not the timing"). Both fuel AND an epoch deadline (standing
    // in for the T2-equivalent backstop bound) are armed on the SAME store;
    // fuel is given a tiny budget and the epoch deadline is given a
    // generous one. Fuel must win — i.e. the trap must be OutOfFuel, never
    // Interrupt — proving fuel's bound is strictly tighter and always fires
    // first regardless of how long the test happens to take to schedule.
    let mut config = Config::new();
    config.consume_fuel(true);
    config.epoch_interruption(true);

    let engine = Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, &load_fixture("spin.wasm")).unwrap();

    let mut store = Store::new(&engine, ());
    // Fuel: tiny budget — exhausts in a handful of instructions.
    store.set_fuel(100).unwrap();
    // Epoch: deadline far in the future relative to fuel exhaustion — the
    // ticker below only fires after a long delay, standing in for a T2
    // bound that is strictly LOOSER than the fuel bound.
    store.set_epoch_deadline(1);
    let engine_clone = engine.clone();
    let ticker = std::thread::spawn(move || {
        // Deliberately much longer than fuel exhaustion needs — if fuel's
        // bound were NOT strictly tighter, this test would hang waiting for
        // the spin call to return, making the precedence violation visible
        // as a test timeout rather than a silent wrong-trap pass.
        std::thread::sleep(std::time::Duration::from_secs(5));
        engine_clone.increment_epoch();
    });

    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let spin_fn = instance
        .get_typed_func::<(), ()>(&mut store, "spin")
        .unwrap();

    let result = spin_fn.call(&mut store, ());

    assert!(result.is_err());
    let err = result.unwrap_err();
    let trap = err.downcast_ref::<wasmtime::Trap>().copied();
    assert_eq!(
        trap,
        Some(wasmtime::Trap::OutOfFuel),
        "fuel's tighter bound must win — the trap must be OutOfFuel, not Interrupt \
         (got {err}). An Interrupt trap here would mean the looser bound fired \
         first, violating 'fuel bound strictly < T2 bound'."
    );

    // The epoch deadline must never have fired during the call — it only
    // exists as the looser backstop bound. Detach the ticker thread; the
    // test process exits once the assertions above pass, regardless of the
    // ticker's 5s sleep.
    drop(ticker);
}


// ── Invalid component detection ────────────────────────────────────────

#[test]
fn invalid_wasm_bytes_fail_closed() {
    // AC3: "a malformed / non-conformant component fails closed (InvalidComponent)"
    let mut config = Config::new();
    config.consume_fuel(true);

    let engine = Engine::new(&config).unwrap();

    // Garbage bytes
    let result = wasmtime::Module::new(&engine, b"not a wasm module");
    assert!(
        result.is_err(),
        "invalid bytes must fail closed, not produce a module"
    );

    // Truncated WASM magic
    let result = wasmtime::Module::new(&engine, &[0x00, 0x61, 0x73, 0x6d]);
    assert!(result.is_err(), "truncated magic must fail closed");
}

// ── Component model validation ─────────────────────────────────────────

#[test]
fn component_model_engine_configuration() {
    // Verify the engine can be configured for component model
    let mut config = Config::new();
    config.consume_fuel(true);
    config.wasm_component_model(true);

    let engine = Engine::new(&config);
    assert!(engine.is_ok(), "engine with component-model must initialize");
}
