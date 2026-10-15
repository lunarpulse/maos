# Breaking Changes

This file is the **CI-enforced** ledger of breaking changes to the MAOS ABI and
manifest surface (NFR-Maint-7). Every breaking change MUST land with a dated
`## YYYY-MM-DD` entry below, and every entry MUST carry a `**Migration:**` line
describing how a Spirit author / operator adapts. The `check-breaking-md`
discipline gate fails CI when an entry is missing its migration path.

Entry taxonomy (newest first):

- `## YYYY-MM-DD — <short title>` heading
- prose describing what changed and why
- `**Migration:**` — the concrete steps to adapt

The ABI Stability Triple and the compatibility window live in
[`STABILITY.md`](./STABILITY.md) (generated from workspace state). This file is
the human-authored change ledger that complements it.

## 2026-10-14 — T3 retained container lifecycle and terminal facts (Story 17-7)

The sandbox inspect report replaces `pid: u32` with `payload_pid: {"historical": N}`
and adds `container_id` (64 lowercase hexadecimal characters). The PID is a
historical startup observation and does not claim liveness. T3 `cli.subprocess.exit`
rows gain a fifth outer key, `container`, an object with six nested keys; native
rows retain their existing four-key shape. T3 classification now reports external
SIGKILL as `fault.signaled`, OOM as `fault.oom_killed`, and unavailable terminal
facts as `fault.truncated`.

- **Migration:** Update sandbox-inspect readers to consume tagged historical
  `payload_pid` and `container_id`; never treat the historical PID as live or as a
  kill target.
- **Migration:** Update T3 exit-row readers to accept the exact fifth `container`
  object and its six nested keys. Keep native rows at four keys.
- **Migration:** Handle `fault.signaled`, `fault.oom_killed`, and
  `fault.truncated` as distinct T3 outcomes; do not infer SIGKILL from runtime
  code 137 when payload evidence is unavailable.
- **Migration:** Low-level `T3Launch` and `T3SpawnContext` constructors now carry
  optional `launch_custody`. Root-managed Workers register `ContainerLaunchCustody`
  before creation and retain it through initialization and teardown; standalone
  low-level callers without a root coordinator pass `None`.
- **Migration:** `Invocation::Exited` includes `stdout_complete` and
  `stderr_complete`. Preserve those flags when consuming bounded runtime output;
  incomplete inspect output cannot establish terminal facts, and incomplete
  removal diagnostics cannot certify volume cleanup.

Review repairs preserve the wire schema above. Cleanup now fails closed when
removal diagnostics are incomplete, keeps failed CID-kill reporting independent
of eventual removal, and shares launch cancellation and teardown deadlines across
the binding and bridge. Temporary state fallback parents are effective-UID-scoped.
The `i9_exempt` attribute now preserves generic parameters and where clauses,
including the borrowed launch-custody context.

## 2026-10-12 — Worker resource-ceiling admission and scheduler API cutover (Story 17-2)

Worker manifests now use schema 5 for the optional `resources.pids_max` positive
`u32`; omission remains `None`. Existing schema-4 and schema-5 manifests that omit
the field remain accepted, while pre-17-2 strict readers reject `pids_max` as an
unknown field. Worker CPU percentage, PID/tasks count, and FD count are independent
ceilings: `fd_max` is never a PID alias. CPU, memory MiB, PID/tasks, and FD limits
are each resolved independently to the strictest manifest/operator authority. T3
CPU percentages map to container `--cpus` (10 → 0.10, 100 → 1.00), with independent
PID default 64, memory default 256 MiB, and CPU default 1 when omitted. T3 FD caps
use explicit soft/hard `--ulimit=nofile`; a request above the inherited hard ceiling
is refused, not clamped. Native T2 uses pre-exec placement in a delegated cgroup-v2
domain with a daemon process leaf and owned per-spawn leaf; explicit CPU or PID caps
refuse when writable delegation is unavailable. A memory-only missing-cgroup fallback
retains RLIMIT_AS and optional RLIMIT_NOFILE and records one
`ResourceCeilingDowngraded=20` informational lifecycle row per successful fallback
launch. This event is not an SCB state transition and is produced only after live
worker ownership/observer registration. Journal append failure still panics; sync
errors remain ignored, so this adds no power-loss durability guarantee.

The public `scheduler::resource_ceiling::{apply_resource_ceiling,
ResourceCeilingHandle, IoError}` API is retired; this is a cutover, not a
compatibility-shim promise.

**Migration:** Stop calling the retired scheduler helper and do not replace it with
a compatibility shim. Submit the requested limits through the admitted `SandboxSpec`
and use the actual spawn path's owned guard/lifecycle; derive the spec from the
manifest and the same live operator authority used for admission. Preserve omitted
`pids_max` as `None`; update strict pre-17-2 readers before sending schema-5
manifests with that field. Keep CPU percentage, PID/tasks, and FD semantics
independent; in particular, migrate any caller that used `fd_max` as a process
count to explicit `pids_max`. Native T2 callers must arrange delegated writable
cgroup-v2 placement for explicit CPU/PID caps; T3 uses the supported Linux
rootless-container route. Do not treat a memory-only downgrade event as a state
transition or a durability guarantee.

## 2026-05-31 — v0.x → v1.0 ABI stability commitments activated (Story 7.5a)

Story 7.5a turns the v1.0 **ABI Stability Triple** `(kernel_version, abi_version,
manifest_schema_version)` from a published promise into mechanically-enforced
substrate. This entry documents a **policy activation**, not a source-level break
— all surface changes are strictly **additive**:

- The kernel now ENFORCES `min_substrate_version` at load: a Spirit whose
  declared minimum exceeds the running kernel is refused with a typed
  `SecurityError::ESubstrateTooOld` (FR8). Previously the field was parsed but
  never compared, so an incompatible Spirit admitted silently.
- The `manifest_schema_version` window is now fail-closed in BOTH directions at
  admission: below `MIN_SUPPORTED` → typed `SecurityError::EAbiTooOld` (N-2 hard
  refusal); above `MAX_SUPPORTED` → typed `SecurityError::EAbiTooNew` (a future
  Spirit is told a newer kernel is required — no silent warn-and-ignore window).
- `STABILITY.md` is published and generated from workspace state;
  `BREAKING.md` (this file) is published and CI-grep-enforced.

These are additive `SecurityError` variants on a kernel-internal enum; no
existing variant changed, `ABI_VERSION` stays `1`, `MANIFEST_SCHEMA_VERSION`
stays `2`, and `compliance.rs` is untouched. A manifest authored at
`manifest_schema_version = 1` (N-1) still loads — now with WARN-level
degradation notes for the newer sections it omits.

**Migration:** No action is required for any Spirit whose `[class]` section
declares a truthful `min_substrate_version` at or below the running kernel and a
`manifest_schema_version` within `MIN_SUPPORTED..=MAX_SUPPORTED`. A Spirit that
previously relied on a too-high `min_substrate_version` or an out-of-window
`manifest_schema_version` admitting silently must correct that field to match the
kernel it targets; the typed error names the exact mismatch (declared vs running
/ declared vs supported window).
