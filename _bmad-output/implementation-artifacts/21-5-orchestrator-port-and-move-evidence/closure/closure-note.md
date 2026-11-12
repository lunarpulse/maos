# 21-5 closure note (2026-11-12)

Record-only closure (Close215). No source, kernel file, or persisted review was changed.

## Commits and hosted proof

| Item | Value |
|---|---|
| L1 (kernel-free) | `257a0fd7` |
| L2 (kernel move) | `637721bf` |
| Reviews persisted | `41b937cc` (`landing-2/review/`) |
| Discipline run on `637721bf` | 43175946640, `success` except the ADVISORY job below |
| Green on that run | `abi-diff`, `check-kernel-baseline`, `check-empty-kernel`, `workspace-test-suite`, ship gates (199 jobs) |
| Failed job | `reproducible-build` (job 131040106237): `No space left on device`, `collect2: ld terminated with signal 7 [Bus error]`, runner free space 79 MB. Hosted-runner disk exhaustion, infrastructure, not a code failure. Advisory per `.github/workflows/discipline.yml:60-66` (`continue-on-error`). It was `success` on run 43150757804 (`d00802aa`). Filed as a rule-10 row in `deferred-work.md` (21-5 L2 review section). |
| Earlier L1 run 43153299381 on `257a0fd7` | failed only on the 1M erasure oracle (fixed by `5fe97eda`, ruling `ac5`); `5fe97eda`'s run failed on the `ac4_v3b` ROOT_GRACE load flake (20-7 family, dispositioned) |

Measured at `637721bf`: kernel 26,062 physical / 20,429 tokei / 90 files (from 27,103 / 21,256 / 95: -1,041 / -827); new leaf `maos-orchestrator-buffer` 833; xtask 45,462; hardfail 203,251; surface baseline 401 -> 380 (21 symbols removed, ADR-070), one refresh `SpiritSchedulerAdapter`.

## AC table

| AC | Verdict | Evidence |
|---|---|---|
| AC1 command + kernel pins | LANDED (`637721bf`) | `landing-2/gates/`, hosted `check-kernel-baseline`, `check-empty-kernel` success |
| AC2 the move | LANDED (`637721bf`) | `landing-2/evidence.md`, `green/` |
| AC3 control / gate refusing a path back | LANDED (`637721bf`) | `xtask/tests/orchestrator_out_of_kernel_21_5.rs`, plants in `landing-2/plants/` |
| AC4 port in the domain | LANDED (`257a0fd7`) | `landing-1/evidence.md` |
| AC5 equivalence goldens | LANDED (`257a0fd7`, imports updated `637721bf`) | `landing-1/`, `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs` |
| AC6 routed FR21 exemption | VOID | ruling D-21-5-14 B (ask `d21`, 2026-11-12); the row stays open in `deferred-work.md` with no 21-5 owner |
| AC7 records | LANDED (`637721bf`) | pins, surface, ADR-070, STABILITY, tracker |

## Claim boundaries

- The move is a verbatim relocation; the one behavioural change is the D-21-5-07 B one-lock replay (identity on the queue).
- FR54 gateway host remains unwired: zero production callers, allow-all stubs (row F1).
- `journal_*` audit writers left the kernel freeze gates; one maos-bin golden pins their row shapes (row F2).
- `resume` skips the replay when the port is `None` (row F3).
- FR20 frame-level delivery stays deferred to Story 5.4.
- AC6 voided: the FR21 no-prior-distillate exemption and the `with_capacity(0)` rows stay open, owned by no story.

## Unverified

- No local clean-tree `abi-diff` run; only the hosted `abi-diff` job (success on 43175946640).
- `reproducible-build` did not complete on the landing run (infrastructure); no hosted reproducibility reading for `637721bf`.
- Hashes in the hand-spliced pins were not independently recomputed by the reviewers; the hosted gates passed.
