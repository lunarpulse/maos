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

---

> **Status (v0.3):** functional path; WCAG-AA polish + canonical publication are
> deferred to **Story 9.5** (NFR-Onb-3).
