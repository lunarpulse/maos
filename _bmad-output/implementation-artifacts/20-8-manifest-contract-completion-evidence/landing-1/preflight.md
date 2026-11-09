# 20-8 landing 1 preflight

## Resumption at `a13140b8`

The operator-provided exact pins are aggregate 197,451, bin 35,586, registry
6,151, domain 10,144, iac 8,625 and xtask 44,696. The L1 rebased ceilings are
aggregate 199,078, bin 36,160, registry 6,151, domain 10,174, iac 8,865 and
xtask 44,749; unchanged other ceilings appear below. The old measurement and
STOP remain historical evidence, not current pins.

Operator ask `ac13i2` ruled A: keep I2 fail-stop, append a declaration before
issuing a pid, and reject validation/serialization errors before pid issuance.
The orchestrator clarified that the implication is one-way: a subsequent load
or admission refusal may leave a declaration row, separately accompanied by
the refusal. The declaration does not prove admission. This clarification must
be disclosed to the operator. No kernel or audit-writer changes are authorized.

The initial declaration witness failed (exit 101) because the transparency
log redacted compact 64-digit hexadecimal values. Operator `enc208` A ruled
colon-delimited hex with exact-byte decoding. After that correction,
`cargo test --locked -p maos-bin --test manifest_declarations_20_8` with CI
environment variables unset passed both tests (exit 0), including recovery
of the digest and publisher bytes from the stored, redacted row.
Removing the emitter's TL insertion then caused the row witness to fail
(exit 101 at `manifest_declarations_20_8.rs:65`); the source was restored by
copy-back, `cmp` and `touch` (exit 0). This direct-writer test is not the
required pid-ordering or installed-package command witness.
The real admission witness `manifest_declaration_precedes_pid_and_refusal_is_distinct`
in `hook_pump_17_4.rs` passed with `env -u CI -u GITHUB_ACTIONS cargo test
--locked -p maos-bin --test hook_pump_17_4
manifest_declaration_precedes_pid_and_refusal_is_distinct` (exit 0): the
declaration exists at the loaded callback, and a later duplicate refusal leaves
another declaration plus a separate rejection. Source ordering places the
writer before `load_obj`; the callback alone observes it after pid issuance.

AC2 `schema_six_degrades_all_fr8_sections` passed (exit 0) using
`env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-manifest --test
manifest_n_minus_1_compat schema_six_degrades_all_fr8_sections`. Removing
`swap_invariants` from `POST_V1_SCHEMA_SECTIONS` caused the witness to fail
with `missing degradation for swap_invariants` (exit 101); the source was
restored by copy-back, `cmp` and `touch` (exit 0).

AC3 `skill_declarations_20_8::unavailable_pending_escalation_and_admitted_skills_are_distinguished`
passed (exit 0) under `env -u CI -u GITHUB_ACTIONS cargo test --locked -p
maos-bin --test skill_declarations_20_8` with real filesystem discovery and
an approval row. Bypassing the capability-subset check made its escalation
assertion fail (exit 101 at line 50). Original source was restored by
copy-back, `cmp` and `touch` (exit 0). The isolated binary owns a default-size
test-budget exception and serial nextest group.

AC4 witness `recipient_promotion_set_checks_stored_lineage_before_enqueue`:
`env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-iac --test
intent_promotion_20_8` passed (exit 0) before the plant. Removing the
`self.check_promotion(&frame)?` call made the same test fail its refusal
assertion (exit 101); the original was restored with `cp`, verified with `cmp`
and touched (exit 0). The source was not yet formatted at this observation.

AC6 real runner witness `wasm_skill_authoring_enforces_root_identity_and_capability`
passed (exit 0, 1.00 s) with `env -u CI -u GITHUB_ACTIONS
MAOS_17_2_CGROUP_MODE=undelegated cargo test --locked -p maos-bin --features
maos-bin/wasm-host --test wasm_skill_authoring_20_8`. Earlier runs found a
missing budget context field (exit 101), then a missing principal-index table
in the test fixture (exit 101); both fixture errors were corrected. Replacing
the daemon's skill-author arm with `Unavailable` caused the real witness to
fail (exit 101, no pending entry); copy-back, `cmp`, and `touch` restored the
original (exit 0). This does not establish the hosted four-image proof.

## Formatted measurement and STOP

`cargo fmt --all` exited 0; prebuilt `kloc-check --json` then exited 1
(pins have not been changed). The domain measured 10,190 against the rebased
10,174 ceiling, exceeding it by 16; its pin was 10,144. This triggers the
story's STOP and requires a new operator D-row. Other measured values remain
within rebased ceilings: bin 35,851 / 36,160; IAC 8,693 / 8,865; manifest
4,710 / 4,812; wasm-host 2,491 / 2,874; xtask 44,700 / 44,749; aggregate
198,019 / 199,078. Registry remains 6,151, kernel 21,042. This is an
intermediate measurement, not a landing or exact re-pin.
The orchestrator adjudicated a domain overdraft to 10,205 with 15 lines slack,
disclosed to the operator; aggregate ceiling remains 199,078. The 20-9,
20-13 and 20-14 domain draws must re-base on the final landed pin.

Intermediate formatted candidate on the `a13140b8` base: `cargo fmt --all` exited 0,
then `kloc-check --json` measured aggregate 198,063 (ceiling 199,078), bin
35,850/36,160, manifest 4,710/4,812, wasm-host 2,521/2,874, domain
10,190/10,205 (orchestrator-adjudicated overdraft, disclosed to the operator),
IAC 8,693/8,865, skill 1,234/1,294 and xtask 44,715/44,749. Registry is 6,151,
CLI 6,429 and kernel 21,042 unchanged. These intermediate pins were set and
the aggregate hardfail was set to 198,064; the repinned `kloc-check --json`
exited 0. Subsequent unload cleanup requires a fresh final measurement. The
original error `xtask kloc --json` was an unrecognized subcommand (exit 2);
the actual gate is `kloc-check`.

After the unload cleanup, a second `cargo fmt --all` exited 0 and
`kloc-check --json` measured bin 35,861 and aggregate 198,074 (the other
measurements above unchanged). It exited 1 against intermediate pins, as
expected; bin was re-pinned to 35,861 and aggregate hardfail to 198,075.
Both remain within their respective 36,160 and 199,078 L1 ceilings.

The first locked workspace metadata producer obtained `cargo metadata` (exit 0)
but its `cargo nextest list --workspace --locked --list-type binaries-only
--message-format json` compilation was interrupted by the 3,000 s cell deadline;
no test was executed. It is a build-producer interruption, not a failed test or
a retry to green; the compilation may be continued using the same target.
The continuation of the binary-only producer exited 0 after 11m14s; it used the
same target and executed no tests. The workspace run uses the resulting
binary metadata and the locked Cargo metadata without conflicting package
selection flags.

The single workspace execution exited 100: 5,841 passed, 6 failed, 137
skipped (`workspace-nextest.log`, `workspace-junit.xml`). Two failures were
caused by missing Python `cryptography` from the separately declared verifier
requirements; two were existing 20-7 timing families (same-line rule-10
records in `deferred-work.md`); neither was retried. Two were real 20-8
regressions: `fr4_classifier_16_2::every_writer_site_demanding_a_disposition_has_one`
identified the new promotion refusal as an unclassified tokenless
`CapabilityInvocation`, and `wit_corpus::corpus_covers_all_record_types`
found the added `queued` WIT record (28, not 27). The refusal is a non-call
policy decision, now logged as `TelemetryEvent` with the same typed intent;
the WIT corpus now enumerates `queued` and its two fields and tests all three
`skill-error` variants. No audit writer, redaction, kernel or budget changed.
These are deterministic corrections, not retries of the timing observations.

After those deterministic corrections, `cargo test --locked -p maos-audit
--test fr4_classifier_16_2 every_writer_site_demanding_a_disposition_has_one`
exited 0; `cargo test --locked -p maos-wasm-host --test wit_corpus` exited 0
(24 tests), and `cargo test --locked -p maos-iac --test
intent_promotion_20_8` exited 0. The latter asserts the denial has a
non-call telemetry kind and no capability token. After `cargo fmt --all`,
`kloc-check --json` again exited 0 with unchanged exact final pins.
The one workspace execution preceded these corrections; it is not described
as a green final-tree workspace run. The declared verifier dependencies were
installed in `/tmp/20-8-verifier-venv` using `pip install
--only-binary=:all: --require-hashes -r tools/verify-erasure/requirements.txt`
(exit 0) for the separate prescribed prepush, not as a retry of those tests.
`check-fkcs --json` completed in its own 2,700 s cell with exit 0 and a
green oracle (282.54 s). An earlier structural run of
`check-pub-field-constructors`, `check-serde-error-handling`,
`check-composition-root-completeness` and `check-service-boundary` exited 0.
The standalone workspace budget gate exited 1: IAC participant-recall summed
30.319 s (rule-10 owner 20-7) and the feature-only WASM binary was absent from
default-feature JUnit (`workspace-budget.txt`). A real default-feature WIT
AST witness now checks the versioned skills import in that owned binary;
the first `cargo fmt --all` after moving the live witness into a module
exited 1 because its relative fixture module moved. Moving the fixture module
back to crate root fixed the path; `cargo fmt --all` then exited 0. Both
`cargo test --locked -p maos-bin --test wasm_skill_authoring_20_8` (1 test)
and the same command with `--features maos-bin/wasm-host` and undelegated
cgroup mode (2 tests) exited 0. No existing flake was retried.

The new production I2 seam witness
`hook_pump_17_4::declaration_storage_failure_stops_before_pid_issuance`
installs a SQLite trigger that aborts only declaration inserts. It expects
the unmodified writer's I2 panic, and an unwind guard checks that neither the
scheduler nor the pid lookup has an entry. The focused command
`env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --test
hook_pump_17_4 declaration_storage_failure_stops_before_pid_issuance` exited 0.
No production panic is caught or suppressed.

After adding a skill-id-in-record assertion and guest-visible reply assertions,
`cargo test --locked -p maos-bin --test skill_declarations_20_8` exited 0;
`cargo test --locked -p maos-kernel-core --test spirit_boundary_invariants`
exited 0. The focused wasm-host command exited 101 before guest execution:
the fixture freshness guard reported `maos-wasm-runner` older than
`crates/maos-wasm-host/src/recall_wire.rs`. This is a deterministic missing
fixture rebuild, not an asserted flake; the prescribed `build-test-fixtures`
then completed all 19 artifacts (exit 0) before subsequent execution.
The same `MAOS_17_2_CGROUP_MODE=undelegated cargo test --locked -p maos-bin
--features maos-bin/wasm-host --test wasm_skill_authoring_20_8` then passed
(exit 0, 0.92 s), including the actual guest-visible author/proposal outcomes.
The genuine `cargo build --locked -p maos-bin -p maos-cli` after the fixture
builder exited 0.

IAC cost disclosure: `deliver_typed` now executes a payload discriminant test,
and for TaskAssign an optional-reference test. Only a referenced distillate
does per-recipient map lookup; only a recipient with a registered set incurs
receipt lookup/deserialization and policy checking. Denial adds a journal row.
The ordinary non-TaskAssign path adds no allocation or database lookup. No
measurement establishes a new instruction baseline; the existing 997449.662
instruction count and 2% tolerance are not widened.

Local contract gates (`env -u CI -u GITHUB_ACTIONS
CARGO_MANIFEST_DIR=$PWD/xtask /mnt/build/cargo-maos/debug/xtask <gate>`):
`check-manifest-schema-version --json`, `check-equiv-fixture-provenance --json`,
`check-corpus --json`, `check-env-contract --json`, and
`check-service-boundary --json` exited 0. `gen-abi-docs --check` initially
exited 1 for stale schema-6 constants; after `gen-abi-docs` regenerated nine
pages (exit 0), its check exited 0. The requested spelling
`check-stability-matrix` does not exist (exit 2); the actual
`stability-matrix --check --json` initially exited 1 for schema drift, then
`stability-matrix --json` regenerated the file (exit 0).
Bare `abi-diff --json` attempted an internal git checkout, which refused the
dirty tree (exit 1); it is deferred to the orchestrator's clean post-commit
run, not repeated here. The checkout hook produced no `graphify-out` status
entries (`git status --short -- graphify-out`, exit 0).

`cargo test --locked -p maos-cli --test manifest_intent_20_8` exited 0.
`cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3`
exited 0; `cargo check --locked --workspace --all-targets --target
aarch64-apple-darwin` exited 101 because the local C compiler rejects Apple's
`-arch` and `-mmacosx-version-min` flags (Apple SDK/cross toolchain unavailable).
After the fixture builder, `cargo build --locked -p maos-bin -p maos-cli`
exited 0. Node locally is 20.19, below the example's required 22.20; the
branch-only hosted workflow is authored but unverified.

The regenerated `stability-matrix --check --json`, `check-kernel-baseline
--json` (26,841 lines, 95 files, unchanged set hash), `check-dev-record-completeness
--json`, `check-epic-close-coherence --json`, `check-exit-commands --json`,
`check-decision-register --json` and `templates-regen --check --json` exited 0.
`check-test-cost --json` initially rejected two owned exceptions at 30 s
because exceptions must be above the 30 s default (exit 1). They were set to
the minimum valid whole second, 31 s, and the gate exited 0. These are
new serial disk-backed binaries, not a flake retry; observed focused durations
were 0.01 s and 1.00 s respectively.

`coverage-matrix --json` and `check-coverage-matrix-completeness --json`
exited 0. The story-named `check-invariant-claims` is not an available xtask
subcommand (exit 2). Node's installed `yaml` module parsed the hosted recipe
and enumerated all four jobs (exit 0); Ruby is absent (exit 127) and Python's
PyYAML is absent (exit 1). Parsing is not hosted execution.

The prebuilt `kloc-check --json` run against the current tree reported aggregate 196,964, kernel 21,042, maos-manifest 4,604, maos-bin 35,589, maos-wasm-host 2,412, maos-domain 10,137, maos-iac 8,625, maos-skill 1,234 and xtask 44,696 (exit 0). Rebase the creation-time L1 allowances under D02 A onto these pins; ceilings are not reservations:

The following table is the superseded initial rebase, retained as historical T0 evidence; the effective later ceiling is in the story and flag decision.

| Crate | Pin | L1 allowance | Initial ceiling (superseded) |
|---|---:|---:|---:|
| maos-manifest | 4,604 | 208 | 4,812 |
| maos-bin | 35,589 | 574 | 36,163 |
| maos-wasm-host | 2,412 | 462 | 2,874 |
| maos-domain | 10,137 | 30 | 10,167 |
| maos-iac | 8,625 | 240 | 8,865 |
| maos-skill | 1,234 | 60 | 1,294 |
| xtask | 44,696 | 53 | 44,749 |
| aggregate | 196,964 | 1,627 | 198,591 |

Kernel is unchanged in L1. The later rebase supersedes this table; it is not the final measurement or ceiling.

## Observed failing-before witness

`env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-spirit-abi --test manifest_n_minus_1_test schema_seven_is_supported_for_fr8_declarations` exited 101 before the schema bump, failing the assertion that `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION >= 7` at `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs:31`.

## T0 contradiction — STOP

AC1.3 requires a journal-write failure to return `AdmissionRefusal::Admission`
and roll the load back. The creation `gate` prototype calls
`TransparencyLogAdapter::insert_frame_event` and only handles serialization
errors. The current writer documents a binding I2 panic on SQLite write failure
(`crates/maos-iac/src/adapter/transparency_log.rs:1135-1166`), enforced in its
shared row writer (`:1431-1443`); it does not return a write error. The
prototype therefore cannot implement the ruled rollback. T0 requires STOP and
a D-row for a result contradicting a ruled option. No audit emitter has been
installed; an operator ruling is needed on the I2 failure boundary before
implementing it. The partial schema-7 tree is not a completed landing and must
not be released.

The prescribed prototype compilation command was run on the partially
re-cut tree: `env -u CI -u GITHUB_ACTIONS cargo check --locked -p maos-manifest
-p maos-domain -p maos-bin -p maos-wasm-host -p maos-kernel-core --features
maos-bin/wasm-host`. It finished successfully with warnings (exit 0); this
does not compile the as-yet-unimplemented WIT bridge or kernel prototype and
is not proof that those prototypes compile.
