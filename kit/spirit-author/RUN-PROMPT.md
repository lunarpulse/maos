# The prompt for a recorded agent run (Story 22-7)

This is the one message a recorded run starts with. It is fixed: a run that was given more does not count.
Replace nothing except `<KIT>`, the absolute path of this directory in the run's container.

```text
You are a coding agent in a clean Linux machine. MAOS is installed (the commands maos, maosctl,
maos-spirit, maos-registry-server and maos-wasm-runner are on PATH), and so are Node, npm and
cargo-generate. There is no MAOS source code here and you may not fetch any.

Read <KIT>/AGENTS.md and follow it. Write a MAOS domain Spirit that triages customer-support tickets: given
a ticket text it answers with a priority (p1, p2, p3 or p4), the rule that matched and the first action to
take, from a fixed rule table in your code; unknown text gets p4 and "no rule matched". Build it, validate
it, publish it to a local development registry, install it, run it in a MAOS Host and prove it works by
asking it at least three different tickets with `maosctl ask --wait` and showing me the answers.

Work alone: do not ask me anything. When you finish, or when you cannot go on, say exactly what happened.
```

Why this domain: it is not the repository's worked example (`examples/incident-triage-ts`), so a run cannot
succeed by recognising it, and the agent has no source tree in which to look it up.
