# 14. Open Questions

These are the genuine "I'm not sure" items. They are **not** blockers for v0.1 — they are signals for where the design will need to learn.

1. **Spirit hot-swap semantics for in-flight LLM streams.** Mid-stream, the predecessor's `on_swap_out` fires. What happens to the partial response? Drop it (waste a half-completion of a Sonnet-tier model, but keep semantics simple)? Hand it to the successor as a `partial_response` input (clean but every successor must know what to do)? Stash it in `private` memory for later retrieval (easy but never actually used)? **Lean: drop it, log it, charge the user, keep semantics simple.** Revisit if cost data says the dropped completions are material.

2. **Approval prompt fatigue.** Every survey project has it. The substrate (`prompt_with_diff`, persistent allow, posture presets) exists. What is missing is **the heuristic** for "this looks like the same kind of thing the user has approved before, batch it." Possible answers: per-(Spirit, capability, target-fingerprint) cached decisions; an LLM-mediated batcher; plain-English summary of "the next 10 things the agent wants to do" as a single approval. **Probably need real usage data to pick.**

3. **A2A trust establishment under churn.** TOFU + mTLS is the v1.0 plan for the bilateral case. For longer-running deployments where one of the two Hosts is replaced (laptop swap, prod-edge node migration), pin re-establishment is operator workflow. Probably need a documented playbook before v1.5 ships.

4. **Spirit class portability across kernels — committed to a triple.** Compatibility is `(kernel_version, abi_version, manifest_schema_version)` — a triple, not a pair. `abi_version` governs the `Spirit`/`KernelHandle` vtable + capability ID space (SemVer; major break = vtable layout or capability semantics change). `manifest_schema_version` governs the TOML surface independently. `kernel_version` is product-facing and includes both as a compatibility set. **Rule:** Spirit declares `abi`; kernel adapts down via `Compat` shim layer; N-1 supported, N-2 hard refusal with typed `EAbiTooOld`. **Deprecation:** 2 minor releases of warning, 1 major to remove. The live version-compatibility matrix lives in `STABILITY.md`. v0.5→v1.0 transition is breaking by design; documented in CHANGELOG with migration path.

5. **Loom-lite contention on the diagnostic-architect bilateral pair.** When Mira is fetching the latest detection patterns and Nash is publishing fix templates concurrently, Loom-lite is the hot service. Single-instance Postgres+pgvector at the bilateral scale is well-understood (well within Postgres's normal load envelope), but pattern-search latency under concurrent reads-and-writes is worth measuring. **The v1.5 deployment will reveal the right index strategy.**

6. **Researcher's "novel hypothesis" mode operationally.** A Researcher running on a corporate Host with constraints needs to know what is allowed for hypothesis generation (data-residency for the source material; provider selection for the cogitation; collective tier writes for the conjectures). Probably resolved by the operator's deployment policy. **Mark as PDP-integration test if a Spirit class needs PDP.**

7. **Mobile push UX for halt resolution.** Lunarpulse approves from his phone in the founder loop; Elena approves from her phone in the diagnostic-architect pair. For v1.0 the substrate ships HTTP push; native mobile clients are a v1.5 deliverable. Editor banners can lean on ACP's existing diff-display for in-editor resolution.

8. **Prompt-injection defense at the tool-output boundary.** The kernel hosts a generic post-tool-output filter; the **content** of the filter (what is a leak? what is instruction injection?) is data, not code. Ship a default rule pack; let Spirits add to it. **Will not be perfect; aim for "raises the bar."**

9. **What is the smallest viable Loom-lite?** Postgres + pgvector + a single MCP server is enough for the diagnostic-architect bilateral pair. The shape of the Loom-lite data model needs to be sound from day one even if v1.5 is the first deployment. **One serious schema review before v1.5 ships.**

10. **Cohort interop signal.** A first cohort project (openclaw / ironclaw / hermes / paperclip / rustain / codex) integrating MAOS as their substrate or interoperating cleanly via ACP/MCP/A2A is a v1.0 success criterion. **Cohort interop is a sociological signal, not an engineering metric.** It is achievable but not gateable.
