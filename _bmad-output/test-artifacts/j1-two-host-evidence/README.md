# J1 two-host evidence — the admission criteria, published

> **Executing the run?** This file is the **admission contract** — what a capture must contain and what
> the gate does *not* check. The **linear execution sequence** is
> [`../runbook-j1-t8-two-host-paid-run.md`](../runbook-j1-t8-two-host-paid-run.md), whose every command
> was executed on 2026-08-22 (fake-`claude` fixture) through a green `reconcile-hosts`.

This directory is what `check-j1-two-host-signed-run` leg 9 (`paid-run-capture`) reads.

> ⚠ **UPDATED 2026-08-25 — THE PAID RUN HAPPENED.** Until that date this paragraph read *"It is empty of
> real artifacts until the paid run happens"*, and that was true. It is no longer: `two-host-capture.json`,
> `host-a-bundle.json` and `host-b-bundle.json` are **real evidence from a real run** (`j1-crosshost-2d`
> AC8, 2026-08-25, `claude-haiku-4-5-20251001`, $0.014644). `paid_run_capture_present` is now `true` and the
> `two-host-signed-run` demo beat renders **INDETERMINATE**, not ABSENT. Corrected by the §A6 Acceptance
> Auditor, which caught this file still describing its own directory as empty while holding three artifacts.
> **Do not delete or overwrite those three files** — reproducing them costs money and re-signing them under
> different keys would break the `PUBLISHED-FINGERPRINTS.md` commitment. `two-host-capture.example.json` is
> the template and is deliberately NOT the validated filename.

The pre-run state, retained because it is the state a reader will find in git history and because it is what
the gate does when nothing is there: with no capture present the gate is GREEN and the
`two-host-signed-run` demo beat stays **ABSENT**. The gate refuses the *claim*, not the absence.

> **Why this file exists.** The capture contract used to live only inside the validator source and a
> fixture buried in a proven-red test, while the runbook said `cp two-host-capture.json …` without
> ever stating what was in it. An operator would run a paid agent, follow the runbook exactly, invent
> a capture, and be rejected **after the agent was billed**. A judge that will not publish its own
> admission criteria is not finished.

## The four artifacts — and which leg actually reads each

| File | Required when | Read by | What it is |
|---|---|---|---|
| `two-host-capture.json` | always, for a claimed run | **leg 9** (`paid-run-capture`) | the bounded claim (fields below) |
| `host-a-bundle.json` | whenever the capture is present | **leg 9 _and_ leg 4** | host A's signed `AuditBundle` half |
| `host-b-bundle.json` | whenever the capture is present | **leg 9 _and_ leg 4** | host B's signed `AuditBundle` half |
| `two-host-evidence.txt` | **never — the const is DELETED** | *nothing* | see "The transcript has no producer" below |

A capture present without both bundle halves is refused: *a claim without its artifact is the thing
this gate exists to refuse.*

**Leg 4 is not conditional on the capture.** `leg_bundle_schema` validates **any** bundle half present
in the tree against `bundle-schema.json` whether or not `two-host-capture.json` exists
(`xtask/src/check_j1_two_host_signed_run.rs:605-625`). Leg 9 asks a different question: that each
half's `host` string equals the capture field naming it (`:1006-1034`). Two legs, two questions — an
earlier revision of this table attributed both to leg 9, which would have sent an operator hunting the
wrong validator when a bundle was rejected.

### The transcript has no producer — and the const is now deleted (`2d` F2 → `2e` AC3)

`two-host-evidence.txt` was declared as `CAPTURE_TRANSCRIPT` and read by **no leg of this gate**.
`PROVEN_LIVE_SIGNED` required `capture_signature_verified`, which required a `MAOS-EVIDENCE-V1` record
whose `nonce` is recomputed **at gate-run time** — `format!("{gate}.{:x}.{nanos:x}", process::id())`
(`crates/.../evidence_ledger.rs:415-431`), fresh per process and per nanosecond. **No file written
beforehand can carry it.** Compounding this, the binding's `commit` is `local_worktree_commit()`, a hash
over HEAD *plus every untracked file's bytes*, so writing the transcript changed the value the
transcript had to contain. And nothing produced the file: the sole signer is
`tests/harness/evidence_record.rs`, which emits only when a gate exports `MAOS_EVIDENCE_{GATE,COMMIT,NONCE}`
— this gate does not, and J1 is not in `ledger_gates()`. The four sibling ledger gates produce their
transcript **in the same process**; this one was specified to read a static file.

**Re-scoped 2026-08-21 (R1, `j1-crosshost-2d` round-table): the evidence of this run is the TWO BUNDLE
SIGNATURES**, verified by the third-party `tools/verify-audit-bundle/verify.py`, plus a
`reconcile-hosts` that actually executes. That is exactly how **T6** — the only signed run this project
has ever performed — was evidenced; it predates `MAOS-EVIDENCE-V1` entirely. The transcript target was
**mis-specified, not merely unbuilt**.

✅ **`j1-crosshost-2e` AC3 executed the re-scope (2026-08-22).** `verify_capture_signature`, the
`CAPTURE_TRANSCRIPT` const, and the `capture_signature_verified` / `capture_signature_reason` JSON
fields are **deleted from the gate**. No replacement term was added and the gate still contains zero
`Command::new` — an `operator_evidence_verified` boolean would have been the same self-report-as-control
defect wearing a new name. The beat lands `INDETERMINATE` and `two_host_signed_run_claimed: false` is
published **as a true fact**, not concealed as a failure. Do not write this file; nothing reads it.

## The capture's seven required fields

Copy **`two-host-capture.example.json`** to `two-host-capture.json` and fill it in. Do not author one
from scratch.

| Field | JSON type | Rule |
|---|---|---|
| `host_a` | string | host A's `HostId`, matching that half's bundle `host` field. Compared **trimmed** (`:949`). |
| `host_b` | string | **must differ from `host_a` after trimming** — `"host-a"` vs `"  host-a  "` is one host, and is refused |
| `shape` | string | which it actually was — but the obvious wording is REFUSED; see "Saying it was two machines" below |
| `claim_scope` | string | **verbatim**, see below — compared as exact bytes, **untrimmed** |
| `trust_anchor_established_out_of_band` | **boolean** | the mTLS anchor was established by a human, not the protocol |
| `host_b_audit_key_provisioned_separately` | **boolean** | host B holds an **independent** audit root |
| `stranger_verification` | string | **non-emptiness is the only check** — see below |

**Type contract: four JSON strings, two JSON booleans.** The booleans are read with `as_bool()` and
must be exactly `true` (`:932`). The **string** `"true"` is not a boolean and fails, as does `1`, as
does an absent field. Verified by execution against the real validator, 2026-08-22.

**`stranger_verification` is a sworn statement, not a verified result.** The validator checks only that
the string is non-empty (`:910-926`). It never executes `verify.py`, never parses the string, and
cannot distinguish `"verify.py OK on both halves"` from `"looks fine to me"`. The template even ships
an acceptable string (`two-host-capture.example.json:8`), so this field can be satisfied by copying a
file. Describing it as *"the result of `verify.py`"* — as an earlier revision of this table did —
overstates it by a whole control.

### `claim_scope` — exact bytes, no paraphrase

```
two keyed identities signed; not two machines, two processes, or two operators
```

### Overclaim tripwires

Every **top-level** string field is scanned (case-insensitive, with `-` and `_` normalized to spaces)
for `two machines`, `two operators` and `fully automated pairing`. An occurrence is excused only when
the word `not` **immediately** precedes it in the same field (`preceded_by_not`, `:1039-1045`); a
negation elsewhere in the capture disarms nothing.

**`not` must be its own word** (added 2026-08-25, proven by execution by the §A6 Edge Case Hunter). The
check requires a word boundary before `not`, so `"cannot two machines"` REDs — the `not` is fused into
`cannot` and does not count. `"definitely not two machines"` is fine (what precedes `not` is irrelevant),
and `"not-two-machines"` is fine (hyphens normalize to spaces before the scan, so the negation still
binds). Each occurrence is scanned independently: `"not two machines, but it really was two machines"`
REDs on the second one. The behaviour is fail-closed and correct; it was simply never written down. The verbatim `claim_scope` field is pinned
byte-for-byte and is the one field never scanned (`:978-980`).

**Top-level strings only.** The scan iterates `capture.as_object()` and skips every non-string value
(`:976-983`). It never recurses, so a phrase nested inside an array or an object is invisible to it.
That is a property of the scanner, not a licence — the previous wording, *"each operator-authored
string field"*, implied a depth the code does not have.

### ⚠ The `claim_scope` bytes are un-repeatable anywhere else

`claim_scope` is exempt from the scan. **Its text is not.** Copy those 78 bytes into any other
top-level string — an `operator_note`, a `summary`, a `provenance` field — and the capture REDS,
because the scope's own `or two operators` is preceded by `or `, not `not `. Reproduced against the
real validator 2026-08-22:

```
two-host-capture.json.operator_note asserts `two operators`, which no control in this story proves
```

There is no documentation of this anywhere else, and it is the likeliest way a careful operator — one
who restates the claim to be helpful — gets refused after the agent is billed.

### Saying it was two machines

The obvious sentence is refused. `shape: "two processes on one box, or two machines"` REDS, because
`two machines` is preceded by `or `. **A genuine two-machine run must therefore say so without using
that adjacent bigram.** The sanctioned honest string, verified admissible against the real validator
on 2026-08-22:

```
two distinct physical machines on separate hardware, separate OS kernels, separate NICs
```

It contains no contiguous `two machines`, so it passes; it says exactly what happened, so it does not
lie. Use it only if that is what you actually ran. `j1-crosshost-2d` ran the other shape (R3: *two real
OS processes on one box*, the shipped template string) — but the defect outlives that story, so the
sanctioned string is published here rather than in it.

⚠ **Do not reuse the CLI's token.** `maosctl audit record-capture` *mandates* `two-machines` for its
own `two_host_shape` field (`CAPTURE_TWO_HOST_SHAPES`, `crates/maos-cli/src/subcommands.rs:2406`),
while this gate normalizes the hyphen and **refuses** it. Two documents, two contracts, opposite
requirements on the same words — see "Two documents" below.

### Two documents, two rulesets

`two-host-capture.json` (this file's subject) and the **`CaptureDoc`** consumed by `maosctl audit
record-capture` are different files read by different validators, and their extra-field rules are
opposites:

| | `CaptureDoc` (`record-capture`) | `two-host-capture.json` (this gate) |
|---|---|---|
| Extra fields | **explicitly permitted**, preserved verbatim in the journaled row (`subcommands.rs:2409-2412`) | tolerated only by **silence** — no `additionalProperties` rule exists |
| Extra top-level strings | screened for credential *prefixes* only | **fed to the overclaim tripwire** |
| Shape vocabulary | `two-processes-one-box` \| `two-machines`, closed set | free prose, tripwire-scanned |

Put durable extra metadata — a release-binary `sha256`, build provenance — in the **`CaptureDoc`**,
where it is covered by the bundle signature. Anything added here must stay clear of `sk-` / `AKIA` /
`ASIA` substrings and of all three tripwire phrases.

## Two operator steps that must actually be performed

1. **Boot-nonce pairing.** ✅ **Repaired and executable as of `j1-crosshost-2e` AC5 (2026-08-22).**
   The previous procedure was **topologically wrong, not merely undocumented**: it said to read host A's
   nonce from its `cohort:daemon-started` Transparency-Log row, but that row is written **only in daemon
   mode** (`crates/maos-bin/src/main.rs:9381`, emitted `:9548-9555`), while host A is `maos run --once`,
   which takes the cross-host arm *precisely because* `MAOS_ONE_SHOT != "cohort-a2a-daemon"` (`:2455`).
   **Host A never emitted the row the procedure named** — host A is the *sender*, and that row is a
   *receiver's*. "Run host A as a daemon instead" is not a fix either: daemon mode sets the cross-host
   router to `None`. The nonce also did not exist before the dial (minted `:1865-1878`, transport binds
   `:2454-2471`, delegation emitted `:3237-3270`, one process, no pause), and there is **no retry window
   to wait in**: a refused or timed-out `TcpStream::connect` returns `Io` immediately, because
   `is_retryable` admits only `BadCertificate`/`CertExpired`
   (`crates/maos-a2a-core/src/mtls.rs:73-83` — note the crate: there is **no**
   `maos-a2a-tcp/src/mtls.rs`). The `[100, 300, 1000] ms` ±20% schedule (`:12-28`) is the cert-class
   retry budget, not a startup grace period. The file this README used to name, `a2a-peers.toml`, exists
   nowhere: the real surface is `tcp.peer_pins[].boot_nonce`
   (`crates/maos-a2a-tcp/src/config.rs:24-37`) under `deny_unknown_fields`.

   **The procedure now:** host A publishes its own nonce under its own
   `cohort:crosshost-started` intent — **after** the transport binds and **before** the dial — then
   **holds** on an opt-in barrier so a human has time to transcribe it:

   ```bash
   # on host A, before `maos run spirits/topologies/j1-founder-loop-crosshost.toml --once`
   export MAOS_CROSSHOST_PAIRING_READY_FILE=/tmp/host-b-ready
   export MAOS_CROSSHOST_PAIRING_TIMEOUT_SECS=300      # default 300; expiry FAILS CLOSED
   ```

   Host A prints the nonce **in decimal** and waits. Read it back from the TL if you prefer a machine
   path — `maosctl audit query --intent-contains cohort:crosshost-started --format ndjson` — but **never
   with `--format plain`**, whose `boot_nonce` column renders as `{:016x}` **hex** under a header
   literally named `boot_nonce`. Put the decimal value in host B's `tcp.peer_pins[].boot_nonce`, start
   host B, then `touch /tmp/host-b-ready` to release the dial. If the file never appears, host A
   **refuses to dial** rather than spending its single non-retryable connect attempt.

   ⚠ The nonce is still a fresh random value per process, deliberately: making it stable or derived
   would defeat `NFR-Rel-6` restart detection (`tofu.rs:351-372`). A wrong transcription is **worse than
   a failure** — it invalidates the pin, and recovery needs a host B restart, because the second attempt
   fails *differently* (`invalidate_if_boot_nonce_differs`).
2. **Host B's audit key, provisioned separately.** `reconcile_two_host_bundles` hard-refuses
   `key_a == key_b` with `SharedAttesterRoot`. Two processes on one box default to one `HOME` and
   therefore **one key file** — so without this step reconciliation fails by construction, and "two
   hosts" would degrade to "two identities" even if it did not. ⚠ `MAOS_HOME` **does not** redirect the
   audit signing key (`crates/maos-domain/src/audit_key.rs:88-118`); use `--audit-key` or
   `MAOS_AUDIT_KEY`.

### What the release build must be, and how to prove it

A debug-build run produces a **byte-identical, gate-admissible capture**. Nothing in these four
artifacts distinguishes one, and `check-mock-not-in-release` cannot help — it greps the symbol table
for `MockHaltResolver`/`FailingHaltResolver` only (`xtask/src/check_mock_not_in_release.rs:31`), while
`MAOS_TEST_BOOT_NONCE` is gated on a **runtime** `cfg!(debug_assertions)` (`main.rs:1865-1878`), a
codegen flag that `RUSTFLAGS="-C debug-assertions=yes" cargo build --release` re-enables silently.

The falsifier is free, needs no cohort manifest, no daemon and no API key: run the **loopback**
topology `--once` **twice** with the **same** `MAOS_TEST_BOOT_NONCE`, then read both nonces back from
the TL. ⚠ **Give each run its OWN state home.** A second run against a home the first one populated
fails with `orchestrator dispatch references raw worker output not a distillate` — a real pre-existing
defect, unrelated to this check, and one that will read as a falsifier failure if you let it happen.
The nonce is journaled *before* that error, so the check still works dirty; it just looks broken.

```bash
# Run this from the REPOSITORY ROOT. `maos run` takes a <manifest> path — there is
# no `--help` for the subcommand and a bad path fails only AFTER ~30 init lines.
for i in 1 2; do
  H=$(mktemp -d)
  HOME=$H MAOS_HOME=$H XDG_DATA_HOME=$H MAOS_TEST_BOOT_NONCE=424242 \
    ./target/release/maos run spirits/topologies/j1-founder-loop.toml --once >/dev/null 2>&1
  echo -n "run $i (rc=$?) distinct boot_nonce: "
  HOME=$H MAOS_HOME=$H XDG_DATA_HOME=$H \
    ./target/release/maosctl audit query --range 1d --format ndjson \
    | python3 -c 'import json,sys
seen={r["boot_nonce"] for l in sys.stdin
      for r in [json.loads(l)] if r.get("boot_nonce") is not None}
print(sorted(seen))'
done
```

Every TL row of one boot carries that boot's nonce, so **de-duplicate** — the raw query prints the
same value ~19 times and an operator scrolling past it cannot see whether run 2 differed.

- **Two distinct values, neither equal to the override ⇒ genuine release build.**
- **The override read back ⇒ debug assertions are on. STOP; do not spend.**

Executed 2026-08-22 (`j1-crosshost-2d` AC1), override `424242`: release read back `9046754445710571789`
then `1928460524043859277`; the **debug** control read back `424242` twice. Re-executed with the
fresh-home block above, verbatim, on 2026-08-22 (`2e` follow-up): release `3493138385670016305` then
`7671059280199450930`, both `rc=0`; `target/debug/maos` read back `424242` **twice**, both `rc=0`. The
falsifier is proven to **discriminate**, not merely to pass. Record the release binary's `sha256` in
the **`CaptureDoc`** (not here) so the claim is covered by the bundle signature.

## Verifying before you claim

```bash
python3 tools/verify-audit-bundle/verify.py host-a-bundle.json <pubkey-a>
python3 tools/verify-audit-bundle/verify.py host-b-bundle.json <pubkey-b>
cargo run -p xtask -- check-j1-two-host-signed-run --json | jq .
```

`verify.py` is the **stranger's path** — field-agnostic, not our Rust verifier. Our own `verify-bundle`
verifying our own artifact is a self-check.

> ✅ **`verify.py` was BROKEN and is FIXED as of `j1-crosshost-2e` AC1 (2026-08-22).** For the record,
> because this was the single defect that would have killed the paid run: `verify.py:93` omitted
> `ensure_ascii=False`, so Python escaped non-ASCII to `\uXXXX` while Rust's `canonicalize_value`
> (`crates/maos-audit/src/sealed_export.rs:632-639`) emits raw UTF-8. **Any bundle containing a single
> non-ASCII byte failed verification even though its signature was valid** — and runbook Phase 7.4 makes
> that a mandatory abort, so the run died there *after both agents were billed*. Reproduced free against
> the real T6 artifact (12 non-ASCII bytes, valid signature) on 2026-08-19 and 2026-08-22; **T6, the only
> signed run this project has ever performed, was unverifiable by its own published stranger's path from
> the day it was signed until the fix landed.** Now:
>
> ```
> $ python3 tools/verify-audit-bundle/verify.py \
>     _bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json \
>     61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766
> OK — signature verified                       # exit 0
> ```
>
> ⚠ **The OpenSSL fallback in `tools/verify-audit-bundle/README.md` had THREE defects, not one**, and
> could never have worked: the same missing `ensure_ascii=False`; `-pkeyopt digest:SHA256`, which OpenSSL
> **refuses** for Ed25519 (it is not prehashed, and `-rawin` already says the digest IS the message); and
> `xxd`-ing a raw 32-byte key, which OpenSSL **cannot read** (it needs SPKI DER/PEM). It is rewritten and
> every line of the replacement was executed. CI now **executes** `verify.py` against the committed T6
> bundle (`.github/workflows/discipline.yml`) — installing `cryptography` for a path never run is exactly
> how a one-argument defect survived to be found by hand.
>
> Still run this on a representative bundle *before* spending anything. The fix is verified; your
> environment is not.

## What this gate cannot check, and who must

Leg 9 accepts a **single-root forgery**. `xtask/tests/j1_crosshost_2c_proven_red.rs:386-414` commits
the proof: the published template plus two bundle halves carrying **`"attester_pubkey": "aa".repeat(32)`
— identical in both halves, one root** — and `"signature": "bb".repeat(64)`, with no transcript, and
asserts `verdict.passed && verdict.success`. It passes because the two-root property is never checked
against the artifacts: leg 3 (`:395-478`) is a **source-text grep** that executes nothing, leg 9
(`:1006-1034`) compares only each `bundle["host"]` string to the capture, and leg 3 at `:458-470`
**forbids** reconciliation from reading `attester_pubkey` (R-RG1) — so the artifacts structurally
cannot carry the discriminator. The gate has **zero `Command::new`**. The only evidence of independence
is a boolean that ships **pre-filled `true`** in the very template the operator is told to copy
(`two-host-capture.example.json:7`).

**Therefore the discriminators are operator-performed and pasted, not gate-checked.** Read the JSON
fields — `paid_run_capture_present` and `two_host_signed_run_claimed` — and never the exit code:
`passed` and `oracle_green` are green whether the capture is absent, valid, **or fabricated**.

⚠ `capture_signature_verified` and `capture_signature_reason` **no longer exist** (`2e` AC3 deleted
them, see above). If you are reading a script or a checklist that greps for either field, it is
pre-`2e` and its green is meaningless.
