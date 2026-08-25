# Published audit-root fingerprints for the J1 two-host signed run

> **This file is a COMMITMENT, and its only value is that it was committed BEFORE the run.**
>
> A public key read out of our own `sealed-export` *after* the artifacts exist proves that the
> bundle is internally consistent with whatever key happened to sign it. It proves nothing about
> *identity* — it is a self-check wearing a stranger's coat. Publishing the fingerprints first
> converts the later verification into a real check: a stranger compares the bundle's signer
> against a value that was fixed in git history before the bundle could exist.
>
> Runbook `runbook-j1-tier-2-signed-live-run.md` §7.1 says *"publish FPR_A / FPR_B"* and, until
> 2026-08-22, named no file. **This is that file.** Discharges `j1-crosshost-2d` AC4.1 / T6.

Published **2026-08-22**, at repository HEAD `dd4cf959`, by `j1-crosshost-2d-paid-two-host-run`.
The paid run (AC8) has **not** happened. As of 2026-08-22 it is **no longer gated on code**:
`j1-crosshost-2e` closed all six blockers. It is gated on operator substrate — two provisioned
hosts, a clean sandbox home, and a funded metered API key.

---

## The two host roots

Both keys were minted with `maosctl audit keygen` on 2026-08-22 and are **independent** — neither
is derived from the other, and neither is derived from a shared base seed. Private key material
lives outside the repository, mode `0600`, and never enters this file, any bundle, or any sandbox.

| Role | 64-hex Ed25519 public key | Private key (never committed) |
|---|---|---|
| **`FPR_A`** — host A audit root | `4bbc1187ddf5908d9e96eecdbef6bb9fdfbc42a7977bc886c5d41046220be344` | `~/.maos/keys/j1-2d-host-a-audit.key` (64 bytes, `0600`) |
| **`FPR_B`** — host B audit root | `843dc5a83dbbebcf3c5c5fbe79a45bdba405f61879f77eb932a741708e7296f3` | `~/.maos/keys/j1-2d-host-b-audit.key` (64 bytes, `0600`) |

`FPR_A != FPR_B`, verified at publication. This is the property `reconcile_two_host_bundles`
refuses with `SharedAttesterRoot` when it does not hold, and the property the capture's
`host_b_audit_key_provisioned_separately: true` swears to.

### The run is bound to these two keys

The paid run **must** export host A's half with `--audit-key ~/.maos/keys/j1-2d-host-a-audit.key`
and host B's half with `--audit-key ~/.maos/keys/j1-2d-host-b-audit.key`. If either half's signer
does not match the fingerprint above, the run is **not** the run this file committed to, and the
correct response is to say so — not to update this file.

---

## The operator receipt key is a THIRD key, and it is not T6's signer

`reconcile-hosts --receipt-key $OPERATOR_KEY` signs the two-host receipt. That key must be
distinct from both host roots: it attests the *reconciliation*, not either half.

| Role | 64-hex Ed25519 public key | Key file |
|---|---|---|
| `$OPERATOR_KEY` — receipt signer | `433b27c18643a7a3abbd593fb381a1ae32a695d563e691bd746f39af33d48a3a` | `~/.config/maos/audit-signing.key` (64 bytes, `0600`) |

> ⚠ **Measured 2026-08-22: this is NOT the key that signed T6.**
> The published T6 bundle (`_bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json`)
> was signed under pubkey `61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766`.
> The key currently at `~/.config/maos/audit-signing.key` derives to `433b27c1…`. **They are
> different keys.** An operator who assumes `$OPERATOR_KEY` is "the J1 signing key" and publishes
> T6's fingerprint for the two-host receipt will produce a receipt that verifies against nothing,
> and the failure will look like a broken signature rather than a wrong fingerprint. Recorded here
> because there was no other place in the repository where the two values could be compared.

---

## How a stranger uses this file

```bash
# 1. Take the fingerprints from THIS file at a commit that predates the bundles.
git log --oneline -- _bmad-output/test-artifacts/j1-two-host-evidence/PUBLISHED-FINGERPRINTS.md

# 2. Verify each half against the fingerprint published for that half — never against a
#    pubkey read out of the artifact (R-RG1 forbids trusting `attester_pubkey`, and
#    xtask/tests/j1_crosshost_2c_proven_red.rs:458-470 machine-enforces the prohibition).
python3 tools/verify-audit-bundle/verify.py host-a-bundle.json <FPR_A above>
python3 tools/verify-audit-bundle/verify.py host-b-bundle.json <FPR_B above>

# 3. Confirm the two halves do not share a root.
#    The GATE cannot do this for you — see "What the gate cannot check" in README.md.
test "<FPR_A>" != "<FPR_B>"
```

> ✅ **Step 2 works as of `j1-crosshost-2e` AC1 (2026-08-22).** For the record: `verify.py:93` omitted
> `ensure_ascii=False` and failed on any bundle containing a non-ASCII byte, valid signature or not —
> reproduced against the real T6 artifact on 2026-08-19 and 2026-08-22. **The fix is one keyword
> argument, and until it landed the only signed run this project has ever performed was unverifiable
> by the very path this file tells a stranger to use.** Now prints `OK — signature verified`.

---

## Change log

| Date | Change |
|---|---|
| 2026-08-22 | File created by `j1-crosshost-2d-paid-two-host-run` T6 / AC4.1. `FPR_A` and `FPR_B` minted and published **before** the paid run, discharging the runbook §7.1 instruction that had named no file since it was written. Operator receipt key recorded, and its **non-identity with T6's signer** measured and disclosed. |
