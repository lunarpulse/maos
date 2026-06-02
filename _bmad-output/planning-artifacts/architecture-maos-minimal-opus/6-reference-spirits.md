# 6. Reference Spirits

Five reference Spirit classes ship with the substrate, plus three skill-package overlays for the founder loop. The reference Spirits prove the substrate; third-party authors ship their own classes via the Spirit registry. **Reference Spirits are sibling workspace crates under `spirits/`, NOT compiled into the `maos-bin` kernel binary** — they compile against the published Spirit ABI exactly like `examples/example-spirit`, keeping kernel KLOC at zero (epic-8 mandate; ADR-002 / Decision A, ratified §12 Story-8.1 block). Butler (§6.1) is the first, landed at `spirits/butler/` (Story 8.1, workspace count → 31).

## 6.1 Butler — Proactive Personal Agent

**Purpose:** Anticipatory single-Spirit assistant. Watches the user's calendar, communications, and active work; surfaces useful pre-emptive notifications without acting unsupervised.

**Cognitive shape (design report ¶154–¶168):** Active-Inference-style belief update over the user's current goal-state, then candidate-action ranking by expected free energy. The Butler maintains its own POMDP over user-goal-states; the kernel does not compute or interpret this — it only persists the Butler's tagged scalars and fires the `[epistemic_policy]` rules the Butler declared.

**Memory scope:** Per-user private memory. Episodic tier for the `notification_acceptance_log` (feeds future POMDP refinement across sessions). Optional shared scope on a single Host (the Butler can subscribe to other Spirits' "what are you working on" telemetry).

**Capabilities:** Calendar read (Google Calendar / Outlook via MCP), Slack read + draft (write gated by approval), Linear write (gated), Figma read, browser read. **At v0.3 these are declared as `Scope::McpCall` capability scopes and validated by fixture-replay of scenario inputs — live external drivers are application-layer and land at v0.5+/Epic 9** (ADR-005 / Decision B, ratified §12 Story-8.1 block). The kernel-mediated capability path is exercised and audited at v0.3; only the provider wire behavior is deferred.

**Posture:** `assistive` by default. Notifies, but does not act unsupervised. The user can shift to `cautious` (every notification prompts) or `autonomous-with-halt` (Butler may schedule small reversible actions, halts on uncertainty).

**Lifecycle hook anchor:** `on_idle`. The Butler runs its anticipatory reasoning loop whenever the kernel calls `on_idle` (no pending IAC frames + user activity stream shows >12 minutes since last meaningful interaction).

**Output shape:** Notifications carry structured `{pattern, confidence, evidence, options[]}` payload — the kernel rejects emit without these fields.

**Epistemic policy:** Butler's `[epistemic_policy]` halts on `self.belief_variance > 0.7` (the Butler computes variance using its own preferred uncertainty proxy; the kernel does universal-arithmetic comparison only). Halts on `claim.user_preference_drift` with `confidence_below = 0.6`. Verbalize-only on routine pattern-detection.

**Eval metrics:** notification precision (fraction acted on; floor ≥0.85 at v0.3), notification recall (fraction of relevant moments caught; floor ≥0.7), user-correction rate, time-to-action savings.

## 6.2 Researcher — Insightful Research Assistant

**Purpose:** Exploratory single-Spirit. Conducts rigorous literature surveys; proactively generates novel hypotheses and proposes future research directions.

**Cognitive shape (design report ¶87–¶89, ¶191):** Survey-mode posture (exploratory, reactive, divergent). Hypothesize-mode posture declared in the manifest's posture-set but ships fully at v1.0 (ILP + LLM hybrid for novel-hypothesis generation; ILP's structured rule discovery joined with LLM's pattern completion, output submitted to a Critic Spirit for refinement).

**Memory scope:** Private (the researcher's working knowledge graph). Episodic for cross-session bibliography persistence. Loom-lite collective tier opt-in for cross-team pattern reuse.

**Capabilities:** Web search, arXiv search, GitHub search, citation-graph traversal, MCP tool invocation broadly. Manifest `[capabilities.parallelism] = 8` — up to 8 concurrent tool dispatches.

**Posture:** `survey-mode` (exploratory, reactive, divergent) for routine surveys. `hypothesize-mode` for generative work (ILP+LLM hybrid).

**Output shape:** "findings + Open Questions + Confidence Map + Bibliography" — the kernel rejects emit without all four.

**Adaptive-chunk-ratio summarization** (design report ¶362; hermes-informed per §9.5): each paper digest under 4K tokens; citation-graph traversal identifies tight clusters of related work; the Spirit reads abstracts for many papers, full intros for some, full methods for the load-bearing few.

**Distillation pattern.** Researcher is the first opt-in production user of the §9.5 distillation pattern in a single-Spirit context. The five-metric gate applies (digest-recall ≥0.90, faithfulness ≥0.98, hedge-preservation ≥0.95, traceability=100%, secret-leakage=0%).

**Epistemic policy:** halts on `claim.methodology_strength` when two papers report contradictory findings both with strong methodology by the Spirit's scoring rubric. Halts on `claim.load_bearing` with `confidence_below = 0.7`. Verbalize-only on speculation tagged `claim.exploratory`.

**Eval metrics:** synthesis accuracy, citation correctness (≥95% reachable URLs), novelty of hypotheses (LLM-as-judge with rubric, in hypothesize-mode), Open Questions quality (rubric-judged).

## 6.3 Diagnostic Engineer (Mira-class)

**Purpose:** Production-edge diagnostic Spirit. Monitors live systems, detects anomalies, formulates and logs hypotheses, designs targeted software remediations, ships them, incorporates runtime feedback into iterative fixes.

**Memory scope:** Per-Spirit private (Mira's working diagnostic context). Loom-lite collective tier read+write — Mira reads ADR-pattern library to recognize known incident shapes; Mira writes new patterns when an incident produces a reusable diagnostic recipe.

**Capabilities:** Read-only on production runtime knobs (telemetry queries, log ingestion, thread-dump retrieval). Write on revert/scale/flag-toggle (gated by approval). Bash-exec whitelist for containment actions (`kubectl rollout undo`, `flag set`, etc.). Cross-environment telemetry queries to peer Hosts via bilateral A2A (sends to Architect-class Nash Spirit).

**Posture:** `sre-diagnostician` — `readonly_*` = silent allow; `mutating` = notify and log; `exec_capable` = prompt with diff (revert, scale, flag-toggle); `control_plane` = prompt.

**Asymmetric capability gates.** This is enforced by the Capability Registry, not by Spirit-side discipline. Read-only on source code; read-write on production runtime knobs. The asymmetry is what makes Mira safe to run on a production-edge node.

**Epistemic policy at production fidelity:**
- `diagnosis.root_cause` halts at `confidence_below = 0.6` or evidence conflict — when telemetry is consistent with multiple incompatible root-cause hypotheses, Mira does not pick the most plausible; she invokes `epistemic.halt(gap_kind: "evidence_conflict", evidence_so_far, query_strategies)`.
- `diagnosis.observation` is `verbalize_only` — observations of metrics or thread dumps don't need confidence gating; the data is what it is.
- `containment.action` halts at `confidence_below = 0.5` — Mira would rather block on an uncertain containment plan than execute one and worsen the incident.

**Bilateral A2A escalation.** When Mira's diagnosis confidence is high enough (above the halt threshold) but the fix requires source-code changes, Mira escalates to Nash via cross-host A2A. The frame carries typed intent `diagnosis-handoff:read-only-evidence` — Nash's consent policy permits this from the prod-edge peer; Nash's policy explicitly excludes `remote-write-request` and `code-mutation-directive` from the same source. The bilateral consent envelope ensures Mira cannot trigger Nash's source-modification capabilities directly; Nash decides whether to act on Mira's diagnosis.

*Corpus methodology for Mira (and other safety-critical Spirits) is in §6.6.*

## 6.4 Senior Architect (Nash-class)

**Purpose:** Dev-environment principal architect. Produces production-grade code conforming to organizational standards; owns testing, deployment, CI/CD; closes the loop with telemetry from the Diagnostic Engineer.

**Memory scope:** Per-repo private memory (the user's coding style guide, prior decisions, ADRs). Shared on-Host for cross-Spirit coordination during the founder loop. Collective (Loom-lite) read+write for ADR-pattern library, fix templates, regression-test references.

**Capabilities:** Full RW source repo. CI/CD orchestration (GitHub Actions / equivalent). `provider.complete` for code generation. `git.commit` (gated). `bash.exec` whitelist for build and test commands. Cross-environment telemetry queries to peer Hosts (bilateral A2A from Mira).

**Posture:** `principal-architect` — silent on source mutations within a configured workspace; prompts on every deploy (the human gate is load-bearing); uses the granular approval mode for fine-grained "yes to this PR but not the next one" control.

**Failure mode to design against:** Nash autonomously deploying something subtly wrong because the test suite has gaps. Mitigation: deploys always go through the Sentinel-validated canary protocol when an Observer Spirit is colocated.

**Bilateral A2A receive-side.** Nash receives diagnosis-handoff frames from Mira via cross-host A2A. The receive-side consent policy admits `diagnosis-handoff:read-only-evidence` only; explicitly excludes `remote-write-request`. Nash decides whether to confirm the diagnosis (read source, walk patterns), propose a fix (PR draft + ADR), and close the loop back to Mira.

## 6.5 Observer

**Purpose:** Read-only perceptual layer. Subscribes broadly to the Telemetry Stream; renders the local "what's happening" view; can be a passive Spirit or a notification source.

**Memory scope:** Per-Spirit private (rolling window of observations). Optional shared on-Host for surfacing aggregate views.

**Capabilities:** Telemetry Stream broadcast subscription (broad). `scalar.tap` subscription to see pre-halt scalar drift across peer Spirits. No write capabilities by default; the Observer cannot send IAC frames except `notification.surface` (kernel-rendered to the user).

**Posture:** `passive-observer` — silent allow on all reads; no exec; no mutating; no control-plane.

**Use case in the founder loop.** The Observer subscribes to the Orchestrator's `task.assign` and Worker `task.complete` frames (read-only); renders a live "what are the agents doing" view in the operator's TUI or editor banner.

**Use case in the diagnostic-architect bilateral pair.** The Observer colocated with Nash watches `scalar.tap` from Mira; surfaces pre-halt scalar drift before Mira's halt actually fires, so Nash can pre-stage source-walks while Mira is still gathering evidence.

## 6.6 Safety-critical Spirit corpus methodology

Applies to §6.3 Mira and §6.4 Nash; future safety-critical Spirit classes inherit this methodology when specified. Confidence-threshold halts (Mira's `diagnosis.root_cause` <0.6, `containment.action` <0.5, and equivalent thresholds for other safety-critical Spirits) require empirical calibration against a labeled corpus. The corpus is the substrate for §8.0 floor 6 (precision ≥0.85, recall ≥0.95).

**Corpus size.** N ≥ 150 labeled scenarios per safety-critical Spirit at v1.0. v0.5 lower bound: N ≥ 50 (calibration phase, no enforcement). v0.7: N ≥ 100 (precision floor enforced, recall floor advisory). v1.0: N ≥ 150 (both floors enforced).

**Stratification (mandatory, all five classes represented):**
- **40% true-halt-required** — scenarios where halt is the correct action (root cause genuinely ambiguous, containment action genuinely risky).
- **20% borderline** — scenarios near the threshold where reasonable labelers might disagree; tests calibration sensitivity.
- **20% false-positive-bait** — scenarios that *appear* to warrant halt but where confident proceed is correct (tests over-cautious failure mode).
- **10% adversarial-evasion** — scenarios constructed to slip past the halt logic (tests under-cautious failure mode).
- **10% benign-suspicious** — scenarios with surface markers of risk but benign ground truth (tests false-positive baseline).

**Labeling protocol.** Two independent labelers per scenario, blind to each other's judgments. Inter-rater agreement Cohen's κ ≥ 0.7 required for corpus admission; below 0.7, scenario is rewritten or discarded. Disagreements resolved by third labeler; if third labeler also disagrees, scenario is discarded (do not force consensus on genuinely ambiguous cases — they are precisely what the corpus needs to discriminate).

**Refresh cadence.** 20% of corpus rotated quarterly. Rotation is mandatory, not opportunistic — it prevents corpus overfit and surfaces drift in operational distribution. Retired scenarios retained in versioned archive for regression checks.

**Refresh IAA gate (no grandfathering).** Each refresh requires Cohen's κ ≥ 0.75 computed over a sample of ≥ 20 scenarios drawn (stratified by scenario class above) from the refresh corpus, with ≥ 2 independent annotators labeling each sampled scenario. Disagreements are adjudicated per the labeling protocol before κ is computed. Sample is re-drawn each refresh; scenarios are not reused across consecutive refreshes. Quarterly rotation report MUST publish per-scenario κ values; any scenario with κ < 0.7 is rejected and the slot is refilled before the rotation closes. **No grandfathering** — scenarios admitted under prior policy versions are re-IAA'd at the next refresh cycle they appear in.

**Reporting.** Each Spirit's halt-precision/recall reported per-stratum AND overall. Aggregate floor (≥0.85 / ≥0.95) is necessary but not sufficient — any single stratum below 0.7 precision or 0.85 recall triggers calibration review even if aggregate passes.

The asymmetric floor (precision ≥0.85, recall ≥0.95) encodes the cost model: missed halt is unrecoverable (a safety event escapes), over-halting is recoverable (operator can resume). Recall floor is therefore higher than precision floor by ~10× cost asymmetry.

## 6.7 Skill-package overlays for the Founder Loop

Three skill packs — **Orchestrator** (`orchestrator-bmad`), **Developer-Worker**, **Reviewer-Worker** — load into agent CLI processes (Claude Code, opencode, gemini-cli, kimi-cli) via the `maos-bridge` skill. These are not first-class Rust Spirit crates; they are skill-package overlays on the kernel-builtin `CliWrapperSpirit` class.

**CliWrapperSpirit specification.** Configured with: CLI binary path; skill bundle (`maos-bridge` + persona skills); `output_shape_version: "<semver>"` (kernel asserts on startup; refuses to start with typed `EOutputShapeAdapterMismatch` if observed != declared); posture declaration (stdio shape, control-channel mechanism, shutdown signal); capability scope mapping; output-shape adapter implementation (registered as `cli-wrapper-template:<cli-name>:<shape-version>` in the Spirit registry); crash semantics (kernel observes EOF on stdio + non-zero exit → `SpiritDied` event journaled; recovery policy declared in wrapper config: `respawn-with-context` / `respawn-fresh` / `escalate`).

**Fail-loud rule:** wrappers cannot fall back to "best-effort parsing" on shape mismatch. Audit drift is the failure mode the substrate cannot tolerate.

**Founder loop usage.** Lunarpulse spawns the Orchestrator (a Claude Code process loaded with `orchestrator-bmad` and `maos-bridge` skills, posture `autonomous-with-halt`, halt policy preferring recall over precision). The Orchestrator dispatches `task.assign` IAC frames to Developer-Worker Spirits (local + remote on a second laptop, A2A loopback within the same logical "founder topology"). Each Worker is an agent CLI process with `developer` + `maos-bridge` skills. Reviewer-Worker handles code review per `bmad-code-review`. Distillation pattern operational: raw Worker output → Transparency Log → Spirit-side LLM distillation → digest in working memory + episodic.

## 6.8 Extensibility — defining a new Spirit class

A new Spirit class is a new manifest plus a Spirit binary that conforms to the Spirit ABI. The kernel does not need modification. The author writes the manifest, implements the lifecycle hooks (handling whichever subset they care about), declares the capability surface, declares the epistemic policy, declares the output shape, and signs the binary with their Ed25519 key. Diego's `code-reviewer-pro` is the canonical example.

Spirits the substrate has not yet imagined — Negotiator, Tutor, Wet-Lab Coordinator — slot in by declaring their own capability surfaces, epistemic policies, and output shapes. The kernel grows slowly so the ecosystem can grow fast.
