;; Spin-loop module — infinite loop for fuel exhaustion testing (AC4).
;; crypto-free (D9). The wasmtime fuel meter should kill this with OutOfFuel.
(module
  (func (export "spin")
    (loop $infinite
      br $infinite
    )
  )
  (memory (export "memory") 1)
)
