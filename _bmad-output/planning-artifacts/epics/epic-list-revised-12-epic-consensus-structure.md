# Epic List (Revised — 12-Epic Consensus Structure)

> **Crate-path retrofit status (round-3 + retrofit pass):**
>
> Stories with **full crate-path treatment** — every AC cites a concrete file or test path that a dev agent can act on directly:
> - **Story 1a.1** (workspace bootstrap — original exemplar Amelia praised)
> - **Story 1b.5a / 1b.5b / 1b.5c** (split from original 1b.5 with concrete paths throughout)
> - **Story 4.1** (halt mechanism + `MockHaltResolver` pattern + `crates/maos-eval/fixtures/halt-corpus-v0/` synthetic corpus)
> - **Story 3.3** (Director halt UX — integration partner of 4.1)
> - **Story 5.2** (Hot-Swap Coordinator + HSIS corpus authoring schedule — integration partner of 4.1)
> - **Story 5.5a–5.5e** (5 stories split from original 5.5 with paths)
> - **Story 7.5a / 7.5b** (2 stories split from original 7.5 with paths)
> - **Story 0.5** (corpus generators in `crates/maos-corpus-gen/`)
> - **Story 9.2** (GDPR cascade + proof-of-erasure with paths)
>
> Stories with **partial crate-path treatment** (some ACs cite paths, some still say "the kernel"):
> - **Stories 0.1–0.4** (CI gates cite `xtask/`, `tests/coverage-matrix.yaml` but not all kernel-touching ACs)
> - **Stories 1a.2 / 1a.3 / 1a.4** (cite `maos-kernel-core/` and `maos-spirit-abi/` but variably)
> - **Stories 1b.1–1b.4** (cite some crate paths but not consistently)
> - **Stories 2.1–2.4** (cite `maos-spirit-sdk` etc. but not all ACs)
> - **Stories 3.1 / 3.2 / 3.4** (some kernel-touching ACs still generic)
> - **Stories 4.2–4.5** (cite ADR numbers but not all crate paths)
> - **Stories 5.1 / 5.3 / 5.4** (cite some component names but not all)
> - **Stories 6.1–6.5** (cite ADR numbers but not all crate paths)
> - **Stories 7.1–7.4** (cite some interfaces but not all)
> - **Stories 8.1–8.5** (mostly cite `spirits/<class>/` paths which is appropriate since these are subprocess Spirit stories)
> - **Stories 9.1 / 9.3 / 9.4 / 9.5** (cite some `maosctl` subcommand paths but not all)
> - **Stories 10.1–10.5** (ship-gate coordination — cite gate artifacts but less crate-internal detail since these are integration-test stories)
>
> **For dev-agent consumption:** when an AC says "the kernel" without a crate path, the implementing agent should consult `architecture-maos-minimal-opus.md` §4.0.2 for the canonical crate-to-responsibility mapping. The 17-crate workspace is bounded; "the kernel" almost always maps to `crates/maos-kernel-core/<service>/` based on the AC's subject matter.
>
> **User decisions (party-mode convergence):** (1) **E4 is the single halt owner** — schema in E1a/E1b types only; mechanism + I14 invariant in E4; continuity-across-hot-swap dependent in E5. (2) **ComplianceClaim schema frozen at E1b** after E0 adversarial review; ABI break required to change thereafter. (3) **rust-inproc form gated on §13.1 measurement** — story lives in E5 with go/no-go before v0.5 ships; if subprocess form meets latency budgets, rust-inproc may be deferred. (4) **12 epics** per Winston's structure with E1 split explicitly and E8 single epic with sub-stories per Spirit cohort.

> **Architectural seam discipline carried by E0 (Murat's adoption, Winston's yield):** the kernel-API surface invariant (NFR-Test-2), empty-kernel invariant I9 (ADR-006), Loom-not-in-kernel grep (NFR-Test-9), KLOC budget alarm (`tokei`, ≤20 KLOC, alarm at 16), reproducible build gate, zero-`unsafe` capability-path gate, content-addressed corpus infrastructure (NFR-Test-1), coverage matrix CI gate (NFR-Meta-3), and ComplianceClaim schema adversarial review all live in E0 and run on every PR forever. E0 is a founding sprint with a v0.1 acceptance criterion that thereafter transitions to a maintenance discipline owned by whoever holds the repo.

> **KLOC budget tally (Winston's estimate):** ~18–27 KLOC for kernel trusted core across E1a + E1b + E2 + E3 + E4 + E5 + E6 + E7 (env+adapter) + E9 + E10. Alarm at 16 expected to fire during E6/E7 — budget tracked per-merge via E0's `tokei` gate. Reference Spirits (E8) and Spirit-side ecosystem code carry zero kernel KLOC. Upper bound bleeds past 20 KLOC; if alarm fires hard, scope-cut decisions happen at merge time, not ship time.
