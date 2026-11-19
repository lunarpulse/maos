# MAOS DSA 세션 인수인계

## 현재 상태

**기능 검증 완료, 병합 보류.** DSA(Domain-Specific Agent, 도메인 특화 에이전트) 실험은 별도 Git worktree `/mnt/build/wt-maos-dsa`, 브랜치 `dsa-topology`에 있다. 이전 경로 `~/dev_ws-dsa`는 사용하지 않는다. 변경은 미커밋이며 공용 체크아웃이 아니라 이 worktree에서 이어간다. Cargo 빌드 결과도 분리한다: `CARGO_TARGET_DIR=/mnt/build/cargo-maos-dsa`. 커널과 Spirit ABI는 수정하지 않았다.

## 무엇을 만들었나

1. **업무 흐름을 Rust 밖에서 정의한다.** 업무 담당자는 TOML에서 역할과 결과별 이동 경로를 정한다. `maos workflow plan`이 실행 계획과 해시를 만들고, `apply`는 승인한 해시와 현재 카탈로그 권한을 확인한 뒤 활성화한다. `run`은 활성 버전을 실행하며, `eval`은 최종 결과와 실제 단계 경로·단계별 결과를 확인할 수 있다. 최초 실험에서는 인보이스와 도서 반납을 같은 실행기로 검증했다. 새 검사기의 권한을 계획 후 철회하면 활성화가 거부되고 기존 계획이 유지됐다.
2. **한 에이전트가 새 증거를 만들고 다음 에이전트가 실제로 사용한다.** GPT-6.2 Astra의 독립 계획에 따라 `Governed Evidence Handoff`를 구현했다. `extract`는 인보이스 발췌문에서 `purchase_order_present`와 원문 인용 `quote`를 생성한다. `review`는 원래 문서가 아니라 그 결과를 받아 판단한다. 같은 입력과 응답으로 TOML의 경로만 바꿨을 때 `request_order`가 `hold_for_procurement`로 바뀌었다. 주문서가 있는 경우는 두 번째 계획에서도 `ready_for_approval`이며, 실제 승인이나 결제는 일어나지 않는다.
3. **권한과 전달을 검증한다.** 추론은 기존 MAOS 포트에서 Spirit의 PID에 발급한 권한으로 실행한다. 선언되지 않은 결과, 잘못된 JSON·필드 타입, 원문에 없는 인용, 프롬프트 해시 불일치와 와일드카드 해시는 거부한다. 운영자 정책이 추론을 금지하면 추론 호출과 검토 단계 모두 실행되지 않는다. 감사 로그에서 생성 단계의 출력 해시와 검토 단계의 입력 해시가 일치하는 것을 확인했다. 입력·출력 스키마가 같아도 새로 생성한 출력의 실제 내용으로 해시를 계산한다.

## 검증 기록과 한계

```sh
cd /mnt/build/wt-maos-dsa
CARGO_TARGET_DIR=/mnt/build/cargo-maos-dsa cargo test --locked --offline \
  -p maos-topology -p maos-bin --test infer --test workflow_evidence_cli --test workflow_cli
```

**6건 통과:** 기존 CLI 1건, 증거 전달 CLI 3건, 계약 검증 2건. 별도 CLI 실행에서도 `extract → review`, `request_order`, `purchase_order_present: false`, 인용 `No purchase order was supplied.`를 확인했다. [실행 가이드](README.md#run-the-governed-evidence-handoff).

이번 추론은 **정확한 프롬프트 해시에 묶인 합성 응답 재생(strict replay)** 이다. 실제 모델이나 네트워크 제공자를 호출하지 않았다. 인용이 원문에 존재함을 확인할 뿐, 인용이 결론을 뒷받침하는지까지 판정하지 않는다. 따라서 실제 모델의 정확도·비용·운영 품질을 검증한 결과가 아니다. 기존 추론 감사 경로가 프롬프트에 포함된 발췌문을 기록할 수 있으므로 합성 데이터만 사용한다. 상시 실행 데몬의 배포 API, 원격 에이전트, 반복·병렬 합류, 외부 업무 실행은 이번 범위가 아니다.

## 병합 차단: 코드 크기 제한

`kloc-check --json`은 `missing KLOC budget for measured crate 'maos-topology'`로 실패했다. 이후 Tokei로 확인한 전체 패치 규모는 아래와 같다. 현재 세션에서 한도를 올리거나 예외를 추가하지 않았다.

| 항목 | 측정 코드 줄 | 현재 제한 |
|---|---:|---:|
| 전체 | 205,153 | 203,445 이상 실패 |
| `maos-bin` | 39,552 | 최대 38,767 |
| `maos-topology` | 923 | 승인된 예산 없음 |
| `xtask` | 45,463 | 최대 45,462 |
| `maos-kernel-core` | 20,429 | 최대 20,429 (변경 없음) |

## 다시 시작할 때

- **병합이 목표라면 먼저** 코드 정리·통합으로 전체 패치가 제한에 맞는지 측정한다. 별도 예산 승인이 없으면 한도·측정 제외 규칙을 수정하지 말고 실험 상태로 유지한다.
- **다음 기능은 활성화 전 후보 평가다.** 현재 `workflow eval`은 이미 활성화된 계획을 평가한다. 활성 버전을 바꾸지 않고 후보와 기준 계획을 같은 라벨 사례에서 실행하고, 최종 결과가 맞더라도 생성한 증거 필드가 틀리면 실패하도록 한다. 이 기능은 아직 구현하지 않았다.
- 진입점: [호스트 실행·권한·감사](../../crates/maos-bin/src/workflow.rs), [정확한 재생](../../crates/maos-bin/src/cassette_replay.rs), [계약·라우팅 엔진](../../crates/maos-topology/src/), [증거 전달 CLI 검증](../../crates/maos-bin/tests/workflow_evidence_cli.rs), [같은 스키마에서 실제 출력 해시를 검증하는 테스트](../../crates/maos-topology/tests/infer.rs).
