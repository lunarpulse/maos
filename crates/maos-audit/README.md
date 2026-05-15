# `maos-audit` — Read-side Audit Surface (FR4 Mechanical Verification)

**Crate role:** read-only SQLite query adapter for the on-disk Transparency
Log. Pure read-side surface; the kernel writes, this crate reads.

**Story 1b.5b status:** ships the FR4 100%-mediation verification path at
v0.1-β — `maosctl audit query --spirit hello-spirit` enumerates every
external call hello-Spirit made, with its issuing capability token,
spirit_pid, and boot_nonce. The 1000-entry fixture proves 100% mediation
mechanically; FR4 is no longer asserted in a README — it is verified by
`cargo test -p maos-audit -- test_fr4_full_mediation`.

---

## Canonical FR4 verification path

This is the evaluator-facing path. It is the AC4 contract for Story 1b.5b
and the canonical reproduction recipe for FR4 mediation. The full
end-to-end path is exercised in CI by
`.github/workflows/discipline.yml::audit-query-fr4-smoke`.

```sh
# 1. Clone and build (Story 1a.4 evaluator path).
git clone <maos-repo> && cd maos
cargo build --workspace --locked

# 2. Install hello-Spirit (Story 1b.5a).
cargo run -p maos-cli --bin maosctl -- install hello-spirit

# 3. Run hello-Spirit once. This writes inference.call + capability.invocation
#    rows to the on-disk Transparency Log at the XDG-resolved path
#    `$XDG_DATA_HOME/maos/audit/transparency.sqlite` (or
#    `~/.local/share/maos/audit/transparency.sqlite`). Override the path
#    via `MAOS_AUDIT_DB=<path>` for hermetic test runs.
cargo run -p maos-cli --bin maosctl -- run hello-spirit

# 4. Query the read side. Each line is a JSON object with the FR4 six-key
#    schema. At least one entry (the inference.call) is mechanically
#    observable — FR4 is verified, not asserted.
cargo run -p maos-cli --bin maosctl -- audit query --spirit hello-spirit
```

Sample output (line wrapped for readability — actual NDJSON is one object per line):

```json
{"call_id":"019e2ab2cba9e92118334deb67e9e19a",
 "capability_token":"240b78fe...0000",
 "spirit_pid":0,
 "boot_nonce":2597302644735822443,
 "call_type":"inference.call",
 "timestamp_ns":1778832821161969014}
```

---

## FR4 schema (AC1)

Every entry emitted by `--format ndjson` contains **exactly** these six keys.
All five mandatory fields are non-null; a missing or null mandatory field
fails the command with **exit code 2** and a diagnostic naming the missing
field and 1-indexed line number — no silent pass on partial coverage.

| Field              | Type               | Mandatory | Meaning                                                                                |
| ------------------ | ------------------ | --------- | --------------------------------------------------------------------------------------- |
| `call_id`          | 32-char lower hex  | yes¹      | 16-byte frame_id (Transparency-Log PRIMARY KEY)                                         |
| `capability_token` | 64-char lower hex  | **yes**   | 32-byte Ed25519 capability token; FR4 100%-mediation guarantee — never `null` on read   |
| `spirit_pid`       | u32                | **yes**   | Spirit process ID at the time of the call (`0` for the in-process hello-Spirit)         |
| `boot_nonce`       | u64                | **yes**   | Kernel boot nonce (rotates per `maos-bin` invocation)                                   |
| `call_type`        | string             | **yes**   | Dot-separated kind: `inference.call`, `capability.invocation` (see kernel `FrameKind`)  |
| `timestamp_ns`     | u64                | **yes**   | Wall-clock nanoseconds                                                                   |

¹ `call_id` is always present in the SQLite row (PRIMARY KEY); the projection
treats it as mandatory and never emits a row without it.

The raw `AuditEntry` shape (Story 1b.1) also carries `intent` and
`payload_redacted`. Those fields are NOT in the FR4 projection — they
belong to Story 9.1's subject-access surface.

---

## `cap-audit` ↔ Transparency Log join

The kernel-side `CapAuditWriter::spawn` (Story 1b.2) drains
`cap-audit` MPSC events into the same on-disk SQLite Transparency Log that
the read side opens. The two adapters share one file; no per-event
synchronization is required because the writer task is single-writer.

```
                  ┌──────────────────────┐
                  │ CapabilityRegistry   │  audit_tx.send(CapAuditEvent)
                  │  (cap-tokens / cap-  │ ───┐
                  │   policy / cap-audit)│    │
                  └──────────────────────┘    │  bounded mpsc(8192)
                                              ▼
                                ┌─────────────────────────────┐
                                │ CapAuditWriter task         │
                                │ (single-writer; spawned at  │
                                │  composition root)          │
                                └─────────────────────────────┘
                                              │  insert_frame_event(...)
                                              ▼
                                ┌─────────────────────────────┐
                                │ TransparencyLogAdapter      │
                                │  (SQLite on-disk; I9-exempt;│
                                │   log-before-deliver per I2)│
                                └─────────────────────────────┘
                                              ▲
                                              │  open_with_flags(SQLITE_OPEN_READ_ONLY)
                                              │
                                ┌─────────────────────────────┐
                                │ maos_audit::query(...)      │
                                │  (this crate; read-only)    │
                                └─────────────────────────────┘
                                              ▲
                                              │
                                ┌─────────────────────────────┐
                                │ maosctl audit query         │
                                │  (Story 1b.5b dispatcher)   │
                                └─────────────────────────────┘
```

The one-shot evaluator path closes the cap-audit channel deterministically
on exit (`drop(audit_tx); drop(inference); drop(capability);
audit_writer.await.ok();`) so the inference.call row is guaranteed to
reach SQLite before `maos-bin` exits. Without that drain the row is
intermittently lost — see Story 1b.5b Decision D3.

---

## Dep-direction rule

`maos-audit` depends only on `maos-domain` — never on `maos-kernel-core`.
The CLI dispatcher (`maos-cli`) routes all read-side audit traffic through
this crate, preserving the Story 1a.4 rule that `maos-cli` MUST NOT depend
on `maos-kernel-core`. Verify with:

```sh
cargo tree -p maos-cli | grep maos-kernel-core   # must return empty
```

This boundary holds the kernel's write surface isolated. The read side is a
pure projection; redaction is enforced at the kernel write boundary, so the
read side never sees raw payloads.

---

## Scope (v0.1-β)

This crate ships:

- `query(path, filter)` — read-side SQLite query
- `to_ndjson(entries, out)` — raw NDJSON export (Story 9.1 will reuse)
- `to_fr4_ndjson(entries, out)` — FR4 projection (Story 1b.5b, AC1/AC2)
- `to_plain(entries, out)` — accessibility-clean tabular text (AC3)
- `project_to_fr4(entry)` — single-row projection with typed errors
- `Fr4Entry`, `Fr4SchemaError`, `AuditError::Fr4SchemaViolation`

The `--spirit` filter accepts only `hello-spirit` at v0.1-β (maps to
`spirit_pid = 0`); other names exit with code 2 and a diagnostic. The full
Spirit-name resolution service lives in Epic 5. Subject-access /
posture-delta / sealed-export surfaces are Story 9.1 (Epic 9) and do NOT
ship here.

---

## Cross-links

- **PRD FR4** — `_bmad-output/planning-artifacts/prd/functional-requirements.md` line 27:
  > Operator can verify every Spirit's external call … was mediated by
  > kernel-issued capability tokens … verification floor is 100% mediation
  > in any 1000-call sample.
- **Architecture §8.4 Audit** —
  `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md`:
  Transparency Log is the personal audit trail; queryable via a
  control-plane API.
- **NFR-Obs-4** — Transparency Log SQLite, append-only, with JSONL export.
  This crate is the v0.1-β JSONL-export half.
- **NFR-Ops-5** — Accessibility cascade (`--plain` / `NO_COLOR=1` /
  `TERM=dumb`). Both `--format ndjson` and `--format plain` emit zero ANSI
  bytes when the cascade engages.
- **Story 1b.1** — `TransparencyLogAdapter::open()` and the append-only
  SQLite schema this crate reads.
- **Story 1b.2** — `CapAuditWriter::spawn()` and the cap-audit MPSC channel
  this crate's `capability_token` column originates from.
- **Story 1b.5a** — One-shot `maos-bin` path; this story (1b.5b) closes the
  in-memory → on-disk SQLite gap so the read side can observe the writes.
- **`crates/maos-kernel-core/tests/fr4_1000_call_fixture.rs`** —
  complementary kernel-side mediation test; this crate's
  `fr4_full_mediation_test.rs` is the read-side companion.

---

## Testing

```sh
# Unit tests (projection + writers).
cargo test -p maos-audit --lib

# AC1 — schema test against hermetic SQLite seed.
cargo test -p maos-audit --test query_schema_test

# AC2 — 1000-entry fixture + determinism sub-test.
cargo test -p maos-audit --test fr4_full_mediation_test

# AC3 — accessibility cascade (spawns the maosctl binary).
cargo test -p maos-cli --test audit_no_color_test

# AC4 — end-to-end integration smoke.
bash tests/integration/audit_query_fr4_smoke.sh
```
