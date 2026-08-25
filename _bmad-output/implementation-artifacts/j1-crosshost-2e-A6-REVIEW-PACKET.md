# §A6 Review Packet — `j1-crosshost-2e-two-host-run-enablement`

> **Run this in a session whose model is NOT `anthropic/claude-opus-5`.**
> That model authored the story AND devved it. §A6 is NON-DEGRADABLE and requires the reviewer model
> to differ — the point is uncorrelated blind spots, so a fresh context on the same model does not
> satisfy it. Suggested: `zai/glm-5.3` (matches `2b`/`2c`). Any of `glm-5.*`, `gpt-5.*`, `opus-4-*`
> is allowlisted by `xtask/src/check_dev_model_tier.rs`.
>
> **This packet was assembled BY the dev model.** Treat it as a map, not as testimony. Every claim
> below is checkable and you should check the load-bearing ones. Where the dev believes something is
> proven, the exact command is given so you can re-run it rather than trust it.

## 0. Invocation

```bash
# In the reviewer session, from the repository root:
#   skill: bmad-code-review
#   target: _bmad-output/implementation-artifacts/j1-crosshost-2e-two-host-run-enablement.md
# Unlike its predecessor `j1-crosshost-2d` (which was code-free and needed the review
# re-targeted at documents), THIS story has a real Rust diff, so all four layers run as-specified
# against {diff_output} with NO re-targeting.
```

**Four layers, all required:** Blind Hunter · Edge Case Hunter · Acceptance Auditor · Test-Infra
Auditor. The Test-Infra layer is NOT skippable here: `dev_model_used` is `anthropic/claude-opus-5`
and the §A6 skip rule applies only when the dev model was non-Claude.

## 1. Review scope — the exact diff

12 files, **+579 / −109**. Nothing is committed; the diff is the working tree against `dd4cf959`.

| File | + | − | AC | What it does |
|---|---|---|---|---|
| `tools/verify-audit-bundle/verify.py` | 14 | 2 | AC1 | `ensure_ascii=False` (F5) + the rewritten OpenSSL fallback |
| `.github/workflows/discipline.yml` | 14 | 0 | AC1 | CI now **executes** `verify.py` against the committed T6 bundle |
| `crates/maos-cli/tests/two_host_reconcile_2c.rs` | 63 | 0 | AC1 | the non-ASCII proven-red regression |
| `crates/maos-cli/src/cli.rs` | 50 | 0 | AC2 | `Subcommand::Cohort`, `CohortArgs`, `CohortOp::Sign` |
| `crates/maos-cli/src/subcommands.rs` | 168 | 0 | AC2 | `dispatch_cohort` + `cohort_sign` |
| `crates/maos-cli/Cargo.toml` | 7 | 0 | AC2 | `maos-cohort` + direct `ed25519-dalek` |
| `xtask/src/check_j1_two_host_signed_run.rs` | 48 | **73** | AC3 | F2: deletes the unreachable verifier, the transcript const, two JSON fields |
| `xtask/src/demo_j1.rs` | 37 | 29 | AC3 | F3: conditional `executed`; drops the deleted verifier call |
| `crates/maos-bin/src/main.rs` | 133 | 2 | AC4, AC5 | F7 two-tier delegated goal; F4 pairing rendezvous |
| `crates/maos-bin/src/env_contract.rs` | 40 | 0 | AC4, AC5 | three registry entries |
| `xtask/kloc.toml` | 3 | 3 | AC6 | two measured grants; `xtask` unchanged |
| `Cargo.lock` | 2 | 0 | AC2 | dependency closure |

**Net deletion in the gate is the point of AC3**, not an accident: `−73` there is the unreachable
claim branch coming out.

## 2. The six ACs and where to attack each

### AC1 (F5) — `verify.py` canonicalization

```bash
python3 tools/verify-audit-bundle/verify.py \
  _bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json \
  61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766     # expect: OK, exit 0
cargo test -q -p maos-cli --test two_host_reconcile_2c                  # expect: 10 passed
```

**Attack surface.** Is the new test non-vacuous? The dev claims it fails with the fix reverted.
Revert `ensure_ascii=False` and re-run — it must fail. Also: the OpenSSL fallback in
`tools/verify-audit-bundle/README.md` was rewritten and the dev claims every line was executed;
three defects were claimed (missing `ensure_ascii`, `-pkeyopt digest:SHA256` unsupported for
Ed25519, raw-key `-inkey`). **Execute the replacement block.** If any line does not run, the same
defect class the story exists to fix has been reintroduced one layer down.

### AC2 (F1) — `maosctl cohort sign`

```bash
cargo build -q --release -p maos-cli
./target/release/maosctl cohort sign --help
```

**Attack surface, and this is the highest-risk AC.** A signer is a forgery tool if it is wrong.
Three refusals are claimed:
1. `--authority-key` explicit, **no env fallback** — verify that no code path reaches
   `MAOS_AUDIT_KEY` here. Welding the cohort root to the audit root is the exact collapse
   `reconcile_two_host_bundles` refuses.
2. validate-before-sign.
3. **refuses to sign a manifest whose `authority.keys` omits the signer.** `signed_with` does not
   check this. Try to defeat it.

Also check: does it re-verify its own output before writing? Does it write on failure? Does the
`--output` path get created/truncated before validation passes?

### AC3 (F2+F3) — the gate says less

```bash
cargo test -q -p xtask --test j1_crosshost_2c_proven_red     # expect: 42 passed
cargo run -q -p xtask -- check-j1-two-host-signed-run --json | jq '{passed, paid_run_capture_present, two_host_signed_run_claimed}'
cargo run -q -p xtask -- demo-j1 --skip-build; echo "exit=$?"  # expect 0
```

**Attack surface.** The dev claims **no replacement term was added** and the gate still has **zero
`Command::new`**. Verify both mechanically:

```bash
grep -c 'Command::new' xtask/src/check_j1_two_host_signed_run.rs      # expect 0
grep -n 'capture_signature\|CAPTURE_TRANSCRIPT\|operator_evidence' xtask/src/check_j1_two_host_signed_run.rs
```

Then ask the harder question: is `two_host_signed_run_claimed: false` as a **hardcoded literal** the
right shape, or did deleting the computation remove a control? The dev's argument is that the third
term was structurally unsatisfiable so the conjunction was always `false` anyway. Check that
argument, don't accept it.

### AC4 (F7) — the delegated goal

```bash
cargo test -q -p maos-bin --test two_host_delegation_2b       # expect: 3 passed
cargo test -q -p maos-bin --test topology_delegation_1a       # expect: 14 passed
cargo run -q -p xtask -- check-j1-loopback-delegation
```

**Attack surface — the dev already broke this once.** First attempt keyed fail-closed on the
cross-host arm alone and **red `two_host_delegation_2b`**; the story's own E7 had predicted it. The
shipped discriminator is cross-host arm **AND** `MAOS_LIVE_AGENT`. Ask: is that the right seam? Is
there a path where the paid arm runs without `MAOS_LIVE_AGENT`, so a missing goal silently falls
back to the rehearsal constant on a run that bills money?

### AC5 (F4) — the pairing rendezvous

New env vars: `MAOS_CROSSHOST_PAIRING_READY_FILE`, `MAOS_CROSSHOST_PAIRING_TIMEOUT_SECS`.
Claimed properties: publishes host A's nonce under `cohort:crosshost-started` after the bind and
before the dial; holds on a bounded barrier; **fails closed** on expiry; nonce stays a fresh random
per-process value so `NFR-Rel-6` restart detection is untouched.

**Attack surface.** Is the hold really between bind and dial, or does it sit somewhere that changes
observable ordering? Does a pre-existing ready-file skip the hold entirely (TOCTOU / a stale file
from a previous run)? Is the timeout parse fail-closed on a garbage value? Does the new code path
run at all when the cross-host router is `None`?

### AC6 — the budget

```bash
cargo run -q -p xtask -- kloc-check --json | jq '.over_budget'
cargo run -q -p xtask -- check-kernel-baseline --json | jq '{passed, actual_lines}'   # 24472 == 24472
```

Claimed: `xtask` 39966 → **39918** (the 2c `+6` breach **retired, not granted**); grants taken only
for `maos-cli` (+140) and `maos-bin` (+85); net **+177** == the aggregate delta exactly; **no
aggregate grant**.

**Attack surface.** Re-measure. `cargo fmt --all` first — the dev reports the numbers moved twice
under formatting, and the convention requires FORMATTED measurement. Confirm the arithmetic closes
and that no ceiling was raised beyond the measured actual.

## 3. Declared pre-existing reds — do NOT file these as new findings

All verified pre-existing at HEAD by stashing and re-running. The dev disclosed and deliberately did
not fix them (scope). If you disagree with that judgment, say so as a **process** finding, not as a
regression:

| Gate | State |
|---|---|
| `kloc-check` | RED — `aggregate`, `maos-domain` (D14), `maos-kernel-core` (D13) |
| `check-env-contract` | RED — `MAOS_OPERATOR_BEARER_TOKEN`, `MAOS_OPERATOR_HTTP_BIND` unregistered |
| `check-empty-kernel` | RED — I9 violations + two undocumented `#[i9_exempt]` |
| `check-service-boundary` | RED — P3, same root cause |

All four live in `crates/maos-kernel-core/src`, which this story never touched. The story's three new
env vars **are** registered.

## 4. Declared claim boundaries — already published, not findings

`RELEASE-HOLDS.md` rows 13–16. Row 16 was added by this story after a free probe and is the one most
worth your scrutiny:

> **`claude`'s completion verdict is NOT an effect oracle.** `codex_jsonl_oracle` requires native
> effect evidence and emits `NoEffectEvidence` without it (`worker_cli.rs:462-465`).
> `claude_result_object_oracle` (`:498-537`) has no effect check at all. Measured: a permission
> denial → `not_completed:permission_denied`, exit 1 ✅; a clean object claiming *"I have written the
> file"* over an untouched tree → **`completed=true`, exit 0** ⚠. Host B runs claude, and
> `completed=true` is the admission condition for signing.

The dev **did not fix this** and argues it is a design decision out of scope (claude's result object
carries no per-file change list, so an effect oracle needs a different signal). **Challenge that
call if you think it is wrong** — it is the difference between a signed artifact attesting remote
*work* and one attesting remote *non-refusal*.

## 5. Preconditions — verified green before handoff

```bash
for g in check-j1-two-host-signed-run check-kernel-baseline check-j1-loopback-delegation \
         check-dev-record-completeness check-dev-model-used-populated check-dev-model-tier \
         check-ship-gate-completeness check-mock-not-in-release check-dependency-closure \
         check-bare-review-findings; do
  printf "%-34s " "$g"; cargo run -q -p xtask -- $g --json >/dev/null 2>&1 && echo OK || echo "rc=$?"
done
cargo fmt --all --check && echo "fmt CLEAN"
```

All OK at handoff. Suites: `two_host_reconcile_2c` 10 · `signing_identity_2c` 7 ·
`two_host_bundle_2c` 12 · `j1_crosshost_2c_proven_red` 42 · `two_host_delegation_2b` 3 ·
`worker_completion_2a` 45 · `topology_delegation_1a` 14 · `consent_refusal_1b` 7 ·
`xtask --bin xtask` 492. Zero failures.

## 6. What the dev got wrong, self-reported

Two of these were found by the **operator**, not the dev. Weight that when calibrating how much of
the dev record to trust:

1. **AC4 discriminator** — took a scout's recommendation over the story's own E7 warning, red
   `two_host_delegation_2b`. Corrected.
2. **Runbook placeholders** — the Phase 7 blocks the dev wrote used `<topology>`; the operator
   substituted `./topology/a` literally and hit `failed to read manifest` after ~30 lines of healthy
   boot. Led to a new `Phase 0.0` invocation contract.
3. **Host B command was wrong** — the dev wrote `maos run …crosshost.toml` for the daemon. The daemon
   takes **no `run` and no topology**; that command makes host B execute host A's founder loop and
   never receive the delegation. Found by executing it.
4. **Falsifier output unreadable** — printed the nonce once per TL row (~19 identical lines). Found
   by executing the dev's own instructions.

## 7. Output contract

Write findings into the story's own record so `check-bare-review-findings` and
`check-dev-record-completeness` stay green:

- a `## Senior Developer Review (AI)` section with outcome (Approve / Changes Requested / Blocked),
  date, reviewer **model**, and per-finding severity;
- a `### Review Follow-ups (AI)` subsection under Tasks/Subtasks with `[AI-Review]`-prefixed
  checkboxes;
- the review-net marker (`§A6`, `Blind Hunter`, `Acceptance Auditor`, or `REVIEW COMPLETE`) —
  `check_dev_model_tier.rs:44-48` looks for at least one.

Then: `cargo run -p xtask -- check-dev-record-completeness --json | jq .violation_count` → `0`.
