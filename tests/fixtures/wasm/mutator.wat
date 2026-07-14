;; Mutator module — flips a field (returns input XOR 1).
;; Proven-red for AC2: a mutator guest must produce different bytes.
;; crypto-free (D9).
(module
  (func (export "mutate") (param i32) (result i32)
    local.get 0
    i32.const 1
    i32.xor
  )
  (memory (export "memory") 1)
)
