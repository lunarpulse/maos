# Spirits + journeys scout (HEAD 4657cace)
ABI offers 14 hooks (lifecycle.rs:233+); EVERY reference Spirit overrides exactly one: on_idle.
| Spirit | LOC | LLM class | launch | model pin |
| hello-spirit | 382 | LIVE-LLM (lib.rs:79 -> InferencePortAdapter main.rs:3136 -> AnthropicProvider) iff MAOS_ANTHROPIC_API_KEY, else canned "Inference is unconfigured" | maos shell | haiku-4-5 live |
| butler | 1219 | LOGIC (no InferencePort import; rule-based Assessment; scalar->kernel halt) | maos run spirits/butler/manifest.toml --live --once | claude-3-haiku RETIRED, never called |
| researcher | 1596 | LOGIC default / LIVE opt-in (--live, lib.rs:915) with silent fallback to deterministic (lib.rs:925); cassette replay | maos run ... --live --once | claude-3-5-sonnet RETIRED |
| observer | 1103 | LOGIC; NOT LAUNCHABLE (absent from LoadedSpiritKind main.rs:447-454; no dependents) | none | none |
| orchestrator | 683 | LOGIC; on_idle = lock and return (lib.rs:149-154); real work driven from maos-bin | j1 topology | none |
| worker | 129 | CANNED fixture echo (CANNED_OUTPUT_LINES lib.rs:43-47); manifest-codex/claude swap in real agent CLI (not Inference Port) | j1 topologies; live only xtask demo-j1 --live-codex (never CI) | n/a |
| architect 235 / reviewer 269 | LOGIC seeded | j1 topology | none |
| mira 617 / nash 319 | LOGIC deterministic (covered_model_id maos.mira.deterministic-diagnostic-v1) | j4 topology | none |
| digest 744 | LOGIC; daemon derives TeamDigest from j3-digest-inputs.json fixture | maos run spirits/digest --once | none |
JOURNEYS:
- J0: LIVE-E2E possible via `maos init && MAOS_ANTHROPIC_API_KEY=… maos shell` -> @hello-spirit; test asserts banner strings only; run-maos.md never mentions API key. Only @hello-spirit dispatchable (butler pick-only).
- J-Butler: LIVE-HERMETIC (real daemon, MockMcp fixtures, no LLM); zero "7PM" refs; JB8 posture-shift #[ignore] RED (journey_butler.rs:285).
- J-Researcher: LIVE-HERMETIC (cassette or opt-in live); no 90-min budget.
- J1: LIVE-HERMETIC, worker canned; xtask demo-j1; live paid via --live-codex never CI; demo renders unlanded beats as ABSENT (demo_j1.rs:23,356); no "halt on AC ambiguity"/overnight-digest beat.
- J3: FIXTURE-REPLAY through real daemon (fixtures/j3/day-30-raw.json); N=8 mesh digest-read test #[ignore] (t_12_4a_digest_read.rs:298).
- J4: LIVE-HERMETIC load+drain only; rupture oracle #[ignore]; 50-scenario corpus (300 lines) consumed only by maos-eval kappa calc, never fed to Mira.
- Reza: LIVE-HERMETIC multi-process + live Postgres, #[ignore]d (needs MAOS_TEST_POSTGRES_TEAM_A/B/C + psql); no LLM; check-reza-production-path GATE-ONLY AdvisorySubstrate (absence -> banner).
- J6 Diego: GATE-ONLY -> ABSENT (parses trial-results.toml; only schema exists -> "advisory pass"); no runtime for cargo generate path.
CLI: `maos` hand-rolled argv: init, shell, audit query, traceback, run <manifest|topology> [--live] [--once]; env arms MAOS_ONE_SHOT=... `maosctl` clap: install,start,stop,unload,uninstall,run,audit,posture,halt,orchestrator,pause,resume,revoke-token,spirit,revocations,import,skills,forget,legal-hold,governance,backup,migrate,cohort.
GAPS: 1 LLM-backed Spirit and it's an echo; 9/10 ABI Spirits deterministic; 1 of 14 hooks used; closers judge files not runs; no time/scale substrate for scenes; Observer unlaunchable; onboarding claims exceed binary.
