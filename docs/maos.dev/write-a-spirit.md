# Write a Spirit — the 30-minute first-Spirit path

This is the **"write a Spirit"** door of [the three-door landing page](./index.md)
(NFR-Onb-3). It targets the NFR-Onb-1 floor: a newcomer with only these public
docs produces a working first Spirit in **about 30 minutes**.

## 1. Scaffold from the official template

Run this **verbatim**:

```sh
cargo generate --git https://github.com/lunarpulse/maos templates/spirit-rust --name my-spirit
```

This pulls the Story 2.3 Spirit template at
[`templates/spirit-rust/`](../../templates/spirit-rust/) — the same template the
in-repo [`examples/example-spirit`](../../examples/example-spirit/) is baked from.
You get a compiling Spirit with a lifecycle hook, a manifest, and a smoke test.

## 2. Implement one hook

Open `src/lib.rs` and edit `on_idle` (or another lifecycle hook). The
`#[spirit]` macro wires your `impl` block into the `SpiritVtable` the kernel
calls.

> **`maosctl ask` reaches your Spirit as the shell request shape.** A task the operator hands a running,
> installed Spirit with `maosctl ask` arrives exactly like a shell message: a `HumanAuthored` `TaskAssign` from
> `@maos-shell` with the success criteria `Answer the operator's shell message`. Answer it with one
> `TaskComplete` to the sender, as the template and `example-spirit-ts` already do; no code change is needed
> to be directable. The guest sees "shell message" wording for a door task.

## 3. Run it locally — no kernel required

The `LocalRunner` (and the fuller `SpiritTest` harness) fire your Spirit's hooks
against a mock `Ctx` with zero kernel dependency:

```sh
cargo test -p my-spirit
```

## 4. What "done" means

Your first Spirit is done when it **compiles against the published ABI**, its
smoke test passes, and it behaves correctly against the Butler-class regression
scenarios (the NFR-Onb-1 corpus). The 30-Minute First Spirit Validation Gate
scores exactly this.

## 5. Write a domain Spirit with the kit

A **domain Spirit** is a TypeScript WebAssembly component that answers one kind of request from code in its own
source (a component has no inference import, so it is deterministic). The authoring kit takes a coding agent, or
you, from nothing to a signed, installed Spirit that answers `maosctl ask`, using only the release binaries,
Node and `cargo-generate`:

- [`kit/spirit-author/AGENTS.md`](../../kit/spirit-author/AGENTS.md): the whole path as one instruction file
  (scaffold from `templates/spirit-ts`, build, `maos-spirit validate`, publish to a loopback dev registry,
  `maosctl spirit install`, `maos serve` with `[[host.spirits]] installed = "<name>"` under a delegated cgroup,
  `maosctl ask <name> "<text>" --wait`).
- [`kit/spirit-author/skills/maos-spirit-author/SKILL.md`](../../kit/spirit-author/skills/maos-spirit-author/SKILL.md):
  the same steps as an Agent Skill.
- [`examples/incident-triage-ts`](../../examples/incident-triage-ts/): a worked domain Spirit.

Only `templates/spirit-ts` works for this path: `templates/spirit-rust` builds an in-process first-party Spirit
that a registry refuses and `maosctl ask` cannot reach. The kit's evidence is agent runs, not a human trial; see
`RELEASE-HOLDS.md`, "Claim boundaries — Spirit authoring kit".

---

> **Status (v0.3):** functional path; WCAG-AA polish + canonical publication are
> deferred to **Story 9.5** (NFR-Onb-3).
