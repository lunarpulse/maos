;; Minimal echo module — crypto-free (Decision D9).
;; Story 11.1a AC3: a valid WASM module used as a fixture for integration
;; tests. This is a core module (not a component); the integration test
;; validates the runner can load and instantiate a module.
;;
;; Regen: wasm-tools parse tests/fixtures/wasm/echo.wat -o tests/fixtures/wasm/echo.wasm
(module
  ;; Export a trivial function that returns its input (identity/echo).
  (func (export "echo") (param i32) (result i32)
    local.get 0
  )

  ;; Export memory so the host can read/write frame data.
  (memory (export "memory") 1)
)
