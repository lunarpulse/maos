# 20-14 non-kernel KLOC overdraft adjudication record — 2026-11-12

Purpose: answer the round-3 GPT review's single P2 (`landing-2/review-r3/gpt-review.md`): the final KLOC approvals
(capability 1454, domain 10274, cli 6612, iac 8773) were labelled "Main-adjudicated" in `landing-2/review-r2/evidence.md:31`,
in `xtask/kloc.toml` HISTORY rows and in `deferred-work.md` while citing a review passage that says measurement consistency
is not approval. The persisted evidence files are NOT rewritten; this note is the pointer they should have carried.

## Who adjudicated, on what authority, and what is NOT claimed

- **Adjudicator:** Main (the orchestrator), per overdraft, as each was measured. Main disclosed each one to the operator in
  the 2026-11-12 progress reports; the operator did not object. The same wording ("orchestrator-adjudicated overdraft,
  disclosed to the operator") is the established practice already used for the 20-14 L1 overdrafts (domain 10215, CLI
  6545/6610, iac 8770; story `landing_1_grant`) and for 20-8 / 20-9 / 21-5 (`xtask/kloc.toml` HISTORY rows).
- **Standing rule these raises satisfy:** the operator-ratified RECOVERY-LANE CEILING RULE (2026-09-12, Epic-15 retrospective;
  `xtask/kloc.toml` header): any story may raise any non-kernel per-crate ceiling provided (1) the code exists and was
  `cargo fmt --all`-measured before the ask, (2) the raise cites a `sprint-status.yaml` story key that is not `done`
  (20-14 was `in-progress` at every landing), (3) the measured figure and driver land in the same commit. Each commit
  below (edeedd25, 456793ca, 9996fe09) carried the exact formatted figure and the driver in `xtask/kloc.toml`.
- **NOT claimed:** no operator numeric ruling for these specific figures exists; the creation-time grant (`kloc_grant`,
  D-20-14-09 A) allowed these crates the creation-priced ceilings only, so every amount below is an overdraft over that
  grant resting on Main's adjudication plus the recovery-lane rule above, not on a named operator ruling. The kernel
  (`maos-kernel-core`) is NOT part of this note: it stays at its own ledger and FLAG-Winston grant (kernel 21201 -> 21203 -> 21202
  tokei, 27040 -> 27043 -> 27044 physical across the landings; STOP 22000, ruling `lim=stop22` 2026-11-12; 20-14 raises no kernel pin
  beyond its ruled KT row). If the operator wants a numeric ruling, it is a one-line `kloc.toml` re-pin, not a re-run.
- **Reason (all kernel-free correctness work):** the token-bound cost scope in `maos-capability` (shape B after STEP 0 showed
  the surface-hash-moving shape A; round 1 removed the wide accessor and the second map); the identity codec
  (`encode_identity`/`decode_identity`, round 1 P2-2) and the `x:` colon-hex cover in `maos-domain`; the session-principal
  length cap in the reader (`maos-cli`); and, in round 2, the walker key parameter and the shared replay predicate call
  (`maos-iac`, `maos-bin`) that fix GPT P1/P2.

## Per crate (numbers from `git show <sha>:xtask/kloc.toml`, read-only)

| Crate | Before 20-14 L2 (`edeedd25^`) | edeedd25 | 456793ca | 9996fe09 (final 20-14) | Over the before-L2 ceiling at final | `xtask/kloc.toml` today (uncommitted tree) |
|---|---:|---:|---:|---:|---:|---:|
| maos-capability | 1433 | 1444 | 1454 | **1454** | +21 | 1454 |
| maos-domain | 10217 | 10217 | 10274 | **10274** | +57 | 10277 (21-5 +3, not 20-14) |
| maos-iac | 8770 | 8770 | 8771 | **8773** | +3 | 8875 (20-9 L2, not 20-14) |
| maos-audit | 8308 | 8308 | 8308 | **8308** | 0 | 8320 (20-9 L2) |
| maos-cli | 6609 | 6609 | 6612 | **6612** | +3 (ceiling was 6610 after L1: +2) | 6612 |
| maos-bin | 38475 | 38549 | 38556 | **38564** | +89 | 38619 (20-8 / 20-9 / 20-13 / 20-1b work, not 20-14) |
| maos-wasm-host | 3141 | 3141 | 3141 | **3141** | 0 | 3149 (20-9 L2) |
| aggregate hardfail | 202751 | 202856 | 202936 | **202921** | +170 | 203268 |
| maos-kernel-core (own ledger, context) | 21181 | 21201 | 21203 | 21202 | +21 | 20429 (21-5 move) |

Notes: the "before" column is the live pin when the 20-14 L2 work began, which already includes 20-14 L1's adjudicated
draws (domain 10215, cli 6610, iac 8770 were adjudicated earlier, 2026-11-09..11; see story `landing_1_grant`). The
maos-bin and aggregate movement from 38475 / 202751 is shared with sibling stories landed between those commits
(20-13 L2 etc.); 20-14's own bin lines are the shell/seam/hello binding and `MAOS_SESSION_PRINCIPAL` parse
(+74 at edeedd25, +7 round 1, +8 round 2, measured against the then-live pin; maos-bin was not among the crates the round-3 P2 named, and this note makes no claim about whether its draws were inside the `landing_1_grant` allowance). maos-audit and maos-wasm-host were
not touched by 20-14 L2 and show 0. The table above is the record of "amounts over the prior ceiling" for the four crates the P2 named: capability (+21), domain (+57), iac (+3) and cli (+3).
The final 20-14 pins stand at 9996fe09; later movement in these rows belongs to other stories' HISTORY rows.

## Pointers

- `landing-2/review-r2/evidence.md:31` ("Main-adjudicated 2026-11-12, supported ... see the review's overdraft assessment"): read
  it as "Main-adjudicated, disclosed to the operator; this note is the dated record". The review assessment it cites says
  measurement consistency is not approval and is correct on that point; it supports the design, it does not approve.
- `xtask/kloc.toml` HISTORY rows for capability / cli (still the words "PENDING Main re-adjudication") and
  `deferred-work.md` row (20-14 L2 close-out): both resolved by this note on 2026-11-12 (the deferred row is stamped CLOSED).
- Round-3 verdict: `landing-2/review-r3/gpt-review.md` (0 P0/P1/P3; this P2 only).
