---
story_id: '21.3'
story_key: 21-3-durable-tofu-and-self-leaf-rotation
created: 2026-11-20
status: backlog
context_commit: ea8c51b6a943f536179fe2238bcf050353580d35
baseline_commit: ea8c51b6
security_sensitive: true
kernel_grant: "Not needed: kernel-Δ forecast 0 (method: read every touched file — maos-a2a-core tofu.rs, maos-a2a-tcp config.rs/transport.rs, maos-persistence, maos-control lib.rs, maos-bin main.rs/operator_door.rs/repin_door.rs; none is under crates/maos-kernel-core/src). Landing still runs check-kernel-baseline --json and kloc-check --json; if the measured kernel delta is non-zero, re-pin with scripts/repin.py --apply and write the HISTORY row in the same commit. No alarm forecast."
kloc_grant: "Read-the-files forecast, not measured growth or permission (recommended D-row options): maos-persistence +170…+320 (ceiling 101, src 1 — exceeds the ceiling's 100 headroom; landing needs an orchestrator-adjudicated measured raise); maos-a2a-core +70…+150 (headroom 101; may exceed at the top); maos-a2a-tcp +10…+40; maos-control +35…+65; maos-bin +70…+160; maos-cohort 0; maos-domain 0."
review: "DIFFICULT: security-critical (peer authentication state) and concurrency (window_lock ordering, persistence under the lock). Follow docs/testing/proof-contract.md. D-E20-6: this story adds a persisted trust-state input read at boot, so treat it as a new trust boundary (option B: hold the landing for the non-GPT security reviewer). Creation anthropic/claude-opus-5-5 (DraftDurablePins). No independent review or execution claimed."
---

# Story 21.3: Durable TOFU pin store and authenticated `GET /v1/a2a/pins`

Status: backlog

**Readiness:** **backlog; operator rulings recorded, no status promotion.** On 2026-11-21 UTC the operator selected **A/A/A/A/A/A/A** for D-21-3-01 through D-21-3-07 in the preflight form. The existing AC1 criteria and forecasts already reflect those options. The story remains ordered after 21-9; this record does not authorize implementation, landing or release, and does not change the shared tracker.

**GA-required, AC1 only, size M.** Authority: `planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md:11` (disposition table), `:112` (AC1 text) and `:119` (the 2026-11-13 correct-course note). The tracker row is `implementation-artifacts/sprint-status.yaml:415`, the GA set is `:484`, and the release record is `RELEASE-HOLDS.md:93` (row 12) and `:113` (v1.0.0 claim boundary 2) at `ea8c51b6`. Paths under `planning-artifacts/` and `implementation-artifacts/` are relative to `_bmad-output/`.

## Story

As **an operator running a `cohort-a2a-daemon` that peers with other hosts**,
I want **the daemon's TOFU peer pins (fingerprint, boot nonce, invalidation, consented re-pins and promoted rotations) to survive a daemon restart, and to read them back from the authenticated operator door**,
so that **a restart no longer discards consented re-pins or re-trusts a fingerprint that a rotation retired, and I can see what the mesh trusts without grepping process memory**.

**Exact scope:** AC1 of the epic only. AC2 (own-leaf rotation caller and `T_grace`), AC3 (`RuptureReason::CertPostGraceReject`) and AC4 (ADR-063 SAN binding and listen-side per-peer scoping) belong to `23-7-peer-identity-rotation` (`sprint-status.yaml:445`). AC6 (`MAOS_REGION_HOME` boot reconciliation, D4a) belongs to `22-1-host-config-and-standing-cast` (`:428`). Not in this story:

- certificate rotation or SAN changes of any kind;
- a new rotation or re-pin write path (the signed cohort-manifest reissue and the 19-7 consent door stay the only writers of pin state);
- persisting rotation windows (`rotation_next`);
- changes to the async `TofuPinStore` trait signatures.

The story key keeps its historical title. The rotation half of that title is out of scope.

## Inventory at base — source measurements, not execution

Measured at `ea8c51b6` (`/home/lunarpulse/dev_ws-ga-wasm`, read-only `git show`; the touched crates are clean in that worktree). No Cargo, build, test, gate or formatter was run. `tokei` 14.0.0 was run read-only on `src/` only.

| Surface | Current state at `ea8c51b6` | Preserve / required change |
|---|---|---|
| Pin record | `TofuPin` (`crates/maos-a2a-core/src/tofu.rs:19-33`): `peer`, `fingerprint`, `boot_nonce`, `pinned_at_ns` (a process-local counter, `now_ns` `:895-899`, meaningless across restarts), `invalidated: Option<Invalidated>` (`:36-39`: `SpiritRestarted { prior_boot_nonce }` or `Manual`), `repin_approval_id`. It has **no `generation` field**, but the epic's route lists one (D-21-3-05). | Keep `TofuPin` serde byte-identical (the rotation side-map doc, `:220-230`, depends on it). Put durable-only fields in the persistence record, not in `TofuPin`. |
| Store | `InMemoryTofuPinStore` (`:217-253`) is the only `impl TofuPinStore` (`:647`). It holds `pins`, `rotation_next` (a deliberately process-local window side-map: a reboot must close any open window, `:220-230`), `window_lock` (`:231-235`, ordered before any `pins` entry lock), `pending_repins` (`Instant` deadlines, `:236-250`), `repin_ttl` and `repin_sink`. The trait doc (`:152-155`) and the struct doc (`:215-216`) claim that a persistence-backed implementation lives in `maos-persistence`. None does. | Durability must not persist `rotation_next`. Every pin mutation needs a durable counterpart. The two stale doc claims are fixed in the same change. |
| Pin mutations | First contact `:648-684`; promote-and-retire `close_rotation_window` `:495-516` and `close_rotation_window_if` `:560-583`; restart invalidation `invalidate_for_restart` `:733-749` and `invalidate_if_boot_nonce_differs` `:829-886` (the one production detection path); consented re-pin `await_repin_consent` `:762-823`. Read paths: `verify_pinned_sync` `:596-638`, `find_active_pin_by_fingerprint` `:389-420`, `get_pin_sync` `:641-643`, and the async trait reads. | Each listed mutation persists (D-21-3-01 seam, D-21-3-04 failure policy). The read paths stay lock-free and allocation-free on the TLS callback, with no I/O in the verifier. |
| Seeding | `TcpA2AConfig::build_pin_store` (`crates/maos-a2a-tcp/src/config.rs:125-145`) calls `pin_first_contact` for each `[[tcp.peer_pins]]` entry at every boot. `transport.rs:375` calls it inside the bind chain. The `PinnedFingerprint` doc (`config.rs:32-40`) says a consented re-pin "lives in memory until the next boot rebuilds pins from this config". | Seeding now reconciles the config with the durable record (D-21-3-02). The config doc is rewritten to match. |
| Boot manifest check | `reconcile_transport_identity_with_manifest` (`crates/maos-bin/src/main.rs:13247`, called `:13389` from `build_cohort_a2a_daemon_runtime` `:13356`) refuses to boot if `tcp.peer_pins` or `file.peers` disagree with the **on-disk** signed manifest. `apply_reissue` writes only the in-memory cache; boot re-reads `file.manifest_path` (`crates/maos-cohort/src/state.rs:460-466`). | Keep this check unchanged and run it first. As a result, after a runtime promotion the durable record can disagree with both config and the on-disk manifest. D-21-3-02 decides what happens then. |
| Concrete store type | `Arc<InMemoryTofuPinStore>` is used in `maos-a2a-tcp/src/transport.rs:25,106,156,526,813,980,1186,1328,1356`; `config.rs:11,125-126`; `verifier.rs:29,63,96` (calls `verify_pinned_sync` `:173`, `find_active_pin_by_fingerprint` `:178`); `maos-bin/src/repin_door.rs:25,46,61`; `maos-cohort/src/rotation.rs:145,330,350,1129` (opens/closes/aborts windows); `maos-cohort/src/state.rs:274`. `A2ARouterCore::try_new` already takes `Arc<dyn TofuPinStore>` (`transport.rs:387`). | The epic counted 9 sites in one file. The base has 22 sites in 6 files, and they use about 13 inherent methods. This is the measured premise behind D-21-3-01. |
| Daemon root | Only `run_cohort_a2a_daemon` (`main.rs:12867`) keeps the concrete transport. It installs rotation (`:13055-13060`), the re-pin sink and the re-pin control (`:13070-13080`). Other roots never install them, and their routes answer 404 (`repin_door.rs:9-14`). `cert_rotation.rs:9-23` explains why a fresh `MAOS_ONE_SHOT` child must not mutate pin state. | Only the daemon root opens the durable store. `maos run` / `maos shell` / one-shot arms stay in memory. Allow one writer per file. Two roots share `RootShared` store locks (`main.rs:2017-2019`, `operator_door.rs:3085-3110`), so the pin file needs its own single-writer refusal. |
| Operator door | Bearer check: `read_head` (`crates/maos-control/src/lib.rs:994`) runs a constant-time compare before route and method resolution. `route()` matches by segment (`:1125`). GET arms: `rotation-windows` `:1157`, `repins` `:1161` (19-7: reads the store's **pending requests**, not pins), `peer-versions` `:1175`, `self-identity` `:1176`. Read ports are `bind` parameters (`RotationWindowSource` `:78`, `CohortConvergenceSource` `:132`, `CohortSelfIdentitySource` `:164`); 19-7 instead added a defaulted `OperatorCommandPort::pending_repins` (`:638`), implemented in `operator_door.rs:2900` through `RepinControl`. Callers of `OperatorHttpServer::bind*` (bind or bind_with_commands): 21 in 7 files. `maos-control` depends on `maos-domain`, `maos-kernel-core`, `ring` and `serde_json` only. | `GET /v1/a2a/pins` goes behind the same `read_head` gate with no second auth path. It answers 405 to other methods (ADR-062 `:227`) and 404 when no store is installed. `maos-control` gains no crate edge. The port shape is D-21-3-06. |
| Persistence crate | `crates/maos-persistence/src/lib.rs` is an 8-line placeholder. `Cargo.toml:9-10` has an empty `[dependencies]`. No crate depends on it; it is only a workspace member (`Cargo.toml:29`). | It becomes the home of `FilePinStore` (epic §H-7: one JSON file, `$MAOS_HOME/a2a-pins.json`). Adding the `maos-a2a-core` edge keeps the graph acyclic: a2a-core depends on `maos-domain`, `maos-spirit-abi` and `maos-capability` only. |
| Atomic write convention | `ControlFile::write_via` (`crates/maos-domain/src/operator_door.rs:463-494`): a same-directory `create_new` temp with a random suffix, mode `0600`, `sync_all`, rename (or no-replace hard link), then a parent-directory `sync_all` and temp cleanup. `MAOS_HOME` is resolved once by `maos_home()` (`:61-75`) and registered at `crates/maos-bin/src/env_contract.rs:417`. | Use the same write semantics and the same home rule. Do not invent another write convention. |
| NFR-Rel-6 proofs | `crates/maos-a2a/tests/restart_invalidates_pin_nfr_rel_6.rs`: 6 `scenario_4_*` tests (`:78-200`) and 15 `repin_19_7_*` tests (`:222-760`). Other 19-7 tests: `maos-bin/tests/repin_door_19_7.rs`, `repin_hold_19_7.rs`; `maos-a2a-tcp/tests/repin_tls_19_7.rs`; `maos-control/tests/repin_surface_19_7.rs`. | All stay green and unedited apart from mechanical constructor migration (if D-21-3-01 B is chosen). |
| Daemon harness | `crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs`: `boot_daemon` `:237`, `operator_get` `:316`, `drive_a_signed_rotation` `:383`. A real daemon is driven by a signed reissue and read through the operator door. | Reuse it for the restart proof (move the helpers to `tests/support/` rather than copying them). |

## Decisions — operator rulings recorded 2026-11-21 UTC

### Already settled; do not ask again

| Settled point | Source |
|---|---|
| GA = AC1 only; AC2–AC4 → 23-7; AC6 → 22-1; 14-2 clause (c.2) stands at v1.0.0 | Epic `:11`, `:119`; SCP 2026-11-13 §2; `RELEASE-HOLDS.md:113` |
| One JSON file `$MAOS_HOME/a2a-pins.json`, not SQLite or TL frames (§H-7) | Epic `:112` |
| `FilePinStore` lives in `maos-persistence` | Epic `:112`; 21-4 AC4 (epic `:128`) |
| The async `TofuPinStore` surface stays unchanged (§H-8, story 8.6 comment) | Epic `:112`; `tofu.rs:376-388` |
| NFR-Rel-6 is preserved, not replaced; the persisted row is what the operator sees, and trust still needs re-pin consent | Epic `:112` |
| Kernel lines inside the ADR-069 envelope are self-service (`kfree`) | ADR-069 amendment 2026-11-12; `decisions-2026-11-12-dedrift.md` |

### D-rows — operator rulings

| ID | Material choice and options (historical) | Recommendation and evidence (historical) | Operator ruling (2026-11-21 UTC) |
|---|---|---|---|
| **D-21-3-01** | **Durability seam.** **A** — Add a sync persistence port in `maos-a2a-core` (e.g. `PinPersistence`: `load() -> snapshot`, `store(&snapshot)`). `InMemoryTofuPinStore` gains a constructor that takes one, and each mutation in the inventory calls it while it still holds `window_lock`. `FilePinStore` (in `maos-persistence`) implements the port. Every concrete `Arc<InMemoryTofuPinStore>` site stays as it is. **B** — The epic's design: a new `SyncPinLookup` sub-trait, with all sites re-typed to `Arc<dyn TofuPinStore + SyncPinLookup>` and `FilePinStore: TofuPinStore` as a second full store. **C** — Like A, but the port is async and writes on a background task (write-behind). | **A.** The epic priced B as 9 sites in one file with 2 inherent methods. At base it is 22 sites in 6 files (`maos-a2a-tcp`, `maos-bin`, `maos-cohort`) using about 13 inherent methods (rotation window open/close/close_if/abort/next, re-pin pending/record/withdraw/sink, plus the two sync lookups). A sub-trait that carried all of them would amount to a second store implementation of the 14-2/19-7 lock-ordering invariants (`rotation.rs:131`, `tofu.rs:231-235`). A keeps one implementation of those invariants and no re-typing. C reopens the crash window this story closes: a promotion would be acknowledged in memory before it is durable. | **Ruled A** |
| **D-21-3-02** | **Boot precedence: `[[tcp.peer_pins]]` entry versus durable record.** **A (anchor rule)** — Each record stores the config entry `(F_a, N_a)` it was seeded from. At boot, for each configured peer: no record → seed from config (as at base). Config equals the anchor and the record's fingerprint equals the config → restore the record (nonce, invalidation, generation). Config equals the anchor but the record's fingerprint differs (a runtime promotion the operator never carried into config and the on-disk manifest) → **typed boot refusal** that names the peer, the configured (retired) fingerprint and the promoted one. Config differs from the anchor (the operator re-paired) → config wins, the anchor is re-seeded, and the record is kept only if its fingerprint equals the new config. Records for peers absent from config are dropped and never trusted. In every branch the durable file can never add a fingerprint that config does not name. **B** — Config always wins. The durable record restores the nonce and invalidation only while the config entry is unchanged; a runtime promotion is silently reverted to the configured fingerprint (as at base). **C** — The durable record always wins and config seeds only new peers. | **A.** Under B, a restart after a promotion re-trusts the fingerprint the rotation retired. At base this already happens silently, because the on-disk manifest and config still name it (`state.rs:461-466`, `main.rs:13247`). C lets a file that only the daemon writes introduce fingerprints that config and the signed on-disk manifest never named, which widens the trust input. A closes the retired-fingerprint resurrection in a way the operator can see. It also keeps the 13.6a rule that disagreement is a boot error, never a warning (`main.rs:13237-13246`), and it keeps the documented "correct the config and restart" recovery (`config.rs:32-40`, `repin_door.rs:155-160`). Under A, AC1's "row shows the promoted fingerprint after restart" holds once config and the on-disk manifest name it; otherwise the daemon refuses to boot and says why. | **Ruled A** |
| **D-21-3-03** | **An invalidated pin across a restart.** At base, intake refuses an invalidated pin before detection runs, and the TLS listener skips it (`tofu.rs:236-250`; `repin_tls_19_7.rs` header). A restored invalidated pin with no pending request therefore can never be approved through the door. **A** — Persist the invalidation together with the pending request's `(peer, pinned_fingerprint, prior_boot_nonce, observed_boot_nonce)`, minus the verdict. At boot, re-arm it with a fresh `repin_ttl` and journal it through the existing detection sink. **B** — Persist the invalidation only. Recovery is "correct the config nonce and restart" (config differs from anchor → re-seed). **C** — Do not persist invalidation: restore the pin as active at its stored nonce, so the peer's next frame re-detects the restart (as at base). | **A.** It keeps both documented recoveries: door consent, and "until it restarts" (`repin_door.rs:155-160`). The `GET` row shows `invalidated` across the restart, as epic AC1 requires. B leaves the door's 404/`repin_request_expired` path as the only signal. C shows an active row for a peer whose restart is known, so the operator sees stale trust until traffic arrives. | **Ruled A** |
| **D-21-3-04** | **Persistence failure policy.** **A** — Transitions that widen trust (first contact at runtime, an approved re-pin) persist first and are refused on failure: the consent answers a typed 500 and the pin stays invalidated, mirroring 19-7's "unwritable row withdraws the verdict" (`repin_door.rs:15-19`). Transitions that narrow trust (promotion close, restart invalidation, dropping a re-armed request) apply in memory and are journaled, the store is marked degraded, and `GET /v1/a2a/pins` answers 503 `pins_unhealthy` until a later write succeeds. A load failure at boot (unreadable file, bad schema or version, duplicate peer, unparsable fingerprint) is a typed boot refusal. **B** — Any persistence failure stops the daemon. **C** — Log and continue (best effort). | **A.** A refused promotion would leave the window open, so the retired generation would stay trusted (`tofu.rs:545-559`), and fail-stop (B) turns a disk fault into a mesh outage in the middle of a rotation. C silently restores base behavior after the next restart. A uses the existing typed 503 shape (`RotationWindowStatus::Unhealthy`, `lib.rs:61-65`) and the 19-7 refusal shape. | **Ruled A** |
| **D-21-3-05** | **`generation` in the row.** **A** — A durable per-peer counter: 0 at first seed, +1 each time the pinned fingerprint changes (runtime promotion or a re-pair to a different fingerprint), and unchanged by nonce-only re-pins. **B** — Report the live open-window `next` fingerprint instead. It is never persisted, and `GET /v1/a2a/rotation-windows` already reports it. **C** — Drop the column. | **A.** `TofuPin` has no generation field and `pinned_at_ns` is process-local (`tofu.rs:27`, `:895-899`). A durable counter is the only field that shows a promotion surviving a restart, which is AC1's observable. B duplicates an existing route. | **Ruled A** |
| **D-21-3-06** | **Route port shape.** **A** — A defaulted `OperatorCommandPort::peer_pins() -> Option<PeerPinStatus>`, implemented by `RepinControl` (which already holds the installed daemon store) and delegated from `operator_door.rs` next to `pending_repins` `:2900`. No `bind*` signature changes. **B** — The epic's new `PinsSource` port, added as a seventh `bind`/`bind_with_commands` parameter and implemented in `cert_rotation.rs` next to the 14-2b sources (`:84,130,174`). That migrates 21 caller sites. | **A.** This is the most recent precedent for exactly this store (19-7's `pending_repins`, `lib.rs:638`). It uses the same set-once install (`main.rs:13076`), so the route answers 404 in every non-daemon root with no new wiring and no caller churn. B matches the older read-port convention, but it touches 7 files of tests and roots and adds no behavior. | **Ruled A** |
| **D-21-3-07** | **RELEASE-HOLDS row 12 after AC1.** Row 12 (`RELEASE-HOLDS.md:93`) has three may-not-claims: (i) TOFU state, including a consented re-pin, survives a restart — closed by AC1; (ii) first contact is observed rather than configured — AC1 does not change this; (iii) the listen side scopes a client leaf to a peer — epic AC4, now 23-7. **A** — Narrow row 12: remove (i), keep (iii) with successor `23-7-peer-identity-rotation`, and keep (ii) as a permanent boundary by design (ADR-012 pre-paired fingerprints, `config.rs:19-22`). **B** — Narrow it the same way, but name a successor for (ii) (23-7). **C** — Keep row 12 whole until 23-7. | **A.** The epic's note that AC1 "closes row 12" (`:119`) over-claims, because (iii) is AC4. Under rule 10 a retained clause names a destination. Configured first contact is ADR-012's intended model, not a defect, so (ii) is a stated boundary rather than deferred work. C hides a real closure. The orchestrator or release owner edits `RELEASE-HOLDS.md`; this story hands over the text. | **Ruled A** |

Kernel: no D-row. The forecast is 0, far from the 22,000 alarm and the 23,000 envelope.

## Acceptance Criteria

The criteria are defensive and observable. They do not describe how to construct or reproduce an attack. Sub-criteria assume the recommended options. If a ruling differs, re-state the affected sub-criterion before development.

1. **AC1 — durable peer pins with authenticated readback (the GA AC).**
   1. **AC1.a Durable store.** `maos-persistence` provides `FilePinStore`, which reads and writes `<maos_home()>/a2a-pins.json`. The file has a versioned, strict schema (unknown fields and versions refused, one record per peer, fingerprints in `sha256:<hex64>` wire form, nonces as decimal strings). Writes follow the `ControlFile::write_via` semantics (unique `create_new` temp, `0600`, `sync_all`, rename, parent `sync_all`, temp cleanup). A second concurrent opener of the same file is refused with a typed error. Only `run_cohort_a2a_daemon` opens it; every other root keeps the in-memory store unchanged.
   2. **AC1.b Every pin mutation is durable** (D-21-3-01 seam, D-21-3-04 policy): first contact, both promotion closes, both restart invalidations, an approved re-pin and the re-seed at boot. `rotation_next`, `pinned_at_ns` and recorded verdicts are **never** persisted. A restart always closes an open window, as at base.
   3. **AC1.c Boot reconciliation** (D-21-3-02): the 13.6a manifest check runs first and is unchanged. The durable file can never add a fingerprint that `[[tcp.peer_pins]]` does not name. If a runtime promotion was not carried into config and the on-disk manifest, the boot refusal names the peer, the retired fingerprint, the promoted fingerprint and the two recoveries.
   4. **AC1.d NFR-Rel-6 preserved** (D-21-3-03): an invalidated pin stays invalidated across a restart, and its pending request is re-armed with a fresh TTL and journaled. `GET /v1/a2a/repins` lists it, and the 19-7 consent door can approve or reject it. All 21 tests in `restart_invalidates_pin_nfr_rel_6.rs` and the other 19-7 suites pass unedited (apart from constructor migration if D-21-3-01 B is chosen).
   5. **AC1.e Route.** `GET /v1/a2a/pins` sits behind `read_head`'s bearer compare. It answers 200 with a JSON array sorted by peer, `{peer, fingerprint, generation, boot_nonce, invalidated}`, where `boot_nonce` is a decimal string and `invalidated` is `null` or `{reason, prior_boot_nonce}`. It answers 404 when no store is installed, 405 to every other method, 401 without a valid bearer, and 503 `pins_unhealthy` while the store is degraded. An empty array means "installed, nothing pinned", never "no control".
   6. **AC1.f Observable across restart (the epic's command).** A real daemon pins a peer, is rotated by a signed reissue past grace, and receives `SIGTERM`. The operator then carries the promoted fingerprint into config and the on-disk signed manifest. After restart, `GET /v1/a2a/pins` shows the promoted fingerprint with `generation` ≥ 1. A consented re-pin (new nonce) also shows its approved nonce after restart.
   7. **AC1.g Proven-red** (re-specified, see the cite corrections): delete `a2a-pins.json` between `SIGTERM` and restart. The re-pinned peer's row reverts to the config seed (config nonce, `generation` 0), and its next frame is re-detected as a restart. With the file kept, the frame is accepted. Separately, restart without carrying a promotion into config → typed boot refusal.
   8. **AC1.h Docs and claims.** Fix the stale claims in `tofu.rs:152-155,215-216` and `config.rs:32-40`. Hand the row-12 narrowing text (D-21-3-07) to the release owner. The GA notes must not claim (ii) or (iii).

**AC2, AC3, AC4 and AC6 are intentionally absent** (transferred, see Exact scope).

## Tasks / Subtasks

- [ ] **T0 — apply rulings.** Record the operator's D-21-3-01…07 selections in this file, then re-state AC1 sub-criteria and forecasts if a non-recommended option is ruled. Re-measure the inventory cites against the actual landing base.
- [ ] **T1 — persistence port and store hooks (AC1.b).** In `maos-a2a-core/src/tofu.rs`, add the port, a snapshot type that carries the durable-only fields (anchor, generation, re-armed request) and a constructor that loads from the port. Call the port from each mutation under `window_lock`, keeping the lock order `window_lock` → `pins` entry (the persistence write comes after the in-memory decision; no I/O on the verifier read paths). Implement the boot re-arm of pending requests (D-21-3-03 A) and the degraded flag (D-21-3-04 A). Fix the stale trait and struct docs.
- [ ] **T2 — `FilePinStore` (AC1.a).** In `crates/maos-persistence`, add the `maos-a2a-core`, `serde` and `serde_json` deps. Implement the versioned strict schema and the atomic write following `ControlFile::write_via`, with typed load errors and a single-writer refusal. Replace the placeholder crate doc.
- [ ] **T3 — seeding and boot reconciliation (AC1.c).** Thread an optional port through `TcpA2AConfig::build_pin_store` and the bind chain (`transport.rs:375`; no re-typing under D-21-3-01 A). Implement the D-21-3-02 rule and its typed refusal at the seeding seam, after `reconcile_transport_identity_with_manifest`. Rewrite the `PinnedFingerprint` doc.
- [ ] **T4 — daemon wiring (AC1.a).** In `run_cohort_a2a_daemon`, open `FilePinStore` at `maos_home()` and pass it to the transport. Add `maos-persistence` to `maos-bin` under the `network` feature, the same gating as `maos-a2a-tcp`/`maos-cohort`. Other roots stay unchanged.
- [ ] **T5 — route (AC1.e).** Add the `["v1","a2a","pins"]` arm via `get_only`, the row/status types as plain owned scalars, and the D-21-3-06 port (A: a defaulted `peer_pins()` on `OperatorCommandPort`, a `RepinControl` mapping and the `operator_door.rs` delegate). If ADR-062 is updated to list the new read route, update its `decision_adrs_and_provisioning.rs` `"062"` arm (`:483`, `:1122`) in the same commit.
- [ ] **T6 — proofs (AC1.d/f/g).** Follow the focused plan below. Move the `cert_rotation_trigger_14_2a.rs` daemon helpers to `tests/support/` instead of copying them.
- [ ] **T7 — landing, orchestrator-owned.** Run independent difficult-tier review and an eligible security review (D-E20-6 B: hold for the non-GPT reviewer, because this is a new persisted trust input). Run `check-kernel-baseline --json` and `kloc-check --json` with the prebuilt xtask. If the kernel delta is non-zero, re-pin with `scripts/repin.py --apply` and write the HISTORY row in the same commit. The forecast `maos-persistence` ceiling breach (and a possible `maos-a2a-core` breach) needs orchestrator adjudication of a measured raise, following the SCP 2026-11-12 `:571` precedent. Hand the row-12 text to the release owner. Only the orchestrator updates tracker, epic or `RELEASE-HOLDS.md`.

## Proof plan — focused, benign, no execution claimed

Every vector uses synthetic fixtures (test-minted certificates, hand-written JSON records). None reproduces an attack. A refusal test asserts the typed refusal only.

| AC | Proof (new unless marked existing) |
|---|---|
| AC1.a | `crates/maos-persistence/tests/file_pin_store_21_3.rs`: round trip; missing file = empty; refuses unknown field, unknown version, duplicate peer, malformed fingerprint and non-decimal nonce, each typed; mode `0600`; a failed write leaves the prior file valid and no temp behind (mirror `maos-skill/tests/admission_store_test.rs:146`); second opener refused. |
| AC1.b/d | `crates/maos-a2a-core` tests over a recording port: each listed mutation produces exactly one durable snapshot reflecting it; `rotation_next` is absent from every snapshot; reloading restores invalidation and re-arms the request with a fresh TTL; a widening write failure refuses the transition; a narrowing write failure applies the transition and marks the store degraded. **Existing, unedited:** `restart_invalidates_pin_nfr_rel_6.rs` (21 tests), `repin_door_19_7.rs`, `repin_tls_19_7.rs`, `repin_hold_19_7.rs`, `repin_surface_19_7.rs`, the `tofu.rs` unit tests. |
| AC1.c | Boot-rule table test at the seeding seam: each D-21-3-02 branch, including "config omits the peer → dropped" and "the file never adds a fingerprint". |
| AC1.e | `crates/maos-control/tests/pins_surface_21_3.rs` over a real socket (the `repin_surface_19_7.rs` pattern): 200 sorted array with the decimal-string nonce; 404 with no port; 405 on POST; 401 without bearer; 503 degraded; an empty array is distinct from 404. |
| AC1.f/g | `crates/maos-bin/tests/durable_pins_21_3.rs`, a real daemon through the shared 14-2a helpers: signed rotation → SIGTERM → config and manifest carried → restart → `GET /v1/a2a/pins` shows the promoted fingerprint with `generation` ≥ 1; consented re-pin survives restart. One planted red (proof-contract `:23-28`, one plant per AC): delete the file between runs → the row reverts to the seed and the frame is re-detected. One refusal leg: promotion not carried → typed boot refusal. |

Focused set (per `docs/testing/proof-contract.md:35-46`): `cargo nextest run -p maos-persistence -p maos-a2a-core -p maos-a2a -p maos-a2a-tcp -p maos-control -p maos-cohort --locked` plus the `maos-bin` tests named above (`network` feature). No manifest or parser change, so the `:49-55` widening does not apply. Green on the matrix once on the final tree; the planted red runs once on one image and is recorded under `implementation-artifacts/21-3-…-evidence/`. Record every invocation, tree and result. Nothing above was run at creation.

## Kernel and kloc forecast — read-the-files method, no prototype

**Kernel:** **0.** Method: I read every file the tasks touch (the files in the inventory table). None is under `crates/maos-kernel-core/src`. Base pin: `src_lines = 26061` (`xtask/kernel-core-baseline.toml:505`); kernel crate row 20,429 (`xtask/kloc.toml:272`). No alarm or envelope interaction. The landing still measures and re-pins if the measured delta is non-zero.

| Crate | Ceiling (`kloc.toml`) | Base `src` tokei code | Forecast (recommended options) | Basis |
|---|---|---|---|---|
| `maos-persistence` | 101 (`:490`) | 1 | **+170…+320 — exceeds the ceiling** | Schema and record types, strict load validation, atomic write, single-writer guard, typed errors, port impl. |
| `maos-a2a-core` | 5,138 (`:481`) | 5,037 | +70…+150 (headroom 101; may exceed) | Port trait and snapshot type (+25…45), a hook in 7 mutation sites (+20…40), load and re-arm constructor (+15…35), degraded flag, generation (+10…30). |
| `maos-a2a-tcp` | 1,542 (`:486`) | 1,442 | +10…+40 | Optional port through `build_pin_store` and the bind chain, plus the reconcile-rule seam. D-21-3-01 B would be +40…+100. |
| `maos-control` | 1,596 (`:495`) | 1,496 | +35…+65 | Route arm, row and status types, JSON mapping, defaulted port method. D-21-3-06 B would be about +10 more. |
| `maos-bin` | 39,719 (`:529`) | not measured (worktree has 21-9 work in progress) | +70…+160 | Daemon open and wiring, boot refusal text, `RepinControl` mapping and door delegate. |
| `maos-cohort` / `maos-domain` / kernel | — | — | 0 / 0 / 0 | Not touched under the recommended options. |

Ceilings are blocking per crate (`xtask/src/kloc_check.rs:114-132`) and are re-based only at retrospectives (ADR-069 amendment 2026-11-13). The `maos-persistence` breach is therefore a landing-time orchestrator adjudication, not a story-creation grant. Tests and docs are uncharged.

## Re-measured cite corrections — OLD → NEW at `ea8c51b6` (parent-owned)

Only this story file is written. These corrections go to the epic's owner.

| Epic premise (`:112`, `:53`, `:110`, `:119`) | OLD → NEW |
|---|---|
| Trait doc names `maos-persistence` | `tofu.rs:90-93` → `:152-155` (also `:215-216`) |
| Sole impl `InMemoryTofuPinStore` | `:508` → struct `:217`, impl `:647` |
| `verify_pinned_sync` | `tofu.rs:454-457` → `:596-638` |
| `find_active_pin_by_fingerprint` and its caller | `:250` → `:389`; called `transport.rs:1188` → `:1191` (fn `:1186`); also `verifier.rs:178` |
| "9 concrete sites" | `transport.rs:25,106,156,526,810,977,1183,1325,1353` → `:25,106,156,526,813,980,1186,1328,1356`, **plus** `config.rs:11,125-126`, `verifier.rs:29,63,96`, `repin_door.rs:25,46,61`, `maos-cohort/src/rotation.rs:145,330,350,1129`, `state.rs:274` (22 sites in 6 files; premise of D-21-3-01) |
| `build_server_config` / `build_client_config` | `:1322-1334` / `:1350-1356` → `:1325` / `:1353` |
| `try_new` takes `Arc<dyn TofuPinStore>` | `:387` holds |
| "nothing reads the pin store" | Stale: 19-7's `GET /v1/a2a/repins` (`lib.rs:1161`, `:1940`) reads its pending requests. Nothing reads **pins**. |
| `maos-control` links only domain + kernel-core | `lib.rs:32-35` → doc `:32-36`; `Cargo.toml` also lists `ring`, `serde_json` |
| `MAOS_HOME` registered | `env_contract.rs:270` → `crates/maos-bin/src/env_contract.rs:417`; resolver `maos-domain/src/operator_door.rs:65` |
| `restart_invalidates_pin_nfr_rel_6.rs` "6 scenarios" | → 21 tests (6 `scenario_4_*` + 15 `repin_19_7_*`) |
| Proven-red "route has no row; next dial `NotPinned`" | **False at base:** pins are re-seeded from `[[tcp.peer_pins]]` at every boot (`config.rs:125-145`, `transport.rs:375`), so a deleted file yields config-seeded rows. Replaced by AC1.g. |
| "After restart the row shows the promoted fingerprint" | Conditional: `apply_reissue` is in memory only and boot re-reads the on-disk manifest (`state.rs:460-466`), and the 13.6a boot check (`main.rs:13247`) requires config = manifest. See D-21-3-02. |
| RELEASE-HOLDS row 12 | `RELEASE-HOLDS.md:59` / `:86` → `:93`; "AC1 closes row 12" → closes clause (i) only (D-21-3-07) |
| 14-2 clause (c.2) | `RELEASE-HOLDS.md:147` (at `1a57035e`) → `:192`; v1.0.0 boundary 2 at `:113` |
| Kloc asks (`:53`, tokei @9f920180) | `maos-a2a-core` `:319` 4856/4856 → `kloc.toml:481` ceiling 5138, src 5037; `maos-a2a-tcp` `:322` 1439/1500 → `:486` 1542/1442; `maos-persistence` `:326` 1/**2000** → `:490` **101**/1; `maos-control` `:329` 803/1500 → `:495` 1596/1496; `maos-cohort` `:482` 5857/5857 → `:674` 5991/5873 (not touched now); `maos-bin` `:343` 17109 → `:529` 39719 |
| `CohortConvergenceSource` impl location | `cert_rotation.rs` → `:130` (rotation `:84`, self-identity `:174`) |
| Tracker comment (`sprint-status.yaml:415`) | Says "closes RELEASE-HOLDS.md:86 row 12" → `:93`, clause (i) only |

## Cross-story dependencies and release-claim limits

- **Prerequisites already done:** 14-2/14-2a/14-2b/14-2c (rotation planes, read ports) and 19-7 (re-pin consent door). This story reuses their store, door and harness, and does not reopen them.
- **Successor:** `23-7-peer-identity-rotation` takes AC2–AC4, including row 12 clause (iii) and 14-2 clause (c.2). Under the recommended options this story changes no rotation behavior, so it creates no ordering edge to 23-7.
- **`22-1-host-config-and-standing-cast`** takes AC6. This story adds no `MAOS_*` variable (it reuses `MAOS_HOME`).
- **Claims:** after landing, the GA notes may claim that the daemon's TOFU pin state, including a consented re-pin, survives a restart and can be read from the authenticated door. They must not claim observed first contact, listen-side per-peer scoping, own-leaf rotation, or that the durable file is integrity-protected against a local writer with the operator's own file permissions.

## Dev Agent Record

### Agent Model Used

Creation: **anthropic/claude-opus-5-5** (`DraftDurablePins`). No implementation, review or execution is claimed.

### Completion Notes List

- I read `bmad-create-story`, the exemplars `21-9-wasm-host-default-on.md` and `28-3-researcher-distill-prompt.md`, `docs/testing/proof-contract.md`, epic §21-3 and its correct-course note, SCP 2026-11-12 B7c/D-E20-6, ADR-069 amendments, the tracker rows and the archived tracker note. Source was measured read-only at `ea8c51b6`.
- `_bmad/scripts/resolve_customization.py` and `_bmad/bmm/config.yaml` are absent in this checkout, so the skill defaults were used. Key, output and language come from the assignment and the exemplars. No sprint-status, epic, `RELEASE-HOLDS.md` or code edits were made. Nothing was staged or committed.
- No Cargo, build, test, gate, formatter, prototype or remote job was run. `tokei` was run read-only on clean crate `src/` trees for the forecast baseline.
- At creation, status stayed `backlog` pending D-21-3-01…07, and forecasts assumed the recommended options.
- 2026-11-21 UTC preflight: recorded the operator's A/A/A/A/A/A/A rulings for D-21-3-01 through D-21-3-07. AC1, forecasts and backlog status are unchanged. D-E20-6 option B still holds landing for an eligible non-GPT security reviewer because the story adds a new persisted trust input. No implementation or execution is claimed.

### File List

- `_bmad-output/implementation-artifacts/21-3-durable-tofu-and-self-leaf-rotation.md` — new, measured story only.

## Change Log

- 2026-11-20: created at `ea8c51b6` with GA scope AC1 only. Recorded the measured inventory, seven open D-rows, a focused benign proof plan, kernel 0 / kloc forecasts and parent-owned cite corrections. Status `backlog` pending operator rulings. Tracker untouched.
- 2026-11-21 UTC: recorded operator option A for each of D-21-3-01 through D-21-3-07. No change to backlog status, GA scope (AC1 only), acceptance criteria, forecasts or the D-E20-6 review requirement.

## §A6 independent review artifact

Reviewer: static read only; model family self-reported GPT; no tests were run by the reviewer. The non-GPT D-E20-6 option B review is STILL OWED before landing. Dispositions as left by the fixing agent (not compiled or run by that agent):

| # | Sev | Finding | Disposition |
|---|-----|---------|-------------|
| F1 | HIGH | A retirement/invalidation whose pin-file write failed lived only in RAM; a restart with the original config and snapshot revived the retired anchor or an active pin. | Fixed: `PinPersistence::fence`, called from `persist_narrowed`, durably marks the held lock file non-empty; `FilePinStore::open` refuses (`PinFileError::Fenced`) until the operator empties it, and any later successful `store` lifts it. Tests: `a_failed_retirement_write_is_not_revived_by_a_restart`, `a_fence_is_refused_at_open_and_lifted_only_by_a_successful_write`, `a_narrowing_write_failure_fences_the_durable_file_and_a_widening_one_does_not`. Residual: if the fence write itself fails the restart is not protected (logged). |
| F2 | MED | A parent-directory fsync failure after a successful rename was reported as `Ok`. | Fixed: `write_atomic` returns `WriteError::Unsynced`; `store` fences and returns an error naming "published but durability uncertain", so callers neither acknowledge consent nor report healthy pins. Test: `a_directory_sync_failure_is_a_distinct_uncertain_error_and_fences` (unit, injected sync). |
| F3 | MED | The pin data file was checked by path and reopened by path. | Fixed: opened once with `O_NOFOLLOW \| O_NONBLOCK`, regular-file checked and read on the descriptor. Test: `the_data_file_is_opened_nofollow_and_a_fifo_never_blocks`. |
| F4 | MED | The `rotating` consent fence was lost on restart, so the re-armed request became approvable. | Fixed: a request armed during an open rotation window is not recorded (`record_of`), so it is never re-armed. Test: `a_request_armed_during_a_rotation_window_is_not_recorded_for_rearming`. |
| F5 | MED | A consumed-request write failure still reported a decision and a restart re-armed it. | Fixed: `commit_consumed` persists the removal before reporting; on failure the request returns to pending with no verdict and `PersistFailed` reaches the door as `pins_unwritable`. Test: `a_consumed_request_that_cannot_be_written_stays_pending_with_no_verdict`. |
| F6 | LOW | The strict schema accepted positional arrays. | Fixed: `parse` refuses a positional root, row or `invalidated`. Test: `positional_arrays_are_not_schema_1`. |
| F7 | LOW | The deletion witness in `crates/maos-bin/tests/durable_pins_21_3.rs` never dials. | Deferred: recorded only, code unchanged. |
| F8 | LOW | `RELEASE-HOLDS.md` rows 12 and 15 said durable pins were absent and re-pins memory-only. | Fixed: both rows now say successful cohort-daemon writes are durable, and keep the limits on operator-mediated pairing, listen-side scoping, local-writer integrity and crash durability (F2). |
