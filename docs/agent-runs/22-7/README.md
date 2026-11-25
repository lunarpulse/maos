<!-- agent-runs-22-7: status=pending -->

# Agent runs of the Spirit authoring kit (Story 22-7)

**Status: pending.** The three runs below are an operator-lane step (D-22-7-08 A: three fresh agent sessions in
clean containers). None has been run, so none is committed, and the release states no `k`. When they are
committed, change the first line to `<!-- agent-runs-22-7: status=recorded k=<n> -->`, replace the K1 status note
in `RELEASE-HOLDS.md` with "`<n>` of 3 agent runs succeeded", and
`cargo test -p maos-spirit-cli --test agent_runs_22_7` holds the records, this file and K1 to one count.

**No human author is measured here.** Every run is agent-authored (`author_kind = "agent"`) and was not repeated
by a human. These records are not the NFR-Test-8 or NFR-Onb-1 cohort (see `RELEASE-HOLDS.md`, K2).

## What a run is

- **The prompt** is fixed: [`kit/spirit-author/RUN-PROMPT.md`](../../../kit/spirit-author/RUN-PROMPT.md). A run that
  was given more does not count.
- **The environment** is a clean container or VM that holds only the branch's five release binaries
  (`maos`, `maosctl`, `maos-registry-server`, `maos-spirit`, `maos-wasm-runner`), Node 24, `cargo-generate`, and
  `kit/spirit-author/` at the final kit commit. No MAOS source checkout.
- **The agents**: three fresh sessions across at least two harness families (for example Claude Code, Codex CLI,
  pi, hermes).
- **Counted**: all three runs are recorded whatever their outcome, and only runs against the final kit commit
  count. AC1 holds when at least one run ends with a correct domain answer from `maosctl ask --wait` and no human
  message after the start prompt. The release states "k of 3 agent runs succeeded" verbatim.

## How to run and record

`scripts/agent_run_22_7.sh run` starts a container and captures the session; `scripts/agent_run_22_7.sh record`
redacts the capture, writes `run-<n>/transcript.txt` and `run-<n>/record.toml`. Read `--help` of each.

## `record.toml`

```toml
author_kind = "agent"                 # the label; the test refuses anything else
harness = "claude-code"               # a harness family (lower-case); the three runs need at least two
model = "..."                         # the model the harness ran
kit_commit = "<40-hex commit>"        # the kit commit the container held
start = "2026-11-24T10:00:00Z"
end = "2026-11-24T10:41:00Z"
outcome = "succeeded"                 # or "failed"; failures are kept
ask_result = '''{"task_id":"...","state":"completed","result":"..."}'''   # the final `maosctl ask --wait` JSON; "" on a failed run
transcript = "transcript.txt"         # a file in this directory, at most 2 MiB
transcript_sha256 = "<sha256 of that file>"
public_digests = []                   # optional: 64-hex digests the transcript may contain (a printed sha256sum)

[binaries_sha256]                     # the five binaries the container held
maos = "..."
maosctl = "..."
maos-registry-server = "..."
maos-spirit = "..."
maos-wasm-runner = "..."
```

The transcript is redacted before it is committed: values of `*_API_KEY`, `*_TOKEN`, `*_SECRET` and
`*_PASSWORD` variables, the contents of every `*.key` file the run created, and `sk-`, `ghp_`, `github_pat_`
shaped tokens become `[REDACTED]`. The test refuses a transcript that still carries one of those shapes, an
`-----BEGIN` block, or a 64-hex run that is neither a binary digest nor named in `public_digests`.
