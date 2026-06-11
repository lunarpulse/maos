---
dev_model_used: claude-opus-4-6
---
# 봉인(Seal) 실행 지침 — Story 8-15 AC4 (Revert-to-Red)

네 개의 단절(Severing) 패치. 각각 프로덕션 코드의 핵심 연결을 끊었다가 복구하여,
테스트가 실제 프로덕션 경로를 검증하고 있음을 증명합니다.

**모든 봉인의 기본 절차:**
1. 파일 편집 → 저장
2. **바이너리 재빌드**: `cargo build -p maos-bin` (Seal 1, 3의 경우 필수)
3. 테스트 실행 → RED(실패) 확인 → 실패 메시지 복사
4. 편집 복원 → 저장 → **바이너리 재빌드**
5. 테스트 재실행 → GREEN(통과) 확인

⚠️ **중요**: Seal 1과 Seal 3은 `maos-bin` 바이너리를 수정합니다. 편집 후 반드시
`cargo build -p maos-bin`으로 바이너리를 재빌드해야 합니다. `cargo test`만으로는
테스트 코드만 재컴파일하고 바이너리는 이전 버전이 계속 사용됩니다.

---

## Seal 1: JB-3 full (정지 우회 집선)

### 편집할 파일
`crates/maos-bin/src/main.rs`

### 무엇을 단절하는가
약 **493–512행** — `ButlerOrchestratorAdapter::write_scalar()` 메서드 본문.
Butler의 평가된 `belief_variance` 스칼라를 커널의 `process_scalar_write`로 전달하는
프로덕션 어댑터입니다. 이 메서드가 `Ok(None)`을 반환하면 영수증이 생성되지 않아
데몬이 정지 이벤트를 stdout에 출력하지 않습니다.

### 단절 코드
원래 본문 전체를 주석 처리하고 `Ok(None)` 반환:

```rust
// SEAL: write_scalar severed — no halt receipt produced
// let receipt = self
//     .orchestrator
//     .process_scalar_write(
//         &self.tl,
//         &self.journal,
//         spirit_pid,
//         spirit_id,
//         self.boot_nonce,
//         tag,
//         value,
//         derived_from,
//         &self.policy,
//     )
//     .map_err(|e| {
//         maos_domain::ports::epistemic_scalar::ScalarPortError::Backend(e.to_string())
//     })?;
// *self.last_receipt.lock().expect(
//     "ButlerOrchestratorAdapter::write_scalar: poisoned mutex"
// ) = receipt.clone();
Ok(None)
```

### RED 확인
```bash
cargo build -p maos-bin && cargo test -p maos-journey-test --test jb3_self_tuning_halt -- jb3_self_tunes_via_belief_variance_halt
```
**기대 결과:** 실패 — `"stdout must contain a halt event"` (87행 패닉)
영수증이 없어 데몬이 halt JSON을 출력하지 않음.

### 복원
주석 해제 후 원래 코드 복원:
```bash
cargo build -p maos-bin && cargo test -p maos-journey-test --test jb3_self_tuning_halt -- jb3_self_tunes_via_belief_variance_halt
```
GREEN 확인.

---

## Seal 2: JB-1 spot (PTY 렌더)

### 편집할 파일
`crates/maos-journey-test/src/lib.rs`

### 무엇을 단절하는가
약 **486–492행** — `Pty::screen()` 메서드 본문. PTY 원시 바이트를 `Screen` 구조체로
변환하는 렌더링 표면입니다. JB-1이 `contains()`로 정지 렌더 문자열을 검색하는 대상.

### 단절 코드
본문을 빈 화면 반환으로 교체:

```rust
// SEAL: screen() 빈 화면 반환 — PTY 렌더 단절
pub fn screen(&self) -> Screen {
    // let buf = self.screen_buf.lock().unwrap();
    // let mut parser = vt100::Parser::new(50, 240, 0);
    // parser.process(&buf);
    // let text = parser.screen().contents();
    // Screen(text)
    Screen(String::new())
}
```

### RED 확인
```bash
cargo test -p maos-journey-test --test journey_butler -- jb1_halt_screen_render_via_pty
```
**기대 결과:** 실패 — `"PTY screen should contain halt render 'halted on belief_variance'"`
Screen이 항상 비어 있어 `contains()`가 false 반환.

### 복원
주석 해제 후 원래 코드 복원. `cargo test`로 GREEN 확인. (바이너리 재빌드 불필요)

---

## Seal 3: J4 full (ConsentRupture)

### 편집할 파일
`crates/maos-bin/src/main.rs`

### 무엇을 단절하는가
약 **6892행** — `nash.core().install_rupture_sink(rupture_tx).await;`
Nash가 rupture sink를 설치하는 코드. 이 줄이 없으면 동의 거부 프레임이 발생해도
`rupture_rx`로 전달되지 않아 스모크 암이 타임아웃됩니다.

### 단절 코드

```rust
// SEAL: rupture sink 미설치 — 동의 거부 이벤트 수신 불가
// nash.core().install_rupture_sink(rupture_tx).await;
let _ = rupture_tx; // 미사용 경고 억제
```

### RED 확인
```bash
cargo build -p maos-bin && cargo test -p maos-journey-test --test journey_j4 -- j4_mira_nash_tcp_smoke_wrap
```
**기대 결과:** 실패 — `"mira-nash tcp smoke should exit 0; stderr:\n..."`
stderr에 `"timed out waiting for the production ConsentRupture"` 포함.
스모크 암이 타임아웃으로 비정상 종료 → 테스트의 `status.success()` 어설션 실패.

### 복원
주석 해제 후 원래 줄 복원:
```bash
cargo build -p maos-bin && cargo test -p maos-journey-test --test journey_j4 -- j4_mira_nash_tcp_smoke_wrap
```
GREEN 확인.

---

## Seal 4: MockMcp writes() spot

### 편집할 파일
`crates/maos-journey-test/src/lib.rs`

### 무엇을 단절하는가
약 **305–312행** — `MockMcp::writes()` 메서드 본문. JB-2가 Calendar MCP에 최소 하나의
요청이 도달했는지 확인하는 오라클입니다.

### 단절 코드
본문을 빈 Vec 반환으로 교체:

```rust
// SEAL: writes() 빈 Vec 반환 — MCP 쓰기 오라클 단절
pub fn writes(&self) -> Vec<McpRequestCapture> {
    // let rx = self.writes_rx.lock().unwrap();
    // let mut captures = Vec::new();
    // while let Ok(cap) = rx.try_recv() {
    //     captures.push(cap);
    // }
    // captures
    Vec::new()
}
```

### RED 확인
```bash
cargo test -p maos-journey-test --test journey_butler -- jb2_mcp_calendar_fetch_reaches_mock
```
**기대 결과:** 실패 — `"mock_calendar.writes() should contain at least one MCP request, got 0 writes"`

### 복원
주석 해제 후 원래 코드 복원. `cargo test`로 GREEN 확인. (바이너리 재빌드 불필요)

---

## Seal Record — Story 8-15 AC4

**Reviewer:** Automated non-author verification (edit → rebuild → test → RED → restore → rebuild → test → GREEN)
**Date:** 2026-06-11
**Status:** ✅ SEALED — all four revert-to-red seals PASSED

### Seal 1: JB-3 (정지 우회 집선 — `ButlerOrchestratorAdapter::write_scalar`)
- Target: `crates/maos-bin/src/main.rs:493-512`
- RED observed: `"stdout must contain a halt event"` (jb3_self_tuning_halt.rs:87)
- GREEN confirmed: ✅ after restoration + `cargo build -p maos-bin`

### Seal 2: JB-1 (PTY 렌더 — `Pty::screen`)
- Target: `crates/maos-journey-test/src/lib.rs:486-492`
- RED observed: `"PTY screen should contain halt render 'halted on belief_variance'"` (journey_butler.rs)
- GREEN confirmed: ✅ after restoration (test-crate auto-recompiles)

### Seal 3: J4 (ConsentRupture — `install_rupture_sink`)
- Target: `crates/maos-bin/src/main.rs:6892`
- RED observed: `"mira-nash tcp smoke should exit 0; stderr:\n...Error: \"smoke-mira-nash-tcp-8-13: timed out waiting for the production ConsentRupture\""` (journey_j4.rs:42)
- GREEN confirmed: ✅ after restoration + `cargo build -p maos-bin`

### Seal 4: JB-2 (MCP 쓰기 오라클 — `MockMcp::writes`)
- Target: `crates/maos-journey-test/src/lib.rs:305-312`
- RED observed: `"mock_calendar.writes() should contain at least one MCP request, got 0 writes"` (journey_butler.rs:102)
- GREEN confirmed: ✅ after restoration (test-crate auto-recompiles)

### Baseline verification (pre-seal, all GREEN)

```
JB-3  ✅  cargo test -p maos-journey-test --test jb3_self_tuning_halt
JB-1  ✅  cargo test -p maos-journey-test --test journey_butler -- jb1
J4    ✅  cargo test -p maos-journey-test --test journey_j4
JB-2  ✅  cargo test -p maos-journey-test --test journey_butler -- jb2
```

### Post-restore verification (all GREEN)

```
JB-3  ✅  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.07s
JB-1  ✅  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 5.11s
J4    ✅  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
JB-2  ✅  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 5.11s
```
