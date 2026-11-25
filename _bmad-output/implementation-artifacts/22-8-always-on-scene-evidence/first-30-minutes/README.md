# First 30 minutes — operator lane (Story 22-8, AC7, D-22-8-09 A)

**Status: NOT RUN.** This run needs the Epic 22 minor release tagged and its image published, so it cannot
happen in the implementation change. It is an operator-lane step: it never blocks the exit
(`journey_always_on`) and no gate claims it. `RELEASE-HOLDS.md` row 6 stays NOT MEASURED until a record
below states the result.

## What to run

1. A blank-context agent: no repository, only the public docs (`docs/maos.dev/`).
2. Pull the multi-arch image from GHCR and run it with the compose file. Install nothing else.
3. In its own Host file, change the cast to Mira and Nash with a `[[host.a2a_peers]]` pair (see
   `docs/maos.dev/run-maos.md`, "Let cast members hand each other evidence") and set
   `inference_mode = "live"` with `MAOS_ANTHROPIC_API_KEY`.
4. Complete one `maosctl ask mira "<incident text>"` task under the real model.

**Pass:** an answer the agent judges useful, wall clock at most 30 minutes.

## What to record here (one file per run)

- `transcript.md` — the agent's commands and outputs, verbatim.
- The wall clock (start and end), the model id (`covered_model_id` of the admitted manifest and the
  provider's reported model), the image digest, and `PASS` or `FAIL` with the reason.
- The result is copied to `RELEASE-HOLDS.md` row 6, in either direction.
