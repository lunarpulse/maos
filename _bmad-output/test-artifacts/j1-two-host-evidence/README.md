# J1 two-host evidence — the admission criteria, published

This directory is what `check-j1-two-host-signed-run` leg 9 (`paid-run-capture`) reads.
It is **empty of real artifacts until the paid run happens** (`j1-crosshost-2d-paid-two-host-run`),
and that is the honest state: with no capture present the gate is GREEN and the
`two-host-signed-run` demo beat stays **ABSENT**. The gate refuses the *claim*, not the absence.

> **Why this file exists.** The capture contract used to live only inside the validator source and a
> fixture buried in a proven-red test, while the runbook said `cp two-host-capture.json …` without
> ever stating what was in it. An operator would run a paid agent, follow the runbook exactly, invent
> a capture, and be rejected **after the agent was billed**. A judge that will not publish its own
> admission criteria is not finished.

## The four artifacts leg 9 expects

| File | Required when | What it is |
|---|---|---|
| `two-host-capture.json` | always, for a claimed run | the bounded claim (fields below) |
| `host-a-bundle.json` | whenever the capture is present | host A's signed `AuditBundle` half |
| `host-b-bundle.json` | whenever the capture is present | host B's signed `AuditBundle` half |
| `two-host-evidence.txt` | for `PROVEN_LIVE_SIGNED` | the operator-signed `MAOS-EVIDENCE-V1` transcript |

A capture present without both bundle halves is refused: *a claim without its artifact is the thing
this gate exists to refuse.*

## The capture's seven required fields

Copy **`two-host-capture.example.json`** to `two-host-capture.json` and fill it in. Do not author one
from scratch.

| Field | Rule |
|---|---|
| `host_a` | host A's `HostId`, matching that half's bundle `host` field |
| `host_b` | **must differ from `host_a`** — identical host claims are one host |
| `shape` | say which it actually was: two processes on one box, or two machines |
| `claim_scope` | **verbatim**, see below — the gate compares exact bytes |
| `trust_anchor_established_out_of_band` | the mTLS anchor was established by a human, not the protocol |
| `host_b_audit_key_provisioned_separately` | host B holds an **independent** audit root |
| `stranger_verification` | the result of `tools/verify-audit-bundle/verify.py` |

### `claim_scope` — exact bytes, no paraphrase

```
two keyed identities signed; not two machines, two processes, or two operators
```

### Overclaim tripwires

The gate scans the whole capture text (case-insensitive) and refuses `two machines`,
`two operators` and `fully automated pairing` unless the phrase is negated in place
(`not two machines`). Nothing in this story proves any of them.

## Two operator steps that must actually be performed

Both are **manual, and neither has ever been executed** — rehearse before the agent is billed.

1. **Boot-nonce pairing on a release build.** The `MAOS_TEST_BOOT_NONCE` override is
   `debug_assertions`-gated, so in release the nonce is random. The operator reads host A's nonce from
   its `cohort:daemon-started` Transparency-Log row and transcribes it into host B's static peer-pin
   config before boot.
2. **Host B's audit key, provisioned separately.** `reconcile_two_host_bundles` hard-refuses
   `key_a == key_b` with `SharedAttesterRoot`. Two processes on one box default to one `HOME` and
   therefore **one key file** — so without this step reconciliation fails by construction, and "two
   hosts" would degrade to "two identities" even if it did not.

## Verifying before you claim

```bash
python3 tools/verify-audit-bundle/verify.py host-a-bundle.json <pubkey-a>
python3 tools/verify-audit-bundle/verify.py host-b-bundle.json <pubkey-b>
cargo run -p xtask -- check-j1-two-host-signed-run --json | jq .
```

`verify.py` is the **stranger's path** — field-agnostic, not our Rust verifier. Our own
`verify-bundle` verifying our own artifact is a self-check.
