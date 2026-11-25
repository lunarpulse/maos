# Story 23-0 evidence (agent Dev230, host: local, branch e23-0-online-adapters-2026-11-24)

Env for every cargo command: `unset CI GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE; CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/mnt/build/cargo-maos-e230`.

## AC1(a) — feature builds, base (576bce70, before any edit) and after

Command: `cargo check --locked -p maos-bin --lib --bins --tests <shape>`.
Base results for `--no-default-features*` were taken on a pristine `git archive HEAD` copy
(the first attempt overlapped with the first edits and was discarded).

| Shape | Base | After |
|---|---|---|
| default | exit 0 | exit 0 |
| `--no-default-features --features network,wasm-host` | exit 0 | exit 0 |
| `--no-default-features --features network` | exit 0 | exit 0 |
| `--no-default-features` (`--lib --bins --tests`) | exit 101: test targets (epic_args_19_1, tenant_audit_phase_a_13_5g, bounded_postures_2b, ...) need `maos_bin::worker_spawn`, `orchestrator_root`, `delegation`, `rcgen`, `maos_cohort` (network-only) | exit 101, same class (erasure_million_20_11, shell_host_16_2, researcher_inference_18_3, epic_halt_resume_19_4, ...: `researcher_root` etc.) |
| `--no-default-features` (`--lib --bins`) | exit 0 | exit 0 |
| `--no-default-features --features air-gap` (`--lib --bins --tests`) | exit 101: same class (`rcgen`, `maos_a2a_tcp`, `orchestrator_root`, ...) | exit 101, same class |
| `--no-default-features --features air-gap` (`--lib --bins`) | exit 0 | exit 0 |
| `--no-default-features --features wasm-host` (`--lib --bins`; extra, not an AC shape) | exit 101 (`spawned_lifecycle` not found in `maos_bin`) | exit 101, same error |

The no-default-features `--tests` and wasm-host-without-network shapes are red at the base and are
recorded, not fixed here (story AC1(a)). CI builds `--no-default-features` only for `--test verb_table_15_3`.

## Proven reds

Each file: the plant, the command, and the failing test names. Every plant carried the token
`E230-PLANT`, was reverted, and `grep -rn E230-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask`
returned nothing afterwards.

| File | Plant | Tests that fail |
|---|---|---|
| `AC2-network-enables-umbrella.txt` | `"online-adapters"` added to the `network` list | `no_feature_but_default_reaches_the_umbrella` only |
| `AC2-default-lacks-umbrella.txt` | `online-adapters` removed from `default` | `the_umbrella_is_in_default_and_enables_network` and `no_feature_but_default_reaches_the_umbrella` (nothing reaches the umbrella) |
| `AC3-http.txt` | http refusal in `vet_url` disabled | `an_http_url_to_a_public_host_is_refused`, plus the three tests that carry one http row (`a_refusal_names_its_class_and_never_the_url`, `a_refused_url_never_reaches_the_inner_port`, `localhost_resolving_to_loopback_is_refused`) |
| `AC3-loopback.txt` | `Loopback` arm of `admit` returns Ok | `a_loopback_host_is_refused_unless_allowed`, plus tests with a loopback row (tls sink, names, refusal-codes, inner-port tables) |
| `AC3-link-local.txt` | `LinkLocal` arm returns Ok | `a_link_local_host_is_refused_even_with_private_allowed`, plus rows in tls sink, loopback-knob, refusal-codes, inner-port tables |
| `AC3-private.txt` | `Private` arm returns Ok | `a_private_host_is_refused_unless_private_is_allowed`, plus rows in tls sink, loopback-knob, mixed-answer, refusal-codes, inner-port tables |
| `AC4-first-answer-only.txt` | `vet_host` checks only the first answer | `a_name_whose_answers_include_a_private_address_is_refused` (and the tls sink mixed-answer row) |
| `AC5-redirect.txt` | `PolicedIo` passes a 3xx exchange through | `a_redirect_answer_is_refused_and_not_followed` only |
| `AC5-never-reaches-inner.txt` | `PolicedIo::vet` returns Ok early | `a_refused_url_never_reaches_the_inner_port` only |
| `D-23-0-03-classifier.txt` | CGNAT test narrowed in `classify_v4` | `classify_public_equals_the_former_egress_predicate_on_the_whole_vector_table`, `a_reserved_host_is_always_refused` |

Note on "its test and no other" (AC3): the class check is one shared function, so every other test that
carries a row of that class (names, TLS sink, knob and refusal-code tables, inner-port table) also fails.
The two class tests of the other classes (e.g. http and link-local under the private plant) keep passing.
Not run: no plant on the egress call sites (`proxy.rs`); egress has no test that pins its classes, the
domain equivalence test pins the classifier.

## AC7 — hosted legs pre-verified locally

| Leg | Local result |
|---|---|
| release size (`cargo build --locked --release -p maos-bin --bin maos`, `strip -o`) | base 38,507,088 B; after 38,524,752 B (+17,664 B); limit 39,069,022 B (headroom 544,270 B) |
| `cargo deny check` (0.19.4) | `advisories ok, bans ok, licenses ok, sources ok`; also green inside prepush |
| `Cargo.lock` | root lock gains only `"url"` in the maos-domain entry; `url` 2.5.9 was already locked |
| standalone lockfiles | maos-domain gained the url edge, so `--locked` fixture builds failed until these were refreshed (`cargo metadata --offline`): `crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.lock` (+1, required by `build-test-fixtures`), `crates/maos-kernel-core/fuzz/Cargo.lock` (+1), `crates/maos-domain/fuzz/Cargo.lock` and `crates/maos-manifest/fuzz/Cargo.lock` (url dependency tree added; the hosted fuzz-build runs without `--locked`). The four `spikes/story-17-3a-wasm-recall/*` locks also want a refresh but their diffs carry unrelated drift and no CI job builds them; left as they were |
| touched tests with a random 0600 hex `MAOS_AUDIT_KEY` file | online_adapters_23_0 (default, network+wasm-host), verb_table_15_3, outbound_policy_23_0, maos-egress: all pass |
| prepush (`xtask prepush --json`, scoped) | 52 records: 48 pass, 1 skip (migration corpus absent), 3 red, all host limits: `docs:build` (docusaurus not installed, exit 127), `docs:routes` (stale build refused), `scoped-nextest` (failing tests visible in the output: `spawned_scope_17_3c` x14 and `wasm_skill_authoring_20_8 signed::installed_signed_manifest_journals_verified_publisher_and_rejects_tamper`; the detail is truncated, so the full failing set was not listed). The scoped run does not include `binary-size` (`--full` only); the size above was measured by hand with the same commands |
| xtask gates | check-composition-root-completeness (11 adapters, 20 constructions), check-env-contract (159), check-dependency-closure, check-export-control, check-kernel-baseline (28,324 = pinned), kloc-check (alarm true, passed), check-pub-field-constructors: all passed |
| kloc | maos-domain 10,890 -> 11,156 (+266); maos-egress 1,776 -> 1,752 (-24); maos-bin 44,851 (0); kernel-core 22,143 (0); aggregate 215,951 -> 216,210 |

## Review fixes (agent Fix230)

| Finding | Result |
|---|---|
| P1 DNS rebinding | Not closed; recorded as a limit. `IoSubsystemPort` takes a URL string and the kernel adapter owns the ureq agent, so binding the connection to the vetted addresses needs a port-trait change (or kernel resolver state), outside this story. For host names `PolicedIo` does not bind the connection to the vetted addresses; consumer stories must not count it as SSRF protection for names until the port can bind. Literal IPs are unaffected. Recorded in the `outbound.rs` module doc and RELEASE-HOLDS 23-0.a. Worker egress connects to the vetted addresses and has no gap. |
| P2 loopback knob | `allow_loopback_http()` alone admits loopback only for a literal loopback host; a name resolving to 127.0.0.1 is refused over http and https. `allow_private()` keeps loopback by any route. Plant: `review-P2-loopback-by-name.txt`. |
| P3a shape test | The self-comparing test is gone. `the_profile_without_online_adapters_has_network_and_wasm_host` exists only in the `network,wasm-host`-without-umbrella build; CI runs it by name and requires `1 passed`. Plant: `review-P3a-shape-test.txt`. |
| P3b redirect | `http_post` now goes through the exchange, so a 3xx is refused for `http_post` and `http_post_exchange`. `http_get` returns bytes only: its redirect refusal stays the inner adapter's (the kernel adapter never follows and fails non-2xx); the doc says so. Plant: `review-P3b-P3c-redirect-and-debug.txt`. |
| P3c Debug | `VettedUrl` Debug shows scheme, host, port and addresses only. Known limit, not changed (kernel code): the kernel `Transport` error text carries the raw URL (`io/mod.rs`, `"ureq GET {url}: ..."`), so a caller must not log that error for a URL with userinfo or a token in the query. Plant in the same file as P3b. |
| P3d egress sites | `a_granted_private_or_link_local_literal_is_refused` pins the literal site (`10.0.0.1`, `169.254.169.254`, `192.168.1.1`); `a_granted_name_resolving_to_private_or_link_local_is_refused` pins the resolved-answer site with names `0xa.1` and `0xa9.0xfe.0xa9fe` (glibc inet_aton hex forms, answered locally with 10.0.0.1 and 169.254.169.254, no DNS traffic; a resolver without inet_aton forms would turn this test vacuous). Plants: `review-P3d-egress-literal-site.txt`, `review-P3d-egress-resolved-site.txt`. All plants carried `E230-PLANT` and were removed (grep over crates, .github, docs, RELEASE-HOLDS.md, Cargo.lock, xtask: nothing). |
| P3e known disagreements with the registries | Not changed. (1) The documentation block rules match the second octet, so whole /16s are refused (192.0.0.0/16, 192.2.0.0/16, 198.51.0.0/16, 203.0.0.0/16) instead of the /24 registry blocks. (2) The IPv6 allow rule (2000::/3) admits 3fff::/20 (RFC 9637 documentation) as Public. Both are inherited from Worker egress's former `public_ip`. RELEASE-HOLDS 23-0.b carries them. |
