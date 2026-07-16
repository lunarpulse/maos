# Runbook — J1 Tier-2 Signed Live-Agent Run (Founder's Loop)

> **Purpose.** The operator (human) procedure to close the **Tier-2** gate in
> `release-gate-8-12-tier-2-cli-wrapper.md`: one real codex Developer-Worker,
> delegated through `maos run`, observed and **signed**. This is **T6** of
> `spec-j1-tier-2-live-agent-demonstration.md` — the one task no test can close.
>
> **Audience.** Lunarpulse (runs + signs). Devs also read it: **this runbook IS
> the acceptance target for T1–T5** — every `[T-task adds]` command below must
> become real before the run.
>
> **Command legend:** `[exists]` works today · `[T# adds]` lands with that task.

---

## Preconditions (before you touch a key)

- [ ] **T1–T5 landed and green** on the pre-Epic-13 bridge branch (`cargo test --workspace --locked`, discipline gates, `check-kernel-baseline` @23147). `[T1–T5]`
- [ ] **§A6 seal done** — Murat's live drill (planted codex crash reds the completion leg), Winston kernel-baseline, Amelia compiler/measurement, Vex redaction+egress. *You sign on top of their seal, not instead of it.*
- [ ] You have decided the **c2 task** (below) and an **exact spend ceiling** (c1).

---

## Phase 0 — Operator decisions & keys (do now; no code needed)

**0.1 — Fix the c2 task.** Write the one bounded, non-destructive-to-the-repo, story-sized task. It runs in a disposable demo dir with full CRUD *inside that dir only*.

- Task text: `__________________________________________________`  *(e.g. "scaffold a tiny Rust CLI in ./: add main.rs + a passing test, run it, then delete the scratch file")*
- Spend ceiling (c1): `$______`  · one-shot, metered API key.

**0.2 — Three secrets, three lives (Vex's rule — never mix):**

| Secret | Role | Where it lives |
|---|---|---|
| `CODEX_API_KEY` | codex **Worker** spends it | inherited host-side into sandbox env; denylisted; scrubbed. **NOT `OPENAI_API_KEY`** — `codex exec` ignores that var and reads `CODEX_API_KEY` (codex-rs `login/src/auth/manager.rs:1226` + `exec/src/lib.rs:571`). Set `CODEX_API_KEY="$OPENAI_API_KEY"`. |
| Orchestrator provider key (e.g. `ANTHROPIC_API_KEY`) | the class Spirits' **reasoning** under `--live` | MAOS provider port; host-side |
| Operator **audit key** (Ed25519) | **you sign** with it | host-only; **never enters the sandbox** |

**0.3 — Generate the signer key (once) and publish its fingerprint:**

```
maosctl audit keygen --output ~/.maos/keys/j1-tier2-signer.key   # [exists]
# prints: key written to … (fingerprint: <FPR>)
```

- [ ] Record `<FPR>` somewhere public ("Lunarpulse signed J1 Tier-2 with `<FPR>`"). Verifiers need it. Run `maosctl audit sealed-export --help` and `maosctl audit keygen --help` to confirm exact flag spellings on your build. `[exists]`

---

## Phase 1 — Clean environment

**1.1 — Fresh worktree (preserve the dirty `epic-12` tree):**

```
git worktree add ../maos-j1-live j1-tier2-live-agent-signed-bridge   # [exists]
cd ../maos-j1-live
```

**1.2 — Disposable demo dir, isolated so a delete cannot reach the repo:**

```
DEMO=$(mktemp -d /tmp/maos-j1-demo.XXXXXX)   # outside the repo tree on purpose
```

- The T3 host grant scopes the Worker's `fs.rw` to **exactly `$DEMO`** — CRUD inside is a feature; anything outside fails closed. `[T3]`

**1.3 — Clean-home invariant (the auth.json footgun):**

- [ ] Confirm **no ambient `~/.codex/auth.json`** in the sandbox home — a live ChatGPT session shadows the injected `OPENAI_API_KEY` and leaves an un-scrubbable token. The run must **refuse or wipe** it, never inherit. `[T5 negative test proves it]`
- [ ] `codex --version` present and pinned; record it (it becomes the "live-agent identity" in the capture).

**1.4 — codex sandbox prerequisite (bubblewrap / user namespaces):** `[found 2026-07-15]`

- [ ] codex's OWN sandbox (`--sandbox workspace-write`) uses **bubblewrap**, which needs **unprivileged user namespaces**. On a hardened host / default Docker you'll see `bwrap: No permissions to create a new namespace` and **every codex write fails**. Enable it at the *container/host* (e.g. `sudo sysctl -w kernel.unprivileged_userns_clone=1`, or run the container `--privileged` / with userns allowed), then re-test **1.5**. This is an environment prerequisite, **not** a MAOS setting.
- [ ] **Do NOT** "fix" it with codex `--sandbox danger-full-access` for the *signed* run — that removes codex's FS jail, and MAOS's T3 FS-scope is *declared-not-enforced* at v0.1, so the c2 demo-dir bound would be enforced by **nothing**. The signed run requires codex's workspace-write jail to actually work.

**1.5 — Pin the exact codex invocation that WRITES to `$DEMO`, standalone:** `[found 2026-07-15]`

```
cd "$DEMO" && git init -q .
RUST_LOG=error codex exec --sandbox workspace-write "write hello to ./hello.txt"
#   PASS = hello.txt exists, exit 0, a final message on stdout.
#   If it fails on bwrap → fix 1.4 first. Then transcribe the EXACT flags into
#   spirits/worker/manifest-codex.toml `argv_prefix` (it is TOCTOU-hashed).
```

> **MAOS admission note (fixed 2026-07-15):** real CLIs do NOT implement MAOS's
> `--maos-bridge-probe` output-shape handshake (only the fixture does), so the
> codex worker is admitted by a **liveness probe** (`codex --version` exits 0) +
> the T3 floor; the output shape is verified at *completion* by the codex
> adapter. Your `manifest-codex.toml` needs no probe handler.

> **MAOS stdin note (fixed 2026-07-15, kernel):** `codex exec` reads its prompt
> from argv **and** keeps reading **stdin-until-EOF**. The bridge used to hold
> the worker's stdin open, so codex hung at `Reading additional input from
> stdin…` (`ps`: `Sl+`, 0 % CPU) while the bridge waited for output — a deadlock.
> The kernel now **closes the worker's stdin** for a `Signals`-driven worker
> (the codex/fixture path) so codex gets EOF and runs on its argv prompt. **No
> `< /dev/null` shim needed** on a `maos` built from `epic-13` at `0a03468f` or
> later. If you see the hang, your `maos` binary predates the fix — rebuild
> (`cargo build --workspace`).

---

## Phase 2 — Dry run (fixture mechanics OR a direct codex sanity check)

**Important:** a *subscription* codex run **through the bridge is refused** — `MAOS_LIVE_AGENT=1` + codex triggers the clean-home refusal whenever `~/.codex/auth.json` exists, and it's the API-key path that Tier-2 requires. So the free dry run is one of:

- **Fixture mechanics (no codex, no cost):** run the hermetic worker to watch the topology/bridge/serving-loop. Needs `worker-cli-fixture` on PATH — easiest is to run the freshly-built binary so it's a daemon-sibling:
  ```
  cargo build --workspace
  ./target/debug/maos run spirits/topologies/j1-founder-loop.toml --once
  ```
- **Direct codex sanity check (subscription OK):** just `codex exec …` outside `maos` (Phase 1.5) — confirms codex itself works; nothing signed.

- [ ] Fixture path: Orchestrator loads Architect + Reviewer + the worker; a typed task routes; the worker subprocess is real (`child_pid`); `worker_completion completed=true`; clean drain.

---

## Phase 3 — The signed live run (API key; the real thing)

**Two "live" axes — set both:** `--live` = real provider for the Spirits' *reasoning*; `MAOS_LIVE_AGENT=1` = real *codex subprocess* instead of the fixture.

```
export CODEX_API_KEY="$OPENAI_API_KEY"   # codex worker — metered, capped, revocable (the child INHERITS this; MAOS never reads/holds it).
                                         # MUST be CODEX_API_KEY: `codex exec` IGNORES OPENAI_API_KEY for auth (→ 401 Missing bearer).
export ANTHROPIC_API_KEY=…         # (or your configured provider) — Orchestrator reasoning
export MAOS_LIVE_AGENT=1           # [lands] permit the real agent subprocess (CI never sets this → CI cannot spawn a paid agent)
export MAOS_HOST_GRANTS=~/.maos/host-grants.toml   # [lands] operator grant for codex (see below) — without it, codex fails closed

#   ~/.maos/host-grants.toml must contain:
#     [[grant]]
#     attested_image = "codex"          # the manifest's [cli_wrapper] command
#     signing_key_id = "OpenAI"         # the manifest's [author] name
#     permitted_tier = "T3"
#     permitted_egress_destinations = ["api.openai.com"]
#   AND a codex worker manifest (command="codex", argv_prefix=["exec","--sandbox","workspace-write"])
#   referenced by the topology (swap the fixture worker entry).

#   Clean-home invariant: MAOS REFUSES the live run if ~/.codex/auth.json exists.
#   (CODEX_API_KEY actually takes precedence OVER auth.json in codex, so this is
#   not about shadowing — it is to keep an un-attestable subscription token out of
#   the signed run's sandbox entirely.) Wipe it first.
#   Run maos FROM $DEMO so codex inherits cwd=$DEMO (workspace-write binds writes
#   to cwd) — otherwise codex writes into the launch dir, breaking the c2 bound.

cd "$DEMO" && maos run <abs>/spirits/topologies/j1-founder-loop-codex.toml --live   # codex topology, NOT the fixture one
#   continuous service; use safe shutdown (Ctrl-C, not --once). NOTE: the full
#   halt/resume digest-citation is a DEFERRED seam (FOLLOWUP-J1-RESUME-SEAM) — not
#   a gate for this run; continuous service + safe shutdown ARE verified.
#   PROVEN 2026-07-15: worker_completion completed=true, exit 0, completion_tl_ref set.
```

- [ ] Delegation → codex executes the c2 task in `$DEMO` → completion **parsed by the adapter** (codex: final line on stdout), never inferred from exit code.
- [ ] Digest cites the Worker-produced Transparency Log reference through the distillate chain.
- [ ] (Ideal) a halt/resume: post-resume digest contains the exact pre-halt typed ref; no in-flight delegation preempted.
- [ ] Revoke the OpenAI API key (`CODEX_API_KEY` / `OPENAI_API_KEY`) when done.

---

## Phase 4 — Capture the evidence (non-secret only)

Write the capture doc as a **JSON file** (e.g. `./j1-tier2-capture.json`). Phase 5 journals it as a `run.capture` audit row (`maosctl audit record-capture`), so the sealed-export signature covers it. `record-capture` **validates these fields fail-closed** and **refuses any capture carrying a credential-shaped value** — so use the exact keys below (non-secret only):

```json
{
  "signer": "<your name> (named human signer)",
  "live_agent_identity": "codex <version>",
  "command_metadata": "codex exec <task>; CODEX_API_KEY injected host-side (value redacted)",
  "host_grant_disposition": "exact-match grant admitted (codex @ OpenAI, T3); a mismatch would have refused",
  "audit_refs": ["<audit TL ref>", "<digest TL ref the digest cited>"],
  "egress": "declared-not-enforced",
  "egress_followup": "FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT",
  "redaction_result": "verified",
  "outcome": "<worker completed; no secret persisted; digest cites the worker ref>"
}
```

- [ ] codex identity + version → `live_agent_identity`
- [ ] non-secret command metadata (argv **with the key redacted**) → `command_metadata` *(a pasted `sk-…`/`ghp_…` value is refused, so redact it)*
- [ ] host-grant disposition (exact-match grant admitted) → `host_grant_disposition`
- [ ] audit + digest Transparency Log refs → `audit_refs` (≥1 required)
- [ ] `egress` **must be exactly** `declared-not-enforced` + `egress_followup` ID (enforced egress = Epic-14 v2.0 hardening; claiming "enforced" is refused)
- [ ] `redaction_result` **must be exactly** `verified` (the injected key's value is absent from the TL — MAOS held it, so it can prove this)
- [ ] `outcome` (+ the row's timestamp is stamped automatically). Extra fields you add are preserved verbatim.

---

## Phase 5 — Journal the capture, then sign (sealed-export = the signature)

**5a — journal the capture as an audit row** so the signature covers it `[record-capture — LANDED dev 2026-07-15; host-level path PROVEN 2026-07-16]`:

```
maosctl audit record-capture --capture ./j1-tier2-capture.json     # [exists — J1 Tier-2]
#   NO --spirit: the v0.1 `resolve_spirit_name` (maos-audit) accepts ONLY
#   `hello-spirit`, so `--spirit orchestrator` (or worker/etc.) is REJECTED at
#   v0.1. Omit --spirit → a host-level attestation (pid/boot = 0), which is the
#   correct posture anyway (an operator/host run attestation). Its audit_refs
#   already cite the worker's completion_tl_ref, so the worker linkage is IN the
#   doc. Cover it with `sealed-export --range <window>` (5b), NOT --spirit.
```

This validates the Phase-4 fields, **refuses** a capture that carries a credential or overclaims a control (egress "enforced", redaction not "verified"), and writes a `run.capture` row (host-level). It prints the row's `frame_id` (e.g. `journaled run.capture d301a233…`).

**5b — sign the covered window (time-range, not --spirit):**

```
maosctl audit sealed-export --range 1d \            # [exists — FR44]
  --audit-key ~/.maos/keys/j1-tier2-signer.key \
  --output ./j1-tier2-bundle.json
#   --range covers the whole run window (worker rows + the fresh capture row).
#   Widen (7d/30d) if the run is older. Prints entry count + the pubkey hex.
#   (sealed-export --spirit would ALSO hit the hello-spirit-only limit — use --range.)
```

**5c — verify the signature (the gate close):**

```
maosctl audit verify-bundle ./j1-tier2-bundle.json --pubkey <FPR>
#   <FPR> = the 64-hex pubkey from keygen == the pubkey sealed-export printed.
#   Expect: "audit verify-bundle — OK (<N> entries, seq <n>)".
```

> **How the capture is covered (dev 2026-07-15 — journal-capture LANDED):** `sealed-export`
> writes **ONE self-contained signed JSON bundle** (`--output` is a FILE) — a canonical bundle
> of the covered **audit entries** signed with Ed25519 over `sha256(canonical)`, signature
> embedded. It is **NOT** a `SHA256SUMS` + separate `.sig` file set (that shape is the
> *offline-import* verify path). Because it signs audit **rows**, the capture is covered by
> running **5a first**: `record-capture` journals the capture as a `run.capture` row, so a
> `sealed-export --range <window>` covering the run signs it alongside the worker's
> `CliSubprocessOutput` + `host_grant_disposition` + `worker_completion` rows. (Earlier drafts
> said this wiring was OPEN — it landed 2026-07-15. **PROVEN 2026-07-16:** `record-capture`
> → `run.capture d301a233…`; `sealed-export --range 1d` → 247 entries incl. the capture row +
> worker completion `019f67ef…`, pubkey `61f4f495…`.)

- [ ] `record-capture` accepted the capture and printed a `run.capture` frame_id (a refusal here means the capture overclaims or carries a secret — fix it; the gate stays open).
- [ ] Signed bundle produced: one JSON file with an embedded Ed25519 signature over the audit entries.
- [ ] **Verify it yourself** before recording — the verify path must pass against `<FPR>`. If it doesn't verify, the gate stays open.

---

## Phase 6 — Record & close the gate

Edit `_bmad-output/test-artifacts/release-gate-8-12-tier-2-cli-wrapper.md`:

- [ ] **Named owner:** Myoungki Jung (Lunarpulse)
- [ ] **Signed artifact path:** `./j1-tier2-signed-bundle/`
- [ ] **Date:** `<YYYY-MM-DD>`
- [ ] Check the five Tier-2 boxes **only on observed evidence**; commit the bundle + gate on the bridge branch.
- [ ] Flip `sprint-status.yaml` `j1-tier2-live-agent-signed-bridge: backlog → done`; merge the bridge before `13-1`.

---

## Abort conditions — ANY of these → Tier-2 stays OPEN, do not sign

- A secret value persisted anywhere in the Transparency Log or capture.
- The Worker created/deleted **outside `$DEMO`** (capability scope escape).
- The signed run used **subscription / `~/.codex/auth.json`** auth (redaction unattestable).
- An ambient `auth.json` was inherited into the sandbox.
- A raw process exit was treated as task completion.
- The sealed-export signature does not verify against `<FPR>`.

---

## What a skeptic re-runs to trust your signature

1. Fetch the bundle + your published `<FPR>`.
2. `maosctl` verify (signature → SHA256) → must pass.
3. Read the capture doc: real codex identity, host-managed grant, resolving digest→TL citations, `egress: declared-not-enforced`, redaction verified, named human signer.

---

## 실행 요약 (한국어)

**T6 = 사람(당신)만 닫을 수 있는 게이트.** 테스트 초록불로는 안 닫힘. 순서:

- **0단계 (지금 가능):** c2 작업 문장 + 지출 상한 확정 → 세 개의 비밀 분리(`CODEX_API_KEY`=codex worker / provider key=Orchestrator 추론 / **audit key**=서명, 샌드박스 진입 금지) → `maosctl audit keygen`으로 서명 키 생성 + 지문 공개. (**codex worker는 `CODEX_API_KEY`를 읽음 — `OPENAI_API_KEY`가 아님**; `codex exec`는 후자를 무시 → 401.)
- **1단계:** `main`에서 브릿지 브랜치 worktree(더러운 epic-12 트리 보존) → 일회용 `$DEMO` 디렉터리(repo 밖) → 샌드박스 홈에 `~/.codex/auth.json` 없음 확인 → `codex --version` 기록.
- **2단계 (선택, 무서명):** 구독으로 한 번 구경 — 서명 안 하니 토큰 상관없음.
- **3단계 (서명 실행):** `CODEX_API_KEY="$OPENAI_API_KEY"`+provider key+`MAOS_LIVE_AGENT=1` → `cd "$DEMO" && maos run …j1-founder-loop-codex.toml --live`(codex 토폴로지, 연속, `--once` 아님 → halt/resume 확인; `$DEMO`에서 실행해야 codex cwd=$DEMO). codex가 `$DEMO`에서 작업 → 완료는 **어댑터가 파싱**(종료코드 아님) → 다이제스트가 Worker TL ref 인용. 끝나면 키 폐기. (**2026-07-15 실증: `worker_completion completed=true`, exit 0, `completion_tl_ref` 발급.**)
- **4단계:** 캡처 문서를 **JSON 파일**로 작성(비밀 제외): `signer`, `live_agent_identity`, `command_metadata`(redacted argv), `host_grant_disposition`, `audit_refs`, `egress`=`declared-not-enforced`+`egress_followup`, `redaction_result`=`verified`, `outcome`. (필수 필드 미달·비밀 포함·과잉주장 시 5a에서 거부됨.)
- **5단계 (저널링 → 서명 → 검증):** **5a** `maosctl audit record-capture --capture <file.json>` (**`--spirit` 없이** — v0.1 `resolve_spirit_name`은 `hello-spirit`만 받으므로 `orchestrator`는 거부됨; 생략하면 host-level 증명) → `run.capture` audit row 저널링(서명이 파일이 아니라 audit **행**을 서명하므로 필수) → **5b** `maosctl audit sealed-export --range 1d --audit-key <signer.key> --output <bundle.json>` (**`--spirit` 대신 `--range`**) → **하나의 서명된 JSON 번들**(embedded Ed25519) → **5c** `maosctl audit verify-bundle <bundle.json> --pubkey <FPR>` → `OK (<N> entries)` 통과해야 함. (`SHA256SUMS`+`.sig`가 아님 — 그건 오프라인 임포트 경로. journal-capture 배선 2026-07-15 착륙; host-level+range+verify 경로 2026-07-16 실증: 247 entries.)
- **6단계:** release-gate에 서명자=Myoungki Jung/날짜/번들 경로 기록, 관찰된 증거로만 체크 → 커밋 → 스프린트 라인 done → `13-1` 앞에서 머지.

**중단 조건(하나라도 → 서명 금지, Tier-2 OPEN 유지):** 비밀 잔존 / `$DEMO` 밖 CRUD / 서명 실행에 구독·auth.json 사용 / auth.json 상속 / 종료코드=완료 오인 / 서명 검증 실패.

**오늘 이미 있는 명령:** `maosctl audit keygen`, `maosctl audit sealed-export`, `maos run … --live/--once`. **T1–T5가 추가:** cli_wrapper Worker의 토폴로지 편입 + `WorkerCli` codex 어댑터 + 실제 task 라우팅 + `MAOS_LIVE_AGENT` 게이트 + 캡처 문서 봉인.
