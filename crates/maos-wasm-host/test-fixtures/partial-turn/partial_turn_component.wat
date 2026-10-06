;; Partial-turn fixture: a maos:spirit@2.0.0 component whose `handle-frame`
;; returns `[inbound, inbound-with-15-byte-frame-id]`. The first frame lifts
;; and encodes; the second fails domain lifting (`BadFrameIdLen(15)`), so the
;; runner must emit neither frame nor TurnComplete for that turn.
;;
;; Component text printed by `wasm-tools print` from this core module after
;; `wasm-tools component embed wit/spirit.wit core.wat --world spirit` and
;; `wasm-tools component new` (iac-frame canonical stride 248, frame-id at 0):
;;
;;   (module
;;     (memory (export "memory") 2)
;;     (global $heap (mut i32) (i32.const 4096))
;;     (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) (local $ptr i32)
;;       global.get $heap local.get 2 i32.const 1 i32.sub i32.add
;;       local.get 2 i32.const 1 i32.sub i32.const -1 i32.xor i32.and local.tee $ptr
;;       local.get 3 i32.add global.set $heap local.get $ptr)
;;     (func (export "on-start") (result i32) i32.const 32)
;;     (func (export "cabi_post_on-start") (param i32))
;;     (func (export "on-shutdown"))
;;     (func (export "handle-frame") (param $in i32) (result i32)
;;       i32.const 65536 local.get $in i32.const 248 memory.copy
;;       i32.const 65784 local.get $in i32.const 248 memory.copy
;;       i32.const 65788 i32.const 15 i32.store
;;       i32.const 16 i32.const 0 i32.store8
;;       i32.const 20 i32.const 65536 i32.store
;;       i32.const 24 i32.const 2 i32.store
;;       i32.const 16)
;;     (func (export "cabi_post_handle-frame") (param i32))
;;   )
;;
(component
  (type $ty-maos:spirit/frames@2.0.0 (;0;)
    (instance
      (type (;0;) (option string))
      (type (;1;) (record (field "spirit-id" string) (field "host-id" 0) (field "role" 0)))
      (export (;2;) "frame-address" (type (eq 1)))
      (type (;3;) (enum "task-assign" "task-complete" "decision-dispatch" "epistemic-halt" "telemetry-event" "consent-request" "retract" "capability-invocation" "sandbox-block" "inference-call" "budget-warning" "budget-exceeded" "cli-subprocess-output" "consent-rupture" "rate-limited" "gateway-inbound" "gateway-outbound"))
      (export (;4;) "frame-kind" (type (eq 3)))
      (type (;5;) (enum "high-privilege" "standard" "readonly"))
      (export (;6;) "intent-class" (type (eq 5)))
      (type (;7;) (record (field "server" string) (field "tool" string)))
      (export (;8;) "scope-mcp-call" (type (eq 7)))
      (type (;9;) (list u8))
      (type (;10;) (record (field "cli-binary-path" string) (field "argv-prefix-hash" 9) (field "output-shape-version" string)))
      (export (;11;) "scope-cli-subprocess-spawn" (type (eq 10)))
      (type (;12;) (record (field "gateway-id" string) (field "recipient" string)))
      (export (;13;) "scope-gateway-send" (type (eq 12)))
      (type (;14;) (variant (case "fs-read" string) (case "fs-write" string) (case "net-https" string) (case "proc-exec" string) (case "sub-spirit-spawn" string) (case "provider-infer" string) (case "iac-send" string) (case "mem-read" string) (case "mem-write" string) (case "self-telemetry-read") (case "log-recall") (case "log-fetch") (case "distillate-write") (case "mcp-call" 8) (case "cli-subprocess-spawn" 11) (case "gateway-send" 13) (case "skill-author-self") (case "loom-read") (case "loom-write") (case "loom-scan")))
      (export (;15;) "scope" (type (eq 14)))
      (type (;16;) (enum "autonomous-with-halt" "assistive" "cautious"))
      (export (;17;) "posture-hint" (type (eq 16)))
      (type (;18;) (record (field "tag" string) (field "recall-vs-precision" f32)))
      (export (;19;) "halt-policy-override" (type (eq 18)))
      (type (;20;) (option 17))
      (type (;21;) (list 19))
      (type (;22;) (record (field "preferred-posture" 20) (field "halt-policy-overrides" 21)))
      (export (;23;) "posture-preferences" (type (eq 22)))
      (type (;24;) (list string))
      (type (;25;) (record (field "digest-frame-id" 9) (field "distillation-depth" u32) (field "intent-lineage" 24)))
      (export (;26;) "prior-distillate-ref" (type (eq 25)))
      (type (;27;) (list 15))
      (type (;28;) (option 26))
      (type (;29;) (record (field "goal" string) (field "scope" 27) (field "success-criteria" string) (field "posture-preferences" 23) (field "prior-distillate-ref" 28)))
      (export (;30;) "task-assign-body" (type (eq 29)))
      (type (;31;) (record (field "result-text" string)))
      (export (;32;) "task-complete-body" (type (eq 31)))
      (type (;33;) (record (field "decision-id" u64) (field "approved" bool) (field "working-memory-digest-refs" 24)))
      (export (;34;) "decision-dispatch-body" (type (eq 33)))
      (type (;35;) (option f32))
      (type (;36;) (record (field "halt-id" string) (field "tag" string) (field "value" f32) (field "threshold" 35) (field "policy-id" string) (field "derived-from" string)))
      (export (;37;) "epistemic-halt-body" (type (eq 36)))
      (type (;38;) (record (field "event-type" string) (field "data" string)))
      (export (;39;) "telemetry-event-body" (type (eq 38)))
      (type (;40;) (record (field "capability" string)))
      (export (;41;) "consent-request-body" (type (eq 40)))
      (type (;42;) (option 4))
      (type (;43;) (record (field "original-frame-id" 9) (field "reason" string) (field "original-kind" 42)))
      (export (;44;) "retract-body" (type (eq 43)))
      (type (;45;) (record (field "spirit-pid" u32) (field "hook-name" string) (field "wall-ns" u64) (field "cap-seconds" u64)))
      (export (;46;) "budget-envelope" (type (eq 45)))
      (type (;47;) (enum "intent-allowlist-mismatch" "posture-shifted-during-transmission" "token-revoked" "principal-revoked" "recipient-unloaded" "peer-identity-unverified"))
      (export (;48;) "rupture-reason" (type (eq 47)))
      (type (;49;) (record (field "address" 2) (field "reason" 48)))
      (export (;50;) "rupture-rejection" (type (eq 49)))
      (type (;51;) (list 2))
      (type (;52;) (list 50))
      (type (;53;) (record (field "rupture-id" 9) (field "original-frame-id" 9) (field "original-kind" 4) (field "accepted" 51) (field "rejected" 52) (field "ruptured-at-ns" u64)))
      (export (;54;) "consent-rupture-body" (type (eq 53)))
      (type (;55;) (record (field "provider-id" string) (field "credential-fingerprint-prefix-hex" string) (field "retry-after-ms" u64) (field "bucket-remaining" u32) (field "bucket-capacity" u32) (field "refill-per-sec" u32) (field "schedule-id" 0)))
      (export (;56;) "rate-limited-body" (type (eq 55)))
      (type (;57;) (variant (case "task-assign" 30) (case "task-complete" 32) (case "decision-dispatch" 34) (case "epistemic-halt" 37) (case "telemetry-event" 39) (case "consent-request" 41) (case "retract" 44) (case "budget-warning" 46) (case "budget-exceeded" 46) (case "consent-rupture" 54) (case "rate-limited" 56)))
      (export (;58;) "frame-payload" (type (eq 57)))
      (type (;59;) (enum "human-authored" "spirit-auto" "spirit-drafted-human-approved" "kernel"))
      (export (;60;) "frame-origin" (type (eq 59)))
      (type (;61;) (option u64))
      (type (;62;) (record (field "consent-id" 9) (field "granter" 2) (field "timestamp-ns" u64) (field "intent-class" 0) (field "valid-until-ns" 61)))
      (export (;63;) "consent-envelope" (type (eq 62)))
      (type (;64;) (option 63))
      (type (;65;) (record (field "frame-id" 9) (field "timestamp-ns" u64) (field "logical-clock" u64) (field "frame-from" 2) (field "to" 51) (field "kind" 4) (field "intent" 6) (field "payload" 58) (field "auto-marker" 60) (field "consent-envelope" 64) (field "intent-lineage" 24)))
      (export (;66;) "iac-frame" (type (eq 65)))
      (type (;67;) (variant (case "voluntary") (case "fault" string)))
      (export (;68;) "halt" (type (eq 67)))
    )
  )
  (import "maos:spirit/frames@2.0.0" (instance $maos:spirit/frames@2.0.0 (;0;) (type $ty-maos:spirit/frames@2.0.0)))
  (alias export $maos:spirit/frames@2.0.0 "iac-frame" (type $iac-frame (;1;)))
  (import "iac-frame" (type $"#type2 iac-frame" (@name "iac-frame") (;2;) (eq $iac-frame)))
  (alias export $maos:spirit/frames@2.0.0 "halt" (type $halt (;3;)))
  (import "halt" (type $"#type4 halt" (@name "halt") (;4;) (eq $halt)))
  (core module $main (;0;)
    (type (;0;) (func (param i32 i32 i32 i32) (result i32)))
    (type (;1;) (func (result i32)))
    (type (;2;) (func (param i32)))
    (type (;3;) (func))
    (type (;4;) (func (param i32) (result i32)))
    (memory (;0;) 2)
    (global $heap (;0;) (mut i32) i32.const 4096)
    (export "memory" (memory 0))
    (export "cabi_realloc" (func 0))
    (export "on-start" (func 1))
    (export "cabi_post_on-start" (func 2))
    (export "on-shutdown" (func 3))
    (export "handle-frame" (func 4))
    (export "cabi_post_handle-frame" (func 5))
    (func (;0;) (type 0) (param i32 i32 i32 i32) (result i32)
      (local $ptr i32)
      global.get $heap
      local.get 2
      i32.const 1
      i32.sub
      i32.add
      local.get 2
      i32.const 1
      i32.sub
      i32.const -1
      i32.xor
      i32.and
      local.tee $ptr
      local.get 3
      i32.add
      global.set $heap
      local.get $ptr
    )
    (func (;1;) (type 1) (result i32)
      i32.const 32
    )
    (func (;2;) (type 2) (param i32))
    (func (;3;) (type 3))
    (func (;4;) (type 4) (param $in i32) (result i32)
      i32.const 65536
      local.get $in
      i32.const 248
      memory.copy
      i32.const 65784
      local.get $in
      i32.const 248
      memory.copy
      i32.const 65788
      i32.const 15
      i32.store
      i32.const 16
      i32.const 0
      i32.store8
      i32.const 20
      i32.const 65536
      i32.store
      i32.const 24
      i32.const 2
      i32.store
      i32.const 16
    )
    (func (;5;) (type 2) (param i32))
    (@producers
      (processed-by "wit-component" "0.259.0")
    )
  )
  (core instance $main (;0;) (instantiate $main))
  (alias core export $main "memory" (core memory $memory (;0;)))
  (type (;5;) (list $"#type2 iac-frame"))
  (type (;6;) (result 5 (error $"#type4 halt")))
  (type (;7;) (func (param "frame" $"#type2 iac-frame") (result 6)))
  (alias core export $main "handle-frame" (core func $handle-frame (;0;)))
  (alias core export $main "cabi_realloc" (core func $cabi_realloc (;1;)))
  (alias core export $main "cabi_post_handle-frame" (core func $cabi_post_handle-frame (;2;)))
  (func $handle-frame (;0;) (type 7) (canon lift (core func $handle-frame) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8 (post-return $cabi_post_handle-frame)))
  (export $"#func1 handle-frame" (@name "handle-frame") (;1;) "handle-frame" (func $handle-frame))
  (type (;8;) (result (error $"#type4 halt")))
  (type (;9;) (func (result 8)))
  (alias core export $main "on-start" (core func $on-start (;3;)))
  (alias core export $main "cabi_post_on-start" (core func $cabi_post_on-start (;4;)))
  (func $on-start (;2;) (type 9) (canon lift (core func $on-start) (memory $memory) string-encoding=utf8 (post-return $cabi_post_on-start)))
  (export $"#func3 on-start" (@name "on-start") (;3;) "on-start" (func $on-start))
  (type (;10;) (func))
  (alias core export $main "on-shutdown" (core func $on-shutdown (;5;)))
  (func $on-shutdown (;4;) (type 10) (canon lift (core func $on-shutdown)))
  (export $"#func5 on-shutdown" (@name "on-shutdown") (;5;) "on-shutdown" (func $on-shutdown))
  (@producers
    (processed-by "wit-component" "0.259.0")
  )
)
