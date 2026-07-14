;; Benign module — returns immediately (AC4 no-vacuous-green sanity cell).
;; crypto-free (D9).
(module
  (func (export "run") (result i32)
    i32.const 0
  )
  (memory (export "memory") 1)
)
